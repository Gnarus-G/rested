use crate::lexer::{Location, Token, TokenKind};

#[derive(Debug, PartialEq)]
enum ParseErrorKind {
    Unexpected {
        found: TokenKind,
        expected: TokenKind,
    },
}

#[derive(Debug)]
struct ErrorSourceContext {
    above: Option<String>,
    line: String,
    below: Option<String>,
}

#[derive(Debug)]
pub struct ParseErrorConstructor<'i> {
    source_code: &'i str,
}

impl<'i> ParseErrorConstructor<'i> {
    pub fn new(source: &'i str) -> Self {
        Self {
            source_code: source,
        }
    }

    fn get_context_around(&self, token: &Token) -> ErrorSourceContext {
        let line_of_token = token.location.line;
        let line_before = line_of_token.checked_sub(1);
        let line_after = line_of_token + 1;

        let get_line = |lnum: usize| self.source_code.lines().nth(lnum).map(|s| s.to_string());

        ErrorSourceContext {
            above: line_before.map(|lnum| get_line(lnum).expect("code is not empty")),
            line: get_line(line_of_token).expect("code is not empty"),
            below: get_line(line_after),
        }
    }

    pub fn unexpected_token(&self, token: &Token, expected: TokenKind) -> ParseError {
        ParseError {
            kind: ParseErrorKind::Unexpected {
                found: token.kind,
                expected,
            },
            location: token.location,
            message: None,
            context: self.get_context_around(token),
        }
    }
}

#[derive(Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
    location: Location,
    message: Option<String>,
    context: ErrorSourceContext,
}

impl ParseError {
    pub fn with_message(mut self, msg: &String) -> Self {
        self.message = Some(msg.to_owned());
        self
    }
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line + 1, self.col + 1)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = match &self.kind {
            ParseErrorKind::Unexpected { expected, found } => {
                format!("unexpected token: expected {:?}, got {:?}", expected, found)
            }
        };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser};

    use TokenKind::*;

    macro_rules! assert_errs {
        ($input:literal, $kind:expr) => {
            let lexer = Lexer::new($input);
            let mut parser = Parser::new(lexer);
            let error = parser.parse().unwrap_err();

            assert_eq!(error.kind, $kind)
        };
    }

    use ParseErrorKind::*;

    #[test]
    fn unexpected_token() {
        assert_errs!(
            "get {}",
            Unexpected {
                found: LBracket,
                expected: Url,
            }
        );

        assert_errs!(
            "get",
            Unexpected {
                found: End,
                expected: Url,
            }
        );
    }
}
