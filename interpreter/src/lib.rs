mod error;
pub mod runtime;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;

use colored::Colorize;

use parser;
use parser::ast::{self, Endpoint, Expression, Literal};

use error_meta::Error;
use lexer::locations::{GetSpan, Span};

use crate::error::IntoInterpError;
use crate::runtime::AttributeStore;

use self::error::{InterpError, InterpErrorFactory};
use self::runtime::Environment;

type Result<T> = std::result::Result<T, Error<InterpError>>;

pub struct Interpreter<'source> {
    code: &'source str,
    error_factory: InterpErrorFactory<'source>,
    env: Environment,
    base_url: Option<String>,
    let_bindings: HashMap<&'source str, String>,
}

impl<'source> Interpreter<'source> {
    pub fn new(code: &'source str, env: Environment) -> Self {
        Self {
            error_factory: InterpErrorFactory::new(code),
            code,
            env,
            base_url: None,
            let_bindings: HashMap::new(),
        }
    }

    pub fn run(&mut self, request_names: Option<Vec<String>>) -> Result<()> {
        let mut parser = parser::Parser::new(self.code);

        let ast = parser.parse().map_err(|err| err.into_interp_error())?;

        use ast::Item::*;

        let mut attributes = AttributeStore::new();

        for item in ast.items {
            match item {
                Request {
                    span,
                    method,
                    endpoint,
                    params,
                } => {
                    let span = span.to_end_of(endpoint.span());

                    // Handle @skip
                    if let Some(_) = attributes.get("skip") {
                        attributes.clear();
                        continue;
                    }

                    // Handle @name
                    match attributes.get("name") {
                        Some(att) => {
                            let exp = att.first_params().ok_or_else(|| {
                                self.error_factory
                                    .required_args(att.span, 1, 0)
                                    .with_message(
                                    "@name(..) must be given an argument, like @name(\"req_1\")",
                                )
                            })?;

                            let name = self.evaluate_expression(exp)?;

                            if let Some(names) = &request_names {
                                if !names.contains(&name) {
                                    attributes.clear();
                                    continue;
                                }
                            }
                        }
                        None => {
                            if let Some(_) = &request_names {
                                attributes.clear();
                                continue;
                            }
                        }
                    }

                    let path = &self.evaluate_request_endpoint(endpoint)?;
                    let mut req = match method {
                        ast::RequestMethod::GET => ureq::get(path),
                        ast::RequestMethod::POST => ureq::post(path),
                        ast::RequestMethod::PUT => ureq::put(path),
                        ast::RequestMethod::PATCH => ureq::patch(path),
                        ast::RequestMethod::DELETE => ureq::delete(path),
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

                    // Handle @dbg
                    if let Some(_) = attributes.get("dbg") {
                        println!("    \u{21B3} with request data:");
                        println!("{}", indent_lines(&format!("{:#?}", req), 6));

                        println!(
                            "{}",
                            indent_lines(
                                &format!(
                                    "Body: {}",
                                    body.clone().unwrap_or("(no body)".to_string())
                                ),
                                6
                            )
                        );
                    }

                    let res = if let Some(value) = body {
                        let res = req
                            .send_string(&value)
                            .wrap_error_with(&self.error_factory, span)?;

                        if res.content_type() == "application/json" {
                            let string = &res
                                .into_string()
                                .map_err(|e| self.error_factory.other(span, e))?;
                            prettify_json_string(string)
                                .map_err(|e| self.error_factory.other(span, e))?
                        } else {
                            res.into_string()
                                .map_err(|error| self.error_factory.other(span, error))?
                        }
                    } else {
                        req.call()
                            .map_err(|error| self.error_factory.other(span, error))?
                            .into_string()
                            .map_err(|error| self.error_factory.other(span, error))?
                    };

                    // Handle @log
                    if let Some(att) = attributes.get("log") {
                        if let Some(arg_exp) = att.first_params() {
                            let file_path = self.evaluate_expression(arg_exp)?.into();

                            log(&res, &file_path)
                                .map_err(|error| self.error_factory.other(span, error))?;

                            println!(
                                "    \u{21B3} {}",
                                format!("saved response to {:?}", file_path).blue()
                            );
                        } else {
                            println!("{}", indent_lines(&res, 4));
                        }
                    }

                    attributes.clear();
                }
                Set { identifier, value } => {
                    if identifier.name != "BASE_URL" {
                        return Err(self.error_factory.unknown_constant(&identifier));
                    }

                    self.base_url = Some(self.evaluate_expression(&value)?);
                }
                LineComment(_) => {}
                Attribute {
                    identifier,
                    parameters,
                    ..
                } => match identifier.name {
                    "name" | "log" | "dbg" | "skip" => {
                        if attributes.has(identifier.name) {
                            return Err(self.error_factory.duplicate_attribute(&identifier));
                        }
                        attributes.add(identifier, parameters);
                    }
                    _ => {
                        return Err(self
                            .error_factory
                            .unsupported_attribute(&identifier)
                            .with_message(
                                "@name, @log, @skip and @dbg are the only supported attributes",
                            )
                            .into());
                    }
                },
                Let { identifier, value } => {
                    let value = self.evaluate_expression(&value)?;
                    self.let_bindings.insert(identifier.name, value);
                }
            }
        }

        Ok(())
    }

    fn evaluate_expression(&self, exp: &Expression<'source>) -> Result<String> {
        use Expression::*;
        let value = match exp {
            Identifier(token) => self.evaluate_identifier(&token)?,
            String(token) => token.value.to_string(),
            TemplateSringLiteral { parts, .. } => {
                self.evaluate_template_string_literal_parts(parts)?
            }
            Call {
                identifier,
                arguments,
            } => match identifier.name {
                "env" => {
                    let arg = arguments.first().ok_or_else(|| {
                        self.error_factory
                            .required_args(identifier.span, 1, 0)
                            .with_message("calls to env(..) must include a variable name argument")
                    })?;

                    let value = self.evaluate_expression(arg)?;

                    self.evaluate_env_variable(&Literal {
                        value: &value,
                        span: identifier.span,
                    })?
                }
                "read" => {
                    let arg = arguments.first().ok_or_else(|| {
                        self.error_factory
                            .required_args(identifier.span, 1, 0)
                            .with_message("calls to read(..) must include a file name argument")
                    })?;

                    let file_name = self.evaluate_expression(arg)?;

                    self.read_file(&Literal {
                        value: &file_name,
                        span: identifier.span,
                    })?
                }
                "escape_new_lines" => {
                    let arg = arguments.first().ok_or_else(|| {
                        self.error_factory
                            .required_args(identifier.span, 1, 0)
                            .with_message("calls to escape_new_lines(..) must include an argument")
                    })?;

                    let value = self.evaluate_expression(arg)?;

                    escaping_new_lines(&value)
                }
                _ => {
                    return Err(self
                        .error_factory
                        .undefined_callable(&identifier)
                        .with_message(
                            "env(..), read(..), escape_new_lines(..) are the only calls supported",
                        ))
                }
            },
        };

        Ok(value)
    }

    fn evaluate_env_variable(&self, token: &Literal<'source>) -> Result<String> {
        self.env
            .get_variable_value(token.value.to_string())
            .ok_or_else(|| self.error_factory.env_variable_not_found(token))
            .map(|s| s.to_owned())
    }

    fn read_file(&self, file_path: &Literal) -> Result<String> {
        let handle_error = |e| {
            self.error_factory.other(
                file_path.span,
                &format!("Error reading file '{}': {e}", file_path.value),
            )
        };

        let mut file = File::open(file_path.value).map_err(handle_error)?;

        let mut string = String::new();

        file.read_to_string(&mut string).map_err(handle_error)?;

        Ok(string)
    }

    fn evaluate_request_endpoint(&self, endpoint: Endpoint) -> Result<String> {
        Ok(match endpoint {
            Endpoint::Url(url) => url.value.to_string(),
            Endpoint::Pathname(pn) => {
                if let Some(mut base_url) = self.base_url.clone() {
                    if pn.value.len() > 1 {
                        base_url.push_str(pn.value);
                    }
                    base_url
                } else {
                    return Err(self.error_factory.unset_base_url(pn.span));
                }
            }
        })
    }

    fn evaluate_identifier(&self, token: &ast::Identifier) -> Result<String> {
        self.let_bindings
            .get(token.name)
            .map(|value| value.to_string())
            .ok_or_else(|| {
                self.error_factory
                    .undeclared_identifier(token)
                    .with_message("variable identifiers are not supported")
            })
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

fn escaping_new_lines(text: &str) -> String {
    let mut s = String::new();
    for line in text.lines() {
        s.push_str(line);
        s.push_str("\\n")
    }
    s
}

trait ErrorWrapper {
    fn wrap_error_with(
        self,
        error_factory: &InterpErrorFactory,
        location: Span,
    ) -> Result<ureq::Response>;
}

impl ErrorWrapper for std::result::Result<ureq::Response, ureq::Error> {
    fn wrap_error_with(
        self,
        error_factory: &InterpErrorFactory,
        location: Span,
    ) -> Result<ureq::Response> {
        match self {
            Ok(res) => Ok(res),
            Err(err) => {
                let err_string = match err {
                    ureq::Error::Status(status, response) => {
                        format!(
                            "{}: status code {}: {} {:#}",
                            response.get_url().to_owned(),
                            status,
                            response.status_text().to_owned(),
                            response
                                .into_string()
                                .map_err(|e| error_factory.other(location, e))?
                        )
                    }
                    ureq::Error::Transport(_) => err.to_string(),
                };

                Err(error_factory.other(location, err_string))
            }
        }
    }
}
