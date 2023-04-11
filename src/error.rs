use crate::lexer::{Location, Token, TokenKind};

#[derive(Debug, PartialEq)]
enum ParseErrorKind {
    ExpectedToken {
        found: TokenKind,
        expected: TokenKind,
    },
    ExpectedEitherOfTokens {
        found: TokenKind,
        expected: Vec<TokenKind>,
    },
    UnexpectedToken {
        kind: TokenKind,
        text: String,
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

    pub fn expected_token(&self, token: &Token, expected: TokenKind) -> ParseError {
        ParseError {
            kind: ParseErrorKind::ExpectedToken {
                found: token.kind,
                expected,
            },
            location: token.location,
            message: None,
            context: self.get_context_around(token),
        }
    }

    pub fn expected_one_of_tokens(&self, token: &Token, expected: Vec<TokenKind>) -> ParseError {
        ParseError {
            kind: ParseErrorKind::ExpectedEitherOfTokens {
                found: token.kind,
                expected,
            },
            location: token.location,
            message: None,
            context: self.get_context_around(token),
        }
    }

    pub fn unexpected_token(&self, token: &Token) -> ParseError {
        ParseError {
            kind: ParseErrorKind::UnexpectedToken {
                kind: token.kind,
                text: token.text.to_string(),
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
    pub fn with_message(mut self, msg: &str) -> Self {
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
            ParseErrorKind::ExpectedToken { expected, found } => {
                format!("unexpected token: expected {:?}, got {:?}", expected, found)
            }
            ParseErrorKind::ExpectedEitherOfTokens { found, expected } => {
                format!(
                    "unexpected token: expected either one of {:?}, but got {:?}",
                    expected, found
                )
            }
            ParseErrorKind::UnexpectedToken { text, .. } => {
                format!("illegal or unsupported token {:?}", text)
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
    fn expected_url_after_method() {
        assert_errs!(
            "get {}",
            ExpectedToken {
                found: LBracket,
                expected: Url,
            }
        );

        assert_errs!(
            "post",
            ExpectedToken {
                found: End,
                expected: Url,
            }
        );
    }

    #[test]
    fn expected_name_after_header_keyword() {
        assert_errs!(
            "post http:://localhost {header}",
            ExpectedToken {
                found: RBracket,
                expected: StringLiteral,
            }
        );
    }

    #[test]
    fn expecting_identifier_or_string_lit_after_header_name() {
        assert_errs!(
            r#"get http://localhost { header "name" }"#,
            ExpectedEitherOfTokens {
                found: RBracket,
                expected: vec![StringLiteral, Ident, MultiLineStringLiteral],
            }
        );
    }

    #[test]
    fn reject_unsupported_tokens() {
        assert_errs!(
            r#"("#,
            UnexpectedToken {
                kind: LParen,
                text: "(".to_string()
            }
        );
    }

    #[test]
    fn reject_unfinished_strings() {
        assert_errs!(
            r#""asdfasdf"#,
            UnexpectedToken {
                kind: UnfinishedStringLiteral,
                text: "asdfasdf".to_string()
            }
        );
        assert_errs!(
            r#"`
                     asdfa"#,
            UnexpectedToken {
                kind: UnfinishedMultiLineStringLiteral,
                text: r#"
                     asdfa"#
                    .to_string()
            }
        );
    }
}
