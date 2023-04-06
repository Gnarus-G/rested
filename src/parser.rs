use crate::{
    ast::{Header, Program, Request},
    lexer::{Lexer, Token, TokenKind},
};

#[derive(Debug)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    peeked: Option<Token<'a>>,
}

impl<'i> Parser<'i> {
    pub fn new(lexer: Lexer<'i>) -> Self {
        Self {
            lexer,
            peeked: None,
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

    pub fn parse(&mut self) -> Program<'i> {
        let mut program = Program::new();

        use crate::lexer::TokenKind::*;

        let mut token = self.token();

        while token.kind != End {
            let request = match token.kind {
                Get => self.parse_get_request(),
                tk => todo!("{tk:?}"),
            };
            program.requests.push(request);
            token = self.token();
        }

        program
    }

    fn parse_get_request(&mut self) -> Request<'i> {
        Request::Get(crate::ast::GetRequestParams {
            url: self.token().text,
            headers: self.parse_get_params(),
        })
    }

    fn parse_get_params(&mut self) -> Option<Vec<Header<'i>>> {
        let mut token = self.token();
        if let crate::lexer::TokenKind::LBracket = token.kind {
            token = self.token();
            let mut headers = vec![];
            while token.kind != TokenKind::RBracket {
                let h = match token.kind {
                    TokenKind::Header => self.parse_header(),
                    TokenKind::Ident => todo!(),
                    TokenKind::StringLiteral => todo!(),
                    TokenKind::Assign => todo!(),
                    TokenKind::Quote => todo!(),
                    tk => todo!("{tk:?}"),
                };

                headers.push(h);
                token = self.token();
            }
            Some(headers)
        } else {
            None
        }
    }

    fn parse_header(&mut self) -> Header<'i> {
        let t = self.token();

        let header_name = t.text;

        self.eat_token();

        self.eat_token();
        let header_value = self.token().text;
        self.eat_token();

        Header {
            name: header_name,
            value: header_value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{GetRequestParams, Header, Program, Request};

    macro_rules! assert_program {
        ($input:literal, $program:expr) => {
            let lexer = Lexer::new($input);
            let mut parser = Parser::new(lexer);
            assert_eq!(parser.parse(), $program);
        };
    }

    #[test]
    fn parse_get_url() {
        assert_program!(
            "get http://localhost",
            Program {
                requests: vec![Request::Get(GetRequestParams {
                    url: "http://localhost",
                    headers: None
                })]
            }
        );
    }

    #[test]
    fn parse_get_with_headers() {
        assert_program!(
            "get http://localhost { header Authorization = \"Bearer token\" }",
            Program {
                requests: vec![Request::Get(GetRequestParams {
                    url: "http://localhost",
                    headers: Some(vec![Header {
                        name: "Authorization",
                        value: "Bearer token"
                    }])
                })]
            }
        );
    }
}
