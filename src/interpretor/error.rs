use crate::{ast::TextSlice, error::Error};

#[derive(Debug, PartialEq)]
pub enum InterpError {
    UnknownConstant { constant: String },
    RequiredArguments { required: usize, recieved: usize },
    EnvVariableNotFound { name: String },
    RequestWithPathnameWithoutBaseUrl,
    InapropriateStatementLocation,
    UndefinedCallable { name: String },
}

impl std::error::Error for InterpError {}

impl std::fmt::Display for InterpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_error = match self {
            InterpError::UnknownConstant { constant } => {
                format!("trying to set an unknown constant {}", constant)
            }
            InterpError::RequiredArguments { required, recieved } => {
                format!("{} arguments are required, recieved {}", required, recieved)
            }
            InterpError::EnvVariableNotFound { name } => {
                format!("no variable found by the name {:?}", name)
            }
            InterpError::RequestWithPathnameWithoutBaseUrl => {
                format!("BASE_URL needs to be set first for requests to work with just pathnames")
            }
            InterpError::InapropriateStatementLocation => format!("inapropriate statement"),
            InterpError::UndefinedCallable { name } => {
                format!("attempting to calling an undefined function: {}", name)
            }
        };

        f.write_str(&formatted_error)
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
    pub fn unknown_constant(&self, token: &TextSlice) -> Error<InterpError> {
        Error::new(
            InterpError::UnknownConstant {
                constant: token.value.to_string(),
            },
            token.location,
            self.source_code,
        )
    }

    pub fn variable_not_found(&self, token: &TextSlice) -> Error<InterpError> {
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
        token: &TextSlice,
        required: usize,
        recieved: usize,
    ) -> Error<InterpError> {
        Error::new(
            InterpError::RequiredArguments { required, recieved },
            token.location,
            self.source_code,
        )
    }

    pub fn undefined_callable(&self, token: &TextSlice) -> Error<InterpError> {
        Error::new(
            InterpError::UndefinedCallable {
                name: token.value.to_string(),
            },
            token.location,
            self.source_code,
        )
    }

    pub fn unset_base_url(&self, token: &TextSlice) -> Error<InterpError> {
        Error::new(
            InterpError::RequestWithPathnameWithoutBaseUrl,
            token.location,
            self.source_code,
        )
    }

    pub fn inapropriate_statement(&self, token: &TextSlice) -> Error<InterpError> {
        Error::new(
            InterpError::InapropriateStatementLocation,
            token.location,
            self.source_code,
        )
    }
}
