mod cli;

use clap::{CommandFactory, Parser, Subcommand};
use cli::config::ConfigArgs;
use cli::run::RunArgs;
use cli::scratch::ScratchCommandArgs;
use rested::editing::edit;
use rested::interpreter::environment::Environment;
use tracing::error;

use std::collections::HashMap;
use std::fs;

#[derive(Parser, Debug)]
#[command(author, version, about)]
/// The CLI runtime for Rested, the language/interpreter for easily defining and running requests to an http server.
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Set log level
    #[arg(short, long, default_value = "trace", global = true)]
    level: tracing::Level,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a script written in the language
    Run(RunArgs),
    /// Open your default editor to start editing a temporary file
    Scratch(ScratchCommandArgs),
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
    let mut env = Environment::new(rested::config::env_file_path()?)?;

    match cli.command {
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
            EnvCommand::Show => println!("{}", fs::read_to_string(env.env_file_name)?),
            EnvCommand::Edit => edit(&env.env_file_name)?,
        },
        Command::Completion { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "rstd", &mut std::io::stdout())
        }
        Command::Lsp => rested::language_server::start(cli.level),
        Command::Run(run) => run.handle(env)?,
        Command::Scratch(scratch) => scratch.handle(env)?,
        Command::Config(config) => config.handle()?,
    };

    Ok(())
}
