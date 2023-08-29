use std::{
    borrow::Cow,
    env, fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{Args, Subcommand};
use rested::{
    error::CliError,
    interpreter::{environment::Environment, ureq_runner::UreqRunner, Interpreter},
};

#[derive(Debug, Args)]
pub struct ScratchCommandArgs {
    #[command(subcommand)]
    command: Option<ScratchCommand>,

    /// Run the saved file when done editing
    #[arg(long)]
    run: bool,

    /// Create a new scratch file
    #[arg(short, long)]
    new: bool,
}

impl ScratchCommandArgs {
    pub fn handle(&self, env: Environment) -> Result<(), CliError> {
        let default_editor = env::var("EDITOR").map_err(|e| CliError(e.to_string()))?;

        let file_name = if self.new {
            create_scratch_file()?
        } else {
            let entries = fs::read_dir(".")?
                .map(|res| res.map(|e| e.path()))
                .collect::<Result<Vec<_>, std::io::Error>>()?;

            let mut scratch_files = entries
                .into_iter()
                .filter(|e| {
                    matches!(
                        e.extension().map(|e| e.to_string_lossy()),
                        Some(Cow::Borrowed("rd"))
                    )
                })
                .collect::<Vec<_>>();

            scratch_files.sort();

            if let Some(file) = scratch_files.last().cloned() {
                file
            } else {
                create_scratch_file()?
            }
        };

        std::process::Command::new(default_editor)
            .arg(&file_name)
            .spawn()?
            .wait()?;

        if self.run {
            let code = fs::read_to_string(file_name)?;
            Interpreter::new(&code, env, UreqRunner).run(None)?;
        }

        Ok(())
    }
}

#[derive(Debug, Subcommand)]
pub enum ScratchCommand {
    History {},
}

fn create_scratch_file() -> Result<PathBuf, CliError> {
    let path = format!(
        "scratch-{:?}.rd",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| CliError(e.to_string()))?
            .as_millis()
    )
    .into();

    fs::File::create(&path)?;

    Ok(path)
}
