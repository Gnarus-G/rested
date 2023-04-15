use crate::lexer::{Location, Token};

#[derive(Debug, PartialEq)]
pub struct ExactToken<'i> {
    pub value: &'i str,
    pub location: Location,
}

impl<'i> From<Token<'i>> for ExactToken<'i> {
    fn from(token: Token<'i>) -> Self {
        Self {
            value: token.text,
            location: token.location,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Statement<'i> {
    Request {
        params: RequestParams<'i>,
        location: Location,
    },
    HeaderStatement {
        name: ExactToken<'i>,
        value: Expression<'i>,
    },
    BodyStatement {
        value: Expression<'i>,
        location: Location,
    },
    ExpressionStatement {
        exp: Expression<'i>,
        location: Location,
    },
    SetStatement {
        identifier: ExactToken<'i>,
        value: Expression<'i>,
    },
    LineComment(ExactToken<'i>),
}

#[derive(Debug, PartialEq)]
pub enum Expression<'i> {
    Identifier(ExactToken<'i>),
    StringLiteral(ExactToken<'i>),
    Call {
        identifier: ExactToken<'i>,
        arguments: Vec<Expression<'i>>,
    },
}

#[derive(Debug, PartialEq)]
pub struct RequestParams<'i> {
    pub method: RequestMethod,
    pub endpoint: UrlOrPathname<'i>,
    pub params: Vec<Statement<'i>>,
}

#[derive(Debug, PartialEq)]
pub enum UrlOrPathname<'i> {
    Url(ExactToken<'i>),
    Pathname(ExactToken<'i>),
}

#[derive(Debug, PartialEq)]
pub enum RequestMethod {
    GET,
    POST,
}

#[derive(Debug, PartialEq)]
pub struct Program<'i> {
    pub statements: Vec<Statement<'i>>,
}

impl<'i> Program<'i> {
    pub fn new() -> Self {
        Self { statements: vec![] }
    }
}
