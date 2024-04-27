use super::builtin;
use super::environment::Environment;
use super::value::Value;
use std::collections::HashMap;

use crate::error_meta::ContextualError;
use crate::interpreter::ir::LogDestination;
use crate::interpreter::value::ValueTag;
use crate::lexer;
use crate::parser::ast::{
    self, ConstantDeclaration, Endpoint, Expression, Item, VariableDeclaration,
};

use crate::lexer::locations::GetSpan;

use super::attributes::AttributeStack;
use super::error::{InterpErrorFactory, InterpreterErrorKind};
use super::ir::Header;
use super::ir::RequestItem;

type Result<T> = std::result::Result<T, Box<ContextualError<InterpreterErrorKind>>>;

pub struct Evaluator<'source, 'p, 'env> {
    program: &'p ast::Program<'source>,
    error_factory: InterpErrorFactory<'source>,
    env: &'env Environment,
    base_url: Option<String>,
    pub let_bindings: HashMap<&'source str, Value>,
    attributes: AttributeStack<'source, 'p>,
}

impl<'source, 'p, 'env> Evaluator<'source, 'p, 'env> {
    pub fn new(program: &'p ast::Program<'source>, env: &'env Environment) -> Self {
        Self {
            error_factory: InterpErrorFactory::new(program.source),
            program,
            env,
            base_url: None,
            let_bindings: HashMap::new(),
            attributes: AttributeStack::new(),
        }
    }

    pub fn evaluate(
        &mut self,
    ) -> std::result::Result<Vec<RequestItem>, Box<[ContextualError<InterpreterErrorKind>]>> {
        let mut requests = vec![];

        let mut errors_in_items: Vec<ContextualError<InterpreterErrorKind>> = vec![];

        for item in self.program.items.iter() {
            match self.evaluate_item(item) {
                Ok(Some(r)) => requests.push(r),
                Err(error) => errors_in_items.push(*error),
                _ => {}
            };
        }

        if !errors_in_items.is_empty() {
            return Err(errors_in_items.into());
        }

        Ok(requests)
    }

    fn evaluate_item(&mut self, item: &'p Item<'source>) -> Result<Option<RequestItem>> {
        use ast::Item::*;
        match item {
            Request(ast::Request {
                method,
                endpoint,
                block,
                span,
            }) => {
                // Handle @skip
                if self.attributes.get("skip").is_some() {
                    self.attributes.clear();
                    return Ok(None);
                }

                let span = span.to_end_of(endpoint.span());

                let path = self.evaluate_request_endpoint(endpoint)?;

                let mut headers = vec![];
                let mut body: Option<String> = None;

                if let Some(statements) = block.as_ref().map(|b| &b.statements) {
                    for statement in statements.iter() {
                        match statement {
                            ast::Statement::Header { name, value } => {
                                match self.evaluate_expression(value)? {
                                    Value::String(value) => headers
                                        .push(Header::new(name.get()?.value.to_string(), value)),
                                    val => return Err(self
                                        .error_factory
                                        .type_mismatch(ValueTag::String, val, value.span())
                                        .with_message(
                                            "maybe you want to stringify it with a json(..) call",
                                        )
                                        .into()),
                                }
                            }
                            ast::Statement::Body { value, .. } => {
                                if body.is_none() {
                                    body = match self.evaluate_expression(value)? {
                                            Value::String(value) => Some(value),
                                            val => {
                                                return Err(self
                                                    .error_factory
                                                    .type_mismatch(
                                                        ValueTag::String,
                                                        val,
                                                        value.span(),
                                                    )
                                                    .with_message("maybe you want to stringify it with a json(..) call")
                                                    .into())
                                            }
                                        }
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

                let name_of_request = match self.attributes.get("name") {
                    Some(att) => {
                        if let Some(args) = att.params {
                            let [arg] = self.expect_x_args::<1>(args)?;
                            let value = match self.evaluate_expression(arg)? {
                                Value::String(value) => value,
                                val => {
                                    return Err(self
                                        .error_factory
                                        .type_mismatch(ValueTag::String, val, arg.span())
                                        .into())
                                }
                            };
                            Some(value)
                        } else {
                            return Err(self
                                .error_factory
                                .required_args(att.identifier.span(), 1, 0)
                                .with_message(
                                    "@name(..) must be given an argument, like @name(\"req_1\")",
                                )
                                .into());
                        }
                    }
                    None => None,
                };

                let log_destination = if let Some(att) = self.attributes.get("log") {
                    if let Some(args) = att.params {
                        let [arg] = self.expect_x_args::<1>(args)?;
                        let file_path = match self.evaluate_expression(arg)? {
                            Value::String(value) => value,
                            val => {
                                return Err(self
                                    .error_factory
                                    .type_mismatch(ValueTag::String, val, arg.span())
                                    .into())
                            }
                        };
                        Some(LogDestination::File(file_path.into()))
                    } else {
                        return Err(self
                            .error_factory
                            .required_args(att.identifier.span(), 1, 0)
                            .with_message("@log(..) must be given a file path argument")
                            .into());
                    }
                } else {
                    None
                };

                let r = RequestItem {
                    name: name_of_request,
                    dbg: self.attributes.get("dbg").is_some(),
                    log_destination,
                    span,
                    request: super::ir::Request {
                        method: *method,
                        url: path,
                        headers: headers.into(),
                        body,
                    },
                };

                self.attributes.clear();

                return Ok(Some(r));
            }
            Set(ConstantDeclaration { identifier, value }) => {
                let identifier = identifier.get()?;
                if identifier.text != "BASE_URL" {
                    return Err(self.error_factory.unknown_constant(identifier).into());
                }

                self.base_url = match self.evaluate_expression(value)? {
                    Value::String(s) => Some(s),
                    expr => {
                        return Err(self
                            .error_factory
                            .type_mismatch(ValueTag::String, expr, value.span())
                            .into())
                    }
                };
            }
            LineComment(_) => {}
            Attribute {
                identifier,
                arguments,
                ..
            } => {
                let identifier = identifier.get()?;

                match identifier.text {
                    "name" | "log" | "dbg" | "skip" => {
                        if self.attributes.has(identifier.text) {
                            return Err(self.error_factory.duplicate_attribute(identifier).into());
                        }
                        self.attributes.add(identifier, arguments.as_ref());
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
            Let(VariableDeclaration { identifier, value }) => {
                let value = self.evaluate_expression(value)?;
                self.let_bindings.insert(identifier.get()?.text, value);
            }
            Expr(_) => {}
            Error(err) => {
                unreachable!(
                    "all syntax errors should have been caught, but found {}",
                    err
                )
            }
        }

        Ok(None)
    }

    fn evaluate_expression(&self, exp: &Expression<'source>) -> Result<Value> {
        use Expression::*;

        let value = match exp {
            Identifier(token) => self.evaluate_identifier(token.get()?)?,
            String(token) => token.value.into(),
            TemplateSringLiteral { parts, .. } => {
                self.evaluate_template_string_literal_parts(parts)?
            }
            Bool((_, b)) => Value::Bool(*b),
            Number((_, n)) => Value::Number(*n),
            Call(expr) => self.evaluate_call_expression(expr)?,
            Array(values) => {
                let mut v = vec![];

                for value in values.iter() {
                    v.push(self.evaluate_expression(value)?);
                }

                Value::Array(v.into())
            }
            Object((.., fields)) => {
                let mut props = HashMap::new();

                for node in fields.iter() {
                    let ast::ObjectEntry { key, value } = node.get()?;
                    let value = self.evaluate_expression(value)?;
                    props.insert(key.get()?.value.to_string(), value);
                }

                Value::Object(props)
            }
            EmptyArray(_) => Value::Array(Box::new([])),
            EmptyObject(_) => Value::Object(HashMap::new()),
            Null(_) => Value::Null,
            Error(err) => unreachable!(
                "all syntax errors should have been caught, but found {}",
                err
            ),
        };

        Ok(value)
    }

    fn evaluate_call_expression(&self, expr: &ast::CallExpr) -> Result<Value> {
        let ast::CallExpr {
            identifier,
            arguments,
        } = expr;

        let string_value = match identifier.get()?.text {
            "env" => self.evaluate_env_call(arguments)?,
            "read" => self.evaluate_read_call(arguments)?,
            "escape_new_lines" => self.evaluate_escapes_new_lines_call(arguments)?,
            "json" => self.evaluate_json_call(arguments)?,
            _ => {
                return Err(self
                    .error_factory
                    .undefined_callable(identifier.get()?)
                    .with_message(
                        "env(..), read(..), json(..), and escape_new_lines(..) are the only calls supported",
                    )
                    .into())
            }
        };

        Ok(string_value)
    }

    fn evaluate_env_call(&self, arguments: &ast::ExpressionList) -> Result<Value> {
        let [arg] = self.expect_x_args::<1>(arguments)?;

        let value = match self.evaluate_expression(arg)? {
            Value::String(variable) => builtin::call_env(self.env, &variable).ok_or_else(|| {
                return self
                    .error_factory
                    .env_variable_not_found(variable, arg.span());
            })?,
            value => {
                return Err(self
                    .error_factory
                    .type_mismatch(ValueTag::String, value, arg.span())
                    .into())
            }
        };

        Ok(value)
    }

    fn evaluate_read_call(&self, arguments: &ast::ExpressionList) -> Result<Value> {
        let [arg] = self.expect_x_args::<1>(arguments)?;

        let value = match self.evaluate_expression(arg)? {
            Value::String(file_name) => builtin::read_file(file_name)
                .map_err(|e| self.error_factory.other(arg.span(), e))?,
            value => {
                return Err(self
                    .error_factory
                    .type_mismatch(ValueTag::String, value, arg.span())
                    .into())
            }
        };

        Ok(value)
    }

    fn evaluate_escapes_new_lines_call(&self, arguments: &ast::ExpressionList) -> Result<Value> {
        let [arg] = self.expect_x_args::<1>(arguments)?;

        let v = match self.evaluate_expression(arg)? {
            Value::String(s) => builtin::escaping_new_lines(s),
            value => {
                return Err(self
                    .error_factory
                    .type_mismatch(ValueTag::String, value, arg.span())
                    .into())
            }
        };

        Ok(v)
    }

    fn evaluate_json_call(&self, arguments: &ast::ExpressionList) -> Result<Value> {
        let [arg] = self.expect_x_args::<1>(arguments)?;

        let value = self.evaluate_expression(arg)?;

        Ok(builtin::json_stringify(value))
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
            Endpoint::Expr(expr) => match self.evaluate_expression(expr)? {
                Value::String(s) => s,
                value => {
                    return Err(self
                        .error_factory
                        .type_mismatch(ValueTag::String, value, expr.span())
                        .into())
                }
            },
        };

        Ok(url)
    }

    fn evaluate_identifier(&self, token: &lexer::Token<'source>) -> Result<Value> {
        let value = self
            .let_bindings
            .get(token.text)
            .ok_or_else(|| self.error_factory.undeclared_identifier(token))?;

        Ok(value.to_owned())
    }

    fn evaluate_template_string_literal_parts(
        &self,
        parts: &[Expression<'source>],
    ) -> Result<Value> {
        let mut strings = vec![];

        for part in parts {
            let value = match self.evaluate_expression(part)? {
                Value::String(value) => value,
                val => {
                    return Err(Box::new(
                        self.error_factory
                            .type_mismatch(ValueTag::String, val, part.span())
                            .with_message("try a json(..) call to stringify this expression"),
                    ))
                }
            };
            strings.push(value.to_string());
        }

        Ok(strings.join("").into())
    }

    fn expect_x_args<'a, const N: usize>(
        &self,
        args: &'a ast::ExpressionList<'source>,
    ) -> Result<[&'a ast::Expression; N]> {
        if args.exprs.len() != N {
            return Err(self
                .error_factory
                .required_args(args.span, N, args.exprs.len())
                .into());
        };

        // SAFETY: we're checking above N equals how many args we got
        // so there will be no nulls in the returned value.
        let mut arguments = unsafe {
            let null: *const ast::Expression = std::ptr::null();
            [&*null; N]
        };

        for (i, arg) in args.iter().enumerate() {
            arguments[i] = arg;
        }

        Ok(arguments)
    }
}
