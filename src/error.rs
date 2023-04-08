use crate::lexer::{Location, Token, TokenKind};

#[derive(Debug, PartialEq)]
enum ParseErrorKind {
    Unexpected {
        found: TokenKind,
        expected: TokenKind,
    },
}

#[derive(Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
    location: Location,
    message: Option<String>,
}

impl ParseError {
    pub fn unexpected_token(token: &Token, expected: TokenKind) -> Self {
        Self {
            kind: ParseErrorKind::Unexpected {
                found: token.kind,
                expected,
            },
            location: token.location,
            message: None,
        }
    }

    fn with_message(mut self, msg: &String) -> Self {
        self.message = Some(msg.to_owned());
        self
    }
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = match &self.kind {
            ParseErrorKind::Unexpected { expected, found } => {
                format!("unexpected token: expected {:?}, got {:?}", expected, found)
            }
        };

        match &self.message {
            Some(m) => writeln!(f, "{}\n{}", formatted_error, m),
            None => writeln!(f, "{}", formatted_error),
        }
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
