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
                    TokenKind::Body => self.parse_body()?,
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
        self.expect(TokenKind::StringLiteral)?;

        let header_name = self.token();

        let header_value = self.token();

        Ok(Statement::HeaderStatement {
            name: header_name.text,
            value: match header_value.kind {
                TokenKind::Ident if self.peek_token().kind == TokenKind::LParen => {
                    self.parse_call_expression(header_value)
                }
                TokenKind::Ident => Expression::Identifier(header_value.text),
                TokenKind::StringLiteral => Expression::StringLiteral(header_value.text),
                TokenKind::MultiLineStringLiteral => Expression::StringLiteral(header_value.text),
                _ => todo!(),
            },
        })
    }

    fn parse_body(&mut self) -> Result<Statement<'i>> {
        let token = self.token();

        let value = match token.kind {
            TokenKind::Ident if self.peek_token().kind == TokenKind::LParen => {
                self.parse_call_expression(token)
            }
            TokenKind::Ident => Expression::Identifier(token.text),
            TokenKind::StringLiteral => Expression::StringLiteral(token.text),
            TokenKind::MultiLineStringLiteral => Expression::StringLiteral(token.text),
            _ => todo!(),
        };

        Ok(Statement::BodyStatement { value })
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

    fn parse_call_expression(&mut self, identifier: Token<'i>) -> Expression<'i> {
        self.eat_token();

        let mut token = self.token();

        let mut arguments = vec![];

        while token.kind != TokenKind::RParen {
            match token.kind {
                TokenKind::StringLiteral => arguments.push(Expression::StringLiteral(token.text)),
                _ => todo!(),
            }
            token = self.token();
        }

        Expression::Call {
            identifier: identifier.text,
            arguments,
        }
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
    header "Authorization" "Bearer token" 
    header "random" "tokener Bear" 
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
    header "Authorization" "Bearer token" 
    header "random" "tokener Bear" 
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

    #[test]
    fn parse_post_with_headers_and_body() {
        assert_program!(
            r#"
post http://localhost { 
    header "Authorization" "Bearer token" 
    body "{neet: 1337}" 
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
                        BodyStatement {
                            value: StringLiteral("{neet: 1337}")
                        }
                    ])
                })]
            }
        );
    }

    #[test]
    fn parse_post_with_headers_and_body_as_json_string() {
        assert_program!(
            r#"
post http://localhost { 
    header "Authorization" "Bearer token" 
    body `
        {"neet": 1337}
    `
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
                        BodyStatement {
                            value: StringLiteral("\n        {\"neet\": 1337}\n    ")
                        }
                    ])
                })]
            }
        );
    }

    #[test]
    fn parse_env_call_expression() {
        assert_program!(
            r#"post http://localhost { header "name" env("auth") body env("data") }"#,
            Program {
                statements: vec![Request(RequestParams {
                    method: POST,
                    url: "http://localhost",
                    params: vec![
                        HeaderStatement {
                            name: "name",
                            value: Call {
                                identifier: "env",
                                arguments: vec![StringLiteral("auth")]
                            }
                        },
                        BodyStatement {
                            value: Call {
                                identifier: "env",
                                arguments: vec![StringLiteral("data")]
                            }
                        }
                    ]
                })]
            }
        );
    }
}
