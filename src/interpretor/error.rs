use crate::{
    ast::{Identifier, Literal},
    error::Error,
    lexer::Location,
    parser::error::ParseError,
};

#[derive(Debug, PartialEq)]
pub enum InterpError {
    UnknownConstant { constant: String },
    RequiredArguments { required: usize, recieved: usize },
    EnvVariableNotFound { name: String },
    RequestWithPathnameWithoutBaseUrl,
    UndefinedCallable { name: String },
    UndeclaredIdentifier { name: String },
    UnsupportedAttribute { name: String },
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
            InterpError::Other { error } => error.clone(),
        };

        f.write_str(&formatted_error)
    }
}

impl From<Error<ParseError>> for Error<InterpError> {
    fn from(value: Error<ParseError>) -> Self {
        Self {
            inner_error: InterpError::Other {
                error: value.inner_error.to_string(),
            },
            location: value.location,
            message: value.message,
            context: value.context,
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
            token.location,
            self.source_code,
        )
    }

    pub fn env_variable_not_found(&self, token: &Literal) -> Error<InterpError> {
        Error::new(
            InterpError::EnvVariableNotFound {
                name: token.value.to_string(),
            },
            token.location,
            self.source_code,
        )
    }

    pub fn required_call_args(
        &self,
        at: Location,
        required: usize,
        recieved: usize,
    ) -> Error<InterpError> {
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
            token.location,
            self.source_code,
        )
    }

    pub fn unsupported_attribute(&self, token: &Identifier) -> Error<InterpError> {
        Error::new(
            InterpError::UnsupportedAttribute {
                name: token.name.to_string(),
            },
            token.location,
            self.source_code,
        )
    }

    pub fn undefined_callable(&self, token: &Identifier) -> Error<InterpError> {
        Error::new(
            InterpError::UndefinedCallable {
                name: token.name.to_string(),
            },
            token.location,
            self.source_code,
        )
    }

    pub fn unset_base_url(&self, at: Location) -> Error<InterpError> {
        Error::new(
            InterpError::RequestWithPathnameWithoutBaseUrl,
            at,
            self.source_code,
        )
    }

    pub fn other<E: std::fmt::Display>(&self, location: Location, error: E) -> Error<InterpError> {
        Error::new(
            InterpError::Other {
                error: error.to_string(),
            },
            location,
            self.source_code,
        )
    }
}
