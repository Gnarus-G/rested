use std::{collections::BTreeMap, fmt::Display};

use serde::Serialize;

use crate::lexer::{
    locations::{Location, Span},
    Array, Token,
};

#[derive(Debug, PartialEq, Serialize)]
pub struct Program<'i> {
    pub items: Array<Item<'i>>,
}

impl<'i> Program<'i> {
    pub fn new(items: Array<Item<'i>>) -> Self {
        Self { items }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Identifier<'i> {
    pub name: &'i str,
    pub span: Span,
}

impl<'i> From<&Token<'i>> for Identifier<'i> {
    fn from(token: &Token<'i>) -> Self {
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

impl<'i> From<&Token<'i>> for Literal<'i> {
    fn from(token: &Token<'i>) -> Self {
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

impl<'i> From<&Token<'i>> for StringLiteral<'i> {
    fn from(token: &Token<'i>) -> Self {
        let value = match (token.text.chars().next(), token.text.chars().last()) {
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
pub struct Block<'source> {
    pub statements: Array<Statement<'source>>,
    pub span: Span,
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
        block: Option<Block<'i>>,
        span: Span,
    },
    Expr(Expression<'i>),
    Attribute {
        location: Location,
        identifier: Identifier<'i>,
        parameters: Array<Expression<'i>>,
    },
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize)]
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

pub type Spanned<T> = (Span, T);

#[derive(Debug, PartialEq, Serialize)]
pub enum Expression<'source> {
    Identifier(Identifier<'source>),
    String(StringLiteral<'source>),
    Bool(Literal<'source>),
    Number(Literal<'source>),
    Call {
        identifier: Identifier<'source>,
        arguments: Array<Expression<'source>>,
    },
    Array(Spanned<Array<Expression<'source>>>),
    Object(Spanned<BTreeMap<&'source str, Expression<'source>>>),
    Null(Span),
    EmptyArray(Span),
    EmptyObject(Span),
    TemplateSringLiteral {
        span: Span,
        parts: Array<Expression<'source>>,
    },
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Endpoint<'i> {
    Url(Literal<'i>),
    Pathname(Literal<'i>),
}
