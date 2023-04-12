use crate::lexer::{Location, Token};

#[derive(Debug, PartialEq)]
pub struct TextSlice<'i> {
    pub value: &'i str,
    pub location: Location,
}

impl<'i> From<Token<'i>> for TextSlice<'i> {
    fn from(token: Token<'i>) -> Self {
        Self {
            value: token.text,
            location: token.location,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Statement<'i> {
    Request(RequestParams<'i>),
    HeaderStatement {
        name: TextSlice<'i>,
        value: Expression<'i>,
    },
    BodyStatement {
        value: Expression<'i>,
    },
    ExpressionStatement(Expression<'i>),
    SetStatement {
        identifier: TextSlice<'i>,
        value: Expression<'i>,
    },
}

#[derive(Debug, PartialEq)]
pub enum Expression<'i> {
    Identifier(TextSlice<'i>),
    StringLiteral(TextSlice<'i>),
    Call {
        identifier: TextSlice<'i>,
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
    Url(TextSlice<'i>),
    Pathname(TextSlice<'i>),
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
