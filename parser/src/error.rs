use lexer::{locations::GetSpan, Token, TokenKind};

use error_meta::ContextualError;

use crate::ast::Program;

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
        use TokenKind::*;
        match self.kind {
            Url | Linecomment | IllegalToken => {
                write!(f, "{}<{}>", self.kind, self.text)
            }
            kind => write!(f, "{kind}"),
        }
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
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = match self {
            ParseError::ExpectedToken { expected, found } => {
                format!("expected {}, got {}", expected, found)
            }
            ParseError::ExpectedEitherOfTokens { found, expected } => {
                let expected = expected
                    .iter()
                    .map(|kind| kind.to_string())
                    .collect::<Vec<String>>()
                    .join(",");
                format!("expected either one of {}, but got {}", expected, found)
            }
        };

        f.write_str(&formatted_error)
    }
}

#[derive(Debug)]
pub struct ParserErrors<'source> {
    pub errors: Vec<ContextualError<ParseError>>,
    pub incomplete_rogram: Program<'source>,
}

impl<'source> ParserErrors<'source> {
    pub fn new(
        errors: Vec<ContextualError<ParseError>>,
        incomplete_rogram: Program<'source>,
    ) -> Self {
        Self {
            errors,
            incomplete_rogram,
        }
    }
}

impl<'source> std::error::Error for ParserErrors<'source> {}

impl<'source> std::fmt::Display for ParserErrors<'source> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for err in &self.errors {
            write!(f, "{err}")?
        }
        Ok(())
    }
}

impl<'i> ParseErrorConstructor<'i> {
    pub fn new(source: &'i str) -> Self {
        Self {
            source_code: source,
        }
    }

    pub fn expected_token(
        &self,
        token: &Token,
        expected: TokenKind,
    ) -> ContextualError<ParseError> {
        ContextualError::new(
            ParseError::ExpectedToken {
                found: TokenOwned {
                    text: token.text.to_string(),
                    kind: token.kind,
                },
                expected,
            },
            token.into(),
            self.source_code,
        )
    }

    pub fn expected_one_of_tokens(
        &self,
        token: &Token,
        expected_kinds: Vec<TokenKind>,
    ) -> ContextualError<ParseError> {
        let mut expected_dedpuded = vec![];

        for kind in expected_kinds {
            if !expected_dedpuded.contains(&kind) {
                expected_dedpuded.push(kind)
            }
        }

        ContextualError::new(
            ParseError::ExpectedEitherOfTokens {
                found: token.into(),
                expected: expected_dedpuded,
            },
            token.span(),
            self.source_code,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::Parser;

    use insta::assert_debug_snapshot;

    macro_rules! assert_errs {
        ($input:literal) => {
            let mut parser = Parser::new($input);
            let error = parser.parse().unwrap_err();

            assert_debug_snapshot!(error)
        };
    }

    #[test]
    fn expected_url_after_method() {
        assert_errs!("get {}");

        assert_errs!("post");
    }

    #[test]
    fn expected_name_after_header_keyword() {
        assert_errs!("post http://localhost {header}");
    }

    #[test]
    fn expecting_identifier_or_string_lit_after_header_name() {
        assert_errs!(r#"get http://localhost { header "name" }"#);
    }

    #[test]
    fn expecting_request_or_other_attribute_after_attributes() {
        assert_errs!(
            r#"
            @skip
            @dbg
            let k = "v"
            get http://localhost { header "name" k }"#
        );
    }

    #[test]
    fn expecting_commas_between_certain_json_items() {
        assert_errs!(
            r#"let o = {
                 yo: "joe"
                 hello: "world"
               }"#
        );
        assert_errs!(r#" let o = ["joe" "world"] "#);
    }
}
