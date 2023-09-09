pub mod ast;
mod ast_errors;
mod ast_queries;
mod ast_span;
pub mod error;

use std::collections::BTreeMap;

use ast::{Endpoint, Expression, Item, RequestMethod, Statement};

use self::ast::{Arguments, Block};
use self::error::{Expectations, ParseError};

use crate::error_meta::ContextualError;
use crate::lexer::locations::{GetSpan, Position, Span};
use crate::lexer::TokenKind;
use crate::lexer::TokenKind::*;
use crate::lexer::{Lexer, Token};

macro_rules! match_or_throw {
    ($expression:expr; $expectations:ident; $self:ident; $( $( $pattern:ident )|+ $( if $guard: expr )? => $arm:expr $(,)? )+ $( ,$message:literal )? ) => {
        match $expression {
            $(
                $( TokenKind::$pattern )|+ $( if $guard )? => $arm,
            )+
            _ => return Err($expectations.expected_one_of_tokens($self.curr_token(), &[$( $( $pattern ),+ ),+])$(.with_message($message))?.into())
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
        self.token
            .as_ref()
            .expect("self.token should be initialized at the start of parsing")
    }

    fn next_token(&mut self) -> &Token<'i> {
        self.token = match self.peeked.take() {
            Some(t) => Some(t),
            None => Some(self.lexer.next_token()),
        };
        self.curr_token()
    }

    fn peek_token(&mut self) -> &Token<'i> {
        self.peeked.get_or_insert_with(|| self.lexer.next_token())
    }

    fn eat_till_next_top_level_peek_token(&mut self) {
        loop {
            let is_top_level_token_ahead = matches!(
                self.peek_token().kind,
                Get | Post | Put | Patch | Delete | Set | AttributePrefix | Let | End
            );

            if is_top_level_token_ahead {
                break;
            }

            self.next_token();
        }
    }

    fn span_from(&self, start: Position) -> Span {
        start.to_end_of(self.curr_token().span())
    }

    pub fn parse(&mut self) -> ast::Program<'i> {
        let mut items: Vec<_> = vec![];

        use crate::lexer::TokenKind::*;

        self.next_token();

        while self.curr_token().kind != End {
            let result: std::result::Result<ast::Item<'_>, _> = match self.curr_token().kind {
                Get => self.parse_request(RequestMethod::GET),
                Post => self.parse_request(RequestMethod::POST),
                Put => self.parse_request(RequestMethod::PUT),
                Patch => self.parse_request(RequestMethod::PATCH),
                Delete => self.parse_request(RequestMethod::DELETE),
                Linecomment | Shebang => Ok(Item::LineComment(self.curr_token().into())),
                Set => self.parse_set_statement(),
                AttributePrefix => {
                    let e = Expectations::new(self);
                    let item = self.parse_attribute();

                    if item.is_ok() {
                        let valid_after_attribute =
                            [Get, Post, Put, Patch, Delete, AttributePrefix, Linecomment];

                        if let Err(err) = e.expect_peek_one_of(self, &valid_after_attribute) {
                            items.push(
                                err.with_message(
                                    "after attributes should come requests or more attributes",
                                )
                                .into(),
                            );
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
                Ok(item) => items.push(item),
                Err(error) => {
                    items.push(error.into());
                    self.eat_till_next_top_level_peek_token();
                }
            }

            self.next_token();
        }

        return ast::Program { items };
    }

    fn parse_request(&mut self, method: RequestMethod) -> Result<'i, Item<'i>> {
        let e = Expectations::new(self);

        let url = self.next_token();

        let endpoint = match_or_throw! { url.kind; e; self;
            Url => Endpoint::Url(url.into()),
            Pathname => Endpoint::Pathname(url.into()),
            "expecting only a url and pathname here"
        };

        let url_span: Span = url.span();

        let block = self.parse_block();

        let span_next = if let Some(b) = block.as_ref() {
            b.span
        } else {
            url_span
        };

        Ok(Item::Request {
            span: e.start.to_end_of(span_next),
            method,
            endpoint,
            block,
        })
    }

    fn parse_set_statement(&mut self) -> Result<'i, Item<'i>> {
        let e = Expectations::new(self);
        let identifier = match e.expect_peek(self, TokenKind::Ident) {
            Ok(i) => i.into(),
            Err(error) => {
                return Ok(Item::Set {
                    identifier: ast::TokenNode::Error(error.clone().into()),
                    value: Expression::Error(error),
                })
            }
        };

        self.next_token();

        Ok(Item::Set {
            value: match self.parse_expression() {
                Ok(expr) => expr,
                Err(error) => {
                    return Ok(Item::Set {
                        identifier,
                        value: Expression::Error(error),
                    })
                }
            },
            identifier,
        })
    }

    fn parse_block(&mut self) -> Option<Block<'i>> {
        let LBracket = self.peek_token().kind else {
            return None;
        };

        let span_start = self.next_token().start; // remember LBracket's location
        self.next_token();
        let mut statements: Vec<Statement<'i>> = vec![];

        while self.curr_token().kind != RBracket && self.curr_token().kind != End {
            let statement = match self.parse_statement() {
                Ok(s) => s,
                Err(error) => error.into(),
            };
            statements.push(statement);
            self.next_token();
        }

        return Some(Block {
            statements,
            span: Span::new(span_start, self.curr_token().start), // span to RBracket's location
        });
    }

    fn parse_statement(&mut self) -> Result<'i, Statement<'i>> {
        let e = Expectations::new(self);

        let statement = match_or_throw! { self.curr_token().kind; e; self;
            Header => self.parse_header()?,
            Body => self.parse_body()?,
            Linecomment | Shebang => Statement::LineComment(self.curr_token().into()),
            "may only declare headers or a body statement here"
        };

        Ok(statement)
    }

    fn parse_header(&mut self) -> Result<'i, Statement<'i>> {
        let e = Expectations::new(self);
        let header_name = e
            .expect_peek(self, TokenKind::StringLiteral)
            .map(|t| t.into())
            .into();

        self.next_token();

        let value = match self.parse_expression() {
            Ok(e) => e,
            Err(error) => {
                return Ok(Statement::Header {
                    name: header_name,
                    value: Expression::Error(error),
                })
            }
        };

        Ok(Statement::Header {
            name: header_name,
            value,
        })
    }

    fn parse_body(&mut self) -> Result<'i, Statement<'i>> {
        let start = self.curr_token().start;

        self.next_token();

        let value = match self.parse_expression() {
            Ok(e) => e,
            Err(error) => {
                return Ok(Statement::Body {
                    value: Expression::Error(error),
                    start,
                })
            }
        };

        Ok(Statement::Body { value, start })
    }

    fn parse_expression(&mut self) -> Result<'i, Expression<'i>> {
        let e = Expectations::new(self);
        let kind = self.curr_token().kind;

        let exp = match_or_throw! { kind; e; self;
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
        let e = Expectations::new(self);

        let object = match start_token.kind {
            LBracket => {
                if self.peek_token().kind == RBracket {
                    self.next_token();
                    return Ok(Expression::EmptyObject(self.span_from(e.start)));
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
                        e.expect_peek(self, Comma)?;
                    }
                }

                self.next_token();

                let span = e.start.to_end_of(self.curr_token().span());

                Expression::Object((span, fields))
            }
            LSquare => {
                if self.peek_token().kind == RSquare {
                    self.next_token();
                    return Ok(Expression::EmptyArray(self.span_from(e.start)));
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
                        e.expect_peek(self, Comma)?;
                    }
                }

                self.next_token();

                let span = e.start.to_end_of(self.curr_token().span());

                Expression::Array((span, list))
            }
            _ => self.parse_expression()?,
        };

        Ok(object)
    }

    fn parse_object_property(&mut self) -> Result<'i, (&'i str, Expression<'i>)> {
        let e = Expectations::new(self);
        let key_token = self.next_token();

        let key = match_or_throw! { key_token.kind; e; self;
            Ident => key_token.text,
            StringLiteral => ast::StringLiteral::from(key_token).value
        };

        e.expect_peek(self, TokenKind::Colon)?;
        self.next_token();

        Ok((key, self.parse_json_like()?))
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
        let e = Expectations::new(self);
        let end;

        loop {
            let kind = self.curr_token().kind;

            match_or_throw! { kind; e; self;
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
                    end = self.curr_token().end_position();
                    break;
                },
                MultiLineStringLiteral if parts.is_empty() => {
                    return Ok(Expression::String(self.curr_token().into()));
                }
                MultiLineStringLiteral => {
                    end = self.curr_token().end_position();
                    parts.push(Expression::String(self.curr_token().into()));
                    break;
                }
            };

            self.next_token();
        }

        Ok(Expression::TemplateSringLiteral {
            span: Span::new(e.start, end),
            parts,
        })
    }

    fn parse_attribute(&mut self) -> Result<'i, Item<'i>> {
        let e = Expectations::new(self);

        let identifier = e.expect_peek(self, TokenKind::Ident)?.into();

        if self.peek_token().kind != TokenKind::LParen {
            return Ok(Item::Attribute {
                location: e.start,
                identifier,
                parameters: None,
            });
        }

        self.next_token();

        Ok(Item::Attribute {
            location: e.start,
            identifier,
            parameters: Some(self.parse_parameters()),
        })
    }

    fn parse_parameters(&mut self) -> Arguments<'i> {
        let mut params = vec![];
        let params_start = self.curr_token().start;

        self.next_token();

        while self.curr_token().kind != TokenKind::RParen
            && self.curr_token().kind != TokenKind::End
        {
            let exp = match self.parse_expression() {
                Ok(exp) => exp,
                Err(error) => Expression::Error(error),
            };

            params.push(exp);

            self.next_token();
        }

        let last_token = self.curr_token(); // should be RParen

        Arguments {
            span: Span {
                start: params_start,
                end: last_token.span().end,
            },
            parameters: params,
        }
    }

    fn parse_let_statement(&mut self) -> Result<'i, Item<'i>> {
        let e = Expectations::new(self);
        let identifier = self.next_token().into();

        e.expect_peek(self, TokenKind::Assign)?;

        self.next_token();

        Ok(Item::Let {
            value: match self.parse_expression() {
                Ok(e) => e,
                Err(error) => {
                    return Ok(Item::Let {
                        identifier,
                        value: Expression::Error(error),
                    })
                }
            },
            identifier,
        })
    }
}
