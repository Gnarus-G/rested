use std::error::Error;

use crate::interpreter::ir::{Request, Runner};

pub struct NoopRunner;

impl Runner for NoopRunner {
    fn run_request(&mut self, _request: &Request) -> std::result::Result<String, Box<dyn Error>> {
        Ok("".to_string())
    }
}
