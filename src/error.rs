use crate::error_meta::ErrorDisplay;
use crate::interpreter::error::InterpreterError;
use crate::lexer::locations::Location;
use colored::{ColoredString, Colorize};
use std::fmt::Display;

pub struct CliError(pub String);

impl From<std::io::Error> for CliError {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}

impl From<confy::ConfyError> for CliError {
    fn from(value: confy::ConfyError) -> Self {
        Self(value.to_string())
    }
}

impl<'source> From<InterpreterError<'source>> for CliError {
    fn from(value: InterpreterError) -> Self {
        match value {
            InterpreterError::ParseErrors(errors) => {
                let string: String = errors
                    .iter()
                    .map(|e| ColoredMetaError(e).to_string())
                    .collect();
                CliError(string)
            }
            InterpreterError::Error(e) => CliError(ColoredMetaError(&e).to_string()),
        }
    }
}

#[derive(Debug)]
pub struct ColoredMetaError<'e, EK: Display + std::error::Error>(
    pub &'e crate::error_meta::ContextualError<EK>,
);

impl<'e, EK: Display + std::error::Error> std::error::Error for ColoredMetaError<'e, EK> {}

impl<'e, EK: Display + std::error::Error> std::fmt::Display for ColoredMetaError<'e, EK> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format(f)
    }
}

impl<'e, EK: Display + std::error::Error> crate::error_meta::ErrorDisplay<ColoredString>
    for ColoredMetaError<'e, EK>
{
    fn formatted_error(&self) -> ColoredString {
        self.0.inner_error.to_string().red()
    }

    fn location(&self) -> ColoredString {
        self.0.span.start.to_string().dimmed().bold()
    }

    fn line(&self) -> ColoredString {
        self.0.context.line.bold()
    }

    fn squiggle(&self) -> ColoredString {
        "\u{2248}".repeat(self.0.span.width()).red()
    }

    fn message(&self) -> Option<ColoredString> {
        self.0.message.as_ref().map(|m| m.bright_red())
    }

    fn line_above(&self) -> Option<ColoredString> {
        self.0.line_above().map(|l| l.normal())
    }

    fn line_below(&self) -> Option<ColoredString> {
        self.0.line_below().map(|l| l.normal())
    }

    fn error_start(&self) -> Location {
        self.0.error_start()
    }
}
