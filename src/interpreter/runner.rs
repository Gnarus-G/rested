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

impl<'source> ir::Program<'source> {
    pub fn run_ureq(self, request_names: Option<&[String]>) {
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

    pub fn run(&mut self, request_names: Option<&[String]>) {
        let requests = self
            .program
            .items
            .iter()
            .filter(|r| match (&request_names, &r.name) {
                (None, _) => true,
                (Some(_), None) => false,
                (Some(desired), Some(name)) => desired.contains(name),
            });

        for RequestItem {
            span,
            request,
            dbg,
            log_destination,
            ..
        } in requests
        {
            info!(
                "sending {} request to {}",
                request.method.to_string().yellow().bold(),
                request.url.bold()
            );

            if *dbg {
                info!(" \u{21B3} with request data:");
                eprintln!("{}", indent_lines(&format!("{:#?}", request), 6));

                eprintln!(
                    "{}",
                    indent_lines(
                        &format!(
                            "Body: {}",
                            request.body.clone().unwrap_or("(no body)".to_string())
                        ),
                        6
                    )
                );
            }

            let res = match self.strategy.run_request(request) {
                Ok(res) => res,
                Err(error) => {
                    error!(
                        "{:#}",
                        ColoredMetaError(
                            &error::RunError(error.to_string())
                                .to_contextual_error(*span, self.program.source)
                        )
                    );
                    continue;
                }
            };

            if let Some(log_destination) = log_destination {
                match log_destination {
                    LogDestination::Std => {
                        println!("{}", indent_lines(&res, 4));
                    }
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
        }
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

    pub fn indent_lines(string: &str, indent: u8) -> std::string::String {
        string
            .lines()
            .map(|line| (" ".repeat(indent as usize) + line))
            .collect::<Vec<_>>()
            .join("\n")
    }

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
