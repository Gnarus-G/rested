use std::{fs, path::PathBuf};

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
        let folder_name = "rested-scratch";

        #[cfg(windows)]
        let home_dir_key = "USERPROFILE";

        #[cfg(unix)]
        let home_dir_key = "HOME";

        let home = std::env::var(home_dir_key).unwrap_or_else(|_| {
            panic!(
                "failed to read the user's home directory, using the {} environment variable",
                home_dir_key
            )
        });

        let scratch_dir = PathBuf::from(home).join(folder_name);

        if !scratch_dir.exists() {
            fs::create_dir(&scratch_dir).unwrap_or_else(|_| {
                panic!(
                    "failed to create a directory for the scratch files: {}",
                    scratch_dir.to_string_lossy()
                )
            })
        }

        Self { scratch_dir }
    }
}

#[derive(Debug, Parser)]
pub struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommand,
}

trait ValidateDir {
    fn check_is_dir(self) -> Result<Self, CliError>
    where
        Self: std::marker::Sized;
}

impl ValidateDir for PathBuf {
    fn check_is_dir(self) -> Result<Self, CliError> {
        if !self.exists() {
            return Err(CliError(format!(
                "'{}' does not exist",
                self.to_string_lossy()
            )));
        }

        if !self.is_dir() {
            return Err(CliError(format!(
                "'{}' is not a folder",
                self.to_string_lossy()
            )));
        }

        Ok(self)
    }
}

impl ConfigArgs {
    pub fn handle(self) -> Result<(), CliError> {
        match self.command {
            ConfigCommand::ScratchDirectory { command } => match command {
                MutateScratchDirCommand::Set { value: path } => {
                    let mut config = Config::load()?;
                    config.scratch_dir = path.check_is_dir()?;
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
