use std::error::Error;

use super::ir::RequestMethod;

use super::ir::{prettify_json_string, Header, Request, Runner};

pub struct UreqRunner;

impl Runner for UreqRunner {
    fn run_request(&mut self, request: &Request) -> std::result::Result<String, Box<dyn Error>> {
        let path = &request.url;

        let mut req = match request.method {
            RequestMethod::GET => ureq::get(path),
            RequestMethod::POST => ureq::post(path),
            RequestMethod::PUT => ureq::put(path),
            RequestMethod::PATCH => ureq::patch(path),
            RequestMethod::DELETE => ureq::delete(path),
        };

        for Header { name, value } in request.headers.iter() {
            req = req.set(name, value);
        }

        let res = if let Some(value) = request.body.clone() {
            let res = req.send_string(&value).map_err(ResponseErrorString::from)?;

            if res.content_type() == "application/json" {
                let string = &res.into_string()?;
                prettify_json_string(string)?
            } else {
                res.into_string()?
            }
        } else {
            req.call()?.into_string()?
        };

        Ok(res)
    }
}

#[derive(Debug)]
pub struct ResponseErrorString(String);

impl std::error::Error for ResponseErrorString {}

impl std::fmt::Display for ResponseErrorString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<ureq::Error> for ResponseErrorString {
    fn from(err: ureq::Error) -> Self {
        let value = match err {
            ureq::Error::Status(status, response) => {
                format!(
                    "{}: status code {}: {} {:#}",
                    response.get_url().to_owned(),
                    status,
                    response.status_text().to_owned(),
                    match response.into_string() {
                        Ok(r) => r,
                        Err(err) => return ResponseErrorString(err.to_string()),
                    }
                )
            }
            ureq::Error::Transport(_) => err.to_string(),
        };

        ResponseErrorString(value)
    }
}
