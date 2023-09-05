use crate::lexer::locations::Span;
use crate::parser::ast::{Expression, Identifier};

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

    pub fn add(&mut self, id: &Identifier<'source>, params: Vec<Expression<'source>>) {
        if self.has(id.name) {
            return;
        }

        self.inner.push(Attribute {
            name: id.name,
            span: id.span,
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
