mod error;
pub mod runtime;

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;

use colored::Colorize;

use crate::ast::{self, ExactToken, Expression, UrlOrPathname};

use crate::error::Error;
use crate::parser;

use self::error::{InterpError, InterpErrorFactory};
use self::runtime::Environment;

type Result<T> = std::result::Result<T, Error<InterpError>>;

impl<'i> ast::UrlOrPathname<'i> {}

pub struct Interpreter<'i> {
    code: &'i str,
    error_factory: InterpErrorFactory<'i>,
    env: Environment,
}

impl<'i> Interpreter<'i> {
    pub fn new(code: &'i str, env: Environment) -> Self {
        Self {
            error_factory: InterpErrorFactory::new(code),
            code,
            env,
        }
    }

    pub fn run(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut parser = parser::Parser::new(self.code);

        let ast = parser.parse()?;

        use ast::Item::*;

        let mut attribute_items: Vec<(ast::ExactToken, Vec<Expression>)> = vec![];

        for item in ast.items {
            match item {
                Request {
                    params: request, ..
                } => {
                    let path = &self.evaluate_request_endpoint(request.endpoint)?;
                    let mut req = match request.method {
                        ast::RequestMethod::GET => ureq::get(path),
                        ast::RequestMethod::POST => ureq::post(path),
                    };

                    let mut body = None;

                    for statement in request.params {
                        match statement {
                            ast::Statement::HeaderStatement { name, value } => {
                                req = req.set(&name.value, &self.evaluate_expression(&value)?);
                            }
                            ast::Statement::BodyStatement { value, .. } => {
                                if let None = body {
                                    body = Some(self.evaluate_expression(&value)?);
                                }
                            }
                            ast::Statement::LineComment(_) => {}
                        }
                    }

                    println!(
                        "{}",
                        format!(
                            "sending {} request to {}",
                            request.method.to_string().yellow().bold(),
                            path.bold()
                        )
                    );

                    let res = if let Some(value) = body {
                        let res = req.send_string(&value)?;

                        if res.content_type() == "application/json" {
                            prettify_json_string(&res.into_string()?)?
                        } else {
                            res.into_string()?
                        }
                    } else {
                        req.call()?.into_string()?
                    };

                    for (ident, params) in &attribute_items {
                        match ident.value {
                            "log" => {
                                if let Some(arg_exp) = params.first() {
                                    let file_path = self.evaluate_expression(arg_exp)?.into();

                                    log(&res, &file_path)?;

                                    println!(
                                        "    \u{21B3} {}",
                                        format!("saved response to {:?}", file_path).blue()
                                    );
                                } else {
                                    println!("{}", indent_lines(&res, 4));
                                }
                            }
                            _ => {
                                return Err(self
                                    .error_factory
                                    .unsupported_attribute(ident)
                                    .with_message("@log(..) is the only supported attribute")
                                    .into())
                            }
                        }
                    }

                    attribute_items.clear();
                }
                Set { identifier, value } => {
                    if identifier.value != "BASE_URL" {
                        return Err(Box::new(self.error_factory.unknown_constant(&identifier)));
                    }

                    self.env.base_url = Some(self.evaluate_expression(&value)?);
                }
                LineComment(_) => {}
                Attribute {
                    identifier,
                    parameters,
                    ..
                } => {
                    attribute_items.push((identifier, parameters));
                }
            }
        }

        Ok(())
    }

    fn evaluate_expression(&self, exp: &Expression<'i>) -> Result<String> {
        use Expression::*;
        let value = match exp {
            Identifier(token) => self.evaluate_identifier(&token)?,
            StringLiteral(token) => token.value.to_string(),
            Call {
                identifier,
                arguments,
            } => match identifier.value {
                "env" => {
                    let arg = arguments.first().ok_or_else(|| {
                        self.error_factory
                            .required_call_args(&identifier, 1, 0)
                            .with_message("calls to env(..) must include a variable name argument")
                    })?;

                    let value = match arg {
                        Identifier(token) => self.evaluate_identifier(token)?,
                        Call { identifier, .. } => {
                            let value = self.evaluate_expression(&arg)?;

                            self.evaluate_env_variable(&ExactToken {
                                value: &value,
                                location: identifier.location,
                            })?
                        }
                        StringLiteral(n) => self.evaluate_env_variable(&n)?,
                    };

                    value
                }
                "read" => {
                    let arg = arguments.first().ok_or_else(|| {
                        self.error_factory
                            .required_call_args(&identifier, 1, 0)
                            .with_message("calls to read(..) must include a file name argument")
                    })?;

                    let value = match arg {
                        Identifier(token) => self.evaluate_identifier(token)?,
                        Call { identifier, .. } => {
                            let value = self.evaluate_expression(&arg)?;
                            self.read(&ExactToken {
                                value: &value,
                                location: identifier.location,
                            })?
                        }
                        StringLiteral(n) => self.read(n)?,
                    };

                    value
                }
                _ => {
                    return Err(self
                        .error_factory
                        .undefined_callable(&identifier)
                        .with_message("env(..) and read(..) are the only calls supported"))
                }
            },
        };

        Ok(value)
    }

    fn evaluate_env_variable(&self, token: &ExactToken<'i>) -> Result<String> {
        self.env
            .get_variable_value(token.value.to_string())
            .ok_or_else(|| self.error_factory.variable_not_found(token))
            .map(|s| s.to_owned())
    }

    fn read(&self, n: &ExactToken) -> Result<String> {
        let mut file = File::open(n.value).map_err(|e| self.error_factory.other(n, e))?;

        let mut string = String::new();

        file.read_to_string(&mut string)
            .map_err(|e| self.error_factory.other(n, e))?;

        Ok(string)
    }

    fn evaluate_request_endpoint(&self, enpdpoint: UrlOrPathname) -> Result<String> {
        Ok(match enpdpoint {
            UrlOrPathname::Url(url) => url.value.to_string(),
            UrlOrPathname::Pathname(pn) => {
                if let Some(mut base_url) = self.env.base_url.clone() {
                    if pn.value.len() > 1 {
                        base_url.push_str(pn.value);
                    }
                    base_url
                } else {
                    return Err(self.error_factory.unset_base_url(&pn));
                }
            }
        })
    }

    fn evaluate_identifier(&self, token: &ast::ExactToken) -> Result<String> {
        return Err(self
            .error_factory
            .undeclared_variable(token)
            .with_message("variable identifiers are not supported"));
    }
}

fn log(content: &str, to_file: &PathBuf) -> std::io::Result<()> {
    if let Some(dir_path) = to_file.parent() {
        fs::create_dir_all(dir_path)?
    };

    let file = File::options()
        .truncate(true)
        .write(true)
        .create(true)
        .open(to_file)?;

    let mut w = BufWriter::new(file);

    write!(w, "{content}")
}

fn prettify_json_string(string: &str) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&serde_json::from_str::<serde_json::Value>(string)?)
}

fn indent_lines(string: &str, indent: u8) -> String {
    string
        .lines()
        .map(|line| String::from(" ".repeat(indent as usize) + line))
        .collect::<Vec<_>>()
        .join("\n")
}
