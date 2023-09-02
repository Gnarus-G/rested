use std::path::PathBuf;

use clap::{Parser, Subcommand};
use rested::error::CliError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub scratch_dir: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self, CliError> {
        return confy::load("rested", None).map_err(|e| e.into());
    }

    pub fn save(self) -> Result<(), CliError> {
        return confy::store("rested", None, self).map_err(|e| e.into());
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scratch_dir: PathBuf::from("."),
        }
    }
}

#[derive(Debug, Parser)]
pub struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommand,
}

impl ConfigArgs {
    pub fn handle(self) -> Result<(), CliError> {
        match self.command {
            ConfigCommand::ScratchDirectory { command } => match command {
                MutateScratchDirCommand::Set { value } => {
                    let mut config = Config::load()?;
                    config.scratch_dir = value;
                    config.save()?;
                }
                MutateScratchDirCommand::Show {} => {
                    println!("{}", Config::load()?.scratch_dir.to_string_lossy());
                }
            },
            ConfigCommand::Path {} => {
                println!(
                    "{}",
                    confy::get_configuration_file_path("rested", None)?.to_string_lossy()
                );
            }
        };
        Ok(())
    }
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// The folder to contain scratch files that are saved
    ScratchDirectory {
        #[command(subcommand)]
        command: MutateScratchDirCommand,
    },
    /// Where these configurations are persisted
    Path {},
}

#[derive(Debug, Subcommand)]
enum MutateScratchDirCommand {
    /// Set the path
    Set { value: PathBuf },
    /// Print the path
    Show {},
}
