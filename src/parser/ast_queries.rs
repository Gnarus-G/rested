use super::{
    ast::{self, result::ParsedNode, Program},
    ast_visit::VisitWith,
    error::{ErrorsCollector, ParseError},
};
use crate::{
    error_meta::ContextualError,
    lexer::{
        self,
        locations::{GetSpan, Location},
        Token,
    },
};

impl<'source> Program<'source> {
    pub fn variables(&self) -> impl Iterator<Item = (lexer::locations::Span, &Token<'source>)> {
        self.items.iter().filter_map(|i| match i {
            ast::Item::Let {
                value: ast::Expression::Error(..),
                ..
            } => None,
            ast::Item::Let {
                identifier: ParsedNode::Ok(identifier),
                ..
            } => Some((i.span(), identifier)),
            _ => None,
        })
    }

    pub fn variables_before(&self, location: Location) -> Box<[&Token<'source>]> {
        self.variables()
            .filter(|(item_span, _)| Into::<Location>::into(item_span.end).is_before(location))
            .map(|(_, token)| token)
            .collect()
    }

    pub fn errors(&self) -> Vec<ContextualError<ParseError<'source>>> {
        let mut errors = ErrorsCollector { list: vec![] };
        for item in self.items.iter() {
            item.visit_with(&mut errors)
        }

        errors.list
    }
}
