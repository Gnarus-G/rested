use crate::error_meta::ContextualError;
use crate::lexer::locations::{GetSpan, Span};
use crate::lexer::Token;
use crate::parser::error::{ParseError, ParserErrors};

#[derive(Clone, Debug, PartialEq)]
pub enum InterpreterErrorKind {
    UnknownConstant { constant: String },
    RequiredArguments { required: usize, recieved: usize },
    EnvVariableNotFound { name: String },
    RequestWithPathnameWithoutBaseUrl,
    UndefinedCallable { name: String },
    UndeclaredIdentifier { name: String },
    UnsupportedAttribute { name: String },
    DuplicateAttribute { name: String },
    Other { error: String },
}

impl std::error::Error for InterpreterErrorKind {}

impl std::fmt::Display for InterpreterErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = match self {
            InterpreterErrorKind::UnknownConstant { constant } => {
                format!("trying to set an unknown constant {}", constant)
            }
            InterpreterErrorKind::RequiredArguments { required, recieved } => {
                format!("{} argument(s) required, recieved {}", required, recieved)
            }
            InterpreterErrorKind::EnvVariableNotFound { name } => {
                format!("no variable found by the name {:?}", name)
            }
            InterpreterErrorKind::RequestWithPathnameWithoutBaseUrl => {
                "BASE_URL needs to be set first for requests to work with just pathnames; try writing like set BASE_URL \"<api orgin>\" before this request".to_string()
            }
            InterpreterErrorKind::UndefinedCallable { name } => {
                format!("attempting to calling an undefined function: {}", name)
            }
            InterpreterErrorKind::UndeclaredIdentifier { name } => {
                format!("undeclared variable: {}", name)
            }
            InterpreterErrorKind::UnsupportedAttribute { name } => {
                format!("unsupported attribute: {}", name)
            }
            InterpreterErrorKind::DuplicateAttribute { name } => {
                format!(
                    "duplicate attribute: @{} is already set for this request",
                    name
                )
            }
            InterpreterErrorKind::Other { error } => error.clone(),
        };

        f.write_str(&formatted_error)
    }
}

pub enum InterpreterError<'source> {
    ParseErrors(ParserErrors<'source>),
    Error(ContextualError<InterpreterErrorKind>),
}

impl<'source> std::error::Error for InterpreterError<'source> {}

impl<'source> std::fmt::Debug for InterpreterError<'source> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl<'source> std::fmt::Display for InterpreterError<'source> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpreterError::Error(err) => write!(f, "{err}"),
            InterpreterError::ParseErrors(ParserErrors { errors }) => {
                for err in errors.iter() {
                    write!(f, "{err}")?
                }
                Ok(())
            }
        }
    }
}

impl<'source> From<ContextualError<InterpreterErrorKind>> for InterpreterError<'source> {
    fn from(value: ContextualError<InterpreterErrorKind>) -> Self {
        Self::Error(value)
    }
}

impl<'source> From<ContextualError<ParseError<'source>>> for InterpreterError<'source> {
    fn from(value: ContextualError<ParseError<'source>>) -> Self {
        Self::Error(ContextualError {
            inner_error: InterpreterErrorKind::Other {
                error: value.inner_error.to_string(),
            },
            span: value.span,
            message: value.message,
            context: value.context,
        })
    }
}

impl<'source> From<ParserErrors<'source>> for InterpreterError<'source> {
    fn from(value: ParserErrors<'source>) -> Self {
        Self::ParseErrors(value)
    }
}

pub struct InterpErrorFactory<'i> {
    source_code: &'i str,
}

impl<'i> InterpErrorFactory<'i> {
    pub fn new(source: &'i str) -> Self {
        Self {
            source_code: source,
        }
    }
    pub fn unknown_constant(&self, token: &Token) -> ContextualError<InterpreterErrorKind> {
        ContextualError::new(
            InterpreterErrorKind::UnknownConstant {
                constant: token.text.to_string(),
            },
            token.span(),
            self.source_code,
        )
    }

    pub fn env_variable_not_found(
        &self,
        variable: String,
        span: Span,
    ) -> ContextualError<InterpreterErrorKind> {
        ContextualError::new(
            InterpreterErrorKind::EnvVariableNotFound { name: variable },
            span,
            self.source_code,
        )
    }

    pub fn required_args(
        &self,
        at: Span,
        required: usize,
        recieved: usize,
    ) -> ContextualError<InterpreterErrorKind> {
        ContextualError::new(
            InterpreterErrorKind::RequiredArguments { required, recieved },
            at,
            self.source_code,
        )
    }

    pub fn undeclared_identifier(&self, token: &Token) -> ContextualError<InterpreterErrorKind> {
        ContextualError::new(
            InterpreterErrorKind::UndeclaredIdentifier {
                name: token.text.to_string(),
            },
            token.span(),
            self.source_code,
        )
    }

    pub fn unsupported_attribute(&self, token: &Token) -> ContextualError<InterpreterErrorKind> {
        ContextualError::new(
            InterpreterErrorKind::UnsupportedAttribute {
                name: token.text.to_string(),
            },
            token.span(),
            self.source_code,
        )
    }

    pub fn duplicate_attribute(&self, token: &Token) -> ContextualError<InterpreterErrorKind> {
        ContextualError::new(
            InterpreterErrorKind::DuplicateAttribute {
                name: token.text.to_string(),
            },
            token.span(),
            self.source_code,
        )
    }

    pub fn undefined_callable(&self, token: &Token) -> ContextualError<InterpreterErrorKind> {
        ContextualError::new(
            InterpreterErrorKind::UndefinedCallable {
                name: token.text.to_string(),
            },
            token.span(),
            self.source_code,
        )
    }

    pub fn unset_base_url(&self, at: Span) -> ContextualError<InterpreterErrorKind> {
        ContextualError::new(
            InterpreterErrorKind::RequestWithPathnameWithoutBaseUrl,
            at,
            self.source_code,
        )
    }

    pub fn other<E: std::fmt::Display>(
        &self,
        span: Span,
        error: E,
    ) -> ContextualError<InterpreterErrorKind> {
        ContextualError::new(
            InterpreterErrorKind::Other {
                error: error.to_string(),
            },
            span,
            self.source_code,
        )
    }
}
