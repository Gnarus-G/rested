use std::fmt::Display;

use serde::Serialize;

use crate::{
    error_meta::ContextualError,
    lexer::{
        locations::{GetSpan, Position, Span},
        Token,
    },
    utils,
};

use self::result::ParsedNode;

use super::error::ParseError;

type Error<'source> = ContextualError<ParseError<'source>>;

#[derive(Debug, PartialEq, Serialize)]
pub struct Program<'i> {
    pub source: &'i str,
    pub items: utils::Array<Item<'i>>,
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
    pub statements: Vec<Statement<'source>>,
    pub span: Span,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Item<'source> {
    Set {
        identifier: ParsedNode<'source, Token<'source>>,
        value: Expression<'source>,
    },
    Let {
        identifier: ParsedNode<'source, Token<'source>>,
        value: Expression<'source>,
    },
    LineComment(Literal<'source>),
    Request {
        method: RequestMethod,
        endpoint: Endpoint<'source>,
        block: Option<Block<'source>>,
        span: Span,
    },
    Expr(Expression<'source>),
    Attribute {
        location: Position,
        identifier: ParsedNode<'source, Token<'source>>,
        parameters: Option<Arguments<'source>>,
    },
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
    LineComment(Literal<'i>),
    Error(Box<Error<'i>>),
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Arguments<'source> {
    pub span: Span,
    pub parameters: Vec<Expression<'source>>,
}

impl<'source> Arguments<'source> {
    pub fn iter(&self) -> impl Iterator<Item = &Expression<'source>> {
        self.parameters.iter()
    }
}

pub type Spanned<T> = (Span, T);

#[derive(Debug, PartialEq, Serialize)]
pub enum Expression<'source> {
    Identifier(ParsedNode<'source, Token<'source>>),
    String(StringLiteral<'source>),
    Bool(Literal<'source>),
    Number(Literal<'source>),
    Call {
        identifier: ParsedNode<'source, Token<'source>>,
        arguments: Arguments<'source>,
    },
    Array(Spanned<Vec<ArrayElement<'source>>>),
    Object(Spanned<Vec<ObjectEntry<'source>>>),
    Null(Span),
    EmptyArray(Span),
    EmptyObject(Span),
    TemplateSringLiteral {
        span: Span,
        parts: Vec<Expression<'source>>,
    },
    Error(Box<Error<'source>>),
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ArrayElement<'source> {
    pub expr: Expression<'source>,
    // For missing commas actually. I don't know if this is a good idea generally though.
    pub errors: Vec<Error<'source>>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ObjectEntry<'source> {
    pub key: ParsedNode<'source, StringLiteral<'source>>,
    pub value: Expression<'source>,
    // For missing commas...
    pub errors: Vec<Error<'source>>,
}

impl<'source> ObjectEntry<'source> {
    pub fn new(
        key: ParsedNode<'source, StringLiteral<'source>>,
        value: Expression<'source>,
    ) -> Self {
        Self {
            key,
            value,
            errors: vec![],
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Endpoint<'i> {
    Url(Literal<'i>),
    Pathname(Literal<'i>),
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

    impl<'source> From<Expression<'source>> for ArrayElement<'source> {
        fn from(value: Expression<'source>) -> Self {
            Self {
                expr: value,
                errors: vec![],
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
