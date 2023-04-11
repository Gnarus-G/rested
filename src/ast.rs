#[derive(Debug, PartialEq)]
pub enum Statement<'i> {
    Request(RequestParams<'i>),
    HeaderStatement {
        name: &'i str,
        value: Expression<'i>,
    },
    BodyStatement {
        value: Expression<'i>,
    },
    ExpressionStatement(Expression<'i>),
}

#[derive(Debug, PartialEq)]
pub enum Expression<'i> {
    Identifier(&'i str),
    StringLiteral(&'i str),
    Call {
        identifier: &'i str,
        arguments: Vec<Expression<'i>>,
    },
}

#[derive(Debug, PartialEq)]
pub struct RequestParams<'i> {
    pub method: RequestMethod,
    pub url: &'i str,
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
