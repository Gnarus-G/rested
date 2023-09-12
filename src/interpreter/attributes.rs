use crate::lexer::locations::{GetSpan, Span};
use crate::lexer::Token;
use crate::parser::ast::Expression;

pub struct Attribute<'source> {
    pub name: &'source str,
    pub span: Span,
    pub params: Vec<Expression<'source>>,
}

impl<'source> Attribute<'source> {
    pub fn first_params(&self) -> Option<&Expression<'source>> {
        self.params.first()
    }
}

pub struct AttributeStore<'source> {
    inner: Vec<Attribute<'source>>,
}

impl<'source> AttributeStore<'source> {
    pub fn new() -> Self {
        Self { inner: vec![] }
    }

    pub fn add(&mut self, id: &Token<'source>, params: Vec<Expression<'source>>) {
        if self.has(id.text) {
            return;
        }

        self.inner.push(Attribute {
            name: id.text,
            span: id.span(),
            params,
        })
    }

    pub fn get(&self, name: &str) -> Option<&Attribute<'source>> {
        self.inner.iter().find(|att| att.name == name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}
