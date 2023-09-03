use crate::lexer::{locations::GetSpan, Array, Token, TokenKind};

use crate::error_meta::ContextualError;

use super::ast::Program;

#[derive(Debug, PartialEq)]
pub struct ErroneousToken<'source> {
    kind: TokenKind,
    text: &'source str,
}

impl<'source> From<&Token<'source>> for ErroneousToken<'source> {
    fn from(token: &Token<'source>) -> Self {
        Self {
            text: token.text,
            kind: token.kind,
        }
    }
}

impl<'source> std::fmt::Display for ErroneousToken<'source> {
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
pub enum ParseError<'source> {
    ExpectedToken {
        found: ErroneousToken<'source>,
        expected: TokenKind,
    },
    ExpectedEitherOfTokens {
        found: ErroneousToken<'source>,
        expected: Array<TokenKind>,
    },
}

impl<'source> std::error::Error for ParseError<'source> {}

impl<'source> std::fmt::Display for ParseError<'source> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = match self {
            ParseError::ExpectedToken { expected, found } => {
                format!("expected '{}' but got {}", expected, found)
            }
            ParseError::ExpectedEitherOfTokens { found, expected } => {
                let expected = expected
                    .iter()
                    .map(|kind| format!("'{}'", kind))
                    .collect::<Vec<String>>()
                    .join(",");
                format!("expected either one of {} but got {}", expected, found)
            }
        };

        f.write_str(&formatted_error)
    }
}

#[derive(Debug)]
pub struct ParserErrors<'source> {
    pub errors: Array<ContextualError<ParseError<'source>>>,
    pub incomplete_program: Program<'source>,
}

impl<'source> ParserErrors<'source> {
    pub fn new(
        errors: Array<ContextualError<ParseError<'source>>>,
        incomplete_program: Program<'source>,
    ) -> Self {
        Self {
            errors,
            incomplete_program,
        }
    }
}

impl<'source> std::error::Error for ParserErrors<'source> {}

impl<'source> std::fmt::Display for ParserErrors<'source> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for err in self.errors.iter() {
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
        token: &Token<'i>,
        expected: TokenKind,
    ) -> ContextualError<ParseError<'i>> {
        ContextualError::new(
            ParseError::ExpectedToken {
                found: ErroneousToken {
                    text: token.text,
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
        token: &Token<'i>,
        expected_kinds: &[TokenKind],
    ) -> ContextualError<ParseError<'i>> {
        let mut expected_dedpuded: Vec<TokenKind> = vec![];

        for kind in expected_kinds {
            if !expected_dedpuded.contains(kind) {
                expected_dedpuded.push(*kind)
            }
        }

        ContextualError::new(
            ParseError::ExpectedEitherOfTokens {
                found: token.into(),
                expected: expected_dedpuded.into(),
            },
            token.span(),
            self.source_code,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::Parser;

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
