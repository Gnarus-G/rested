pub mod error;

use crate::{
    ast::{Expression, Item, Program, RequestMethod, Statement, UrlOrPathname},
    error::Error,
    lexer::{Lexer, Token, TokenKind},
};

use self::error::{ParseError, ParseErrorConstructor};

pub type Result<T> = std::result::Result<T, Error<ParseError>>;

#[derive(Debug)]
pub struct Parser<'i> {
    lexer: Lexer<'i>,
    peeked: Option<Token<'i>>,
}

impl<'i> Parser<'i> {
    pub fn new(code: &'i str) -> Self {
        Self {
            peeked: None,
            lexer: Lexer::new(code),
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

        self.expect_one_of(vec![Set, Get, Post, Linecomment, Shebang, AttributePrefix])?;

        let mut token = self.token();

        while token.kind != End {
            let statement = match token.kind {
                Get => self.parse_request(RequestMethod::GET, token)?,
                Post => self.parse_request(RequestMethod::POST, token)?,
                Linecomment | Shebang => Item::LineComment(token.into()),
                Set => self.parse_set_statement()?,
                AttributePrefix => self.parse_attribute(token)?,
                _ => {
                    unreachable!("we properly expect items at this level of the program structure")
                }
            };
            program.items.push(statement);
            token = self.token();
        }

        Ok(program)
    }

    fn parse_request(&mut self, method: RequestMethod, token: Token<'i>) -> Result<Item<'i>> {
        self.expect_one_of(vec![TokenKind::Url, TokenKind::Pathname])?;
        let url = self.token();
        Ok(Item::Request {
            location: token.location,
            method,
            endpoint: match url.kind {
                TokenKind::Url => UrlOrPathname::Url(url.into()),
                TokenKind::Pathname => UrlOrPathname::Pathname(url.into()),
                _ => unreachable!("we're properly expecting only url and pathname tokens here"),
            },
            params: self.parse_request_params()?,
        })
    }

    fn parse_set_statement(&mut self) -> Result<Item<'i>> {
        self.expect(TokenKind::Ident)?;

        let identifier = self.token();

        self.expect_one_of(vec![
            TokenKind::Ident,
            TokenKind::StringLiteral,
            TokenKind::MultiLineStringLiteral,
        ])?;

        let value_token = self.token();

        Ok(Item::Set {
            identifier: identifier.into(),
            value: self.parse_expression(value_token)?,
        })
    }

    fn parse_request_params(&mut self) -> Result<Vec<Statement<'i>>> {
        use TokenKind::*;
        if let LBracket = self.peek_token().kind {
            self.eat_token();
            let mut token = self.token();
            let mut headers = vec![];
            while token.kind != TokenKind::RBracket {
                let header = match token.kind {
                    Header => self.parse_header()?,
                    Body => self.parse_body(token)?,
                    Linecomment | Shebang => Statement::LineComment(token.into()),
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

        Ok(Statement::Header {
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

    fn parse_body(&mut self, t: Token) -> Result<Statement<'i>> {
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

        Ok(Statement::Body {
            value,
            location: t.location,
        })
    }

    fn parse_expression(&mut self, start_token: Token<'i>) -> Result<Expression<'i>> {
        use TokenKind::*;

        let exp = match start_token.kind {
            Ident if self.peek_token().kind == LParen => self.parse_call_expression(start_token)?,
            Ident => Expression::Identifier(start_token.into()),
            StringLiteral | MultiLineStringLiteral => Expression::StringLiteral(start_token.into()),
            _ => return Err(self.error().unexpected_token(&start_token)),
        };

        Ok(exp)
    }

    fn parse_call_expression(&mut self, identifier: Token<'i>) -> Result<Expression<'i>> {
        self.eat_token();

        let mut token = self.token();

        let mut arguments = vec![];

        while token.kind != TokenKind::RParen {
            let exp = self.parse_expression(token)?;
            arguments.push(exp);
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

    fn parse_attribute(&mut self, token: Token<'i>) -> Result<Item<'i>> {
        let location = token.location;

        self.expect(TokenKind::Ident)?;

        let ident = self.token();

        let mut params = vec![];

        if let TokenKind::LParen = self.peek_token().kind {
            self.eat_token();

            let mut token = self.token();
            while token.kind != TokenKind::RParen {
                let exp = self.parse_expression(token)?;

                params.push(exp);

                token = self.token();
            }
        }

        Ok(Item::Attribute {
            location,
            identifier: ident.into(),
            parameters: params,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{Expression, Identifier, Literal, Program, RequestMethod, Statement};

    use Expression::*;
    use Item::*;
    use RequestMethod::*;
    use Statement::*;

    macro_rules! assert_program {
        ($input:literal, $program:expr) => {
            let mut parser = Parser::new($input);
            assert_eq!(parser.parse().unwrap(), $program);
        };
    }

    #[test]
    fn parse_get_urls() {
        assert_program!(
            r#"get http://localhost:8080
get http://localhost:8080 {}"#,
            Program {
                items: vec![
                    Item::Request {
                        location: (0, 0).into(),

                        method: GET,
                        endpoint: UrlOrPathname::Url(Literal {
                            value: "http://localhost:8080",
                            location: (0, 4).into()
                        }),
                        params: vec![]
                    },
                    Item::Request {
                        location: (1, 0).into(),

                        method: GET,
                        endpoint: UrlOrPathname::Url(Literal {
                            value: "http://localhost:8080",
                            location: (1, 4).into()
                        }),
                        params: vec![]
                    }
                ]
            }
        );
    }

    #[test]
    fn parse_post_url() {
        assert_program!(
            "post http://localhost",
            Program {
                items: vec![Item::Request {
                    location: (0, 0).into(),
                    method: POST,
                    endpoint: UrlOrPathname::Url(Literal {
                        value: "http://localhost",
                        location: (0, 5).into()
                    }),
                    params: vec![]
                }]
            }
        );

        assert_program!(
            "post /api/v2",
            Program {
                items: vec![Request {
                    location: (0, 0).into(),
                    method: POST,
                    endpoint: UrlOrPathname::Pathname(Literal {
                        value: "/api/v2",
                        location: (0, 5).into()
                    }),
                    params: vec![]
                }]
            }
        );
    }

    #[test]
    fn parse_attributes() {
        assert_program!(
            r#"@log("path/to/file")"#,
            Program {
                items: vec![Attribute {
                    location: (0, 0).into(),
                    identifier: Identifier {
                        name: "log",
                        location: (0, 1).into()
                    },
                    parameters: vec![StringLiteral(Literal {
                        value: "path/to/file",
                        location: (0, 5).into()
                    })]
                }]
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
                items: vec![Request {
                    location: (1, 0).into(),
                    method: GET,
                    endpoint: UrlOrPathname::Url(Literal {
                        value: "http://localhost",
                        location: (1, 4).into()
                    }),
                    params: (vec![
                        Header {
                            name: Literal {
                                value: "Authorization",
                                location: (2, 11).into()
                            },
                            value: StringLiteral(Literal {
                                value: "Bearer token",
                                location: (2, 27).into()
                            })
                        },
                        Header {
                            name: Literal {
                                value: "random",
                                location: (3, 11).into()
                            },
                            value: StringLiteral(Literal {
                                value: "tokener Bear",
                                location: (3, 20).into()
                            })
                        }
                    ])
                }]
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
                items: vec![Request {
                    location: (1, 0).into(),
                    method: POST,
                    endpoint: UrlOrPathname::Url(Literal {
                        value: "http://localhost",
                        location: (1, 5).into()
                    }),
                    params: (vec![
                        Header {
                            name: Literal {
                                value: "Authorization",
                                location: (2, 11).into()
                            },
                            value: StringLiteral(Literal {
                                value: "Bearer token",
                                location: (2, 27).into()
                            })
                        },
                        Header {
                            name: Literal {
                                value: "random",
                                location: (3, 11).into()
                            },
                            value: StringLiteral(Literal {
                                value: "tokener Bear",
                                location: (3, 20).into()
                            })
                        }
                    ])
                }]
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
                items: vec![Request {
                    location: (1, 0).into(),
                    method: POST,
                    endpoint: UrlOrPathname::Url(Literal {
                        value: "http://localhost",
                        location: (1, 5).into()
                    }),
                    params: (vec![
                        Header {
                            name: Literal {
                                value: "Authorization",
                                location: (2, 11).into()
                            },
                            value: StringLiteral(Literal {
                                value: "Bearer token",
                                location: (2, 27).into()
                            })
                        },
                        Body {
                            value: StringLiteral(Literal {
                                value: "{neet: 1337}",
                                location: (3, 9).into()
                            }),
                            location: (3, 4).into()
                        }
                    ])
                }]
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
                items: vec![Request {
                    location: (1, 0).into(),
                    method: POST,
                    endpoint: UrlOrPathname::Url(Literal {
                        value: "http://localhost",
                        location: (1, 5).into()
                    }),
                    params: (vec![
                        Header {
                            name: Literal {
                                value: "Authorization",
                                location: (2, 11).into()
                            },
                            value: StringLiteral(Literal {
                                value: "Bearer token",
                                location: (2, 27).into()
                            })
                        },
                        Body {
                            value: StringLiteral(Literal {
                                value: "\n        {\"neet\": 1337}\n    ",
                                location: (3, 9).into()
                            }),
                            location: (3, 4).into()
                        }
                    ])
                }]
            }
        );
    }

    #[test]
    fn parse_env_call_expression() {
        assert_program!(
            r#"post http://localhost { header "name" env("auth") body env("data") }"#,
            Program {
                items: vec![Request {
                    location: (0, 0).into(),
                    method: POST,
                    endpoint: UrlOrPathname::Url(Literal {
                        value: "http://localhost",
                        location: (0, 5).into()
                    }),
                    params: vec![
                        Header {
                            name: Literal {
                                value: "name",
                                location: (0, 31).into()
                            },
                            value: Call {
                                identifier: Identifier {
                                    name: "env",
                                    location: (0, 38).into()
                                },
                                arguments: vec![StringLiteral(Literal {
                                    value: "auth",
                                    location: (0, 42).into()
                                })]
                            }
                        },
                        Body {
                            location: (0, 50).into(),
                            value: Call {
                                identifier: Identifier {
                                    name: "env",
                                    location: (0, 55).into()
                                },
                                arguments: vec![StringLiteral(Literal {
                                    value: "data",
                                    location: (0, 59).into()
                                })]
                            }
                        }
                    ]
                }]
            }
        );
    }

    #[test]
    fn parse_global_constant_setting() {
        assert_program!(
            "set BASE_URL \"stuff\"",
            Program {
                items: vec![Set {
                    identifier: Identifier {
                        name: "BASE_URL",
                        location: (0, 4).into()
                    },
                    value: StringLiteral(Literal {
                        value: "stuff",
                        location: (0, 13).into()
                    })
                }]
            }
        );
    }
}
