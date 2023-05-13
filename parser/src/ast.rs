use std::fmt::Display;

use serde::Serialize;

use lexer::{
    locations::{GetSpan, Location, Span},
    Token,
};

#[derive(Debug, PartialEq, Serialize)]
pub struct Program<'i> {
    pub items: Vec<Item<'i>>,
}

impl<'i> Program<'i> {
    pub fn new() -> Self {
        Self { items: vec![] }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Identifier<'i> {
    pub name: &'i str,
    pub span: Span,
}

impl<'i> From<Token<'i>> for Identifier<'i> {
    fn from(token: Token<'i>) -> Self {
        Self {
            name: token.text,
            span: token.into(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Literal<'i> {
    pub value: &'i str,
    pub span: Span,
}

impl<'i> From<Token<'i>> for Literal<'i> {
    fn from(token: Token<'i>) -> Self {
        Self {
            value: token.text,
            span: token.into(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct StringLiteral<'source> {
    pub raw: &'source str,
    pub value: &'source str,
    pub span: Span,
}

impl<'i> From<Token<'i>> for StringLiteral<'i> {
    fn from(token: Token<'i>) -> Self {
        let value = match (token.text.chars().nth(0), token.text.chars().last()) {
            (Some('"'), Some('"')) if token.text.len() > 1 => &token.text[1..token.text.len() - 1],
            (Some('`'), Some('`')) if token.text.len() > 1 => &token.text[1..token.text.len() - 1],
            (_, Some('`')) => &token.text[..token.text.len() - 1],
            (Some('`'), _) => &token.text[1..],
            _ => token.text,
        };

        Self {
            raw: token.text,
            value,
            span: token.into(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Item<'i> {
    Set {
        identifier: Identifier<'i>,
        value: Expression<'i>,
    },
    Let {
        identifier: Identifier<'i>,
        value: Expression<'i>,
    },
    LineComment(Literal<'i>),
    Request {
        method: RequestMethod,
        endpoint: Endpoint<'i>,
        params: Vec<Statement<'i>>,
        span: Span,
    },
    Attribute {
        location: Location,
        identifier: Identifier<'i>,
        parameters: Vec<Expression<'i>>,
    },
}

#[derive(Debug, PartialEq, Serialize)]
pub enum RequestMethod {
    GET,
    POST,
    DELETE,
    PATCH,
    PUT,
}

impl Display for RequestMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Statement<'i> {
    Header {
        name: StringLiteral<'i>,
        value: Expression<'i>,
    },
    Body {
        value: Expression<'i>,
        start: Location,
    },
    LineComment(Literal<'i>),
}

impl<'source> GetSpan for Statement<'source> {
    fn span(&self) -> lexer::locations::Span {
        match self {
            Statement::Header { name, value } => name.span.to_end_of(value.span()),
            Statement::Body { value, start } => start.to_end_of(value.span()),
            Statement::LineComment(literal) => literal.span,
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Expression<'i> {
    Identifier(Identifier<'i>),
    String(StringLiteral<'i>),
    Call {
        identifier: Identifier<'i>,
        arguments: Vec<Expression<'i>>,
    },
    TemplateSringLiteral {
        span: Span,
        parts: Vec<Expression<'i>>,
    },
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
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Endpoint<'i> {
    Url(Literal<'i>),
    Pathname(Literal<'i>),
}

impl<'source> GetSpan for Endpoint<'source> {
    fn span(&self) -> Span {
        match self {
            Endpoint::Url(l) => l.span,
            Endpoint::Pathname(l) => l.span,
        }
    }
}
