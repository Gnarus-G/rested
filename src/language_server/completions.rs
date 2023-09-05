use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat, Position};

use crate::lexer::TokenKind::*;

use crate::parser::ast::{Item, Statement};
use crate::{
    lexer::locations::GetSpan,
    parser::{self, ast::Expression},
};

use super::position::ContainsPosition;

pub struct CompletionsStore {
    pub functions: Vec<CompletionItem>,
    pub methods: Vec<CompletionItem>,
    pub header_body: Vec<CompletionItem>,
    pub attributes: Vec<CompletionItem>,
    pub variables: Vec<CompletionItem>,
}

pub trait GetCompletions {
    fn completions(
        &self,
        position: &Position,
        comps: &CompletionsStore,
    ) -> Option<Vec<CompletionItem>>;
}

impl<'source> GetCompletions for Expression<'source> {
    fn completions(
        &self,
        position: &Position,
        comps: &CompletionsStore,
    ) -> Option<Vec<CompletionItem>> {
        if !self.span().contains(position) {
            return None;
        }

        match self {
            Expression::Call { .. }
            | Expression::Array(..)
            | Expression::Object(..)
            | Expression::TemplateSringLiteral { .. } => {
                return Some([comps.variables.clone(), comps.functions.clone()].concat());
            }
            Expression::Error(error) => match &error.inner_error {
                parser::error::ParseError::ExpectedToken { expected, .. } => match expected {
                    Ident | StringLiteral | MultiLineStringLiteral => {
                        return Some([comps.variables.clone(), comps.functions.clone()].concat());
                    }
                    _ => {}
                },
                parser::error::ParseError::ExpectedEitherOfTokens { expected, .. } => {
                    let mut comp = vec![];
                    for token in expected.iter() {
                        match token {
                            Ident | StringLiteral | MultiLineStringLiteral => {
                                comp.extend(
                                    [comps.variables.clone(), comps.functions.clone()].concat(),
                                );
                            }
                            _ => {}
                        }
                    }

                    return Some(comp);
                }
            },
            _ => {}
        };

        return None;
    }
}

impl<'source> GetCompletions for Statement<'source> {
    fn completions(
        &self,
        position: &Position,
        comps: &CompletionsStore,
    ) -> Option<Vec<CompletionItem>> {
        if !self.span().contains(position) {
            return None;
        }

        match self {
            Statement::Header { value, .. } => value.completions(position, comps),
            Statement::Body { value, .. } => value.completions(position, comps),
            _ => None,
        }
    }
}

impl<'source> GetCompletions for Item<'source> {
    fn completions(
        &self,
        position: &Position,
        comps: &CompletionsStore,
    ) -> Option<Vec<CompletionItem>> {
        if !self.span().contains(position) {
            return None;
        }

        match self {
            Item::Set { identifier, value } => {
                if identifier.span.contains(position) {
                    return Some(vec![CompletionItem {
                        label: "BASE_URL".to_string(),
                        kind: Some(CompletionItemKind::CONSTANT),
                        ..CompletionItem::default()
                    }]);
                }

                value.completions(position, comps)
            }
            Item::Let { value, .. } => value.completions(position, comps),
            Item::LineComment(_) => todo!(),
            Item::Request {
                block: Some(block), ..
            } => {
                for st in &block.statements {
                    if let Some(c) = st.completions(position, comps) {
                        return Some(c);
                    }
                }

                return Some(comps.header_body.clone());
            }
            Item::Expr(expr) => expr.completions(position, comps),
            Item::Attribute {
                identifier,
                parameters,
                ..
            } => {
                if identifier.span.contains(position) {
                    return Some(comps.attributes.clone());
                }

                for param in parameters.iter() {
                    if let Some(c) = param.completions(position, comps) {
                        return Some(c);
                    }
                }

                None
            }
            _ => None,
        }
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

pub fn http_method_completions() -> Vec<CompletionItem> {
    ["get", "post", "put", "patch", "delete"]
        .map(|keyword| CompletionItem {
            label: keyword.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            insert_text: Some(keyword.to_string()),
            ..CompletionItem::default()
        })
        .to_vec()
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
