use crate::lexer::locations::Span;
pub use crate::parser::ast::RequestMethod;

#[derive(Debug)]
pub struct Program<'source> {
    pub source: &'source str,
    pub items: Box<[RequestItem]>,
}

impl<'source> Program<'source> {
    pub fn new(source: &'source str, items: Box<[RequestItem]>) -> Self {
        Self { source, items }
    }
}

#[derive(Debug)]
pub struct RequestItem {
    pub name: Option<String>,
    pub dbg: bool,
    pub span: Span,
    pub request: Request,
    pub log_destination: Option<LogDestination>,
}

#[derive(Debug)]
pub enum LogDestination {
    File(std::path::PathBuf),
}

#[derive(Debug)]
pub struct Header {
    pub name: String,
    pub value: String,
}

impl Header {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }
}

#[derive(Debug)]
pub struct Request {
    pub method: RequestMethod,
    pub url: String,
    pub headers: Box<[Header]>,
    pub body: Option<String>,
}
