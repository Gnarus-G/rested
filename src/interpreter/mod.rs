mod attributes;
mod builtin;
pub mod environment;
pub mod error;
pub mod ir;
pub mod runner;
pub mod ureq_runner;
mod value;

use anyhow::Context;
use environment::Environment;
use error::InterpreterError;
use std::collections::HashMap;

use crate::error_meta::ContextualError;
use crate::interpreter::ir::LogDestination;
use crate::lexer;
use crate::parser::ast::{self, Endpoint, Expression};

use crate::lexer::locations::{GetSpan, Span};
use crate::parser::error::ParserErrors;

use self::error::{InterpErrorFactory, InterpreterErrorKind};
use self::ir::RequestItem;
use attributes::AttributeStore;
use ir::Header;

type Result<T> = std::result::Result<T, Box<ContextualError<InterpreterErrorKind>>>;

impl<'source> ast::Program<'source> {
    pub fn interpret(
        &self,
        env: &Environment,
    ) -> std::result::Result<ir::Program<'source>, InterpreterError<'source>> {
        let parse_errors = self.errors();

        if !parse_errors.is_empty() {
            return Err(ParserErrors::new(parse_errors).into());
        }

        let mut interp = Interpreter::new(self, env);

        let items = interp.evaluate()?;

        Ok(ir::Program::new(self.source, items.into()))
    }
}

pub struct Interpreter<'source, 'p, 'env> {
    program: &'p ast::Program<'source>,
    error_factory: InterpErrorFactory<'source>,
    env: &'env Environment,
    base_url: Option<String>,
    let_bindings: HashMap<&'source str, String>,
}

impl<'source, 'p, 'env> Interpreter<'source, 'p, 'env> {
    pub fn new(program: &'p ast::Program<'source>, env: &'env Environment) -> Self {
        Self {
            error_factory: InterpErrorFactory::new(program.source),
            program,
            env,
            base_url: None,
            let_bindings: HashMap::new(),
        }
    }

    pub fn evaluate(&mut self) -> Result<Vec<RequestItem>> {
        use ast::Item::*;

        let mut attributes = AttributeStore::new();

        let mut requests = vec![];

        for item in self.program.items.iter() {
            match item {
                Request {
                    method,
                    endpoint,
                    block,
                    span,
                } => {
                    // Handle @skip
                    if attributes.get("skip").is_some() {
                        attributes.clear();
                        continue;
                    }

                    let span = span.to_end_of(endpoint.span());

                    let path = self.evaluate_request_endpoint(endpoint)?;

                    let mut headers = vec![];
                    let mut body: Option<String> = None;

                    if let Some(statements) = block.as_ref().map(|b| &b.statements) {
                        for statement in statements.iter() {
                            match statement {
                                ast::Statement::Header { name, value } => {
                                    headers.push(Header::new(
                                        name.get()?.value.to_string(),
                                        self.evaluate_expression(value)?.to_string(),
                                    ))
                                }
                                ast::Statement::Body { value, .. } => {
                                    if body.is_none() {
                                        body = Some(self.evaluate_expression(value)?.to_json());
                                    }
                                }
                                ast::Statement::LineComment(_) => {}
                                ast::Statement::Error(err) => {
                                    unreachable!(
                                        "all syntax errors should have been caught, but found {}",
                                        err
                                    )
                                }
                            }
                        }
                    }

                    let name_of_request = match attributes.get("name") {
                        Some(att) => {
                            let exp = att.first_params().ok_or_else(|| {
                                self.error_factory
                                    .required_args(att.span, 1, 0)
                                    .with_message(
                                    "@name(..) must be given an argument, like @name(\"req_1\")",
                                )
                            })?;

                            Some(self.evaluate_expression(exp)?)
                        }
                        None => None,
                    };

                    let log_destination = if let Some(att) = attributes.get("log") {
                        if let Some(arg_exp) = att.first_params() {
                            let value = self.evaluate_expression(arg_exp)?;
                            let file_path = value.to_string().into();
                            Some(LogDestination::File(file_path))
                        } else {
                            Some(LogDestination::Std)
                        }
                    } else {
                        None
                    };

                    requests.push(RequestItem {
                        name: name_of_request.map(|nr| nr.to_string()),
                        dbg: attributes.get("dbg").is_some(),
                        log_destination,
                        span,
                        request: ir::Request {
                            method: *method,
                            url: path,
                            headers: headers.into(),
                            body,
                        },
                    });

                    attributes.clear();
                }
                Set { identifier, value } => {
                    let identifier = identifier.get()?;
                    if identifier.text != "BASE_URL" {
                        return Err(self.error_factory.unknown_constant(identifier).into());
                    }

                    self.base_url = match self.evaluate_expression(value)? {
                        value::Value::String(s) => Some(s),
                        value::Value::Null => todo!(),
                        value::Value::Bool(_) => todo!(),
                        value::Value::Number(_) => todo!(),
                        value::Value::Array(elements) => todo!(),
                        value::Value::Object(map) => todo!(),
                    };
                }
                LineComment(_) => {}
                Attribute {
                    identifier,
                    parameters,
                    ..
                } => {
                    let identifier = identifier.get()?;

                    match identifier.text {
                        "name" | "log" | "dbg" | "skip" => {
                            if attributes.has(identifier.text) {
                                return Err(self
                                    .error_factory
                                    .duplicate_attribute(identifier)
                                    .into());
                            }

                            let att_params = parameters.as_ref().map(|p| &p.parameters);

                            attributes.add(identifier, att_params);
                        }
                        _ => {
                            return Err(self
                                .error_factory
                                .unsupported_attribute(identifier)
                                .with_message(
                                    "@name, @log, @skip and @dbg are the only supported attributes",
                                )
                                .into());
                        }
                    }
                }
                Let { identifier, value } => {
                    let value = self.evaluate_expression(value)?;
                    self.let_bindings
                        .insert(identifier.get()?.text, value.to_string());
                }
                Expr(_) => continue,
                Error(err) => {
                    unreachable!(
                        "all syntax errors should have been caught, but found {}",
                        err
                    )
                }
            }
        }

        Ok(requests)
    }

    fn evaluate_expression(&self, exp: &Expression<'source>) -> Result<value::Value> {
        use Expression::*;

        let expression_span = exp.span();

        let value = match exp {
            Identifier(token) => self.evaluate_identifier(token.get()?)?,
            String(token) => token.value.into(),
            TemplateSringLiteral { parts, .. } => {
                self.evaluate_template_string_literal_parts(parts)?
            }
            Bool(literal) => value::Value::Bool(
                literal
                    .value
                    // TODO: do this in the parser
                    .parse()
                    .expect("our parse should not allow this"),
            ),
            Number(literal) => value::Value::Number(
                literal
                    .value
                    // TODO: do this in the parser
                    .parse()
                    .context("failed to parse as an unsigned int")
                    .expect("our parser should not allow this"),
            ),
            Call {
                identifier,
                arguments,
            } => self.evaluate_call_expression(
                identifier.get()?,
                &arguments.parameters,
                expression_span,
            )?,
            Array((.., values)) => {
                let mut v = vec![];

                for value in values.iter() {
                    v.push(self.evaluate_expression(&value.expr)?);
                }

                value::Value::Array(v.into())
            }
            Object((.., fields)) => {
                let mut props = HashMap::new();

                for ast::ObjectEntry { key, value, .. } in fields {
                    let value = self.evaluate_expression(value)?;
                    props.insert(key.get()?.value.to_string(), value);
                }

                value::Value::Object(props)
            }
            EmptyArray(_) => value::Value::Array(Box::new([])),
            EmptyObject(_) => value::Value::Object(HashMap::new()),
            Null(_) => value::Value::Null,
            Error(err) => unreachable!(
                "all syntax errors should have been caught, but found {}",
                err
            ),
        };

        Ok(value)
    }

    fn evaluate_call_expression(
        &self,
        identifier: &lexer::Token<'source>,
        arguments: &[Expression<'source>],
        expression_span: Span,
    ) -> Result<value::Value> {
        let string_value = match identifier.text {
            "env" => {
                let arg = arguments.first().ok_or_else(|| {
                    self.error_factory
                        .required_args(expression_span, 1, 0)
                        .with_message("calls to env(..) must include a variable name argument")
                })?;

                let value = self.evaluate_expression(arg)?;

                builtin::call_env(self.env, value).map_err(|e| match e {
                    builtin::CallEnvError::NotFound(v) => self
                        .error_factory
                        .env_variable_not_found(v, expression_span),
                    _ => self.error_factory.other(expression_span, e),
                })?
            }
            "read" => {
                let arg = arguments.first().ok_or_else(|| {
                    self.error_factory
                        .required_args(expression_span, 1, 0)
                        .with_message("calls to read(..) must include a file name argument")
                })?;

                let file_name = self.evaluate_expression(arg)?;

                builtin::read_file(file_name)
                    .map_err(|e| self.error_factory.other(expression_span, e))?
            }
            "escape_new_lines" => {
                let arg = arguments.first().ok_or_else(|| {
                    self.error_factory
                        .required_args(expression_span, 1, 0)
                        .with_message("calls to escape_new_lines(..) must include an argument")
                })?;

                let value = self.evaluate_expression(arg)?;

                escaping_new_lines(value)
            }
            _ => {
                return Err(self
                    .error_factory
                    .undefined_callable(identifier)
                    .with_message(
                        "env(..), read(..), escape_new_lines(..) are the only calls supported",
                    )
                    .into())
            }
        };

        Ok(string_value)
    }

    fn evaluate_request_endpoint(&self, endpoint: &Endpoint) -> Result<String> {
        let url = match endpoint {
            Endpoint::Url(url) => url.value.to_string(),
            Endpoint::Pathname(pn) => {
                if let Some(mut base_url) = self.base_url.clone() {
                    if pn.value.len() > 1 {
                        base_url.push_str(pn.value);
                    }
                    base_url
                } else {
                    return Err(self.error_factory.unset_base_url(pn.span).into());
                }
            }
        };

        Ok(url)
    }

    fn evaluate_identifier(&self, token: &lexer::Token<'source>) -> Result<value::Value> {
        let value = self
            .let_bindings
            .get(token.text)
            .map(|value| value.to_string())
            .ok_or_else(|| {
                self.error_factory
                    .undeclared_identifier(token)
                    .with_message("variable identifiers are not supported")
            })?;

        Ok(value.into())
    }

    fn evaluate_template_string_literal_parts(
        &self,
        parts: &[Expression<'source>],
    ) -> Result<value::Value> {
        let mut strings = vec![];

        for part in parts {
            let value = self.evaluate_expression(part)?;
            strings.push(value.to_string());
        }

        Ok(strings.join("").into())
    }
}

fn escaping_new_lines(text: value::Value) -> value::Value {
    if let value::Value::String(text) = text {
        let mut s = String::new();
        for line in text.lines() {
            s.push_str(line);
            s.push_str("\\n")
        }
        return s.into();
    }
    text
}
