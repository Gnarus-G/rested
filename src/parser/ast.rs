use std::fmt::Display;

use serde::Serialize;

use crate::{
    error_meta::ContextualError,
    lexer::{
        locations::{GetSpan, Position, Span},
        Token,
    },
    utils::OneOf,
};

use self::result::ParsedNode;

use super::error::ParseError;

type Error<'source> = ContextualError<ParseError<'source>>;

#[derive(Debug, PartialEq, Serialize)]
pub struct Program<'i> {
    pub source: &'i str,
    pub items: Box<[Item<'i>]>,
}

impl<'i> Program<'i> {
    pub fn new(source: &'i str, items: Vec<Item<'i>>) -> Self {
        Self {
            source,
            items: items.into(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Literal<'i> {
    pub value: &'i str,
    pub span: Span,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct StringLiteral<'source> {
    pub raw: &'source str,
    pub value: &'source str,
    pub span: Span,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Block<'source> {
    pub statements: Box<[Statement<'source>]>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Request<'source> {
    pub method: RequestMethod,
    pub endpoint: Endpoint<'source>,
    pub block: Option<Block<'source>>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct VariableDeclaration<'source> {
    pub identifier: ParsedNode<'source, Token<'source>>,
    pub value: Expression<'source>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ConstantDeclaration<'source> {
    pub identifier: ParsedNode<'source, Token<'source>>,
    pub value: Expression<'source>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Attribute<'source> {
    pub location: Position,
    pub identifier: ParsedNode<'source, Token<'source>>,
    pub arguments: Option<ExpressionList<'source>>,
}

type Comment<'source> = Literal<'source>;

#[derive(Debug, PartialEq, Serialize)]
pub enum Item<'source> {
    Set(ConstantDeclaration<'source>),
    Let(VariableDeclaration<'source>),
    LineComment(Comment<'source>),
    Request(Request<'source>),
    Expr(Expression<'source>),
    Attribute(Attribute<'source>),
    Error(Box<Error<'source>>),
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize)]
pub enum RequestMethod {
    GET,
    POST,
    DELETE,
    PATCH,
    PUT,
}

impl Display for RequestMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Statement<'i> {
    Header {
        name: ParsedNode<'i, StringLiteral<'i>>,
        value: Expression<'i>,
    },
    Body {
        value: Expression<'i>,
        start: Position,
    },
    LineComment(Comment<'i>),
    Error(Box<Error<'i>>),
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ExpressionList<'source> {
    pub span: Span,
    pub items: Box<[OneOf<Expression<'source>, Comment<'source>>]>,
}

impl<'source> ExpressionList<'source> {
    pub fn expressions(&self) -> impl Iterator<Item = &Expression<'source>> {
        self.items.iter().filter_map(|e| e.this())
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub enum TemplateSringPart<'source> {
    ExpressionPart(Expression<'source>),
    StringPart(StringLiteral<'source>),
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Expression<'source> {
    Identifier(ParsedNode<'source, Token<'source>>),
    String(StringLiteral<'source>),
    Bool((Span, bool)),
    Number((Span, f64)),
    Call(CallExpr<'source>),
    Array(ExpressionList<'source>),
    Object(ObjectEntryList<'source>),
    Null(Span),
    EmptyArray(Span),
    EmptyObject(Span),
    TemplateSringLiteral {
        span: Span,
        parts: Box<[TemplateSringPart<'source>]>,
    },
    Error(Box<Error<'source>>),
}

#[derive(Debug, PartialEq, Serialize)]
pub struct CallExpr<'source> {
    pub identifier: ParsedNode<'source, Token<'source>>,
    pub arguments: ExpressionList<'source>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ObjectEntry<'source> {
    pub key: ParsedNode<'source, StringLiteral<'source>>,
    pub value: Expression<'source>,
}

impl<'source> ObjectEntry<'source> {
    pub fn new(
        key: ParsedNode<'source, StringLiteral<'source>>,
        value: Expression<'source>,
    ) -> Self {
        Self { key, value }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ObjectEntryList<'source> {
    pub span: Span,
    pub items: Box<[OneOf<ParsedNode<'source, ObjectEntry<'source>>, Comment<'source>>]>,
}

impl<'source> ObjectEntryList<'source> {
    pub fn nodes(&self) -> impl Iterator<Item = &ParsedNode<'source, ObjectEntry<'source>>> {
        self.items.iter().filter_map(|e| e.this())
    }

    pub fn entries(&self) -> impl Iterator<Item = &ObjectEntry<'source>> {
        self.nodes().flat_map(|node| node.get())
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Endpoint<'source> {
    Expr(Expression<'source>),
    Url(Literal<'source>),
    Pathname(Literal<'source>),
}

pub mod result {

    use super::*;

    #[derive(Debug, PartialEq, serde::Serialize)]
    pub enum ParsedNode<'i, T: GetSpan> {
        Ok(T),
        Error(Box<Error<'i>>),
    }

    impl<'source, T: GetSpan> ParsedNode<'source, T> {
        pub fn get(&self) -> std::result::Result<&T, Box<Error<'source>>> {
            match self {
                ParsedNode::Ok(node) => Ok(node),
                ParsedNode::Error(error) => Err(error.clone()),
            }
        }
    }
}

mod convert {
    use super::*;

    impl<'i> From<&Token<'i>> for ParsedNode<'i, Token<'i>> {
        fn from(token: &Token<'i>) -> Self {
            Self::Ok(Token {
                kind: token.kind,
                text: token.text,
                start: token.start,
            })
        }
    }
    impl<'i> From<&Token<'i>> for Literal<'i> {
        fn from(token: &Token<'i>) -> Self {
            Self {
                value: token.text,
                span: token.span(),
            }
        }
    }
    impl<'i> From<&Token<'i>> for StringLiteral<'i> {
        fn from(token: &Token<'i>) -> Self {
            let value = match (token.text.chars().next(), token.text.chars().last()) {
                (Some('"'), Some('"')) if token.text.len() > 1 => {
                    &token.text[1..token.text.len() - 1]
                }
                (Some('`'), Some('`')) if token.text.len() > 1 => {
                    &token.text[1..token.text.len() - 1]
                }
                (_, Some('`')) => &token.text[..token.text.len() - 1],
                (Some('`'), _) => &token.text[1..],
                _ => token.text,
            };

            Self {
                raw: token.text,
                value,
                span: token.span(),
            }
        }
    }

    impl<'source, T: GetSpan> From<std::result::Result<T, Box<Error<'source>>>>
        for ParsedNode<'source, T>
    {
        fn from(value: std::result::Result<T, std::boxed::Box<Error<'source>>>) -> Self {
            match value {
                Ok(value) => ParsedNode::Ok(value),
                Err(error) => ParsedNode::Error(error),
            }
        }
    }

    impl<'source> From<Error<'source>> for Expression<'source> {
        fn from(value: Error<'source>) -> Self {
            Self::Error(value.into())
        }
    }

    impl<'source> From<Error<'source>> for Statement<'source> {
        fn from(value: Error<'source>) -> Self {
            Self::Error(value.into())
        }
    }

    impl<'source> From<Box<Error<'source>>> for Statement<'source> {
        fn from(value: Box<Error<'source>>) -> Self {
            Self::Error(value)
        }
    }

    impl<'source> From<Error<'source>> for Item<'source> {
        fn from(value: Error<'source>) -> Self {
            Self::Error(value.into())
        }
    }

    impl<'source> From<std::result::Result<Self, Box<Error<'source>>>> for Expression<'source> {
        fn from(value: std::result::Result<Self, Box<Error<'source>>>) -> Self {
            match value {
                Ok(v) => v,
                Err(error) => Self::Error(error),
            }
        }
    }
}
