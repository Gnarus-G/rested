pub mod ast;
pub mod error;

use crate::ast::{Endpoint, Expression, Item, Program, RequestMethod, Statement};
use ast::Block;
use error_meta::Error;
use lexer::{
    locations::{GetSpan, Span},
    Lexer, Token, TokenKind,
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

        use lexer::TokenKind::*;

        self.expect_one_of(vec![
            Set,
            Get,
            Post,
            Put,
            Patch,
            Delete,
            Linecomment,
            Shebang,
            AttributePrefix,
            Let,
            End,
        ])?;

        let mut token = self.token();

        while token.kind != End {
            let statement = match token.kind {
                Get => self.parse_request(RequestMethod::GET, token)?,
                Post => self.parse_request(RequestMethod::POST, token)?,
                Put => self.parse_request(RequestMethod::PUT, token)?,
                Patch => self.parse_request(RequestMethod::PATCH, token)?,
                Delete => self.parse_request(RequestMethod::DELETE, token)?,
                Linecomment | Shebang => Item::LineComment(token.into()),
                Set => self.parse_set_statement()?,
                AttributePrefix => {
                    let item = self.parse_attribute(token)?;
                    self.expect_one_of(vec![Get, Post, Put, Patch, Delete, AttributePrefix])
                        .map_err(|e| {
                            e.with_message(
                                "after attributes should come requests or more attributes",
                            )
                        })?;
                    item
                }
                Let => self.parse_let_statement()?,
                _ => {
                    unreachable!("we properly expect items at this level of the program structure, found token {token:?}")
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
        let block = self.parse_block()?;
        let url_span: Span = url.span();

        let span_next = if let Some(b) = block.as_ref() {
            b.span
        } else {
            url_span
        };

        Ok(Item::Request {
            span: token.start.to_end_of(span_next),
            method,
            endpoint: match url.kind {
                TokenKind::Url => Endpoint::Url(url.into()),
                TokenKind::Pathname => Endpoint::Pathname(url.into()),
                _ => unreachable!("we're properly expecting only url and pathname tokens here"),
            },
            block,
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

    fn parse_block(&mut self) -> Result<Option<Block<'i>>> {
        use TokenKind::*;
        let LBracket = self.peek_token().kind else {
            return Ok(None);
        };

        let span_start = self.token().start; // remember LBracket's location
        let mut token = self.token();
        let mut statements = vec![];

        while token.kind != RBracket {
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

            statements.push(header);
            token = self.token();
        }

        return Ok(Some(Block {
            statements,
            span: Span::new(span_start, token.start), // span to RBracket's location
        }));
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
                TokenKind::StringLiteral => Expression::String(header_value.into()),
                TokenKind::MultiLineStringLiteral => {
                    self.parse_multiline_string_literal(header_value)?
                }
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
            TokenKind::StringLiteral => Expression::String(token.into()),
            TokenKind::MultiLineStringLiteral => self.parse_multiline_string_literal(token)?,
            _ => unreachable!(),
        };

        Ok(Statement::Body {
            value,
            start: t.start,
        })
    }

    fn parse_expression(&mut self, start_token: Token<'i>) -> Result<Expression<'i>> {
        use TokenKind::*;

        let exp = match start_token.kind {
            Ident if self.peek_token().kind == LParen => self.parse_call_expression(start_token)?,
            Ident => Expression::Identifier(start_token.into()),
            StringLiteral => Expression::String(start_token.into()),
            MultiLineStringLiteral => self.parse_multiline_string_literal(start_token)?,
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

    fn parse_multiline_string_literal(&mut self, start_token: Token<'i>) -> Result<Expression<'i>> {
        let mut parts = vec![];
        let start = start_token.start;
        let end;
        let mut token = start_token;

        loop {
            match &token.kind {
                TokenKind::MultiLineStringLiteral
                    if self.peek_token().kind == TokenKind::DollarSignLBracket =>
                {
                    let s_literal = Expression::String(token.into());

                    parts.push(s_literal);

                    self.eat_token();

                    token = self.token();

                    parts.push(self.parse_expression(token)?);

                    token = self.token();
                }
                TokenKind::MultiLineStringLiteral if parts.is_empty() => {
                    return Ok(Expression::String(token.into()));
                }
                TokenKind::MultiLineStringLiteral => {
                    end = token.end_location();
                    parts.push(Expression::String(token.into()));
                    break;
                }
                tk => unreachable!(
                    "expecting to start parsing multiline string literals only on the {:?} token, found: {:?}",
                    TokenKind::MultiLineStringLiteral,
                    tk
                ),
            };
        }

        Ok(Expression::TemplateSringLiteral {
            span: Span::new(start, end),
            parts,
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
        let location = token.start;

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

    fn parse_let_statement(&mut self) -> Result<Item<'i>> {
        let ident = self.token();

        self.expect(TokenKind::Assign)?;
        self.eat_token();

        let token = self.token();

        Ok(Item::Let {
            identifier: ident.into(),
            value: self.parse_expression(token)?,
        })
    }
}
