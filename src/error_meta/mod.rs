use crate::{
    lexer::locations::{Location, Span},
    utils,
};
use std::{fmt::Display, ops::Deref};

use serde::Serialize;

pub trait ErrorDisplay<D: Display + Deref<Target = str>> {
    fn formatted_error(&self) -> D;
    fn location(&self) -> D;
    fn line(&self) -> D;
    fn line_above(&self) -> Option<D>;
    fn line_below(&self) -> Option<D>;
    fn error_start(&self) -> Location;
    fn squiggle(&self) -> D;
    fn message(&self) -> Option<D>;

    fn format(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = self.formatted_error();
        let location = self.location();

        if let Some(line) = &self.line_above() {
            writeln!(f, "{line}")?
        }

        writeln!(f, "{}", self.line())?;
        let error_start = self.error_start();
        let indent_to_error_location = " ".repeat(error_start.col);

        writeln!(
            f,
            "{}{}\n{}\u{21B3} {} {}",
            indent_to_error_location,
            self.squiggle(),
            indent_to_error_location,
            location,
            formatted_error
        )?;

        if let Some(m) = &self.message() {
            writeln!(
                f,
                "{}   {}",
                " ".repeat(error_start.col + location.len()),
                m
            )?
        };

        if let Some(line) = self.line_below() {
            writeln!(f, "{line}")?;
        };

        Ok(())
    }
}

#[derive(Clone, PartialEq, Serialize)]
pub struct ErrorSourceContext {
    above: Option<utils::String>,
    pub line: utils::String,
    below: Option<utils::String>,
}

impl ErrorSourceContext {
    pub fn new(location: &Location, code: &str) -> Self {
        let line_of_token = location.line;
        let line_before = line_of_token.checked_sub(1);
        let line_after = line_of_token + 1;

        let get_line = |lnum: usize| code.lines().nth(lnum).map(|s| s.to_string());

        ErrorSourceContext {
            above: line_before.map(|lnum| get_line(lnum).expect("code should not be empty").into()),
            line: get_line(line_of_token)
                .expect("code should not be empty")
                .into(),
            below: get_line(line_after).map(|l| l.into()),
        }
    }
}

#[derive(Clone, PartialEq, Serialize)]
pub struct ContextualError<EK: Display + std::error::Error> {
    pub inner_error: EK,
    pub span: Span,
    pub message: Option<utils::String>,
    pub context: ErrorSourceContext,
}

impl<E: Display + std::error::Error + Clone> std::fmt::Debug for ContextualError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl<E: Display + std::error::Error + Clone> ContextualError<E> {
    pub fn new(inner_error: E, span: Span, source_code: &str) -> Self {
        Self {
            inner_error,
            message: None,
            context: ErrorSourceContext::new(&span.end.into(), source_code),
            span,
        }
    }

    pub fn with_message(mut self, msg: &str) -> Self {
        self.message = Some(msg.into());
        self
    }
}

impl<E: Display + std::error::Error + Clone> ErrorDisplay<utils::String> for ContextualError<E> {
    fn formatted_error(&self) -> utils::String {
        self.inner_error.to_string().into()
    }

    fn location(&self) -> utils::String {
        Location::from(self.span.start).to_string().into()
    }

    fn line(&self) -> utils::String {
        self.context.line.clone()
    }

    fn squiggle(&self) -> utils::String {
        "\u{2248}".repeat(self.span.width()).into()
    }

    fn message(&self) -> Option<utils::String> {
        self.message.clone()
    }

    fn line_above(&self) -> Option<utils::String> {
        self.context.above.clone()
    }

    fn line_below(&self) -> Option<utils::String> {
        self.context.below.clone()
    }

    fn error_start(&self) -> Location {
        self.span.start.into()
    }
}

impl<E: Display + std::error::Error + Clone> std::error::Error for ContextualError<E> {}

impl<E: Display + std::error::Error + Clone> std::fmt::Display for ContextualError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format(f)
    }
}
