use serde::Serialize;

use super::Token;

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Default)]
pub struct Position {
    /// Byte position. Zero-based.
    pub value: usize,
    pub line: usize,
    pub col: usize,
}

impl Position {
    pub fn new(line: usize, col: usize, value: usize) -> Self {
        Self { value, line, col }
    }
}

impl From<Position> for Location {
    fn from(value: Position) -> Self {
        Self {
            line: value.line,
            col: value.col,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Default)]
pub struct Location {
    pub line: usize,
    pub col: usize,
}

impl Position {
    pub fn to_end_of(self, span: Span) -> Span {
        Span {
            start: self,
            end: span.end,
        }
    }
}

impl Location {
    pub fn is_before(self, location: Location) -> bool {
        if self.line == location.line {
            return self.col <= location.col;
        }
        self.line < location.line
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn to_end_of(self, other_span: Span) -> Span {
        Span {
            start: self.start,
            end: other_span.end,
        }
    }

    pub fn width(&self) -> usize {
        let left = self.start.col;
        let right = self.end.col;

        if right > left {
            return right - left + 1;
        }

        return left - right + 1;
        // The + 1's are because the col positions are zero-based, but we need the absolute
        // length
    }
}

pub trait GetSpan {
    fn span(&self) -> Span;
}

impl<'source> GetSpan for Token<'source> {
    fn span(&self) -> Span {
        Span {
            start: self.start,
            end: self.end_position(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::{
        locations::{GetSpan, Position, Span},
        Lexer,
    };

    #[test]
    fn it_lexes_tokens_with_correct_span() {
        let s = "get /members";

        let tokens = Lexer::new(s).collect::<Vec<_>>();

        assert_eq!(
            tokens[0].span(),
            Span::new(Position::new(0, 0, 0), Position::new(0, 2, 2))
        );

        assert_eq!(
            tokens[1].span(),
            Span::new(Position::new(0, 4, 4), Position::new(0, 11, 11))
        )
    }
}
