use std::{
    fs,
    io::{stdin, Read},
    path::PathBuf,
};

use clap::Args;
use rested::{
    fmt,
    parser::{ast::Program, ast_visit::VisitWith},
};

#[derive(Debug, Args)]
pub struct FormatArgs {
    /// Path to the script to format
    pub file: Option<PathBuf>,
}

impl FormatArgs {
    pub fn handle(self) -> anyhow::Result<()> {
        let code = self.file.map(fs::read_to_string).unwrap_or_else(|| {
            let mut buf = String::new();
            stdin().read_to_string(&mut buf)?;
            Ok(buf)
        })?;

        let program = Program::from(&code);

        let mut formatter = fmt::FormattedPrinter::new();

        program.visit_with(&mut formatter);

        if !formatter.has_error {
            println!("{}", formatter.into_output());
        }

        Ok(())
    }
}
