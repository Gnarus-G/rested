use crate::lexer::locations::{GetSpan, Span};

use super::ast::{
    result::ParsedNode, CallExpr, ConstantDeclaration, Endpoint, Expression, Item, ObjectEntry,
    Request, Statement, StringLiteral, VariableDeclaration,
};

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

impl<'source> GetSpan for ObjectEntry<'source> {
    fn span(&self) -> Span {
        self.key.span().to_end_of(self.value.span())
    }
}

impl<'source> GetSpan for Expression<'source> {
    fn span(&self) -> Span {
        match self {
            Expression::Identifier(i) => i.span(),
            Expression::String(l) => l.span,
            Expression::Call(expr) => expr.span(),
            Expression::TemplateSringLiteral { span, .. } => *span,
            Expression::Array(list) => list.span,
            Expression::Object((span, ..)) => *span,
            Expression::Bool((span, _)) => *span,
            Expression::Number((span, _)) => *span,
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
            Item::Set(ConstantDeclaration { identifier, value }) => {
                identifier.span().to_end_of(value.span())
            }
            Item::Let(decl) => decl.span(),
            Item::LineComment(l) => l.span,
            Item::Request(Request { span, .. }) => *span,
            Item::Attribute {
                location,
                identifier,
                arguments,
            } => arguments
                .as_ref()
                .map(|p| p.span)
                .unwrap_or(Span::new(*location, identifier.span().end)),
            Item::Expr(e) => e.span(),
            Item::Error(e) => e.span,
        }
    }
}

impl<'source> GetSpan for VariableDeclaration<'source> {
    fn span(&self) -> Span {
        let VariableDeclaration { identifier, value } = self;
        identifier.span().to_end_of(value.span())
    }
}

impl<'source> GetSpan for Endpoint<'source> {
    fn span(&self) -> Span {
        match self {
            Endpoint::Url(l) => l.span,
            Endpoint::Pathname(l) => l.span,
            Endpoint::Expr(e) => e.span(),
        }
    }
}

impl<'source> GetSpan for StringLiteral<'source> {
    fn span(&self) -> Span {
        self.span
    }
}

impl<'source, T: GetSpan> GetSpan for ParsedNode<'source, T> {
    fn span(&self) -> Span {
        match self {
            ParsedNode::Ok(ok) => ok.span(),
            ParsedNode::Error(error) => error.span,
        }
    }
}
