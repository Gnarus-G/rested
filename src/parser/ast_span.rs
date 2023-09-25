use crate::lexer::locations::{GetSpan, Span};

use super::ast::{self, CallExpr, Endpoint, Expression, Item, Statement, StringLiteral};

impl<'source> GetSpan for Statement<'source> {
    fn span(&self) -> crate::lexer::locations::Span {
        match self {
            Statement::Header { name, value } => name.span().to_end_of(value.span()),
            Statement::Body { value, start } => start.to_end_of(value.span()),
            Statement::LineComment(literal) => literal.span,
            Statement::Error(e) => e.span,
        }
    }
}

impl<'source> GetSpan for CallExpr<'source> {
    fn span(&self) -> crate::lexer::locations::Span {
        let CallExpr {
            identifier,
            arguments,
        } = self;
        identifier.span().to_end_of(arguments.span)
    }
}

impl<'source> GetSpan for Expression<'source> {
    fn span(&self) -> Span {
        match self {
            Expression::Identifier(i) => i.span(),
            Expression::String(l) => l.span,
            Expression::Call(expr) => expr.span(),
            Expression::TemplateSringLiteral { span, .. } => *span,
            Expression::Array((span, ..)) => *span,
            Expression::Object((span, ..)) => *span,
            Expression::Bool(l) => l.span,
            Expression::Number(l) => l.span,
            Expression::EmptyArray(s) => *s,
            Expression::EmptyObject(s) => *s,
            Expression::Null(s) => *s,
            Expression::Error(e) => e.span,
        }
    }
}
impl<'source> GetSpan for Item<'source> {
    fn span(&self) -> Span {
        match self {
            Item::Set { identifier, value } => identifier.span().to_end_of(value.span()),
            Item::Let { identifier, value } => identifier.span().to_end_of(value.span()),
            Item::LineComment(l) => l.span,
            Item::Request { span, .. } => *span,
            Item::Attribute {
                location,
                identifier,
                parameters,
            } => parameters
                .as_ref()
                .map(|p| p.span)
                .unwrap_or(Span::new(*location, identifier.span().end)),
            Item::Expr(e) => e.span(),
            Item::Error(e) => e.span,
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

impl<'source> GetSpan for StringLiteral<'source> {
    fn span(&self) -> Span {
        self.span
    }
}

impl<'source, T: GetSpan> GetSpan for ast::result::ParsedNode<'source, T> {
    fn span(&self) -> Span {
        match self {
            ast::result::ParsedNode::Ok(ok) => ok.span(),
            ast::result::ParsedNode::Error(error) => error.span,
        }
    }
}
