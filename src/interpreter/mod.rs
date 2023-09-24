mod attributes;
pub mod environment;
pub mod error;
pub mod ir;
pub mod runner;
pub mod ureq_runner;

use environment::Environment;
use error::InterpreterError;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use crate::error_meta::ContextualError;
use crate::interpreter::ir::LogDestination;
use crate::lexer;
use crate::parser::ast::{self, Endpoint, Expression, Literal};

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
                    let mut body = None;

                    if let Some(statements) = block.as_ref().map(|b| &b.statements) {
                        for statement in statements.iter() {
                            match statement {
                                ast::Statement::Header { name, value } => {
                                    headers.push(Header::new(
                                        name.get()?.value.to_string(),
                                        self.evaluate_expression(value)?,
                                    ))
                                }
                                ast::Statement::Body { value, .. } => {
                                    if body.is_none() {
                                        body = Some(self.evaluate_expression(value)?);
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
                            let file_path = value.into();
                            Some(LogDestination::File(file_path))
                        } else {
                            Some(LogDestination::Std)
                        }
                    } else {
                        None
                    };

                    requests.push(RequestItem {
                        name: name_of_request,
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

                    self.base_url = Some(self.evaluate_expression(value)?);
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
                    self.let_bindings.insert(identifier.get()?.text, value);
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

    fn evaluate_expression(&self, exp: &Expression<'source>) -> Result<String> {
        self.evaluate_expression_and_quote_string(exp, false)
    }

    fn evaluate_expression_and_quote_string(
        &self,
        exp: &Expression<'source>,
        quote_string_literal: bool,
    ) -> Result<String> {
        use Expression::*;

        let expression_span = exp.span();

        let value = match exp {
            Identifier(token) => self.evaluate_identifier(token.get()?)?,
            String(token) => {
                if quote_string_literal {
                    format!("{:?}", token.value)
                } else {
                    token.value.to_string()
                }
            }
            TemplateSringLiteral { parts, .. } => {
                self.evaluate_template_string_literal_parts(parts)?
            }
            Bool(literal) => literal.value.to_string(),
            Number(literal) => literal.value.to_string(),
            Call {
                identifier,
                arguments,
            } => {
                let value = self.evaluate_call_expression(
                    identifier.get()?,
                    &arguments.parameters,
                    expression_span,
                )?;
                if quote_string_literal {
                    format!("{:?}", value)
                } else {
                    value
                }
            }
            Array((.., values)) => {
                let mut v = vec![];

                for value in values.iter() {
                    v.push(self.evaluate_expression_and_quote_string(&value.expr, true)?);
                }

                format!(
                    "[{}]",
                    v.iter()
                        .map(|value| value.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
            Object((.., fields)) => {
                let mut props = HashMap::new();

                for ast::ObjectEntry { key, value, .. } in fields {
                    let value = self.evaluate_expression_and_quote_string(value, true)?;
                    props.insert(key.get()?.value.to_string(), value);
                }

                format!(
                    "{{{}}}",
                    props
                        .iter()
                        .map(|(key, value)| format!("\"{}\": {}", key, value))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
            EmptyArray(_) => "[]".to_string(),
            EmptyObject(_) => "{}".to_string(),
            Null(_) => "null".to_string(),
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
    ) -> Result<String> {
        let string_value = match identifier.text {
            "env" => {
                let arg = arguments.first().ok_or_else(|| {
                    self.error_factory
                        .required_args(expression_span, 1, 0)
                        .with_message("calls to env(..) must include a variable name argument")
                })?;

                let value = self.evaluate_expression(arg)?;

                self.evaluate_env_variable(value, expression_span)?
            }
            "read" => {
                let arg = arguments.first().ok_or_else(|| {
                    self.error_factory
                        .required_args(expression_span, 1, 0)
                        .with_message("calls to read(..) must include a file name argument")
                })?;

                let file_name = self.evaluate_expression(arg)?;

                self.read_file(&Literal {
                    value: &file_name,
                    span: expression_span,
                })?
            }
            "escape_new_lines" => {
                let arg = arguments.first().ok_or_else(|| {
                    self.error_factory
                        .required_args(expression_span, 1, 0)
                        .with_message("calls to escape_new_lines(..) must include an argument")
                })?;

                let value = self.evaluate_expression(arg)?;

                escaping_new_lines(&value)
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

    fn evaluate_env_variable(&self, variable: String, span: Span) -> Result<String> {
        let value = self
            .env
            .get_variable_value(&variable)
            .ok_or_else(|| self.error_factory.env_variable_not_found(variable, span))
            .map(|s| s.to_owned())?;

        Ok(value)
    }

    fn read_file(&self, file_path: &Literal) -> Result<String> {
        let handle_error = |e| {
            self.error_factory.other(
                file_path.span,
                format!("Error reading file '{}': {e}", file_path.value),
            )
        };

        let mut file = File::open(file_path.value).map_err(handle_error)?;

        let mut string = String::new();

        file.read_to_string(&mut string).map_err(handle_error)?;

        Ok(string)
    }

    fn evaluate_request_endpoint(&self, endpoint: &Endpoint) -> Result<String> {
        Ok(match endpoint {
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
        })
    }

    fn evaluate_identifier(&self, token: &lexer::Token<'source>) -> Result<String> {
        let value = self
            .let_bindings
            .get(token.text)
            .map(|value| value.to_string())
            .ok_or_else(|| {
                self.error_factory
                    .undeclared_identifier(token)
                    .with_message("variable identifiers are not supported")
            })?;

        Ok(value)
    }

    fn evaluate_template_string_literal_parts(
        &self,
        parts: &[Expression<'source>],
    ) -> Result<String> {
        let mut strings = vec![];

        for part in parts {
            let s = self.evaluate_expression(part)?;
            strings.push(s);
        }

        Ok(strings.join(""))
    }
}

fn escaping_new_lines(text: &str) -> String {
    let mut s = String::new();
    for line in text.lines() {
        s.push_str(line);
        s.push_str("\\n")
    }
    s
}
