mod error;
pub mod runtime;

use crate::ast::{self, Expression, UrlOrPathname};

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

        for statement in ast.statements {
            match statement {
                ast::Statement::Request {
                    params: request, ..
                } => {
                    let mut req = match request.method {
                        ast::RequestMethod::GET => {
                            ureq::get(&self.evaluate_request_endpoint(request.endpoint)?)
                        }
                        ast::RequestMethod::POST => {
                            ureq::post(&self.evaluate_request_endpoint(request.endpoint)?)
                        }
                    };

                    let mut body = None;

                    for statement in request.params {
                        match statement {
                            ast::Statement::Request { location, .. } => {
                                return Err(Box::new(
                                    self.error_factory
                                        .inapropriate_statement(location)
                                        .with_message(
                                            "requests may not be defined inside other requests",
                                        ),
                                ))
                            }
                            ast::Statement::HeaderStatement { name, value } => {
                                req = req.set(&name.value, &self.evaluate_expression(&value)?);
                            }
                            ast::Statement::BodyStatement { value, .. } => {
                                if let None = body {
                                    body = Some(self.evaluate_expression(&value)?);
                                }
                            }
                            ast::Statement::ExpressionStatement { exp, .. } => {
                                self.evaluate_expression(&exp)?;
                            }
                            ast::Statement::SetStatement { identifier, .. } => {
                                return Err(Box::new(
                                    self.error_factory
                                        .inapropriate_statement(identifier.location)
                                        .with_message(
                                            "set statements are not allowed inside requests",
                                        ),
                                ))
                            }
                            ast::Statement::LineComment(_) => {}
                        }
                    }

                    let res = if let Some(value) = body {
                        req.send_string(&value)?.into_string()?
                    } else {
                        req.call()?.into_string()?
                    };

                    println!("{res}");
                }
                ast::Statement::HeaderStatement { name, .. } => {
                    return Err(Box::new(
                        self.error_factory
                            .inapropriate_statement(name.location)
                            .with_message("header statements only allowed inside requests"),
                    ));
                }
                ast::Statement::BodyStatement { location, .. } => {
                    return Err(Box::new(
                        self.error_factory.inapropriate_statement(location),
                    ));
                }
                ast::Statement::ExpressionStatement { exp, .. } => {
                    self.evaluate_expression(&exp)?;
                }
                ast::Statement::SetStatement { identifier, value } => {
                    if identifier.value != "BASE_URL" {
                        return Err(Box::new(self.error_factory.unknown_constant(&identifier)));
                    }

                    self.env.base_url = Some(self.evaluate_expression(&value)?);
                }
                ast::Statement::LineComment(_) => {}
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
                            .with_message("calls to env() must include a variable name argument")
                    })?;

                    let value = match arg {
                        Identifier(token) => self.evaluate_identifier(token)?,
                        StringLiteral(n) => self
                            .env
                            .get_variable_value(n.value.to_string())
                            .ok_or_else(|| self.error_factory.variable_not_found(n))?
                            .to_string(),
                        Call { .. } => self.evaluate_expression(&arg)?,
                    };

                    value
                }
                _ => {
                    return Err(self
                        .error_factory
                        .undefined_callable(&identifier)
                        .with_message("currently only env() identifier is allowed"))
                }
            },
        };

        Ok(value)
    }

    fn evaluate_request_endpoint(&self, enpdpoint: UrlOrPathname) -> Result<String> {
        Ok(match enpdpoint {
            UrlOrPathname::Url(url) => url.value.to_string(),
            UrlOrPathname::Pathname(pn) => {
                if let Some(mut base_url) = self.env.base_url.clone() {
                    base_url.push_str(pn.value);
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
