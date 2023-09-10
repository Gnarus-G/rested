use super::{
    ast::{self, Program},
    ast_errors::GetErrors,
    error::ParseError,
};
use crate::{
    error_meta::ContextualError,
    lexer::{
        locations::{GetSpan, Location},
        Token,
    },
    utils::Array,
};

impl<'source> Program<'source> {
    pub fn variables(
        &self,
    ) -> impl Iterator<Item = &ast::result::ParsedNode<'source, Token<'source>>> {
        self.items.iter().filter_map(|i| match i {
            ast::Item::Let {
                value: ast::Expression::Error(..),
                ..
            } => None,
            ast::Item::Let { identifier, .. } => Some(identifier),
            _ => None,
        })
    }

    pub fn variables_before(
        &self,
        location: Location,
    ) -> Array<&ast::result::ParsedNode<'source, Token<'source>>> {
        self.variables()
            .filter(|i| Into::<Location>::into(i.span().start).is_before(location))
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
