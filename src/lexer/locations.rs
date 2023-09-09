use serde::Serialize;

use super::Token;

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Default)]
pub struct Position {
    pub value: usize,
    pub line: usize,
    pub col: usize,
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
            return right - left;
        }

        left - right
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

pub trait GetSpanOption {
    fn get_span(&self) -> Option<Span>;
}

impl<T: GetSpan> GetSpanOption for Vec<T> {
    fn get_span(&self) -> Option<Span> {
        match (self.first(), self.last()) {
            (None, None) => None,
            (None, Some(e)) => Some(e.span()),
            (Some(e), None) => Some(e.span()),
            (Some(f), Some(l)) => Some(f.span().to_end_of(l.span())),
        }
    }
}
