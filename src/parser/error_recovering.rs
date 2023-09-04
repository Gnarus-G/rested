use crate::error_meta::ContextualError;

use super::{ast, error::ParseError};

#[derive(Debug)]
pub struct RecoveredItem<'source> {
    pub item: Option<ast::Item<'source>>,
    pub error: ContextualError<ParseError<'source>>,
}

impl<'source> std::error::Error for RecoveredItem<'source> {}

impl<'source> RecoveredItem<'source> {
    pub fn new(error: ContextualError<ParseError<'source>>, item: ast::Item<'source>) -> Self {
        Self {
            item: Some(item),
            error,
        }
    }
}

impl<'source> From<ContextualError<ParseError<'source>>> for RecoveredItem<'source> {
    fn from(value: ContextualError<ParseError<'source>>) -> Self {
        Self {
            item: None,
            error: value,
        }
    }
}

impl<'source> std::fmt::Display for RecoveredItem<'source> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

#[derive(Debug)]
pub struct RecoveredBlock<'source> {
    pub block: ast::Block<'source>,
    pub error: ContextualError<ParseError<'source>>,
}

impl<'source> std::error::Error for RecoveredBlock<'source> {}

impl<'source> RecoveredBlock<'source> {
    pub fn new(error: ContextualError<ParseError<'source>>, block: ast::Block<'source>) -> Self {
        Self { block, error }
    }
}

impl<'source> std::fmt::Display for RecoveredBlock<'source> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
