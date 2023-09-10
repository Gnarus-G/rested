use super::{
    ast::{self, Program},
    ast_errors::GetErrors,
    error::ParseError,
};
use crate::{
    error_meta::ContextualError,
    lexer::{
        self,
        locations::{GetSpan, Location},
        Token,
    },
    utils::Array,
};

impl<'source> Program<'source> {
    pub fn variables(&self) -> impl Iterator<Item = (lexer::locations::Span, &Token<'source>)> {
        self.items.iter().filter_map(|i| match i {
            ast::Item::Let {
                value: ast::Expression::Error(..),
                ..
            } => None,
            ast::Item::Let {
                identifier: ast::result::ParsedNode::Ok(identifier),
                ..
            } => Some((i.span(), identifier)),
            _ => None,
        })
    }

    pub fn variables_before(&self, location: Location) -> Array<&Token<'source>> {
        self.variables()
            .filter(|(item_span, _)| Into::<Location>::into(item_span.end).is_before(location))
            .map(|(_, token)| token)
            .collect()
    }

    pub fn errors(&self) -> Vec<ContextualError<ParseError<'source>>> {
        let mut errors = vec![];
        for item in &self.items {
            errors.extend(item.errors())
        }

        errors
    }
}
