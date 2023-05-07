use clap::{Parser, Subcommand};
use interpreter::{runtime::Environment, Interpreter};

use std::{
    collections::HashMap,
    error::Error,
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
        /// namespace in which to look for environment variables
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// path to the script to run
        file: PathBuf,
    },
    /// Operate on the environment variables available in the runtime
    Env {
        #[command(subcommand)]
        command: EnvCommand,
    },
}

#[derive(Debug, Subcommand)]
enum EnvCommand {
    /// Set environment variables available in the runtime
    Set {
        /// namespace for which to set environment variable
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// of the environment variable
        name: String,

        /// of the environment variable
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
        /// of the namespace
        name: String,
    },
    /// Remove a namespace
    Rm {
        /// of the namespace
        name: String,
    },
}

fn main() {
    if let Err(e) = run() {
        eprint!("{e}");
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let mut env = Environment::new(PathBuf::from(".vars.rd.json"))?;

    match cli.command {
        Some(command) => match command {
            Command::Run { file, namespace } => {
                if let Some(ns) = namespace {
                    env.select_variables_namespace(ns);
                }

                let code = fs::read_to_string(file)?;

                Interpreter::new(&code, env).run()?;
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
        },
        None => {
            print!(":>> ");
            stdout().flush()?;

            for line in stdin().lines() {
                let code = line?;

                let env = Environment::new(PathBuf::from(".vars.rd.json"))?;

                Interpreter::new(&code, env).run()?;

                print!(":>> ");
                stdout().flush()?;
            }
        }
    }

    Ok(())
}
