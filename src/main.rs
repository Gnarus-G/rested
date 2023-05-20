mod error;

use clap::{CommandFactory, Parser, Subcommand};
use error::ColoredError;
use interpreter::{environment::Environment, ureq_runner::UreqRunner, Interpreter};

use std::{
    collections::HashMap,
    fs,
    io::{stdin, stdout, Write},
    path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(author, version, about)]
/// The CLI runtime for Rested, the language/interpreter for easily defining and running requests to an http server.
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
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
        file: PathBuf,
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

fn main() {
    if let Err(e) = run() {
        eprint!("{e}");
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut env = Environment::new(PathBuf::from(".vars.rd.json"))?;

    match cli.command {
        Some(command) => match command {
            Command::Run {
                file,
                namespace,
                request,
            } => {
                if let Some(ns) = namespace {
                    env.select_variables_namespace(ns);
                }

                let code = fs::read_to_string(file)?;

                Interpreter::new(&code, env, UreqRunner)
                    .run(request)
                    .map_err(ColoredError)?;
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
                    env.set_variable(name, value)?;
                }
                EnvCommand::NS { command } => match command {
                    EnvNamespaceCommand::Add { name } => {
                        env.namespaced_variables.insert(name, HashMap::new());
                        env.save_to_file()?;
                    }
                    EnvNamespaceCommand::Rm { name } => {
                        env.namespaced_variables.remove(&name);
                        env.save_to_file()?;
                    }
                },
            },
            Command::Completion { shell } => {
                clap_complete::generate(shell, &mut Cli::command(), "rstd", &mut std::io::stdout())
            }
            Command::Lsp => language_server::start(),
        },
        None => {
            print!(":>> ");
            stdout().flush()?;

            for line in stdin().lines() {
                let code = line?;

                let env = Environment::new(PathBuf::from(".vars.rd.json"))?;

                Interpreter::new(&code, env, UreqRunner)
                    .run(None)
                    .map_err(ColoredError)?;

                print!(":>> ");
                stdout().flush()?;
            }
        }
    }

    Ok(())
}
