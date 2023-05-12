use std::fmt::Display;

use serde::Serialize;

use crate::Token;

#[derive(PartialEq, Clone, Copy, Serialize)]
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
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at {}:{}", self.line + 1, self.col + 1)
    }
}

impl std::fmt::Debug for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
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
}

impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "starting {} and ending {}", self.start, self.end)
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
