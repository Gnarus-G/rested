use super::{
    ast::{Identifier, Program},
    ast_errors::GetErrors,
    error::ParseError,
};
use crate::{
    error_meta::ContextualError,
    lexer::{locations::Location, Array},
};

impl<'source> Program<'source> {
    pub fn variables(&self) -> impl Iterator<Item = &Identifier<'source>> {
        self.items.iter().filter_map(|i| match i {
            super::ast::Item::Let { identifier, .. } => Some(identifier),
            _ => None,
        })
    }

    pub fn variables_before(&self, location: Location) -> Array<&Identifier<'source>> {
        self.variables()
            .filter(|i| Into::<Location>::into(i.span.start).is_before(location))
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
