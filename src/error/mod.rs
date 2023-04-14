pub mod interpreting;
pub mod parsing;

use crate::lexer::Location;
use std::fmt::Display;

#[derive(Debug)]
struct ErrorSourceContext {
    above: Option<String>,
    line: String,
    below: Option<String>,
}

impl ErrorSourceContext {
    fn new(location: &Location, code: &str) -> Self {
        let line_of_token = location.line;
        let line_before = line_of_token.checked_sub(1);
        let line_after = line_of_token + 1;

        let get_line = |lnum: usize| code.lines().nth(lnum).map(|s| s.to_string());

        ErrorSourceContext {
            above: line_before.map(|lnum| get_line(lnum).expect("code is not empty")),
            line: get_line(line_of_token).expect("code is not empty"),
            below: get_line(line_after),
        }
    }
}

#[derive(Debug)]
pub struct Error<EK: Display + std::error::Error> {
    inner_error: EK,
    location: Location,
    message: Option<String>,
    context: ErrorSourceContext,
}

impl<EK: Display + std::error::Error> Error<EK> {
    pub fn new(inner_error: EK, location: Location, source_code: &str) -> Self {
        Self {
            inner_error,
            location,
            message: None,
            context: ErrorSourceContext::new(&location, source_code),
        }
    }

    pub fn with_message(mut self, msg: &str) -> Self {
        self.message = Some(msg.to_owned());
        self
    }
}

impl<Ek: Display + std::error::Error> std::error::Error for Error<Ek> {}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line + 1, self.col + 1)
    }
}

impl<EK: Display + std::error::Error> std::fmt::Display for Error<EK> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = &self.inner_error;

        let c = &self.context;

        if let Some(line) = &c.above {
            writeln!(f, "{line}")?
        }

        writeln!(f, "{}", c.line)?;

        let indent_to_error_location = " ".repeat(self.location.col);

        let result = match &self.message {
            Some(m) => writeln!(
                f,
                "{}\u{21B3} at {} {}\n{}   {}",
                indent_to_error_location,
                self.location,
                formatted_error,
                indent_to_error_location,
                m
            ),
            None => writeln!(
                f,
                "{}\u{21B3} at {} {}",
                indent_to_error_location, self.location, formatted_error
            ),
        };

        if let Some(line) = &c.below {
            writeln!(f, "{line}")?;
        };

        result
    }
}
