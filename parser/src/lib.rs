pub mod ast;
pub mod error;

use std::collections::BTreeMap;

use crate::ast::{Endpoint, Expression, Item, Program, RequestMethod, Statement};
use ast::Block;
use error::ParserErrors;
use error_meta::ContextualError;
use lexer::{
    locations::{GetSpan, Location, Span},
    Lexer, Token, TokenKind,
};

use self::error::{ParseError, ParseErrorConstructor};

use TokenKind::*;

macro_rules! match_or_throw {
    ($expression:expr; $self:ident; $( $( $pattern:ident )|+ $( if $guard: expr )? => $arm:expr $(,)? )+ $( ,$message:literal )? ) => {
        match $expression {
            $(
                $( TokenKind::$pattern )|+ $( if $guard )? => $arm,
            )+
            _ => return Err($self.error().expected_one_of_tokens($self.curr_token(), vec![$( $( $pattern ),+ ),+])$(.with_message($message))?)
        }
    };
}

pub type Result<T> = std::result::Result<T, ContextualError<ParseError>>;

#[derive(Debug)]
pub struct Parser<'i> {
    lexer: Lexer<'i>,
    token: Option<Token<'i>>,
    peeked: Option<Token<'i>>,
}

impl<'i> Parser<'i> {
    pub fn new(code: &'i str) -> Self {
        Self {
            peeked: None,
            lexer: Lexer::new(code),
            token: None,
        }
    }

    fn curr_token(&self) -> &Token<'i> {
        &self
            .token
            .as_ref()
            .expect("self.token should be initialized at the start of parsing")
    }

    fn next_token(&mut self) -> &Token<'i> {
        self.token = match self.peeked.take() {
            Some(t) => Some(t),
            None => Some(self.lexer.next()),
        };
        self.curr_token()
    }

    fn peek_token(&mut self) -> &Token<'i> {
        self.peeked.get_or_insert_with(|| self.lexer.next())
    }

    fn span_from(&self, start: Location) -> Span {
        start.to_end_of(self.curr_token().span())
    }

    pub fn parse(&mut self) -> std::result::Result<Program<'i>, ParserErrors<'i>> {
        let mut program = Program::new();
        let mut errors = vec![];

        use lexer::TokenKind::*;

        self.next_token();

        while self.curr_token().kind != End {
            let result = match self.curr_token().kind {
                Get => self.parse_request(RequestMethod::GET),
                Post => self.parse_request(RequestMethod::POST),
                Put => self.parse_request(RequestMethod::PUT),
                Patch => self.parse_request(RequestMethod::PATCH),
                Delete => self.parse_request(RequestMethod::DELETE),
                Linecomment | Shebang => Ok(Item::LineComment(self.curr_token().into())),
                Set => self.parse_set_statement(),
                AttributePrefix => {
                    let item = self.parse_attribute();
                    match item {
                        Ok(_) => {
                            let expectation = self
                                .expect_one_of(vec![Get, Post, Put, Patch, Delete, AttributePrefix])
                                .map_err(|e| {
                                    e.with_message(
                                        "after attributes should come requests or more attributes",
                                    )
                                });

                            match expectation {
                                Ok(_) => item,
                                Err(err) => Err(err),
                            }
                        }
                        err => err,
                    }
                }
                Let => self.parse_let_statement(),
                _ => match self.parse_expression() {
                    Ok(exp) => Ok(Item::Expr(exp)),
                    Err(err) => Err(err),
                },
            };

            match result {
                Ok(item) => program.items.push(item),
                Err(error) => errors.push(error),
            };

            self.next_token();
        }

        if errors.is_empty() {
            return Ok(program);
        }

        return Err(ParserErrors::new(errors, program));
    }

    fn parse_request(&mut self, method: RequestMethod) -> Result<Item<'i>> {
        let start = self.curr_token().start;

        self.expect_one_of(vec![Url, Pathname])?;

        let url = self.next_token();
        let endpoint = match_or_throw! { url.kind; self;
            Url => Endpoint::Url(url.into()),
            Pathname => Endpoint::Pathname(url.into()),
            "expecting only a url and pathname here"
        };
        let url_span: Span = url.span();

        let block = self.parse_block()?;

        let span_next = if let Some(b) = block.as_ref() {
            b.span
        } else {
            url_span
        };

        Ok(Item::Request {
            span: start.to_end_of(span_next),
            method,
            endpoint,
            block,
        })
    }

    fn parse_set_statement(&mut self) -> Result<Item<'i>> {
        self.expect(TokenKind::Ident)?;

        let identifier = self.next_token().into();

        self.expect_one_of(vec![
            TokenKind::Ident,
            TokenKind::StringLiteral,
            TokenKind::MultiLineStringLiteral,
        ])?;

        self.next_token();

        Ok(Item::Set {
            identifier,
            value: self.parse_expression()?,
        })
    }

    fn parse_block(&mut self) -> Result<Option<Block<'i>>> {
        let LBracket = self.peek_token().kind else {
            return Ok(None);
        };

        let span_start = self.next_token().start; // remember LBracket's location
        self.next_token();
        let mut statements = vec![];

        while self.curr_token().kind != RBracket && self.curr_token().kind != End {
            let statement = match_or_throw! { self.curr_token().kind; self;
                Header => self.parse_header()?,
                Body => self.parse_body()?,
                Linecomment | Shebang => Statement::LineComment(self.curr_token().into()),
                "may only declare headers or a body statement here"
            };
            statements.push(statement);
            self.next_token();
        }

        return Ok(Some(Block {
            statements,
            span: Span::new(span_start, self.curr_token().start), // span to RBracket's location
        }));
    }

    fn parse_header(&mut self) -> Result<Statement<'i>> {
        self.expect(TokenKind::StringLiteral)?;

        let header_name = self.next_token().into();

        self.expect_one_of(vec![
            TokenKind::StringLiteral,
            TokenKind::Ident,
            TokenKind::MultiLineStringLiteral,
        ])?;

        self.next_token();

        let value = self.parse_expression()?;

        Ok(Statement::Header {
            name: header_name,
            value,
        })
    }

    fn parse_body(&mut self) -> Result<Statement<'i>> {
        let start = self.curr_token().start;

        self.next_token();

        let value = self.parse_expression()?;

        Ok(Statement::Body { value, start })
    }

    fn parse_expression(&mut self) -> Result<Expression<'i>> {
        let kind = self.curr_token().kind;

        let exp = match_or_throw! { kind; self;
            Ident if self.peek_token().kind == LParen => self.parse_call_expression()?,
            Ident => {Expression::Identifier(self.curr_token().into())},
            StringLiteral => Expression::String(self.curr_token().into()),
            Boolean => Expression::Bool(self.curr_token().into()),
            Number => Expression::Number(self.curr_token().into()),
            MultiLineStringLiteral => self.parse_multiline_string_literal()?,
            LBracket | LSquare => self.parse_json_like()?
        };

        Ok(exp)
    }

    fn parse_json_like(&mut self) -> Result<Expression<'i>> {
        let start_token = self.curr_token();
        let start = start_token.start;

        let object = match start_token.kind {
            LBracket => {
                if self.peek_token().kind == RBracket {
                    self.next_token();
                    return Ok(Expression::EmptyObject(self.span_from(start)));
                }

                let mut fields = BTreeMap::new();

                let (key, value) = self.parse_object_property()?;

                fields.insert(key, value);

                while self.peek_token().kind != RBracket {
                    self.expect(Comma)?;
                    self.next_token();

                    let (key, value) = self.parse_object_property()?;

                    fields.insert(key, value);
                }

                self.next_token();

                let span = start.to_end_of(self.curr_token().span());

                Expression::Object((span, fields))
            }
            LSquare => {
                if self.peek_token().kind == RSquare {
                    self.next_token();
                    return Ok(Expression::EmptyArray(self.span_from(start)));
                }

                self.next_token();

                let mut list = vec![];

                list.push(self.parse_json_like()?);

                while self.peek_token().kind != RSquare {
                    self.expect(Comma)?;
                    self.next_token();
                    self.next_token();

                    list.push(self.parse_json_like()?);
                }

                self.next_token();

                let span = start.to_end_of(self.curr_token().span());

                Expression::Array((span, list))
            }
            _ => self.parse_expression()?,
        };

        Ok(object)
    }

    fn parse_object_property(&mut self) -> Result<(&'i str, Expression<'i>)> {
        self.expect(TokenKind::Ident)?;
        let ident = self.next_token().text;

        self.expect(TokenKind::Colon)?;
        self.next_token();
        self.next_token();

        return Ok((ident, self.parse_json_like()?));
    }

    fn parse_call_expression(&mut self) -> Result<Expression<'i>> {
        let identifier = self.curr_token().into();
        self.next_token();

        let mut token = self.next_token();

        let mut arguments = vec![];

        while token.kind != TokenKind::RParen {
            let exp = self.parse_expression()?;
            arguments.push(exp);
            token = self.next_token();
        }

        Ok(Expression::Call {
            identifier,
            arguments,
        })
    }

    fn parse_multiline_string_literal(&mut self) -> Result<Expression<'i>> {
        let mut parts = vec![];
        let start = self.curr_token().start;
        let end;

        loop {
            let c_kind = self.curr_token().kind;
            let p_kind = self.peek_token().kind;

            match_or_throw! { c_kind; self;
                MultiLineStringLiteral
                    if p_kind == TokenKind::DollarSignLBracket =>
                {
                    let s_literal = Expression::String(self.curr_token().into());

                    parts.push(s_literal);

                    self.next_token();

                    self.next_token();

                    parts.push(self.parse_expression()?);
                }
                End => {
                    end = self.curr_token().end_location();
                    break;
                },
                MultiLineStringLiteral if parts.is_empty() => {
                    return Ok(Expression::String(self.curr_token().into()));
                }
                MultiLineStringLiteral => {
                    end = self.curr_token().end_location();
                    parts.push(Expression::String(self.curr_token().into()));
                    break;
                }
            };

            self.next_token();
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
            .expected_one_of_tokens(&self.next_token(), expected_kinds);

        Err(error)
    }

    fn expect(&mut self, expected_kind: TokenKind) -> Result<()> {
        let ahead = self.peek_token();

        if ahead.kind == expected_kind {
            return Ok(());
        }

        let error = self
            .error()
            .expected_token(&self.next_token(), expected_kind);

        Err(error)
    }

    fn error(&self) -> ParseErrorConstructor<'i> {
        ParseErrorConstructor::new(self.lexer.input())
    }

    fn parse_attribute(&mut self) -> Result<Item<'i>> {
        let location = self.curr_token().start;

        self.expect(TokenKind::Ident)?;

        let identifier = self.next_token().into();

        let mut params = vec![];

        if let TokenKind::LParen = self.peek_token().kind {
            self.next_token();

            let mut token = self.next_token();
            while token.kind != TokenKind::RParen {
                let exp = self.parse_expression()?;

                params.push(exp);

                token = self.next_token();
            }
        }

        Ok(Item::Attribute {
            location,
            identifier,
            parameters: params,
        })
    }

    fn parse_let_statement(&mut self) -> Result<Item<'i>> {
        let identifier = self.next_token().into();

        self.expect(TokenKind::Assign)?;
        self.next_token();

        self.next_token();

        Ok(Item::Let {
            identifier,
            value: self.parse_expression()?,
        })
    }
}
