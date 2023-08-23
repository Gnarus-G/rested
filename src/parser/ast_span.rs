use crate::lexer::locations::{GetSpan, Span};

use super::ast::{Endpoint, Expression, Item, Statement};

impl<'source> GetSpan for Statement<'source> {
    fn span(&self) -> crate::lexer::locations::Span {
        match self {
            Statement::Header { name, value } => name.span.to_end_of(value.span()),
            Statement::Body { value, start } => start.to_end_of(value.span()),
            Statement::LineComment(literal) => literal.span,
        }
    }
}

impl<'source> GetSpan for Expression<'source> {
    fn span(&self) -> Span {
        match self {
            Expression::Identifier(i) => i.span,
            Expression::String(l) => l.span,
            Expression::Call {
                identifier,
                arguments,
            } => arguments
                .last()
                .map(|arg| arg.span())
                .map(|span| identifier.span.to_end_of(span))
                .unwrap_or(identifier.span),
            Expression::TemplateSringLiteral { span, .. } => *span,
            Expression::Array((span, ..)) => *span,
            Expression::Object((span, ..)) => *span,
            Expression::Bool(l) => l.span,
            Expression::Number(l) => l.span,
            Expression::EmptyArray(s) => *s,
            Expression::EmptyObject(s) => *s,
            Expression::Null(s) => *s,
        }
    }
}
impl<'source> GetSpan for Item<'source> {
    fn span(&self) -> Span {
        match self {
            Item::Set { identifier, value } => identifier.span.to_end_of(value.span()),
            Item::Let { identifier, value } => identifier.span.to_end_of(value.span()),
            Item::LineComment(l) => l.span,
            Item::Request { span, .. } => *span,
            Item::Attribute {
                location,
                identifier,
                parameters,
            } => parameters
                .last()
                .map(|p| Span::new(*location, p.span().end))
                .unwrap_or(Span::new(*location, identifier.span.end)),
            Item::Expr(e) => e.span(),
        }
    }
}
impl<'source> GetSpan for Endpoint<'source> {
    fn span(&self) -> Span {
        match self {
            Endpoint::Url(l) => l.span,
            Endpoint::Pathname(l) => l.span,
        }
    }
}
