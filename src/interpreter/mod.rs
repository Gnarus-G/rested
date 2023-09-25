mod attributes;
mod builtin;
pub mod environment;
pub mod error;
mod eval;
pub mod ir;
pub mod runner;
pub mod ureq_runner;
mod value;

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

        let mut interp = eval::Evaluator::new(self, env);

        let items = interp.evaluate()?;

        Ok(ir::Program::new(self.source, items.into()))
    }
}
