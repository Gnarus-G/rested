use clap::{Parser, Subcommand};

use std::{
    error::Error,
    fs,
    io::{stdin, stdout, Write},
    path::PathBuf,
};

mod ast;
mod error;
mod lexer;
mod parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run { file: PathBuf },
}

fn main() {
    if let Err(e) = run() {
        print!("{e}");
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(command) => match command {
            Command::Run { file } => {
                let code = fs::read_to_string(file)?;
                interpret(&code)
            }
        },
        None => repl_loop(|code| interpret(&code)),
    }
}

fn repl_loop<F: Fn(String) -> Result<(), Box<dyn Error>>>(
    on_each_line: F,
) -> Result<(), Box<dyn Error>> {
    print!(":>> ");
    stdout().flush()?;

    for line in stdin().lines() {
        let code = line?;
        on_each_line(code)?;
        print!(":>> ");
        stdout().flush()?;
    }

    Ok(())
}

fn interpret(code: &str) -> Result<(), Box<dyn Error>> {
    let lex = lexer::Lexer::new(&code);
    let mut parser = parser::Parser::new(lex);

    let ast = parser.parse().map_err(|e| e.to_string())?;

    for s in ast.requests {
        match s {
            ast::Request::Get(get) => {
                let mut req = ureq::get(get.url);

                if let Some(headers) = get.headers {
                    for h in headers {
                        req = req.set(h.name, h.value);
                    }
                }

                let res = req.call()?.into_string()?;
                println!("{res}");
            }
        }
    }

    Ok(())
}
