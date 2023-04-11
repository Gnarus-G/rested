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
            let statement = match token.kind {
                Get => self.parse_request(RequestMethod::GET)?,
                Post => self.parse_request(RequestMethod::POST)?,
                Header => self.parse_header()?,
                Body => self.parse_body()?,
                Ident | StringLiteral | MultiLineStringLiteral => self.parse_expression(token)?,
                Url => {
                    return Err(self.error().unexpected_token(&token).with_message(
                        "url only make sense after a method keyword, 'get', 'post', etc..",
                    ))
                }
                Assign => return Err(self.error().unexpected_token(&token)),
                LParen | RParen => {
                    return Err(self
                        .error()
                        .unexpected_token(&token)
                        .with_message("parentheses only make sense within an env() call"))
                }
                LBracket | RBracket => {
                    return Err(self.error().unexpected_token(&token).with_message(
                        "brackets only make sense after the url of a request declaration",
                    ))
                }
                End => {
                    return Err(self
                        .error()
                        .unexpected_token(&token)
                        .with_message("unexpected eof"))
                }
                UnfinishedStringLiteral => {
                    return Err(self
                        .error()
                        .unexpected_token(&token)
                        .with_message("terminate the string with a \""))
                }
                UnfinishedMultiLineStringLiteral => {
                    return Err(self
                        .error()
                        .unexpected_token(&token)
                        .with_message("terminate the string with a `"))
                }
                IllegalToken => return Err(self.error().unexpected_token(&token)),
            };
            program.statements.push(statement);
            token = self.token();
        }

        Ok(program)
    }

    fn parse_request(&mut self, method: RequestMethod) -> Result<Statement<'i>> {
        self.expect(TokenKind::Url)?;
        let url = self.token();
        Ok(Statement::Request(crate::ast::RequestParams {
            method,
            url: url.into(),
            params: self.parse_request_params()?,
        }))
    }

    fn parse_request_params(&mut self) -> Result<Vec<Statement<'i>>> {
        if let TokenKind::LBracket = self.peek_token().kind {
            self.eat_token();
            let mut token = self.token();
            let mut headers = vec![];
            while token.kind != TokenKind::RBracket {
                let header = match token.kind {
                    TokenKind::Header => self.parse_header()?,
                    TokenKind::Body => self.parse_body()?,
                    _ => {
                        return Err(self
                            .error()
                            .unexpected_token(&token)
                            .with_message("may only declare headers or a body statement here"))
                    }
                };

                headers.push(header);
                token = self.token();
            }
            return Ok(headers);
        };

        Ok(vec![])
    }

    fn parse_header(&mut self) -> Result<Statement<'i>> {
        self.expect(TokenKind::StringLiteral)?;

        let header_name = self.token();

        self.expect_one_of(vec![
            TokenKind::StringLiteral,
            TokenKind::Ident,
            TokenKind::MultiLineStringLiteral,
        ])?;

        let header_value = self.token();

        Ok(Statement::HeaderStatement {
            name: header_name.into(),
            value: match header_value.kind {
                TokenKind::Ident if self.peek_token().kind == TokenKind::LParen => {
                    self.parse_call_expression(header_value)?
                }
                TokenKind::Ident => Expression::Identifier(header_value.into()),
                TokenKind::StringLiteral => Expression::StringLiteral(header_value.into()),
                TokenKind::MultiLineStringLiteral => Expression::StringLiteral(header_value.into()),
                _ => unreachable!(),
            },
        })
    }

    fn parse_body(&mut self) -> Result<Statement<'i>> {
        self.expect_one_of(vec![
            TokenKind::StringLiteral,
            TokenKind::Ident,
            TokenKind::MultiLineStringLiteral,
        ])?;

        let token = self.token();

        let value = match token.kind {
            TokenKind::Ident if self.peek_token().kind == TokenKind::LParen => {
                self.parse_call_expression(token)?
            }
            TokenKind::Ident => Expression::Identifier(token.into()),
            TokenKind::StringLiteral => Expression::StringLiteral(token.into()),
            TokenKind::MultiLineStringLiteral => Expression::StringLiteral(token.into()),
            _ => unreachable!(),
        };

        Ok(Statement::BodyStatement { value })
    }

    fn parse_expression(&mut self, start_token: Token<'i>) -> Result<Statement<'i>> {
        use TokenKind::*;

        let exp = match start_token.kind {
            Ident if self.peek_token().kind == LParen => self.parse_call_expression(start_token)?,
            Ident => Expression::Identifier(start_token.into()),
            StringLiteral | MultiLineStringLiteral => Expression::StringLiteral(start_token.into()),
            _ => return Err(self.error().unexpected_token(&start_token)),
        };

        Ok(Statement::ExpressionStatement(exp))
    }

    fn parse_call_expression(&mut self, identifier: Token<'i>) -> Result<Expression<'i>> {
        self.eat_token();

        let mut token = self.token();

        let mut arguments = vec![];

        while token.kind != TokenKind::RParen {
            match token.kind {
                TokenKind::StringLiteral => arguments.push(Expression::StringLiteral(token.into())),
                _ => {
                    return Err(self
                        .error()
                        .unexpected_token(&token)
                        .with_message("only string literals are allowed in call expressions"))
                }
            }
            token = self.token();
        }

        Ok(Expression::Call {
            identifier: identifier.into(),
            arguments,
        })
    }

    fn expect_one_of(&mut self, expected_kinds: Vec<TokenKind>) -> Result<()> {
        let ahead = self.peek_token();

        if expected_kinds.contains(&ahead.kind) {
            return Ok(());
        }

        let error = self
            .error()
            .expected_one_of_tokens(&self.token(), expected_kinds);

        Err(error)
    }

    fn expect(&mut self, expected_kind: TokenKind) -> Result<()> {
        let ahead = self.peek_token();

        if ahead.kind == expected_kind {
            return Ok(());
        }

        let error = self.error().expected_token(&self.token(), expected_kind);

        Err(error)
    }

    fn error(&self) -> ParseErrorConstructor<'i> {
        ParseErrorConstructor::new(self.lexer.input())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{AbstractToken, Expression, Program, RequestMethod, RequestParams, Statement};

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
    fn parse_get_urls() {
        assert_program!(
            r#"get http://localhost:8080
get http://localhost:8080 {}"#,
            Program {
                statements: vec![
                    Request(RequestParams {
                        method: GET,
                        url: AbstractToken {
                            text: "http://localhost:8080",
                            location: (0, 4).into()
                        },
                        params: vec![]
                    }),
                    Request(RequestParams {
                        method: GET,
                        url: AbstractToken {
                            text: "http://localhost:8080",
                            location: (1, 4).into()
                        },
                        params: vec![]
                    })
                ]
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
                    url: AbstractToken {
                        text: "http://localhost",
                        location: (0, 5).into()
                    },
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
                    url: AbstractToken {
                        text: "http://localhost",
                        location: (1, 4).into()
                    },
                    params: (vec![
                        HeaderStatement {
                            name: AbstractToken {
                                text: "Authorization",
                                location: (2, 11).into()
                            },
                            value: StringLiteral(AbstractToken {
                                text: "Bearer token",
                                location: (2, 27).into()
                            })
                        },
                        HeaderStatement {
                            name: AbstractToken {
                                text: "random",
                                location: (3, 11).into()
                            },
                            value: StringLiteral(AbstractToken {
                                text: "tokener Bear",
                                location: (3, 20).into()
                            })
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
                    url: AbstractToken {
                        text: "http://localhost",
                        location: (1, 5).into()
                    },
                    params: (vec![
                        HeaderStatement {
                            name: AbstractToken {
                                text: "Authorization",
                                location: (2, 11).into()
                            },
                            value: StringLiteral(AbstractToken {
                                text: "Bearer token",
                                location: (2, 27).into()
                            })
                        },
                        HeaderStatement {
                            name: AbstractToken {
                                text: "random",
                                location: (3, 11).into()
                            },
                            value: StringLiteral(AbstractToken {
                                text: "tokener Bear",
                                location: (3, 20).into()
                            })
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
                    url: AbstractToken {
                        text: "http://localhost",
                        location: (1, 5).into()
                    },
                    params: (vec![
                        HeaderStatement {
                            name: AbstractToken {
                                text: "Authorization",
                                location: (2, 11).into()
                            },
                            value: StringLiteral(AbstractToken {
                                text: "Bearer token",
                                location: (2, 27).into()
                            })
                        },
                        BodyStatement {
                            value: StringLiteral(AbstractToken {
                                text: "{neet: 1337}",
                                location: (3, 9).into()
                            })
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
                    url: AbstractToken {
                        text: "http://localhost",
                        location: (1, 5).into()
                    },
                    params: (vec![
                        HeaderStatement {
                            name: AbstractToken {
                                text: "Authorization",
                                location: (2, 11).into()
                            },
                            value: StringLiteral(AbstractToken {
                                text: "Bearer token",
                                location: (2, 27).into()
                            })
                        },
                        BodyStatement {
                            value: StringLiteral(AbstractToken {
                                text: "\n        {\"neet\": 1337}\n    ",
                                location: (3, 9).into()
                            })
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
                    url: AbstractToken {
                        text: "http://localhost",
                        location: (0, 5).into()
                    },
                    params: vec![
                        HeaderStatement {
                            name: AbstractToken {
                                text: "name",
                                location: (0, 31).into()
                            },
                            value: Call {
                                identifier: AbstractToken {
                                    text: "env",
                                    location: (0, 38).into()
                                },
                                arguments: vec![StringLiteral(AbstractToken {
                                    text: "auth",
                                    location: (0, 42).into()
                                })]
                            }
                        },
                        BodyStatement {
                            value: Call {
                                identifier: AbstractToken {
                                    text: "env",
                                    location: (0, 55).into()
                                },
                                arguments: vec![StringLiteral(AbstractToken {
                                    text: "data",
                                    location: (0, 59).into()
                                })]
                            }
                        }
                    ]
                })]
            }
        );
    }
}
