#[derive(Debug, PartialEq)]
pub enum Statement<'i> {
    Request(RequestParams<'i>),
    HeaderStatement {
        name: &'i str,
        value: Expression<'i>,
    },
}

#[derive(Debug, PartialEq)]
pub enum Expression<'i> {
    Identifier(&'i str),
    StringLiteral(&'i str),
}

#[derive(Debug, PartialEq)]
pub struct RequestParams<'i> {
    pub url: &'i str,
    pub params: Vec<Statement<'i>>,
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
