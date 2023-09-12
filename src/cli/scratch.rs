use std::{
    borrow::Cow,
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{Args, Subcommand};
use colored::Colorize;
use rested::{config::Config, editing::edit, interpreter::environment::Environment};

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

#[derive(Debug, Subcommand)]
pub enum ScratchCommand {
    /// List all the scratch files created or edited from oldest to newest
    History {
        // Don't show scratch file previews
        #[arg(short, long)]
        quiet: bool,
    },
}

impl ScratchCommandArgs {
    pub fn handle(&self, env: Environment) -> anyhow::Result<()> {
        match &self.command {
            Some(command) => match command {
                ScratchCommand::History { quiet } => {
                    for file_path in fetch_scratch_files()? {
                        println!("{}", file_path.to_string_lossy().bold());

                        if !quiet {
                            let three_lines = fs::File::open(file_path)
                                .map(BufReader::new)
                                .map(|reader| reader.lines().flatten().take(3))?;

                            for (idx, line) in three_lines.enumerate() {
                                eprintln!("{}", format!("  {}|  {}", idx + 1, line).dimmed());
                            }
                        }
                    }
                }
            },
            None => {
                let file_name = if self.new {
                    create_scratch_file()?
                } else if let Some(file) = fetch_scratch_files()?.last().cloned() {
                    file
                } else {
                    create_scratch_file()?
                };

                edit(&file_name)?;

                if self.run {
                    RunArgs {
                        request: self.request.clone(),
                        namespace: self.namespace.clone(),
                        file: Some(file_name),
                    }
                    .handle(env)?;
                }
            }
        }

        Ok(())
    }
}

fn create_scratch_file() -> anyhow::Result<PathBuf> {
    let prefix_path = Config::load()?.scratch_dir;

    let path = prefix_path.join::<String>(format!(
        "scratch-{:?}.rd",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
    ));

    fs::File::create(&path)?;

    Ok(path)
}

fn fetch_scratch_files() -> anyhow::Result<Vec<PathBuf>> {
    let prefix_path = Config::load()?.scratch_dir;

    let entries = fs::read_dir(prefix_path)?
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

    Ok(scratch_files)
}
