use clap::{Parser, Subcommand};
use rested::interpretor::{runtime::Environment, Interpreter};

use std::{
    collections::HashMap,
    error::Error,
    fs,
    io::{stdin, stdout, Write},
    path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run {
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        file: PathBuf,
    },
    Env {
        #[command(subcommand)]
        command: EnvCommand,
    },
}

#[derive(Debug, Subcommand)]
enum EnvCommand {
    Set {
        #[arg(short = 'n', long)]
        namespace: Option<String>,

        /// of the environment variable
        name: String,

        /// of the environment variable
        value: String,
    },
    NS {
        #[command(subcommand)]
        command: EnvNamespaceCommand,
    },
}

#[derive(Debug, Subcommand)]
enum EnvNamespaceCommand {
    Add {
        /// of the environment variables namespace
        name: String,
    },

    Rm {
        /// of the environment variables namespace
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

    let mut env = Environment::new(PathBuf::from(".vars.rd.json"));

    env.load_variables_from_file()?;

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

                let mut env = Environment::new(PathBuf::from(".vars.rd.json"));

                env.load_variables_from_file()?;

                Interpreter::new(&code, env).run()?;

                print!(":>> ");
                stdout().flush()?;
            }
        }
    }

    Ok(())
}
