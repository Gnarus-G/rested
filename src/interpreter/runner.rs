pub use crate::parser::ast::RequestMethod;

use crate::{
    error::ColoredMetaError,
    error_meta::ToContextualError,
    interpreter::{
        ir::{self, *},
        ureq_runner::UreqRun,
    },
};
use string_utils::*;

use std::error::Error;

use tracing::{error, info};

#[derive(Debug)]
pub enum RunResponse {
    Success(String),
    Failure(String),
}

impl<'source> ir::Program<'source> {
    pub fn run_ureq(
        self,
        request_names: Option<&[String]>,
    ) -> Vec<(request_id::RequestId, RunResponse)> {
        Runner::new(self, Box::new(UreqRun)).run(request_names)
    }
}

use colored::Colorize;
pub trait RunStrategy {
    fn run_request(&mut self, request: &Request) -> std::result::Result<String, Box<dyn Error>>;
}

struct Runner<'source> {
    program: ir::Program<'source>,
    strategy: Box<dyn RunStrategy>,
}

impl<'source> Runner<'source> {
    pub fn new(program: ir::Program<'source>, strategy: Box<dyn RunStrategy>) -> Self {
        Self { program, strategy }
    }

    pub fn run(
        &mut self,
        request_names: Option<&[String]>,
    ) -> Vec<(request_id::RequestId, RunResponse)> {
        let requests = self.program.items.iter().filter(|r| {
            match (&request_names, r.name.as_deref().unwrap_or(&r.request.url)) {
                (None, _) => true,
                (Some(desired), name) => desired.iter().any(|n| n == name),
            }
        });

        let mut responses = Vec::with_capacity(request_names.map(|names| names.len()).unwrap_or(2));

        for item in requests {
            let request_id = request_id::RequestId::from(item);
            let RequestItem {
                span,
                request,
                dbg,
                log_destination,
                ..
            } = item;

            info!(
                "sending {} request to {}",
                request.method.to_string().yellow().bold(),
                request.url.bold()
            );

            if *dbg {
                eprintln!("{}", &format!("{:#?}", request));
            }

            let res = match self.strategy.run_request(request) {
                Ok(res) => res,
                Err(error) => {
                    let err = &error::RunError(error.to_string())
                        .to_contextual_error(*span, self.program.source);
                    let err = ColoredMetaError(err);
                    error!("{err:#}");
                    responses.push((request_id, RunResponse::Failure(format!("{err:#}"))));
                    continue;
                }
            };

            if let Some(log_destination) = log_destination {
                match log_destination {
                    LogDestination::File(file_path) => match log(&res, file_path) {
                        Ok(_) => {
                            info!("{}", format!("saved response to {:?}", file_path).blue());
                        }
                        Err(error) => {
                            error!(
                                "{:#}",
                                ColoredMetaError(
                                    &error::RunError(error.to_string())
                                        .to_contextual_error(*span, self.program.source)
                                )
                            )
                        }
                    },
                }
            }

            println!("{res}");

            responses.push((request_id, RunResponse::Success(res)));
        }

        return responses;
    }
}

mod error {
    use std::error::Error;

    use crate::error_meta::ToContextualError;

    #[derive(Debug, Clone)]
    pub struct RunError(pub String);

    impl Error for RunError {}
    impl std::fmt::Display for RunError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl ToContextualError for RunError {}
}

mod string_utils {
    use std::{
        fs,
        io::{self, Write},
    };

    pub fn log(content: &str, to_file: &std::path::PathBuf) -> std::io::Result<()> {
        if let Some(dir_path) = to_file.parent() {
            fs::create_dir_all(dir_path)?
        };

        let file = fs::File::options()
            .truncate(true)
            .write(true)
            .create(true)
            .open(to_file)?;

        let mut w = io::BufWriter::new(file);

        write!(w, "{content}")
    }
}

pub mod request_id {
    use std::str::FromStr;

    use anyhow::Context;

    use crate::interpreter::ir;

    #[derive(Debug)]
    pub struct RequestId {
        pub method: String,
        pub url_or_name: String,
    }

    impl From<&ir::RequestItem> for RequestId {
        fn from(r: &ir::RequestItem) -> Self {
            let (m, n) = match r.name.clone() {
                Some(name) => (r.request.method.to_string(), name),
                None => (r.request.method.to_string(), r.request.url.clone()),
            };

            return RequestId {
                method: m,
                url_or_name: n,
            };
        }
    }

    impl FromStr for RequestId {
        type Err = anyhow::Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let mut split = s.split("::");

            let m = split
                .next()
                .context("can't get a prompt entry from an empty string")?;
            let n = split
                .next()
                .context("failed to get url or name from string")?;

            return Ok(RequestId {
                method: m.to_owned(),
                url_or_name: n.to_owned(),
            });
        }
    }

    impl RequestId {
        pub fn as_string(&self) -> String {
            return format!("{}::{}", self.method, self.url_or_name);
        }
    }
}
