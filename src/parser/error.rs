use crate::lexer;
use crate::lexer::locations::Position;
use crate::lexer::{locations::GetSpan, Token, TokenKind};

use crate::error_meta::ContextualError;

use super::{Parser, Result, TokenCheck};

impl<'source> std::fmt::Display for lexer::Token<'source> {
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

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum ParseError<'source> {
    ExpectedToken {
        found: lexer::Token<'source>,
        expected: TokenKind,
    },
    ExpectedEitherOfTokens {
        found: lexer::Token<'source>,
        expected: Box<[TokenKind]>,
    },
}

impl<'source> std::error::Error for ParseError<'source> {}

impl<'source> std::fmt::Display for ParseError<'source> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = match self {
            ParseError::ExpectedToken {
                expected, found, ..
            } => {
                format!("expected '{}' but got {}", expected, found)
            }
            ParseError::ExpectedEitherOfTokens {
                found, expected, ..
            } => {
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
    pub errors: Box<[ContextualError<ParseError<'source>>]>,
}

impl<'source> ParserErrors<'source> {
    pub fn new(errors: Vec<ContextualError<ParseError<'source>>>) -> Self {
        Self {
            errors: errors.into(),
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

#[derive(Debug)]
pub struct Expectations<'i> {
    source_code: &'i str,
    pub start: Position,
}

impl<'i> Expectations<'i> {
    pub fn new(parser: &Parser<'i>) -> Self {
        Self {
            source_code: parser.lexer.input(),
            start: parser.curr_token().start,
        }
    }

    pub fn expect_peek<'p>(
        &self,
        parser: &'p mut Parser<'i>,
        expected_kind: TokenKind,
    ) -> Result<'i, &'p Token<'i>> {
        if parser.peek_token().is(expected_kind) {
            return Ok(parser.next_token());
        }

        let error = self.expected_token(parser.next_token(), expected_kind);

        Err(error.into())
    }

    /// Like `expect_peek` but it doesn't advance the parser.
    pub fn expect_peek_ahead<'p>(
        &self,
        parser: &'p mut Parser<'i>,
        expected_kind: TokenKind,
    ) -> Result<'i, &'p Token<'i>> {
        let peeked = parser.peek_token();
        if peeked.is(expected_kind) {
            return Ok(peeked);
        }

        let error = self.expected_token(peeked, expected_kind);

        Err(error.into())
    }

    pub fn expect_peek_one_of(
        &self,
        parser: &mut Parser<'i>,
        expected_kinds: &[TokenKind],
    ) -> Result<'i, ()> {
        if parser.peek_token().is_one_of(expected_kinds) {
            return Ok(());
        }

        let error = self.expected_one_of_tokens(parser.next_token(), expected_kinds);

        Err(error.into())
    }

    pub fn expected_token(
        &self,
        token: &Token<'i>,
        expected: TokenKind,
    ) -> ContextualError<ParseError<'i>> {
        ContextualError::new(
            ParseError::ExpectedToken {
                found: token.clone(),
                expected,
            },
            self.start.to_end_of(token.span()),
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
                found: token.clone(),
                expected: expected_dedpuded.into(),
            },
            self.start.to_end_of(token.span()),
            self.source_code,
        )
    }
}

pub struct ErrorsCollector<'source> {
    pub list: Vec<ContextualError<ParseError<'source>>>,
}

impl<'source> crate::parser::ast_visit::Visitor<'source> for ErrorsCollector<'source> {
    fn visit_error(&mut self, err: &ContextualError<ParseError<'source>>) {
        self.list.push(err.clone());
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::Parser;

    use insta::assert_ron_snapshot;

    macro_rules! assert_ast {
        ($input:literal) => {
            let mut parser = Parser::new($input);
            let ast = parser.parse();

            insta::with_settings!({
                 description => $input
            }, {
                assert_ron_snapshot!(ast)
            });

            assert!(!ast.errors().is_empty())
        };
    }

    #[test]
    fn expected_url_after_method() {
        assert_ast!("get");
        assert_ast!("post");
    }

    #[test]
    fn expected_name_after_header_keyword() {
        assert_ast!("post http://localhost {header}");
    }

    #[test]
    fn expecting_identifier_or_string_lit_after_header_name() {
        assert_ast!(r#"get http://localhost { header "name" }"#);
    }

    #[test]
    fn expecting_request_or_other_attribute_after_attributes() {
        assert_ast!(
            r#"
            @skip
            @dbg
            let k = "v"
            get http://localhost { header "name" k }"#
        );
    }

    #[test]
    fn expecting_commas_between_certain_json_items() {
        assert_ast!(
            r#"let o = {
                 yo: "joe"
                 hello: "world"
               }"#
        );
        assert_ast!(r#" let o = ["joe" "world"] "#);
    }

    #[test]
    fn expecting_partial_block_with_error() {
        assert_ast!(r#"get /hello { header "test" "value" header }"#);
    }

    #[test]
    fn expecting_partial_block_with_missing_body_expr() {
        assert_ast!(
            r#"
get /sdf {
   header "" s
   body  }
"#
        );
    }

    #[test]
    fn expecting_partial_block_with_errors() {
        assert_ast!(
            r#"
get /adsf {
  header
  body a
}
"#
        );
    }

    #[test]
    fn expecting_partial_object_literal_with_errors() {
        assert_ast!(
            r#"
let b = {
  "key": value, 
  "key2": 
}
"#
        );
    }

    #[test]
    fn expecting_expression_error_for_parameter_of_unfinished_string() {
        assert_ast!(
            r#"
let a = env(")
"#
        );

        assert_ast!(
            r#"
let b = [env(")]
"#
        );

        assert_ast!(
            r#"
let b = {
    key: env(")
}
"#
        );
    }

    #[test]
    fn expected_comma_before_more_parameters() {
        assert_ast!(r#"env("base" "url")"#);
        assert_ast!(r#"env("", false 12)"#);
    }

    #[test]
    fn expected_closing_curly_after_expression_part_in_template_string() {
        assert_ast!(r#"`wowee ${"hello"} error ahead ${env("base")`"#);
        assert_ast!(r#"`wowee ${"hello"} error ahead ${variable_name`"#);
        assert_ast!(r#"`error ahead ${variable_name `"#);
    }
}
