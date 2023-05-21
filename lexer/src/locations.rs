use serde::Serialize;

use crate::Token;

#[derive(Debug, PartialEq, Clone, Copy, Serialize)]
pub struct Location {
    pub line: usize,
    pub col: usize,
}

impl Location {
    pub fn to_end_of(self, span: Span) -> Span {
        Span {
            start: self,
            end: span.end,
        }
    }

    pub fn is_before(self, location: Location) -> bool {
        if self.line == location.line {
            return self.col <= location.col;
        }
        return self.line < location.line;
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize)]
pub struct Span {
    pub start: Location,
    pub end: Location,
}

impl Span {
    pub fn new(start: Location, end: Location) -> Self {
        Self { start, end }
    }
    pub fn extend_to(self, end: Location) -> Span {
        Span {
            start: self.start,
            end,
        }
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

        return left - right;
    }
}

impl<'source> Into<Span> for Token<'source> {
    fn into(self) -> Span {
        self.span()
    }
}

impl<'source> Into<Span> for &Token<'source> {
    fn into(self) -> Span {
        self.span()
    }
}

pub trait GetSpan {
    fn span(&self) -> Span;
}

impl<'source> GetSpan for Token<'source> {
    fn span(&self) -> Span {
        Span {
            start: self.start.clone(),
            end: Location {
                line: self.start.line,
                col: self.start.col + self.text.len(),
            },
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
