use std::collections::HashSet;

use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat, Position};
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
};

#[derive(Debug)]
pub struct CompletionsCollector {
    pub store: CompletionsStore,
    pub comps: Vec<CompletionItem>,
    pub position: Position,
}

/// For "dynamically" generated completions
#[derive(Debug)]
pub struct CompletionsStore {
    pub variables: Vec<CompletionItem>,
    pub env_args: Vec<CompletionItem>,
}

impl CompletionsCollector {
    fn push_vars_and_functions(&mut self) {
        self.comps.extend(self.store.variables.clone());
        self.comps.extend(builtin_functions_completions());
    }
}

impl<'source> ast_visit::Visitor<'source> for CompletionsCollector {
    fn visit_item(&mut self, item: &ast::Item<'source>) {
        debug!("visited item -> {:?}", item);

        if !item.span().contains(&self.position) {
            return;
        }

        match item {
            Item::Set { identifier, value } => {
                if identifier.span().is_on_or_after(&self.position) {
                    return self.comps.push(CompletionItem {
                        label: "BASE_URL".to_string(),
                        kind: Some(CompletionItemKind::CONSTANT),
                        ..CompletionItem::default()
                    });
                }

                self.visit_expr(value);

                if self.comps.is_empty() {
                    self.push_vars_and_functions();
                }
            }
            Item::Let { value, identifier } => {
                if identifier.span().is_on_or_after(&self.position) {
                    return;
                }

                self.visit_expr(value);

                if self.comps.is_empty() {
                    self.push_vars_and_functions();
                }
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

                if self.comps.is_empty() {
                    return self.comps.extend(header_body_keyword_completions());
                }
            }
            Item::Attribute {
                identifier,
                parameters,
                ..
            } => {
                if identifier.span().is_on_or_after(&self.position) {
                    return self.comps.extend(attributes_completions());
                }

                if let Some(args) = parameters {
                    if args.span.contains(&self.position) {
                        return self.push_vars_and_functions();
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
                    return self.comps.extend(http_headers_completions());
                }

                if value.span().is_after(&self.position) && self.comps.is_empty() {
                    return self.push_vars_and_functions();
                }

                self.visit_expr(value)
            }
            Statement::Body { .. } => {
                if self.comps.is_empty() {
                    self.push_vars_and_functions();
                }
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
                            Some(Expression::String(..)) => {
                                self.comps.extend(self.store.env_args.clone())
                            }
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
                                self.comps.extend(self.store.env_args.clone())
                            }
                            None => self.push_vars_and_functions(),
                            _ => {}
                        }
                    }
                }
                ast::result::ParsedNode::Error(_) => {
                    self.comps.extend(builtin_functions_completions())
                }
                _ => {
                    if self.comps.is_empty() && arguments.span.contains(&self.position) {
                        self.push_vars_and_functions();
                    }
                }
            },
            Expression::Array(_) => {
                if self.comps.is_empty() {
                    self.push_vars_and_functions();
                }
            }
            Expression::EmptyObject(_) => {}
            Expression::String(_) => {}
            _ => {}
        };
    }
}

pub fn builtin_functions_completions() -> Vec<CompletionItem> {
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

pub fn item_keywords() -> Vec<CompletionItem> {
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

pub fn header_body_keyword_completions() -> Vec<CompletionItem> {
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

pub fn attributes_completions() -> Vec<CompletionItem> {
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

pub fn env_args_completions() -> anyhow::Result<Vec<CompletionItem>> {
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

pub fn http_headers_completions() -> Vec<CompletionItem> {
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
