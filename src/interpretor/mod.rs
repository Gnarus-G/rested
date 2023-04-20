mod error;
pub mod runtime;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;

use colored::Colorize;

use crate::ast::{self, Expression, Literal, UrlOrPathname};

use crate::error::Error;
use crate::parser;

use self::error::{InterpError, InterpErrorFactory};
use self::runtime::Environment;

type Result<T> = std::result::Result<T, Error<InterpError>>;

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

    pub fn run(&mut self) -> Result<()> {
        let mut parser = parser::Parser::new(self.code);

        let ast = parser.parse()?;

        use ast::Item::*;

        let mut attribute_items: HashMap<&'i str, Vec<Expression>> = HashMap::new();

        for item in ast.items {
            match item {
                Request {
                    location,
                    method,
                    endpoint,
                    params,
                } => {
                    let path = &self.evaluate_request_endpoint(endpoint)?;
                    let mut req = match method {
                        ast::RequestMethod::GET => ureq::get(path),
                        ast::RequestMethod::POST => ureq::post(path),
                    };

                    let mut body = None;

                    for statement in params {
                        match statement {
                            ast::Statement::Header { name, value } => {
                                req = req.set(&name.value, &self.evaluate_expression(&value)?);
                            }
                            ast::Statement::Body { value, .. } => {
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
                            method.to_string().yellow().bold(),
                            path.bold()
                        )
                    );

                    if let Some(_) = attribute_items.get("dbg") {
                        println!("    \u{21B3} with request data:");
                        println!("{}", indent_lines(&format!("{:#?}", req), 6));

                        println!(
                            "{}",
                            indent_lines(
                                &format!("Body: {:#?}", body.clone().unwrap_or_default()),
                                6
                            )
                        );
                    }

                    let res = if let Some(value) = body {
                        let res = req
                            .send_string(&value)
                            .map_err(|e| self.error_factory.other(location, e))?;

                        if res.content_type() == "application/json" {
                            let string = &res
                                .into_string()
                                .map_err(|e| self.error_factory.other(location, e))?;
                            prettify_json_string(string)
                                .map_err(|e| self.error_factory.other(location, e))?
                        } else {
                            res.into_string()
                                .map_err(|error| self.error_factory.other(location, error))?
                        }
                    } else {
                        req.call()
                            .map_err(|error| self.error_factory.other(location, error))?
                            .into_string()
                            .map_err(|error| self.error_factory.other(location, error))?
                    };

                    if let Some(att_params) = attribute_items.get("log") {
                        if let Some(arg_exp) = att_params.first() {
                            let file_path = self.evaluate_expression(arg_exp)?.into();

                            log(&res, &file_path)
                                .map_err(|error| self.error_factory.other(location, error))?;

                            println!(
                                "    \u{21B3} {}",
                                format!("saved response to {:?}", file_path).blue()
                            );
                        } else {
                            println!("{}", indent_lines(&res, 4));
                        }
                    }

                    attribute_items.clear();
                }
                Set { identifier, value } => {
                    if identifier.name != "BASE_URL" {
                        return Err(self.error_factory.unknown_constant(&identifier));
                    }

                    self.env.base_url = Some(self.evaluate_expression(&value)?);
                }
                LineComment(_) => {}
                Attribute {
                    identifier,
                    parameters,
                    ..
                } => match identifier.name {
                    "log" | "dbg" => {
                        attribute_items.insert(identifier.name, parameters);
                    }
                    _ => {
                        return Err(self
                            .error_factory
                            .unsupported_attribute(&identifier)
                            .with_message("@log(..) and @dbg are the only supported attributes")
                            .into())
                    }
                },
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
            } => match identifier.name {
                "env" => {
                    let arg = arguments.first().ok_or_else(|| {
                        self.error_factory
                            .required_call_args(identifier.location, 1, 0)
                            .with_message("calls to env(..) must include a variable name argument")
                    })?;

                    let value = match arg {
                        Identifier(token) => self.evaluate_identifier(token)?,
                        Call { identifier, .. } => {
                            let value = self.evaluate_expression(&arg)?;

                            self.evaluate_env_variable(&Literal {
                                value: &value,
                                location: identifier.location,
                            })?
                        }
                        StringLiteral(n) => self.evaluate_env_variable(&n)?,
                        TemplateSringLiteral { parts } => {
                            self.evaluate_template_string_literal_parts(parts)?
                        }
                    };

                    value
                }
                "read" => {
                    let arg = arguments.first().ok_or_else(|| {
                        self.error_factory
                            .required_call_args(identifier.location, 1, 0)
                            .with_message("calls to read(..) must include a file name argument")
                    })?;

                    let value = match arg {
                        Identifier(token) => self.evaluate_identifier(token)?,
                        Call { identifier, .. } => {
                            let value = self.evaluate_expression(&arg)?;
                            self.read_file(&Literal {
                                value: &value,
                                location: identifier.location,
                            })?
                        }
                        StringLiteral(n) => self.read_file(n)?.escape_debug().to_string(),
                        TemplateSringLiteral { parts } => {
                            self.evaluate_template_string_literal_parts(parts)?
                        }
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
            TemplateSringLiteral { parts } => self.evaluate_template_string_literal_parts(parts)?,
        };

        Ok(value)
    }

    fn evaluate_env_variable(&self, token: &Literal<'i>) -> Result<String> {
        self.env
            .get_variable_value(token.value.to_string())
            .ok_or_else(|| self.error_factory.env_variable_not_found(token))
            .map(|s| s.to_owned())
    }

    fn read_file(&self, file_path: &Literal) -> Result<String> {
        let mut file = File::open(file_path.value)
            .map_err(|e| self.error_factory.other(file_path.location, e))?;

        let mut string = String::new();

        file.read_to_string(&mut string)
            .map_err(|e| self.error_factory.other(file_path.location, e))?;

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
                    return Err(self.error_factory.unset_base_url(pn.location));
                }
            }
        })
    }

    fn evaluate_identifier(&self, token: &ast::Identifier) -> Result<String> {
        return Err(self
            .error_factory
            .undeclared_identifier(token)
            .with_message("variable identifiers are not supported"));
    }

    fn evaluate_template_string_literal_parts(&self, parts: &[Expression]) -> Result<String> {
        let mut strings = vec![];

        for part in parts {
            let s = self.evaluate_expression(part)?;
            strings.push(s);
        }

        Ok(strings.join(""))
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
