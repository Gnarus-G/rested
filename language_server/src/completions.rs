use lexer::locations::Span;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat, Position};

pub trait ContainsPosition {
    fn contains(&self, position: &Position) -> bool;
}

impl ContainsPosition for Span {
    fn contains(&self, position: &Position) -> bool {
        if self.start.line == self.end.line {
            return (self.start.col..=self.end.col).contains(&(position.character as usize));
        }
        (self.start.line..=self.end.line).contains(&(position.line as usize))
    }
}

pub fn builtin_functions_completions() -> Vec<CompletionItem> {
    return ["env", "read", "escape_new_lines"]
        .map(|keyword| CompletionItem {
            label: format!("{}(..)", keyword),
            kind: Some(CompletionItemKind::FUNCTION),
            insert_text: Some(format!("{}(${{1:argument}})", keyword)),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..CompletionItem::default()
        })
        .to_vec();
}

pub fn http_method_completions() -> Vec<CompletionItem> {
    return ["get", "post", "put", "patch", "delete"]
        .map(|keyword| CompletionItem {
            label: format!("{}", keyword),
            kind: Some(CompletionItemKind::KEYWORD),
            insert_text: Some(format!("{}", keyword)),
            ..CompletionItem::default()
        })
        .to_vec();
}

pub fn header_body_keyword_completions() -> Vec<CompletionItem> {
    return ["header", "body"]
        .map(|kw| kw.to_string())
        .map(|keyword| CompletionItem {
            label: keyword.clone(),
            kind: Some(CompletionItemKind::KEYWORD),
            insert_text: Some(keyword),
            ..CompletionItem::default()
        })
        .to_vec();
}
