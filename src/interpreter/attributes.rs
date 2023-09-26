use crate::lexer::Token;
use crate::parser::ast::ExpressionList;

pub struct Attribute<'p, 'source> {
    pub identifier: &'p Token<'source>,
    pub params: Option<&'p ExpressionList<'source>>,
}

pub struct AttributeStack<'source, 'p> {
    inner: Vec<Attribute<'source, 'p>>,
}

impl<'source, 'p> AttributeStack<'source, 'p> {
    pub fn new() -> Self {
        Self { inner: vec![] }
    }

    pub fn add(&mut self, id: &'p Token<'source>, params: Option<&'p ExpressionList<'source>>) {
        if self.has(id.text) {
            return;
        }

        self.inner.push(Attribute {
            identifier: id,
            params,
        })
    }

    pub fn get(&self, name: &str) -> Option<&Attribute<'source, 'p>> {
        self.inner.iter().find(|att| att.identifier.text == name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}
