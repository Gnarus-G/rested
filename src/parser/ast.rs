use std::{collections::BTreeMap, fmt::Display};

use serde::Serialize;

use crate::{
    error_meta::ContextualError,
    lexer::{
        locations::{GetSpan, Position, Span},
        Token,
    },
};

use super::error::ParseError;

type Error<'source> = ContextualError<ParseError<'source>>;

#[derive(Debug, PartialEq, Serialize)]
pub struct Program<'i> {
    pub items: Vec<Item<'i>>,
}

impl<'i> Program<'i> {
    pub fn new(items: Vec<Item<'i>>) -> Self {
        Self { items }
    }
}

impl<'i> From<&Token<'i>> for MaybeNode<'i, Token<'i>> {
    fn from(token: &Token<'i>) -> Self {
        Self::Node(Token {
            kind: token.kind,
            text: token.text,
            start: token.start,
        })
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
            span: token.span(),
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
            span: token.span(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Block<'source> {
    pub statements: Vec<Statement<'source>>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Item<'source> {
    Set {
        identifier: MaybeNode<'source, Token<'source>>,
        value: Expression<'source>,
    },
    Let {
        identifier: MaybeNode<'source, Token<'source>>,
        value: Expression<'source>,
    },
    LineComment(Literal<'source>),
    Request {
        method: RequestMethod,
        endpoint: Endpoint<'source>,
        block: Option<Block<'source>>,
        span: Span,
    },
    Expr(Expression<'source>),
    Attribute {
        location: Position,
        identifier: MaybeNode<'source, Token<'source>>,
        parameters: Option<Arguments<'source>>,
    },
    Error(Error<'source>),
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
        name: MaybeNode<'i, StringLiteral<'i>>,
        value: Expression<'i>,
    },
    Body {
        value: Expression<'i>,
        start: Position,
    },
    LineComment(Literal<'i>),
    Error(Error<'i>),
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Arguments<'source> {
    pub span: Span,
    pub parameters: Vec<Expression<'source>>,
}

impl<'source> Arguments<'source> {
    pub fn iter(&self) -> impl Iterator<Item = &Expression<'source>> {
        self.parameters.iter()
    }
}

pub type Spanned<T> = (Span, T);

#[derive(Debug, PartialEq, Serialize)]
pub enum Expression<'source> {
    Identifier(MaybeNode<'source, Token<'source>>),
    String(StringLiteral<'source>),
    Bool(Literal<'source>),
    Number(Literal<'source>),
    Call {
        identifier: MaybeNode<'source, Token<'source>>,
        arguments: Vec<Expression<'source>>,
    },
    Array(Spanned<Vec<Expression<'source>>>),
    Object(Spanned<BTreeMap<&'source str, Expression<'source>>>),
    Null(Span),
    EmptyArray(Span),
    EmptyObject(Span),
    TemplateSringLiteral {
        span: Span,
        parts: Vec<Expression<'source>>,
    },
    Error(Error<'source>),
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Endpoint<'i> {
    Url(Literal<'i>),
    Pathname(Literal<'i>),
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub enum MaybeNode<'i, T: GetSpan> {
    Node(T),
    Error(Error<'i>),
}

impl<'source, T: GetSpan> MaybeNode<'source, T> {
    pub fn get(&self) -> std::result::Result<&T, Error<'source>> {
        match self {
            MaybeNode::Node(node) => Ok(node),
            MaybeNode::Error(error) => Err(error.clone()),
        }
    }
}

impl<'source, T: GetSpan> From<std::result::Result<T, Error<'source>>> for MaybeNode<'source, T> {
    fn from(value: std::result::Result<T, Error<'source>>) -> Self {
        match value {
            Ok(value) => MaybeNode::Node(value),
            Err(error) => MaybeNode::Error(error),
        }
    }
}

impl<'source> From<Error<'source>> for Expression<'source> {
    fn from(value: Error<'source>) -> Self {
        Self::Error(value)
    }
}

impl<'source> From<Error<'source>> for Statement<'source> {
    fn from(value: Error<'source>) -> Self {
        Self::Error(value)
    }
}

impl<'source> From<Error<'source>> for Item<'source> {
    fn from(value: Error<'source>) -> Self {
        Self::Error(value)
    }
}
