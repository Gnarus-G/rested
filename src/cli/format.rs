use std::{
    fs,
    io::{stdin, Read},
    path::PathBuf,
};

use anyhow::anyhow;
use clap::Args;
use rested::{error::ColoredMetaError, parser::ast::Program};

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

        let formatted_text = program
            .to_formatted_string()
            .map_err(|err| anyhow!(ColoredMetaError(&err).to_string()))?;

        println!("{}", formatted_text);

        Ok(())
    }
}
