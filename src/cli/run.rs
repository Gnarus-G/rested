use std::{
    fs,
    io::{stdin, Read},
    path::PathBuf,
};

use clap::Args;
use rested::{
    error::CliError,
    interpreter::{environment::Environment, ureq_runner::UreqRunner, Interpreter},
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
    pub fn handle(self, mut env: Environment) -> Result<(), CliError> {
        if let Some(ns) = self.namespace {
            env.select_variables_namespace(ns);
        }

        let code = self.file.map(fs::read_to_string).unwrap_or_else(|| {
            let mut buf = String::new();
            stdin().read_to_string(&mut buf)?;
            Ok(buf)
        })?;

        Interpreter::new(&code, env, UreqRunner).run(self.request.map(|r| r.into()))?;

        Ok(())
    }
}
