use std::fmt::Display;

use crate::lexer::{Location, Token};

#[derive(Debug, PartialEq)]
pub struct Identifier<'i> {
    pub name: &'i str,
    pub location: Location,
}

#[derive(Debug, PartialEq)]
pub struct Literal<'i> {
    pub value: &'i str,
    pub location: Location,
}

impl<'i> From<Token<'i>> for Identifier<'i> {
    fn from(token: Token<'i>) -> Self {
        Self {
            name: token.text,
            location: token.location,
        }
    }
}

impl<'i> From<Token<'i>> for Literal<'i> {
    fn from(token: Token<'i>) -> Self {
        Self {
            value: token.text,
            location: token.location,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Item<'i> {
    Set {
        identifier: Identifier<'i>,
        value: Expression<'i>,
    },
    LineComment(Literal<'i>),
    Request {
        method: RequestMethod,
        endpoint: UrlOrPathname<'i>,
        params: Vec<Statement<'i>>,
        location: Location,
    },
    Attribute {
        location: Location,
        identifier: Identifier<'i>,
        parameters: Vec<Expression<'i>>,
    },
}

#[derive(Debug, PartialEq)]
pub enum Statement<'i> {
    Header {
        name: Literal<'i>,
        value: Expression<'i>,
    },
    Body {
        value: Expression<'i>,
        location: Location,
    },
    LineComment(Literal<'i>),
}

#[derive(Debug, PartialEq)]
pub enum Expression<'i> {
    Identifier(Identifier<'i>),
    StringLiteral(Literal<'i>),
    Call {
        identifier: Identifier<'i>,
        arguments: Vec<Expression<'i>>,
    },
}

#[derive(Debug, PartialEq)]
pub enum UrlOrPathname<'i> {
    Url(Literal<'i>),
    Pathname(Literal<'i>),
}

#[derive(Debug, PartialEq)]
pub enum RequestMethod {
    GET,
    POST,
}

impl Display for RequestMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, PartialEq)]
pub struct Program<'i> {
    pub items: Vec<Item<'i>>,
}

impl<'i> Program<'i> {
    pub fn new() -> Self {
        Self { items: vec![] }
    }
}
