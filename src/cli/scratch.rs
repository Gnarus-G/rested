use std::{
    borrow::Cow,
    env, fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{Args, Subcommand};
use rested::{error::CliError, interpreter::environment::Environment};

use super::run::RunArgs;

#[derive(Debug, Args)]
pub struct ScratchCommandArgs {
    #[command(subcommand)]
    command: Option<ScratchCommand>,

    /// Create a new scratch file
    #[arg(long)]
    new: bool,

    /// Run the saved file when done editing
    #[arg(long)]
    run: bool,

    /// Namespace in which to look for environment variables
    #[arg(short = 'n', long, requires = "run")]
    namespace: Option<String>,

    /// One or more names of the specific request(s) to run
    #[arg(short = 'r', long, requires = "run", num_args(1..))]
    request: Option<Vec<String>>,
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
            RunArgs {
                request: self.request.clone(),
                namespace: self.namespace.clone(),
                file: Some(file_name),
            }
            .handle(env)?;
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
