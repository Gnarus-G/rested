use std::error::Error;

pub use parser::ast::RequestMethod;

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
    pub headers: Vec<Header>,
    pub body: Option<String>,
}

pub trait Runner {
    fn run_request(&mut self, request: Request) -> std::result::Result<String, Box<dyn Error>>;
}

pub fn prettify_json_string(string: &str) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&serde_json::from_str::<serde_json::Value>(string)?)
}
