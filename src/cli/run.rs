use std::{
    fs,
    io::{stdin, Read},
    path::PathBuf,
};

use anyhow::anyhow;
use clap::Args;
use rested::{
    error::ColoredMetaError,
    interpreter::{environment::Environment, error::InterpreterError, ir},
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

        let code = read_program_text(self.file)?;

        interpret_program_file(&code, env)?.run_ureq(self.request.as_deref());

        Ok(())
    }
}

pub fn interpret_program_file(code: &str, env: Environment) -> anyhow::Result<ir::Program<'_>> {
    let program = Program::from(code);

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

pub fn read_program_text(file: Option<PathBuf>) -> anyhow::Result<String> {
    let code = file.map(fs::read_to_string).unwrap_or_else(|| {
        let mut buf = String::new();
        stdin().read_to_string(&mut buf)?;
        Ok(buf)
    })?;

    Ok(code)
}
