use crate::ast::{Identifier, Program};
use lexer::locations::Location;

impl<'source> Program<'source> {
    pub fn variables(&self) -> impl Iterator<Item = &Identifier<'source>> {
        self.items.iter().filter_map(|i| match i {
            crate::ast::Item::Let { identifier, .. } => Some(identifier),
            _ => None,
        })
    }

    pub fn variables_before(&self, location: Location) -> Vec<&Identifier<'source>> {
        self.variables()
            .filter(|i| {
                if i.span.start.line == location.line {
                    return i.span.start.col <= location.col;
                }
                return i.span.start.line < location.line;
            })
            .collect()
    }
}
