pub mod ast;
mod ast_queries;
mod ast_span;
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

pub type Result<'source, T> = std::result::Result<T, ContextualError<ParseError<'source>>>;

trait TokenCheck {
    fn is_one_of(&self, kinds: &[TokenKind]) -> bool;
    fn is(&self, kind: TokenKind) -> bool;
}

impl<'source> TokenCheck for Token<'source> {
    fn is_one_of(&self, kinds: &[TokenKind]) -> bool {
        kinds.contains(&self.kind)
    }

    fn is(&self, kind: TokenKind) -> bool {
        self.kind == kind
    }
}

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

    fn eat_till_next_top_level_peek_token(&mut self) {
        loop {
            let is_top_level_token_ahead = match self.peek_token().kind {
                Get | Post | Put | Patch | Delete | Set | AttributePrefix | Let | End => true,
                _ => false,
            };

            if is_top_level_token_ahead {
                break;
            }

            self.next_token();
        }
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

                    if let Ok(_) = item {
                        let valid_after_attribute =
                            vec![Get, Post, Put, Patch, Delete, AttributePrefix, Linecomment];

                        if let Err(err) = self.expect_peek_one_of(valid_after_attribute) {
                            errors.push(err.with_message(
                                "after attributes should come requests or more attributes",
                            ));
                            continue;
                        }
                    }

                    item
                }
                Let => self.parse_let_statement(),
                _ => match self.parse_expression() {
                    Ok(exp) => Ok(Item::Expr(exp)),
                    Err(err) => Err(err),
                },
            };

            match result {
                Ok(item) => program.items.push(item),
                Err(error) => {
                    errors.push(error);
                    self.eat_till_next_top_level_peek_token();
                }
            }

            self.next_token();
        }

        if errors.is_empty() {
            return Ok(program);
        }

        return Err(ParserErrors::new(errors, program));
    }

    fn parse_request(&mut self, method: RequestMethod) -> Result<'i, Item<'i>> {
        let start = self.curr_token().start;

        let url = self.next_expecting_one_of(vec![Url, Pathname])?;

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

    fn parse_set_statement(&mut self) -> Result<'i, Item<'i>> {
        let identifier = self.expect_peek(TokenKind::Ident)?.into();

        self.next_expecting_one_of(vec![
            TokenKind::Ident,
            TokenKind::StringLiteral,
            TokenKind::MultiLineStringLiteral,
        ])?;

        Ok(Item::Set {
            identifier,
            value: self.parse_expression()?,
        })
    }

    fn parse_block(&mut self) -> Result<'i, Option<Block<'i>>> {
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

    fn parse_header(&mut self) -> Result<'i, Statement<'i>> {
        let header_name = self.expect_peek(TokenKind::StringLiteral)?.into();

        self.next_expecting_one_of(vec![
            TokenKind::StringLiteral,
            TokenKind::Ident,
            TokenKind::MultiLineStringLiteral,
        ])?;

        let value = self.parse_expression()?;

        Ok(Statement::Header {
            name: header_name,
            value,
        })
    }

    fn parse_body(&mut self) -> Result<'i, Statement<'i>> {
        let start = self.curr_token().start;

        self.next_token();

        let value = self.parse_expression()?;

        Ok(Statement::Body { value, start })
    }

    fn parse_expression(&mut self) -> Result<'i, Expression<'i>> {
        let kind = self.curr_token().kind;

        let exp = match_or_throw! { kind; self;
            Ident if self.peek_token().kind == LParen => self.parse_call_expression()?,
            Ident => Expression::Identifier(self.curr_token().into()),
            StringLiteral => Expression::String(self.curr_token().into()),
            Boolean => Expression::Bool(self.curr_token().into()),
            Number => Expression::Number(self.curr_token().into()),
            MultiLineStringLiteral => self.parse_multiline_string_literal()?,
            LBracket | LSquare => self.parse_json_like()?,
            Null => Expression::Null(self.curr_token().span()),
        };

        Ok(exp)
    }

    fn parse_json_like(&mut self) -> Result<'i, Expression<'i>> {
        let start_token = self.curr_token();
        let start = start_token.start;

        let object = match start_token.kind {
            LBracket => {
                if self.peek_token().kind == RBracket {
                    self.next_token();
                    return Ok(Expression::EmptyObject(self.span_from(start)));
                }

                let mut fields = BTreeMap::new();

                while self.peek_token().kind != RBracket {
                    if self.peek_token().is(Linecomment) {
                        self.next_token();
                        continue;
                    }

                    let (key, value) = self.parse_object_property()?;
                    fields.insert(key, value);

                    if !self.peek_token().is(RBracket) {
                        self.expect_peek(Comma)?;
                    }
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

                let mut list = vec![];

                while self.peek_token().kind != RSquare {
                    if self.peek_token().is(Linecomment) {
                        self.next_token();
                        continue;
                    }

                    self.next_token();
                    list.push(self.parse_json_like()?);

                    if !self.peek_token().is(RSquare) {
                        self.expect_peek(Comma)?;
                    }
                }

                self.next_token();

                let span = start.to_end_of(self.curr_token().span());

                Expression::Array((span, list))
            }
            _ => self.parse_expression()?,
        };

        Ok(object)
    }

    fn parse_object_property(&mut self) -> Result<'i, (&'i str, Expression<'i>)> {
        let key_token = self.next_token();

        let key = match_or_throw! { key_token.kind; self;
            Ident => key_token.text,
            StringLiteral => ast::StringLiteral::from(key_token).value
        };

        self.expect_peek(TokenKind::Colon)?;
        self.next_token();

        return Ok((key, self.parse_json_like()?));
    }

    fn parse_call_expression(&mut self) -> Result<'i, Expression<'i>> {
        let identifier = self.curr_token().into();
        self.next_token();

        self.next_token();

        let mut arguments = vec![];

        while self.curr_token().kind != TokenKind::RParen {
            let exp = self.parse_expression()?;
            arguments.push(exp);
            self.next_token();
        }

        Ok(Expression::Call {
            identifier,
            arguments,
        })
    }

    fn parse_multiline_string_literal(&mut self) -> Result<'i, Expression<'i>> {
        let mut parts = vec![];
        let start = self.curr_token().start;
        let end;

        loop {
            let kind = self.curr_token().kind;

            match_or_throw! { kind; self;
                MultiLineStringLiteral
                    if self.peek_token().kind == TokenKind::DollarSignLBracket =>
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

    fn next_expecting_one_of(&mut self, expected_kinds: Vec<TokenKind>) -> Result<'i, &Token<'i>> {
        self.expect_peek_one_of(expected_kinds)
            .map(|_| self.next_token())
    }

    fn expect_peek_one_of(&mut self, expected_kinds: Vec<TokenKind>) -> Result<'i, ()> {
        if self.peek_token().is_one_of(&expected_kinds) {
            return Ok(());
        }
        let con = self.error();

        let error = con.expected_one_of_tokens(&self.next_token(), expected_kinds);

        Err(error)
    }

    fn expect_peek(&mut self, expected_kind: TokenKind) -> Result<'i, &Token<'i>> {
        if self.peek_token().is(expected_kind) {
            return Ok(self.next_token());
        }

        let error = self
            .error()
            .expected_token(&self.next_token(), expected_kind);

        Err(error)
    }

    fn error(&self) -> ParseErrorConstructor<'i> {
        ParseErrorConstructor::new(self.lexer.input())
    }

    fn parse_attribute(&mut self) -> Result<'i, Item<'i>> {
        let location = self.curr_token().start;

        let identifier = self.expect_peek(TokenKind::Ident)?.into();

        let mut params = vec![];

        if let TokenKind::LParen = self.peek_token().kind {
            self.next_token();

            self.next_token();
            while self.curr_token().kind != TokenKind::RParen {
                let exp = self.parse_expression()?;

                params.push(exp);

                self.next_token();
            }
        }

        Ok(Item::Attribute {
            location,
            identifier,
            parameters: params,
        })
    }

    fn parse_let_statement(&mut self) -> Result<'i, Item<'i>> {
        let identifier = self.next_token().into();

        self.expect_peek(TokenKind::Assign)?;

        self.next_token();

        Ok(Item::Let {
            identifier,
            value: self.parse_expression()?,
        })
    }
}
