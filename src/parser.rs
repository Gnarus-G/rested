use crate::{
    ast::{Expression, Program, RequestMethod, Statement},
    error::{ParseError, ParseErrorConstructor},
    lexer::{Lexer, Token, TokenKind},
};

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug)]
pub struct Parser<'i> {
    lexer: Lexer<'i>,
    peeked: Option<Token<'i>>,
}

impl<'i> Parser<'i> {
    pub fn new(lexer: Lexer<'i>) -> Self {
        Self {
            peeked: None,
            lexer,
        }
    }

    fn token(&mut self) -> Token<'i> {
        match self.peeked.take() {
            Some(t) => t,
            None => self.lexer.next(),
        }
    }

    fn peek_token(&mut self) -> &Token<'i> {
        self.peeked.get_or_insert_with(|| self.lexer.next())
    }

    fn eat_token(&mut self) {
        self.token();
    }

    pub fn parse(&mut self) -> Result<Program<'i>> {
        let mut program = Program::new();

        use crate::lexer::TokenKind::*;

        let mut token = self.token();

        while token.kind != End {
            let request = match token.kind {
                Get => self.parse_request(RequestMethod::GET)?,
                Post => self.parse_request(RequestMethod::POST)?,
                tk => todo!("{tk:?}"),
            };
            program.statements.push(request);
            token = self.token();
        }

        Ok(program)
    }

    fn parse_request(&mut self, method: RequestMethod) -> Result<Statement<'i>> {
        self.expect(TokenKind::Url)?;
        Ok(Statement::Request(crate::ast::RequestParams {
            method,
            url: self.token().text,
            params: self.parse_request_params()?,
        }))
    }

    fn parse_request_params(&mut self) -> Result<Vec<Statement<'i>>> {
        let mut token = self.token();
        if let crate::lexer::TokenKind::LBracket = token.kind {
            token = self.token();
            let mut headers = vec![];
            while token.kind != TokenKind::RBracket {
                let h = match token.kind {
                    TokenKind::Header => self.parse_header()?,
                    TokenKind::Ident => todo!(),
                    TokenKind::StringLiteral => todo!(),
                    TokenKind::Assign => todo!(),
                    TokenKind::Quote => todo!(),
                    tk => todo!("{tk:?}"),
                };

                headers.push(h);
                token = self.token();
            }
            return Ok(headers);
        };

        Ok(vec![])
    }

    fn parse_header(&mut self) -> Result<Statement<'i>> {
        let t = self.token();

        let header_name = t.text;

        self.expect(TokenKind::Assign)?;

        self.eat_token();

        self.expect(TokenKind::Quote)?;

        self.eat_token();

        self.expect(TokenKind::StringLiteral)?;

        let header_value = self.token();

        self.expect(TokenKind::Quote)?;

        self.eat_token();

        Ok(Statement::HeaderStatement {
            name: header_name,
            value: match header_value.kind {
                TokenKind::Ident => Expression::Identifier(header_value.text),
                TokenKind::StringLiteral => Expression::StringLiteral(header_value.text),
                _ => todo!(),
            },
        })
    }

    fn expect(&mut self, expected_kind: TokenKind) -> Result<()> {
        let ahead = self.peek_token();

        if ahead.kind == expected_kind {
            return Ok(());
        }

        let error = self.error().unexpected_token(&self.token(), expected_kind);

        Err(error)
    }

    fn error(&self) -> ParseErrorConstructor<'i> {
        ParseErrorConstructor::new(self.lexer.input())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{Expression, Program, RequestMethod, RequestParams, Statement};

    use Expression::*;
    use RequestMethod::*;
    use Statement::*;

    macro_rules! assert_program {
        ($input:literal, $program:expr) => {
            let lexer = Lexer::new($input);
            let mut parser = Parser::new(lexer);
            assert_eq!(parser.parse().unwrap(), $program);
        };
    }

    #[test]
    fn parse_get_url() {
        assert_program!(
            "get http://localhost",
            Program {
                statements: vec![Request(RequestParams {
                    method: GET,
                    url: "http://localhost",
                    params: vec![]
                })]
            }
        );
    }

    #[test]
    fn parse_post_url() {
        assert_program!(
            "post http://localhost",
            Program {
                statements: vec![Request(RequestParams {
                    method: POST,
                    url: "http://localhost",
                    params: vec![]
                })]
            }
        );
    }

    #[test]
    fn parse_get_with_headers() {
        assert_program!(
            r#"
get http://localhost { 
    header Authorization = "Bearer token" 
    header random = "tokener Bear" 
}"#,
            Program {
                statements: vec![Request(RequestParams {
                    method: GET,
                    url: "http://localhost",
                    params: (vec![
                        HeaderStatement {
                            name: "Authorization",
                            value: StringLiteral("Bearer token")
                        },
                        HeaderStatement {
                            name: "random",
                            value: StringLiteral("tokener Bear")
                        }
                    ])
                })]
            }
        );
    }

    #[test]
    fn parse_post_with_headers() {
        assert_program!(
            r#"
post http://localhost { 
    header Authorization = "Bearer token" 
    header random = "tokener Bear" 
}"#,
            Program {
                statements: vec![Request(RequestParams {
                    method: POST,
                    url: "http://localhost",
                    params: (vec![
                        HeaderStatement {
                            name: "Authorization",
                            value: StringLiteral("Bearer token")
                        },
                        HeaderStatement {
                            name: "random",
                            value: StringLiteral("tokener Bear")
                        }
                    ])
                })]
            }
        );
    }
}
