use crate::lexer::{Location, Token};

#[derive(Debug, PartialEq)]
pub struct AbstractToken<'i> {
    pub text: &'i str,
    pub location: Location,
}

impl<'i> From<Token<'i>> for AbstractToken<'i> {
    fn from(token: Token<'i>) -> Self {
        Self {
            text: token.text,
            location: token.location,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Statement<'i> {
    Request(RequestParams<'i>),
    HeaderStatement {
        name: AbstractToken<'i>,
        value: Expression<'i>,
    },
    BodyStatement {
        value: Expression<'i>,
    },
    ExpressionStatement(Expression<'i>),
}

#[derive(Debug, PartialEq)]
pub enum Expression<'i> {
    Identifier(AbstractToken<'i>),
    StringLiteral(AbstractToken<'i>),
    Call {
        identifier: AbstractToken<'i>,
        arguments: Vec<Expression<'i>>,
    },
}

#[derive(Debug, PartialEq)]
pub struct RequestParams<'i> {
    pub method: RequestMethod,
    pub url: AbstractToken<'i>,
    pub params: Vec<Statement<'i>>,
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
