use crate::lexer::locations::Span;
use tower_lsp::lsp_types::Position;

pub trait ContainsPosition {
    fn contains(&self, position: &Position) -> bool;
    fn is_after(&self, position: &Position) -> bool;
    fn is_on_or_after(&self, position: &Position) -> bool;
}

impl ContainsPosition for Span {
    fn contains(&self, position: &Position) -> bool {
        if self.start.line == self.end.line && self.start.line == position.line as usize {
            return (self.start.col..=self.end.col).contains(&(position.character as usize));
        }
        (self.start.line..=self.end.line).contains(&(position.line as usize))
    }

    fn is_after(&self, position: &Position) -> bool {
        if self.start.line < position.line as usize {
            return false;
        }

        if self.start.line == position.line as usize {
            return self.start.col > position.character as usize;
        }

        return true;
    }

    fn is_on_or_after(&self, position: &Position) -> bool {
        return self.is_after(position) || self.contains(position);
    }
}

#[cfg(test)]
mod tests {
    use crate::language_server::position::ContainsPosition;
    use crate::lexer::locations::Position;
    use crate::lexer::locations::Span;

    use tower_lsp::lsp_types;

    const SPAN: Span = Span {
        start: Position {
            value: 4,
            line: 0,
            col: 4,
        },
        end: Position {
            value: 9,
            line: 0,
            col: 9,
        },
    };

    const NO_SPAN: Span = Span {
        start: Position {
            value: 13,
            line: 1,
            col: 12,
        },
        end: Position {
            value: 13,
            line: 1,
            col: 12,
        },
    };

    #[test]
    fn test_contains() {
        let position = &tower_lsp::lsp_types::Position {
            line: 0,
            character: 4,
        };

        assert!(SPAN.contains(position));

        let position = &tower_lsp::lsp_types::Position {
            line: 0,
            character: 7,
        };

        assert!(SPAN.contains(position));

        let position = &tower_lsp::lsp_types::Position {
            line: 0,
            character: 10,
        };

        assert!(!SPAN.contains(position));

        let position = &tower_lsp::lsp_types::Position {
            line: 2,
            character: 4,
        };

        assert!(!SPAN.contains(position));

        assert!(NO_SPAN.contains(&tower_lsp::lsp_types::Position {
            line: 1,
            character: 12,
        }))
    }

    #[test]
    fn test_is_after() {
        assert!(SPAN.is_after(&lsp_types::Position {
            line: 0,
            character: 1
        }))
    }
}
