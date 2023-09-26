use std::{
    fs,
    io::{stdin, Read},
    path::PathBuf,
};

use anyhow::anyhow;
use clap::Args;
use rested::{
    error::ColoredMetaError,
    error_meta::ContextualError,
    interpreter::{
        environment::Environment,
        error::{InterpreterError, InterpreterErrorKind},
    },
    parser::ast::Program,
};

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Namespace in which to look for environment variables
    #[arg(short = 'n', long)]
    pub namespace: Option<String>,

    /// One or more names of the specific request(s) to run
    #[arg(short = 'r', long, num_args(1..))]
    pub request: Option<Vec<String>>,

    /// Path to the script to run
    pub file: Option<PathBuf>,
}

impl RunArgs {
    pub fn handle(self, mut env: Environment) -> anyhow::Result<()> {
        if let Some(ns) = self.namespace {
            env.select_variables_namespace(ns);
        }

        let code = self.file.map(fs::read_to_string).unwrap_or_else(|| {
            let mut buf = String::new();
            stdin().read_to_string(&mut buf)?;
            Ok(buf)
        })?;

        let program = Program::from(&code);

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

        program
            .run_ureq(self.request.map(|r| r.into()))
            .map_err(|err| {
                ContextualError::new(
                    InterpreterErrorKind::Other {
                        error: err.to_string(),
                    },
                    err.span,
                    &code,
                )
            })?;

        Ok(())
    }
}
