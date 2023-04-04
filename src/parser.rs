use crate::{
    ast::{Program, Request},
    lexer::{Lexer, Token},
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
                _ => todo!(),
            };
            program.requests.push(request);
            token = self.token();
        }

        program
    }

    fn parse_get_request(&mut self) -> Request<'i> {
        Request::Get(crate::ast::GetRequestParams {
            url: self.token().text,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{GetRequestParams, Program, Request};

    #[test]
    fn parse_get_url() {
        let lex = Lexer::new("get http://localhost");
        let mut p = Parser::new(lex);

        assert_eq!(
            p.parse(),
            Program {
                requests: vec![Request::Get(GetRequestParams {
                    url: "http://localhost"
                })]
            }
        );
    }
}
