use anyhow::anyhow;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommand,
}

trait ValidateDir {
    fn check_is_dir(self) -> anyhow::Result<Self>
    where
        Self: std::marker::Sized;
}

impl ValidateDir for PathBuf {
    fn check_is_dir(self) -> anyhow::Result<Self> {
        if !self.exists() {
            return Err(anyhow!("'{}' does not exist", self.to_string_lossy()));
        }

        if !self.is_dir() {
            return Err(anyhow!("'{}' is not a folder", self.to_string_lossy()));
        }

        Ok(self)
    }
}

impl ConfigArgs {
    pub fn handle(self) -> anyhow::Result<()> {
        match self.command {
            ConfigCommand::ScratchDirectory { command } => match command {
                ManageScratchDirCommand::Set { value: path } => {
                    let mut config = rested::config::Config::load()?;
                    config.scratch_dir = path.check_is_dir()?;
                    config.save()?;
                }
                ManageScratchDirCommand::Show {} => {
                    println!(
                        "{}",
                        rested::config::Config::load()?
                            .scratch_dir
                            .to_string_lossy()
                    );
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
        command: ManageScratchDirCommand,
    },
    /// Where these configurations are persisted
    Path {},
}

#[derive(Debug, Subcommand)]
enum ManageScratchDirCommand {
    /// Set the path
    Set { value: PathBuf },
    /// Print the path
    Show {},
}
