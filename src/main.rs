use clap::{Parser, Subcommand};

use std::{
    collections::HashMap,
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
    Run {
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
        /// of the environment variable
        name: String,

        /// of the environment variable
        value: String,
    },
}

fn main() {
    if let Err(e) = run() {
        print!("{e}");
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let mut env = Environment::new(PathBuf::from(".vars.rd.json"));

    env.load_variables_from_file()?;

    match cli.command {
        Some(command) => match command {
            Command::Run { file } => {
                let code = fs::read_to_string(file)?;
                interpret(&code, &mut env)?
            }
            Command::Env { command } => match command {
                EnvCommand::Set { name, value } => env.set_variable(name, value)?,
            },
        },
        None => repl_loop(|code| interpret(&code, &mut env))?,
    }

    Ok(())
}

fn repl_loop<F: FnMut(String) -> Result<(), Box<dyn Error>>>(
    mut on_each_line: F,
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

struct Environment {
    env_file_name: PathBuf,
    variables: HashMap<String, String>,
    base_url: Option<String>,
}

impl Environment {
    fn new(file_name: PathBuf) -> Self {
        Self {
            env_file_name: file_name,
            variables: HashMap::new(),
            base_url: None,
        }
    }

    fn load_variables_from_file(&mut self) -> Result<(), Box<dyn Error>> {
        let file = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.env_file_name)?;

        let reader = std::io::BufReader::new(file);

        self.variables = serde_json::from_reader(reader).unwrap_or(HashMap::new());

        Ok(())
    }

    fn set_variable(&mut self, name: String, value: String) -> Result<(), Box<dyn Error>> {
        self.variables.insert(name, value);

        let file = std::fs::File::options()
            .write(true)
            .open(&self.env_file_name)?;
        let writer = std::io::BufWriter::new(file);

        serde_json::to_writer_pretty::<_, HashMap<_, _>>(writer, &self.variables)?;

        Ok(())
    }
}

impl<'i> ast::Expression<'i> {
    fn evaluate(self, env: &Environment) -> Result<String, Box<dyn Error>> {
        let value = match self {
            ast::Expression::Identifier(_) => todo!(),
            ast::Expression::StringLiteral(token) => token.value,
            ast::Expression::Call {
                identifier,
                arguments,
            } => match identifier.value {
                "env" => {
                    let arg = arguments
                        .first()
                        .ok_or("calls to env() must include a variable name argument")?;

                    let value = match arg {
                        ast::Expression::Identifier(_) => todo!(),
                        ast::Expression::StringLiteral(n) => env
                            .variables
                            .get(&n.value.to_string())
                            .ok_or(format!("no variable found by the name {:?}", n.value))?,
                        ast::Expression::Call { .. } => todo!(),
                    };

                    value
                }
                _ => todo!("currently only env() identifier is allowed"),
            },
        };

        Ok(value.to_string())
    }
}

impl<'i> ast::UrlOrPathname<'i> {
    fn evaluate(self, env: &Environment) -> Result<String, Box<dyn Error>> {
        Ok(match self {
            ast::UrlOrPathname::Url(url) => url.value.to_string(),
            ast::UrlOrPathname::Pathname(pn) => {
                if let Some(mut base_url) = env.base_url.clone() {
                    base_url.push_str(pn.value);
                    base_url
                } else {
                    panic!(
                        "BASE_URL needs to be set first for requests to work with just pathnames",
                    );
                }
            }
        })
    }
}

fn interpret(code: &str, env: &mut Environment) -> Result<(), Box<dyn Error>> {
    let lex = lexer::Lexer::new(&code);
    let mut parser = parser::Parser::new(lex);

    let ast = parser.parse().map_err(|e| e.to_string())?;

    for s in ast.statements {
        match s {
            ast::Statement::Request(request) => {
                let mut req = match request.method {
                    ast::RequestMethod::GET => ureq::get(&request.endpoint.evaluate(&env)?),
                    ast::RequestMethod::POST => ureq::post(&request.endpoint.evaluate(&env)?),
                };

                let mut body = None;

                for statement in request.params {
                    match statement {
                        ast::Statement::Request(_) => todo!(),
                        ast::Statement::HeaderStatement { name, value } => {
                            req = req.set(&name.value, &value.evaluate(&env)?);
                        }
                        ast::Statement::BodyStatement { value } => {
                            if let None = body {
                                body = Some(value.evaluate(&env)?);
                            }
                        }
                        ast::Statement::ExpressionStatement(_) => todo!(),
                        ast::Statement::SetStatement { .. } => {
                            panic!("set statements are not allowed inside requests")
                        }
                    }
                }

                let res = if let Some(value) = body {
                    req.send_string(&value)?.into_string()?
                } else {
                    req.call()?.into_string()?
                };

                println!("{res}");
            }
            ast::Statement::HeaderStatement { .. } => todo!(),
            ast::Statement::BodyStatement { .. } => todo!(),
            ast::Statement::ExpressionStatement(_) => todo!(),
            ast::Statement::SetStatement { identifier, value } => {
                if identifier.value != "BASE_URL" {
                    panic!("trying to set an unknown constant {}", identifier.value);
                }

                env.base_url = Some(value.evaluate(&env)?);
            }
        }
    }

    Ok(())
}
