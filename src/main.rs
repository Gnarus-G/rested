mod cli;

use anyhow::Context;
use clap::{CommandFactory, Parser, Subcommand};
use cli::config::ConfigArgs;
use cli::format::FormatArgs;
use cli::run::RunArgs;
use cli::scratch::ScratchCommandArgs;
use cli::snapshot::SnapshotArgs;
use rested::config::{
    get_env_from_dir_path, get_env_from_dir_path_or_from_home_dir, get_env_from_home_dir,
};
use rested::editing::edit;
use rested::interpreter::environment::Environment;
use rested::ENV_FILE_NAME;
use tracing::{error, info};

use std::collections::HashMap;
use std::fs;

#[derive(Parser, Debug)]
#[command(author, version, about)]
/// The CLI runtime for Rested, the language/interpreter for easily defining and running requests to an http server.
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Set log level, one of trace, debug, info, warn, error
    #[arg(short, long, default_value = "info", global = true)]
    level: tracing::Level,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a script written in the language
    Run(RunArgs),
    /// Format a script written in the language
    Fmt(FormatArgs),
    /// Open your default editor to start editing a temporary file
    Scratch(ScratchCommandArgs),
    /// Generate a static snapshot of the requests with all dynamic values evaluated.
    Snap(SnapshotArgs),
    /// Operate on the environment variables available in the runtime.
    /// Looking into the `.env.rd.json` in the current directory, or that in the home directory.
    Env {
        /// Set to look at the `.env.rd.json` file in the current working directory.
        /// Otherwise this command and its subcommands operate on the `.env.rd.json` file in your
        /// home directory.
        #[arg(long)]
        cwd: bool,

        #[command(subcommand)]
        command: EnvCommand,
    },
    /// Generate a completions file for a specified shell
    Completion {
        // The shell for which to generate completions
        shell: clap_complete::Shell,
    },
    /// Start the rested language server
    Lsp,

    /// Configure, or view current configurations
    Config(ConfigArgs),
}

#[derive(Debug, Subcommand)]
enum EnvCommand {
    /// View environment variables available in the runtime
    Show,
    /// Edit environment variables in your default editor.
    Edit,
    /// Set environment variables available in the runtime
    Set {
        /// Namespace for which to set environment variable
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// Of the environment variable
        name: String,

        /// Of the environment variable
        value: String,
    },
    /// Operate on the variables namespaces available in the runtime
    NS {
        #[command(subcommand)]
        command: EnvNamespaceCommand,
    },
}

#[derive(Debug, Subcommand)]
enum EnvNamespaceCommand {
    /// Set a new variables namespace available in the runtime
    Add {
        /// Of the namespace
        name: String,
    },
    /// Remove a namespace
    Rm {
        /// Of the namespace
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(cli.level)
        .with_writer(std::io::stderr)
        .init();

    if let Err(e) = run(cli) {
        error!("{:#}", e);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Env { command, cwd } => {
            let mut env = if cwd {
                let path = std::env::current_dir()?;
                get_env_from_dir_path(&path).or_else(|_| {
                    Environment::new(path.join(ENV_FILE_NAME))
                        .context("failed to load the environment for rstd")
                })?
            } else {
                get_env_from_home_dir()?
            };

            match command {
                EnvCommand::Set {
                    name,
                    value,
                    namespace,
                } => {
                    if let Some(ns) = namespace {
                        env.select_variables_namespace(ns);
                    }
                    info!("setting variable '{}' with value '{}'", name, value);
                    env.set_variable(name, value)?;
                }
                EnvCommand::NS { command } => match command {
                    EnvNamespaceCommand::Add { name } => {
                        info!("adding namespace: {name}");
                        env.namespaced_variables.insert(name, HashMap::new());
                        env.save_to_file()?;
                    }
                    EnvNamespaceCommand::Rm { name } => {
                        info!("removing namespace: {name}");
                        env.namespaced_variables.remove(&name);
                        env.save_to_file()?;
                    }
                },
                EnvCommand::Show => println!("{}", fs::read_to_string(env.env_file_name)?),
                EnvCommand::Edit => edit(&env.env_file_name)?,
            }
        }
        Command::Completion { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "rstd", &mut std::io::stdout())
        }
        Command::Lsp => rested::language_server::start(cli.level),
        Command::Run(run) => {
            let full_path = run.file.as_ref().and_then(|path| path.canonicalize().ok());
            let workspace = full_path.as_ref().and_then(|p| p.parent());

            if let Some(path) = full_path.as_ref() {
                info!("script to run: {:?}", path);
            }

            if let Some(workspace) = workspace.as_ref() {
                info!("identified workspace: {:?}", workspace);
            }

            let env = get_env_from_dir_path_or_from_home_dir(workspace)?;
            run.handle(env)?
        }
        Command::Scratch(scratch) => {
            let env = get_env_from_home_dir()?;
            scratch.handle(env)?
        }
        Command::Config(config) => config.handle()?,
        Command::Fmt(fmt) => fmt.handle()?,
        Command::Snap(snap) => {
            let full_path = snap.file.as_ref().and_then(|path| path.canonicalize().ok());
            let workspace = full_path.as_ref().and_then(|p| p.parent());

            if let Some(path) = full_path.as_ref() {
                info!("script to snapshot: {:?}", path);
            }

            if let Some(workspace) = workspace.as_ref() {
                info!("identified workspace: {:?}", workspace);
            }

            let env = get_env_from_dir_path_or_from_home_dir(workspace)?;
            snap.handle(env)?
        }
    };

    Ok(())
}
