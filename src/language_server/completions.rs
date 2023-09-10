use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat, Position};

use crate::parser::ast::{self, Item, Statement};
use crate::{lexer::locations::GetSpan, parser::ast::Expression};

use super::position::ContainsPosition;

#[allow(unused_macros)]
macro_rules! dbg_comp {
    ($value:expr) => {
        return Some(vec![CompletionItem {
            label: format!("{:?}", $value),
            kind: Some(CompletionItemKind::CONSTANT),
            ..CompletionItem::default()
        }]);
    };
}

pub struct CompletionsStore {
    pub functions: Vec<CompletionItem>,
    pub items: Vec<CompletionItem>,
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

        return Some([comps.variables.clone(), comps.functions.clone()].concat());
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
            Statement::Header { name, value } => {
                if name.span().is_on_or_after(position) {
                    if matches!(name, ast::result::ParsedNode::Error(..)) {
                        return Some(vec![]);
                    }
                    return None;
                }

                if value.span().is_on_or_after(position) {
                    return Some([comps.variables.clone(), comps.functions.clone()].concat());
                }

                None
            }
            Statement::Body { .. } => {
                Some([comps.variables.clone(), comps.functions.clone()].concat())
            }
            Statement::Error(..) => None,
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
                if identifier.span().is_on_or_after(position) {
                    return Some(vec![CompletionItem {
                        label: "BASE_URL".to_string(),
                        kind: Some(CompletionItemKind::CONSTANT),
                        ..CompletionItem::default()
                    }]);
                }

                if value.span().is_on_or_after(position) {
                    return value.completions(position, comps);
                }

                None
            }
            Item::Let { value, identifier } => {
                if identifier.span().is_on_or_after(position) {
                    return None;
                }

                if value.span().is_on_or_after(position) {
                    return value.completions(position, comps);
                }

                None
            }
            Item::LineComment(_) => None,
            Item::Request {
                block: Some(block), ..
            } => {
                if !block.span.contains(position) {
                    return None;
                }

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
                if identifier.span().is_on_or_after(position) {
                    return Some(comps.attributes.clone());
                }

                if let Some(args) = parameters {
                    if args.span.contains(position) {
                        return Some([comps.variables.clone(), comps.functions.clone()].concat());
                    }

                    for param in args.iter() {
                        if let Some(c) = param.completions(position, comps) {
                            return Some(c);
                        }
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
