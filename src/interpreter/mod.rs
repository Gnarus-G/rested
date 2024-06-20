mod attributes;
mod builtin;
pub mod environment;
pub mod error;
mod eval;
pub mod ir;
pub mod runner;
pub mod ureq_runner;
pub mod value;

use std::io::{stdin, Read};

use anyhow::anyhow;
use environment::Environment;
use error::InterpreterError;

use crate::error::ColoredMetaError;
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

pub fn interpret_program(code: &str, env: Environment) -> anyhow::Result<ir::Program<'_>> {
    let program = ast::Program::from(code);

    let program = program.interpret(&env).map_err(|value| match value {
        InterpreterError::ParseErrors(p) => {
            let error_string: String = p
                .errors
                .iter()
                .map(|e| ColoredMetaError(e).to_string())
                .collect();

            return anyhow!(error_string);
        }
        InterpreterError::EvalErrors(errors) => {
            let error_string: String = errors
                .iter()
                .map(|e| ColoredMetaError(e).to_string())
                .collect();

            return anyhow!(error_string);
        }
    })?;

    Ok(program)
}

pub fn read_program_text(file: Option<std::path::PathBuf>) -> anyhow::Result<String> {
    let code = file.map(std::fs::read_to_string).unwrap_or_else(|| {
        let mut buf = String::new();
        stdin().read_to_string(&mut buf)?;
        Ok(buf)
    })?;

    Ok(code)
}
