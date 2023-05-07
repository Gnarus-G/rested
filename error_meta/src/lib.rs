use colored::Colorize;

use lexer::Location;
use std::fmt::Display;

#[derive(Debug)]
pub struct ErrorSourceContext {
    above: Option<String>,
    line: String,
    below: Option<String>,
}

impl ErrorSourceContext {
    pub fn new(location: &Location, code: &str) -> Self {
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
    pub inner_error: EK,
    pub location: Location,
    pub message: Option<String>,
    pub context: ErrorSourceContext,
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

impl<EK: Display + std::error::Error> std::fmt::Display for Error<EK> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = &self.inner_error.to_string().red();
        let location = self.location.to_string().bold();

        let c = &self.context;

        if let Some(line) = &c.above {
            writeln!(f, "{line}")?
        }

        writeln!(f, "{}", c.line.bold())?;

        let indent_to_error_location = " ".repeat(self.location.col);

        let result = match &self.message {
            Some(m) => writeln!(
                f,
                "{}\u{21B3} {} {}\n{}   {}",
                indent_to_error_location,
                location,
                formatted_error,
                " ".repeat(self.location.col + location.len()),
                m.bright_red()
            ),
            None => writeln!(
                f,
                "{}\u{21B3} at {} {}",
                indent_to_error_location, location, formatted_error
            ),
        };

        if let Some(line) = &c.below {
            writeln!(f, "{line}")?;
        };

        result
    }
}
