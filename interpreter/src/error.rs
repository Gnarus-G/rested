use error_meta::Error;
use lexer::locations::Span;
use parser::ast::{Identifier, Literal};
use parser::error::ParseError;

#[derive(Debug, PartialEq)]
pub enum InterpError {
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

impl std::error::Error for InterpError {}

impl std::fmt::Display for InterpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = match self {
            InterpError::UnknownConstant { constant } => {
                format!("trying to set an unknown constant {}", constant)
            }
            InterpError::RequiredArguments { required, recieved } => {
                format!("{} argument(s) required, recieved {}", required, recieved)
            }
            InterpError::EnvVariableNotFound { name } => {
                format!("no variable found by the name {:?}", name)
            }
            InterpError::RequestWithPathnameWithoutBaseUrl => {
                format!("BASE_URL needs to be set first for requests to work with just pathnames")
            }
            InterpError::UndefinedCallable { name } => {
                format!("attempting to calling an undefined function: {}", name)
            }
            InterpError::UndeclaredIdentifier { name } => format!("undeclared variable: {}", name),
            InterpError::UnsupportedAttribute { name } => {
                format!("unsupported attribute: {}", name)
            }
            InterpError::DuplicateAttribute { name } => {
                format!(
                    "duplicate attribute: @{} is already set for this request",
                    name
                )
            }
            InterpError::Other { error } => error.clone(),
        };

        f.write_str(&formatted_error)
    }
}

pub trait IntoInterpError {
    fn into_interp_error(self) -> Error<InterpError>;
}

impl IntoInterpError for Error<ParseError> {
    fn into_interp_error(self) -> Error<InterpError> {
        Error {
            inner_error: InterpError::Other {
                error: self.inner_error.to_string(),
            },
            span: self.span,
            message: self.message,
            context: self.context,
        }
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
    pub fn unknown_constant(&self, token: &Identifier) -> Error<InterpError> {
        Error::new(
            InterpError::UnknownConstant {
                constant: token.name.to_string(),
            },
            token.span,
            self.source_code,
        )
    }

    pub fn env_variable_not_found(&self, token: &Literal) -> Error<InterpError> {
        Error::new(
            InterpError::EnvVariableNotFound {
                name: token.value.to_string(),
            },
            token.span,
            self.source_code,
        )
    }

    pub fn required_args(&self, at: Span, required: usize, recieved: usize) -> Error<InterpError> {
        Error::new(
            InterpError::RequiredArguments { required, recieved },
            at,
            self.source_code,
        )
    }

    pub fn undeclared_identifier(&self, token: &Identifier) -> Error<InterpError> {
        Error::new(
            InterpError::UndeclaredIdentifier {
                name: token.name.to_string(),
            },
            token.span,
            self.source_code,
        )
    }

    pub fn unsupported_attribute(&self, token: &Identifier) -> Error<InterpError> {
        Error::new(
            InterpError::UnsupportedAttribute {
                name: token.name.to_string(),
            },
            token.span,
            self.source_code,
        )
    }

    pub fn duplicate_attribute(&self, token: &Identifier) -> Error<InterpError> {
        Error::new(
            InterpError::DuplicateAttribute {
                name: token.name.to_string(),
            },
            token.span,
            self.source_code,
        )
    }

    pub fn undefined_callable(&self, token: &Identifier) -> Error<InterpError> {
        Error::new(
            InterpError::UndefinedCallable {
                name: token.name.to_string(),
            },
            token.span,
            self.source_code,
        )
    }

    pub fn unset_base_url(&self, at: Span) -> Error<InterpError> {
        Error::new(
            InterpError::RequestWithPathnameWithoutBaseUrl,
            at,
            self.source_code,
        )
    }

    pub fn other<E: std::fmt::Display>(&self, span: Span, error: E) -> Error<InterpError> {
        Error::new(
            InterpError::Other {
                error: error.to_string(),
            },
            span,
            self.source_code,
        )
    }
}
