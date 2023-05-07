use lexer::{Token, TokenKind};

use crate::error_meta::Error;

#[derive(Debug, PartialEq)]
pub struct TokenOwned {
    kind: TokenKind,
    text: String,
}

impl<'i> From<&Token<'i>> for TokenOwned {
    fn from(token: &Token<'i>) -> Self {
        Self {
            text: token.text.to_string(),
            kind: token.kind,
        }
    }
}

impl std::fmt::Display for TokenOwned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}({:?})", self.kind, self.text)
    }
}

#[derive(Debug)]
pub struct ParseErrorConstructor<'i> {
    source_code: &'i str,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    ExpectedToken {
        found: TokenOwned,
        expected: TokenKind,
    },
    ExpectedEitherOfTokens {
        found: TokenOwned,
        expected: Vec<TokenKind>,
    },
    UnexpectedToken {
        kind: TokenKind,
        text: String,
    },
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = match self {
            ParseError::ExpectedToken { expected, found } => {
                format!("expected {:?}, got {}", expected, found)
            }
            ParseError::ExpectedEitherOfTokens { found, expected } => {
                format!("expected either one of {:?}, but got {}", expected, found)
            }
            ParseError::UnexpectedToken { text, .. } => {
                format!("unexpected token {:?}", text)
            }
        };

        f.write_str(&formatted_error)
    }
}

impl<'i> ParseErrorConstructor<'i> {
    pub fn new(source: &'i str) -> Self {
        Self {
            source_code: source,
        }
    }

    pub fn expected_token(&self, token: &Token, expected: TokenKind) -> Error<ParseError> {
        Error::new(
            ParseError::ExpectedToken {
                found: TokenOwned {
                    text: token.text.to_string(),
                    kind: token.kind,
                },
                expected,
            },
            token.location,
            self.source_code,
        )
    }

    pub fn expected_one_of_tokens(
        &self,
        token: &Token,
        expected: Vec<TokenKind>,
    ) -> Error<ParseError> {
        Error::new(
            ParseError::ExpectedEitherOfTokens {
                found: token.into(),
                expected,
            },
            token.location,
            self.source_code,
        )
    }

    pub fn unexpected_token(&self, token: &Token) -> Error<ParseError> {
        Error::new(
            ParseError::UnexpectedToken {
                kind: token.kind,
                text: token.text.to_string(),
            },
            token.location,
            self.source_code,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    use TokenKind::*;

    macro_rules! assert_errs {
        ($input:literal, $kind:expr) => {
            let mut parser = Parser::new($input);
            let error = parser.parse().unwrap_err();

            assert_eq!(error.inner_error, $kind)
        };
    }

    use ParseError::*;

    #[test]
    fn expected_url_after_method() {
        assert_errs!(
            "get {}",
            ExpectedEitherOfTokens {
                found: TokenOwned {
                    kind: LBracket,
                    text: "{".into(),
                },
                expected: vec![Url, Pathname],
            }
        );

        assert_errs!(
            "post",
            ExpectedEitherOfTokens {
                found: TokenOwned {
                    kind: End,
                    text: "".into(),
                },
                expected: vec![Url, Pathname],
            }
        );
    }

    #[test]
    fn expected_name_after_header_keyword() {
        assert_errs!(
            "post http://localhost {header}",
            ExpectedToken {
                found: TokenOwned {
                    kind: RBracket,
                    text: "}".into()
                },
                expected: StringLiteral,
            }
        );
    }

    #[test]
    fn expecting_identifier_or_string_lit_after_header_name() {
        assert_errs!(
            r#"get http://localhost { header "name" }"#,
            ExpectedEitherOfTokens {
                found: TokenOwned {
                    kind: RBracket,
                    text: "}".into()
                },
                expected: vec![StringLiteral, Ident, MultiLineStringLiteral],
            }
        );
    }

    #[test]
    fn expecting_request_or_other_attribute_after_attributes() {
        assert_errs!(
            r#"
            @skip
            @dbg
            let k = "v"
            get http://localhost { header "name" k }"#,
            ExpectedEitherOfTokens {
                found: TokenOwned {
                    kind: Let,
                    text: "let".into()
                },
                expected: vec![Get, Post, Put, Patch, Delete, AttributePrefix],
            }
        );
    }
}
