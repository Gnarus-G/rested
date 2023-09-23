use std::collections::HashSet;

use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, InsertTextFormat, Position,
};
use tracing::debug;

use crate::{
    config::env_file_path,
    interpreter::environment::Environment,
    language_server::position::ContainsPosition,
    lexer::{self, locations::GetSpan},
    parser::{
        ast::{self, Expression, Item, Statement},
        ast_visit::{self, VisitWith},
        error::ParseError,
    },
    utils,
};

#[derive(Debug, PartialEq)]
pub enum SuggestionKind {
    Nothing,
    Identifiers,
    SetIdentifiers,
    Functions,
    StatementKeywords,
    ItemKeywords,
    Attributes,
    EnvVars,
    Headers,
}

impl From<SuggestionKind> for Vec<CompletionItem> {
    fn from(value: SuggestionKind) -> Self {
        (&value).into()
    }
}

impl From<&SuggestionKind> for Vec<CompletionItem> {
    fn from(value: &SuggestionKind) -> Self {
        match value {
            SuggestionKind::Nothing => vec![],
            SuggestionKind::Identifiers => builtin_functions_completions(),
            SuggestionKind::Functions => builtin_functions_completions(),
            SuggestionKind::StatementKeywords => header_body_keyword_completions(),
            SuggestionKind::ItemKeywords => item_keywords(),
            SuggestionKind::EnvVars => env_args_completions().unwrap_or(vec![]),
            SuggestionKind::SetIdentifiers => {
                vec![CompletionItem {
                    label: "BASE_URL".to_string(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    ..CompletionItem::default()
                }]
            }
            SuggestionKind::Attributes => attributes_completions(),
            SuggestionKind::Headers => http_headers_completions(),
        }
    }
}

#[derive(Debug)]
/// For collecting and deduping, different types of susgesstions and resolving
/// them into completion items.
struct Suggestions<'source> {
    list: Vec<SuggestionKind>,
    variables: utils::Array<lexer::Token<'source>>,
}

impl<'source> Suggestions<'source> {
    fn push(&mut self, kind: SuggestionKind) {
        if !self.list.contains(&kind) {
            self.list.push(kind)
        }
    }

    fn first(&self) -> Option<Vec<CompletionItem>> {
        let kind = self.list.first();
        debug!("resolving first suggestion given: {:?}", kind);
        return kind.map(|k| self.comps_from_kind(k));
    }

    fn comps_from_kind(&self, kind: &SuggestionKind) -> Vec<CompletionItem> {
        let mut comps: Vec<_> = kind.into();
        if let SuggestionKind::Identifiers = kind {
            debug!("adding variables to {:?}", kind);
            comps.extend(self.variables.iter().map(|var| CompletionItem {
                label: var.text.to_string(),
                kind: Some(CompletionItemKind::VARIABLE),
                insert_text: Some(var.text.to_string()),
                ..CompletionItem::default()
            }));
        }
        comps
    }
}

#[derive(Debug)]
pub struct CompletionsCollector<'source> {
    suggestions: Suggestions<'source>,
    position: Position,
}

impl<'source> CompletionsCollector<'source> {
    pub fn new(program: &ast::Program<'source>, position: Position) -> Self {
        CompletionsCollector {
            suggestions: Suggestions {
                list: vec![],
                variables: program
                    .variables_before(lexer::locations::Location {
                        line: position.line as usize,
                        col: position.character as usize,
                    })
                    .iter()
                    // This clone is avoidable, but I don't want to add more lifetimes params to
                    // Suggestions struct and this struct
                    .map(|t| (*t).clone())
                    .collect(),
            },
            position,
        }
    }

    pub fn suggest(&mut self, kind: SuggestionKind) {
        self.suggestions.push(kind);
    }

    pub fn into_response(self) -> Option<CompletionResponse> {
        // We get the first suggestion here because we traversed depth first in
        // the visitor. The deepest node that suggested something had to have contained
        // the cursor position
        return self.suggestions.first().map(CompletionResponse::Array);
    }
}

impl<'source> ast_visit::Visitor<'source> for CompletionsCollector<'source> {
    fn visit_item(&mut self, item: &ast::Item<'source>) {
        debug!("visited item -> {:?}", item);

        if !item.span().contains(&self.position) {
            return;
        }

        match item {
            Item::Set { identifier, value } => {
                if identifier.span().is_on_or_after(&self.position) {
                    return self.suggest(SuggestionKind::SetIdentifiers);
                }

                self.visit_expr(value);

                self.suggest(SuggestionKind::Identifiers);
            }
            Item::Let { value, identifier } => {
                if identifier.span().is_on_or_after(&self.position) {
                    return;
                }

                self.visit_expr(value);

                self.suggest(SuggestionKind::Identifiers);
            }
            Item::Request {
                block: Some(block), ..
            } => {
                if !block.span.contains(&self.position) {
                    return;
                }

                for st in &block.statements {
                    self.visit_statement(st);
                }

                return self.suggest(SuggestionKind::StatementKeywords);
            }
            Item::Attribute {
                identifier,
                parameters,
                ..
            } => {
                if identifier.span().is_on_or_after(&self.position) {
                    return self.suggest(SuggestionKind::Attributes);
                }

                if let Some(args) = parameters {
                    if args.span.contains(&self.position) {
                        return self.suggest(SuggestionKind::Identifiers);
                    }

                    for param in args.iter() {
                        self.visit_expr(param)
                    }
                }
            }
            _ => {}
        }
    }

    fn visit_statement(&mut self, statement: &ast::Statement<'source>) {
        debug!("visited statement -> {:?}", statement);

        if !statement.span().contains(&self.position) {
            return;
        }

        statement.visit_children_with(self);

        match statement {
            Statement::Header { name, value } => {
                if name.span().is_on_or_after(&self.position) {
                    return self.suggest(SuggestionKind::Headers);
                }

                if value.span().is_after(&self.position) {
                    return self.suggest(SuggestionKind::Identifiers);
                }

                self.visit_expr(value)
            }
            Statement::Body { .. } => {
                self.suggest(SuggestionKind::Identifiers);
            }
            _ => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expression<'source>) {
        debug!("visited expression -> {:?}", expr);

        if !expr.span().contains(&self.position) {
            return;
        }

        expr.visit_children_with(self);

        return match expr {
            Expression::Call {
                identifier,
                arguments,
            } => match identifier {
                ast::result::ParsedNode::Ok(lexer::Token {
                    kind: lexer::TokenKind::Ident,
                    text: "env",
                    ..
                }) => {
                    if arguments.span.contains(&self.position) {
                        match arguments
                            .parameters
                            .iter()
                            .find(|p| p.span().contains(&self.position))
                        {
                            Some(Expression::String(..)) => self.suggest(SuggestionKind::EnvVars),
                            Some(Expression::Error(err))
                                if matches!(
                                    err.inner_error,
                                    ParseError::ExpectedEitherOfTokens {
                                        found: lexer::Token {
                                            kind: lexer::TokenKind::UnfinishedStringLiteral,
                                            ..
                                        },
                                        ..
                                    }
                                ) =>
                            {
                                self.suggest(SuggestionKind::EnvVars)
                            }
                            None => self.suggest(SuggestionKind::Identifiers),
                            _ => {}
                        }
                    }
                }
                ast::result::ParsedNode::Error(_) => self.suggest(SuggestionKind::Functions),
                _ => {
                    if arguments.span.contains(&self.position) {
                        self.suggest(SuggestionKind::Identifiers);
                    }
                }
            },
            Expression::Array(_) | Expression::EmptyArray(_) => {
                self.suggest(SuggestionKind::Identifiers);
            }
            Expression::EmptyObject(_) => self.suggest(SuggestionKind::Nothing),
            Expression::Object((_, entries)) => {
                for entry in entries {
                    if let Expression::Error(_) = entry.value {
                        self.suggest(SuggestionKind::Identifiers)
                    } else {
                        self.visit_expr(&entry.value)
                    }
                }
                self.suggest(SuggestionKind::Nothing)
            }
            Expression::Identifier(_) => self.suggest(SuggestionKind::Identifiers),
            _ => {}
        };
    }
}

fn builtin_functions_completions() -> Vec<CompletionItem> {
    ["env", "read", "escape_new_lines"]
        .map(|keyword| CompletionItem {
            label: format!("{}(..)", keyword),
            kind: Some(CompletionItemKind::FUNCTION),
            insert_text: Some(format!("{}(${{1:argument}})", keyword)),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..CompletionItem::default()
        })
        .to_vec()
}

fn item_keywords() -> Vec<CompletionItem> {
    let methods = vec!["get", "post", "put", "patch", "delete"];

    [vec!["let", "set"], methods]
        .concat()
        .iter()
        .map(|keyword| CompletionItem {
            label: keyword.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            insert_text: Some(keyword.to_string()),
            ..CompletionItem::default()
        })
        .collect()
}

fn header_body_keyword_completions() -> Vec<CompletionItem> {
    ["header", "body"]
        .map(|kw| kw.to_string())
        .map(|keyword| CompletionItem {
            label: keyword.clone(),
            kind: Some(CompletionItemKind::KEYWORD),
            insert_text: Some(keyword),
            ..CompletionItem::default()
        })
        .to_vec()
}

fn attributes_completions() -> Vec<CompletionItem> {
    let mut comp = ["log", "name"]
        .map(|keyword| CompletionItem {
            label: format!("{}(..)", keyword),
            kind: Some(CompletionItemKind::FUNCTION),
            insert_text: Some(format!("{}(${{1:argument}})", keyword)),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..CompletionItem::default()
        })
        .to_vec();

    comp.extend_from_slice(
        &["log", "dbg", "skip"]
            .map(|kw| kw.to_string())
            .map(|keyword| CompletionItem {
                label: keyword.clone(),
                kind: Some(CompletionItemKind::KEYWORD),
                insert_text: Some(keyword),
                ..CompletionItem::default()
            }),
    );

    comp
}

fn env_args_completions() -> anyhow::Result<Vec<CompletionItem>> {
    let env = Environment::new(env_file_path()?)?;
    let env_args = env
        .namespaced_variables
        .values()
        .flat_map(|map| map.keys())
        .collect::<HashSet<_>>()
        .into_iter()
        .map(|var| CompletionItem {
            label: var.to_string(),
            kind: Some(CompletionItemKind::CONSTANT),
            insert_text: Some(var.to_string()),
            ..CompletionItem::default()
        })
        .collect::<Vec<_>>();

    Ok(env_args)
}

fn http_headers_completions() -> Vec<CompletionItem> {
    let headers = [
        "Accept",
        "Accept-Charset",
        "Accept-Encoding",
        "Accept-Language",
        "Authorization",
        "Cache-Control",
        "Connection",
        "Content-Disposition",
        "Content-Encoding",
        "Content-Length",
        "Content-Type",
        "Cookie",
        "Date",
        "ETag",
        "Host",
        "If-Match",
        "If-Modified-Since",
        "If-None-Match",
        "If-Range",
        "If-Unmodified-Since",
        "Last-Modified",
        "Location",
        "Origin",
        "Referer",
        "Server",
        "User-Agent",
        "WWW-Authenticate",
        "X-Forwarded-For",
    ];

    headers
        .map(|header| CompletionItem {
            label: header.to_string(),
            kind: Some(CompletionItemKind::CONSTANT),
            insert_text: Some(header.to_string()),
            ..CompletionItem::default()
        })
        .to_vec()
}
