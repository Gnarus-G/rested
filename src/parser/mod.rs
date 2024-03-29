pub mod ast;
mod ast_queries;
mod ast_span;
pub mod ast_visit;
pub mod error;

use ast::{Endpoint, Expression, Item, RequestMethod, Statement};

use self::ast::result::ParsedNode;
use self::ast::{Block, ExpressionList};
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

pub type Result<'source, T> = std::result::Result<T, Box<ContextualError<ParseError<'source>>>>;

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

impl<'source> From<&'source String> for ast::Program<'source> {
    fn from(s: &'source String) -> Self {
        Parser::new(s).parse()
    }
}

impl<'source> From<&'source str> for ast::Program<'source> {
    fn from(s: &'source str) -> Self {
        Parser::new(s).parse()
    }
}

#[derive(Debug)]
pub struct Parser<'i> {
    lexer: Lexer<'i>,
    token: Option<Token<'i>>,
    peeked: Option<Token<'i>>,
}

impl<'source> Parser<'source> {
    pub fn new(code: &'source str) -> Self {
        Self {
            peeked: None,
            lexer: Lexer::new(code),
            token: None,
        }
    }

    fn curr_token(&self) -> &Token<'source> {
        self.token
            .as_ref()
            .expect("self.token should be initialized at the start of parsing")
    }

    fn next_token(&mut self) -> &Token<'source> {
        self.token = match self.peeked.take() {
            Some(t) => Some(t),
            None => Some(self.lexer.next_token()),
        };
        self.curr_token()
    }

    fn peek_token(&mut self) -> &Token<'source> {
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

    pub fn parse(&mut self) -> ast::Program<'source> {
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
                    items.push(Item::Error(error));
                    self.eat_till_next_top_level_peek_token();
                }
            }

            self.next_token();
        }

        return ast::Program::new(self.lexer.input(), items);
    }

    fn parse_request(&mut self, method: RequestMethod) -> Result<'source, Item<'source>> {
        let e = Expectations::new(self);

        let endpoint = self.parse_endpoint();

        let block = self.parse_block();

        let span_next = if let Some(b) = block.as_ref() {
            b.span
        } else {
            endpoint.span()
        };

        Ok(Item::Request {
            span: e.start.to_end_of(span_next),
            method,
            endpoint,
            block,
        })
    }

    fn parse_endpoint(&mut self) -> Endpoint<'source> {
        let e = Expectations::new(self);

        self.next_token();

        let peek_kind = self.peek_token().kind;

        let endpoint = match self.curr_token().kind {
            Url => return Endpoint::Url(self.curr_token().into()),
            Pathname => return Endpoint::Pathname(self.curr_token().into()),
            Ident if peek_kind == LParen => self.parse_call_expression().into(),
            Ident => Expression::Identifier(self.curr_token().into()),
            StringLiteral => Expression::String(self.curr_token().into()),
            TemplateString { head: true, .. } => self.parse_multiline_string_literal(),
            _ => Expression::Error(
                e.expected_one_of_tokens(self.curr_token(), &[Url, Pathname, StringLiteral, Ident])
                    .into(),
            ),
        };

        Endpoint::Expr(endpoint)
    }

    fn parse_set_statement(&mut self) -> Result<'source, Item<'source>> {
        let e = Expectations::new(self);
        let identifier = match e.expect_peek(self, TokenKind::Ident) {
            Ok(i) => i.into(),
            Err(error) => {
                return Ok(Item::Set {
                    identifier: ParsedNode::Error(error.clone()),
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

    fn parse_block(&mut self) -> Option<Block<'source>> {
        let LBracket = self.peek_token().kind else {
            return None;
        };

        let span_start = self.next_token().start; // remember LBracket's location
        self.next_token();
        let mut statements: Vec<Statement<'source>> = vec![];

        while self.curr_token().kind != RBracket && self.curr_token().kind != End {
            let statement = match self.parse_statement() {
                Ok(s) => s,
                Err(error) => error.into(),
            };
            statements.push(statement);
            self.next_token();
        }

        return Some(Block {
            statements: statements.into(),
            span: Span::new(span_start, self.curr_token().start), // span to RBracket's location
        });
    }

    fn parse_statement(&mut self) -> Result<'source, Statement<'source>> {
        let e = Expectations::new(self);

        let statement = match_or_throw! { self.curr_token().kind; e; self;
            Header => self.parse_header()?,
            Body => self.parse_body()?,
            Linecomment | Shebang => Statement::LineComment(self.curr_token().into()),
            "may only declare headers or a body statement here"
        };

        Ok(statement)
    }

    fn parse_header(&mut self) -> Result<'source, Statement<'source>> {
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

    fn parse_body(&mut self) -> Result<'source, Statement<'source>> {
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

    fn parse_expression(&mut self) -> Result<'source, Expression<'source>> {
        let e = Expectations::new(self);
        let kind = self.curr_token().kind;

        let exp = match kind {
            Ident if self.peek_token().kind == LParen => self.parse_call_expression().into(),
            Ident => Expression::Identifier(self.curr_token().into()),
            StringLiteral => Expression::String(self.curr_token().into()),
            Boolean => Expression::Bool((
                self.curr_token().span(),
                self.curr_token()
                    .text
                    .parse()
                    .expect("failed to parse as a boolean"),
            )),
            Number => Expression::Number((
                self.curr_token().span(),
                self.curr_token()
                    .text
                    .parse()
                    .expect("failed to parse as an unsigned int"),
            )),
            TemplateString { head: true, .. } => self.parse_multiline_string_literal(),
            LBracket => self.parse_object_literal(),
            LSquare => self.parse_array_literal(),
            Null => Expression::Null(self.curr_token().span()),
            _ => {
                return Err(e
                    .expected_one_of_tokens(
                        self.curr_token(),
                        &[
                            Ident,
                            StringLiteral,
                            Boolean,
                            Number,
                            LBracket,
                            LSquare,
                            Null,
                        ],
                    )
                    .into());
            }
        };

        Ok(exp)
    }

    fn parse_object_literal(&mut self) -> ast::Expression<'source> {
        let e = Expectations::new(self);

        if self.peek_token().kind == RBracket {
            self.next_token();
            return Expression::EmptyObject(self.span_from(e.start));
        }

        let mut entries = vec![];

        debug_assert_eq!(self.curr_token().kind, LBracket);

        self.next_token();

        while self.curr_token().kind != RBracket && self.curr_token().kind != End {
            if self.curr_token().is(Linecomment) {
                self.next_token();
                continue;
            }

            let entry = self.parse_object_property();

            entries.push(ParsedNode::Ok(entry));

            if !self.peek_token().is(RBracket) && !self.peek_token().is(Linecomment) {
                let e = Expectations::new(self);
                if let Err(e) = e.expect_peek(self, Comma) {
                    entries.push(ParsedNode::Error(e));
                }
            }

            self.next_token();
        }

        let last_token = self.curr_token();
        debug_assert!(last_token.kind == RBracket || last_token.kind == End);

        let span = e.start.to_end_of(self.curr_token().span());

        Expression::Object((span, entries.into()))
    }

    fn parse_array_literal(&mut self) -> ast::Expression<'source> {
        let e = Expectations::new(self);
        if self.peek_token().kind == RSquare {
            self.next_token();
            return Expression::EmptyArray(self.span_from(e.start));
        }

        let l_square = self.curr_token().clone();
        let list = self.parse_expression_list(&l_square, RSquare);

        Expression::Array(list)
    }

    fn parse_object_property(&mut self) -> ast::ObjectEntry<'source> {
        let e = Expectations::new(self);

        let key = match self.parse_object_key() {
            Ok(k) => ParsedNode::Ok(k),
            Err(error) => ParsedNode::Error(error),
        };

        if let Err(e) = e.expect_peek(self, TokenKind::Colon) {
            return ast::ObjectEntry::new(key, Expression::Error(e));
        }

        self.next_token();

        let entry = match self.parse_expression() {
            Ok(exp) => ast::ObjectEntry::new(key, exp),
            Err(error) => return ast::ObjectEntry::new(key, Expression::Error(error)),
        };

        entry
    }

    fn parse_object_key(&mut self) -> Result<'source, ast::StringLiteral<'source>> {
        let e = Expectations::new(self);

        let key_token = self.curr_token();

        let key = match_or_throw! { key_token.kind; e; self;
            Ident | StringLiteral => key_token.into(),
        };

        Ok(key)
    }

    fn parse_call_expression(&mut self) -> Result<'source, Expression<'source>> {
        let identifier = self.curr_token().into();

        let l_paren = self.next_token().clone();

        debug_assert_eq!(l_paren.kind, LParen);

        Ok(Expression::Call(ast::CallExpr {
            identifier,
            arguments: self.parse_expression_list(&l_paren, RParen),
        }))
    }

    fn parse_multiline_string_literal(&mut self) -> Expression<'source> {
        let mut parts = vec![];
        let e = Expectations::new(self);
        let end;

        loop {
            let kind = self.curr_token().kind;

            match kind {
                TemplateString {
                    head: true,
                    tail: true,
                } => {
                    return Expression::String(self.curr_token().into());
                }
                TemplateString { tail: true, .. } => {
                    end = self.curr_token().end_position();
                    parts.push(Expression::String(self.curr_token().into()));
                    break;
                }
                TemplateString { .. } => {
                    let s_literal = Expression::String(self.curr_token().into());
                    parts.push(s_literal);
                }
                DollarSignLBracket
                    if matches!(self.peek_token().kind, TemplateString { head: true, .. }) =>
                {
                    self.next_token();
                    parts.push(match self.parse_expression() {
                        Ok(expr) => expr,
                        Err(e) => Expression::Error(e),
                    })
                }
                DollarSignLBracket if matches!(self.peek_token().kind, TemplateString { .. }) => {}
                DollarSignLBracket => {
                    self.next_token();

                    parts.push(match self.parse_expression() {
                        Ok(e) => e,
                        Err(e) => Expression::Error(e),
                    });
                }
                _ => {
                    end = self.curr_token().end_position();
                    break;
                }
            };

            self.next_token();
        }

        Expression::TemplateSringLiteral {
            span: Span::new(e.start, end),
            parts: parts.into(),
        }
    }

    fn parse_attribute(&mut self) -> Result<'source, Item<'source>> {
        let e = Expectations::new(self);

        let identifier = e.expect_peek(self, TokenKind::Ident)?.into();

        if self.peek_token().kind != TokenKind::LParen {
            return Ok(Item::Attribute {
                location: e.start,
                identifier,
                arguments: None,
            });
        }

        let l_paren = self.next_token().clone();

        debug_assert!(l_paren.kind == LParen);

        Ok(Item::Attribute {
            location: e.start,
            identifier,
            arguments: Some(self.parse_expression_list(&l_paren, RParen)),
        })
    }

    fn parse_expression_list(
        &mut self,
        start_token: &Token,
        end: TokenKind,
    ) -> ExpressionList<'source> {
        let mut expressions = vec![];
        let start_of_expressions_list = start_token.start;

        debug_assert!(matches!(self.curr_token().kind, LSquare | LParen));

        self.next_token();

        while self.curr_token().kind != end && self.curr_token().kind != TokenKind::End {
            if self.curr_token().is(Linecomment) {
                self.next_token();
                continue;
            }

            let exp = match self.parse_expression() {
                Ok(exp) => exp,
                Err(error) => Expression::Error(error),
            };

            expressions.push(exp);

            if !self.peek_token().is(end) && !self.peek_token().is(Linecomment) {
                let e = Expectations::new(self);
                if let Err(e) = e.expect_peek(self, Comma) {
                    expressions.push(Expression::Error(e));
                }
            }

            self.next_token();
        }

        let last_token = self.curr_token();
        debug_assert!(last_token.kind == end || last_token.kind == End);

        ExpressionList {
            span: Span {
                start: start_of_expressions_list,
                end: last_token.span().end,
            },
            exprs: expressions.into(),
        }
    }

    fn parse_let_statement(&mut self) -> Result<'source, Item<'source>> {
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
