use clap::{CommandFactory, Parser, Subcommand};
use rested::error::CliError;
use rested::interpreter::{environment::Environment, ureq_runner::UreqRunner, Interpreter};

use std::env;
use std::io::{stdin, Read};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about)]
/// The CLI runtime for Rested, the language/interpreter for easily defining and running requests to an http server.
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a script written in the language
    Run {
        /// Namespace in which to look for environment variables
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// One or more names of the specific request(s) to run
        #[arg(short = 'r', long, num_args(1..))]
        request: Option<Vec<String>>,

        /// Path to the script to run
        file: Option<PathBuf>,
    },
    /// Operate on the environment variables available in the runtime
    Scratch {
        #[command(subcommand)]
        command: Option<ScratchCommand>,
    },
    /// Operate on the environment variables available in the runtime
    Env {
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
}

#[derive(Debug, Subcommand)]
enum EnvCommand {
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

#[derive(Debug, Subcommand)]
enum ScratchCommand {
    History {},
}

fn main() {
    if let Err(e) = run() {
        eprint!("{}", e.0);
    }
}

fn run() -> Result<(), CliError> {
    let cli = Cli::parse();

    let mut env = Environment::new(PathBuf::from(".vars.rd.json"))?;

    match cli.command {
        Command::Run {
            file,
            namespace,
            request,
        } => {
            if let Some(ns) = namespace {
                env.select_variables_namespace(ns);
            }

            let code = file.map(fs::read_to_string).unwrap_or_else(|| {
                let mut buf = String::new();
                stdin().read_to_string(&mut buf)?;
                Ok(buf)
            })?;

            Interpreter::new(&code, env, UreqRunner).run(request.map(|r| r.into()))?;
        }
        Command::Env { command } => match command {
            EnvCommand::Set {
                name,
                value,
                namespace,
            } => {
                if let Some(ns) = namespace {
                    env.select_variables_namespace(ns);
                }
                env.set_variable(name, value)
                    .map_err(|e| CliError(e.to_string()))?;
            }
            EnvCommand::NS { command } => match command {
                EnvNamespaceCommand::Add { name } => {
                    env.namespaced_variables.insert(name, HashMap::new());
                    env.save_to_file().map_err(|e| CliError(e.to_string()))?;
                }
                EnvNamespaceCommand::Rm { name } => {
                    env.namespaced_variables.remove(&name);
                    env.save_to_file().map_err(|e| CliError(e.to_string()))?;
                }
            },
        },
        Command::Completion { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "rstd", &mut std::io::stdout())
        }
        Command::Lsp => rested::language_server::start(),
        Command::Scratch { command: _ } => {
            let default_editor = env::var("EDITOR").map_err(|e| CliError(e.to_string()))?;

            let file_name = format!(
                "scratch-{:?}.rd",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| CliError(e.to_string()))?
                    .as_millis()
            );

            fs::File::create(&file_name)?;

            std::process::Command::new(default_editor)
                .arg(file_name)
                .spawn()?
                .wait()?;
        }
    };

    Ok(())
}
