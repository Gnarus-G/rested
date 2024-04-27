mod attributes;
mod builtin;
pub mod environment;
pub mod error;
mod eval;
pub mod ir;
pub mod runner;
pub mod ureq_runner;
pub mod value;

use environment::Environment;
use error::InterpreterError;

use crate::parser::ast::{self};

use crate::parser::error::ParserErrors;

impl<'source> ast::Program<'source> {
    pub fn interpret(
        &self,
        env: &Environment,
    ) -> std::result::Result<ir::Program<'source>, InterpreterError<'source>> {
        let parse_errors = self.errors();

        if !parse_errors.is_empty() {
            return Err(ParserErrors::new(parse_errors).into());
        }

        let mut interpreter = eval::Evaluator::new(self, env);

        let items = interpreter
            .evaluate()
            .map_err(InterpreterError::EvalErrors)?;

        Ok(ir::Program::new(
            self.source,
            items.into(),
            interpreter
                .let_bindings
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        ))
    }
}
