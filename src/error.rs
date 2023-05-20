use std::fmt::Display;

use colored::{ColoredString, Colorize};
use error_meta::ErrorDisplay;
use lexer::locations::Location;

#[derive(Debug)]
pub struct ColoredError<EK: Display + std::error::Error>(pub error_meta::Error<EK>);

impl<EK: Display + std::error::Error> std::error::Error for ColoredError<EK> {}

impl<EK: Display + std::error::Error> std::fmt::Display for ColoredError<EK> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format(f)
    }
}

impl<EK: Display + std::error::Error> error_meta::ErrorDisplay<ColoredString> for ColoredError<EK> {
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
