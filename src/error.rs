use crate::lexer::{Location, TokenKind};

#[derive(Debug)]
pub enum ParseErrorKind {
    Unexpected {
        expected: TokenKind,
        found: TokenKind,
    },
}

#[derive(Debug)]
pub struct ParseError {
    kind: ParseErrorKind,
    location: Location,
    message: Option<String>,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, at: Location) -> Self {
        Self {
            kind,
            location: at,
            message: None,
        }
    }

    fn with_message(mut self, msg: &String) -> Self {
        self.message = Some(msg.to_owned());
        self
    }
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = match &self.kind {
            ParseErrorKind::Unexpected { expected, found } => {
                format!("unexpected token: expected {:?}, got {:?}", expected, found)
            }
        };

        match &self.message {
            Some(m) => writeln!(f, "{}\n{}", formatted_error, m),
            None => writeln!(f, "{}", formatted_error),
        }
    }
}
