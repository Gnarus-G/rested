use crate::ast;
use crate::{
    error::{
        interpreting::{InterpError, InterpErrorFactory},
        Error,
    },
    lexer, parser,
    runtime::Environment,
};

type Result<T> = std::result::Result<T, Error<InterpError>>;

impl<'i> ast::Expression<'i> {
    fn evaluate(self, env: &Environment, error_factory: &InterpErrorFactory<'i>) -> Result<String> {
        let value = match self {
            ast::Expression::Identifier(_) => todo!(),
            ast::Expression::StringLiteral(token) => token.value,
            ast::Expression::Call {
                identifier,
                arguments,
            } => match identifier.value {
                "env" => {
                    let arg = arguments.first().ok_or_else(|| {
                        error_factory
                            .required_call_args(&identifier, 1, 0)
                            .with_message("calls to env() must include a variable name argument")
                    })?;

                    let value = match arg {
                        ast::Expression::Identifier(_) => todo!(),
                        ast::Expression::StringLiteral(n) => env
                            .get_variable_value(n.value.to_string())
                            .ok_or_else(|| error_factory.variable_not_found(n))?,
                        ast::Expression::Call { .. } => todo!(),
                    };

                    value
                }
                _ => {
                    return Err(error_factory
                        .undefined_callable(&identifier)
                        .with_message("currently only env() identifier is allowed"))
                }
            },
        };

        Ok(value.to_string())
    }
}

impl<'i> ast::UrlOrPathname<'i> {
    fn evaluate(self, env: &Environment, error_factory: &InterpErrorFactory) -> Result<String> {
        Ok(match self {
            ast::UrlOrPathname::Url(url) => url.value.to_string(),
            ast::UrlOrPathname::Pathname(pn) => {
                if let Some(mut base_url) = env.base_url.clone() {
                    base_url.push_str(pn.value);
                    base_url
                } else {
                    return Err(error_factory.unset_base_url(&pn));
                }
            }
        })
    }
}

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
        let mut env = &mut self.env;
        let lex = lexer::Lexer::new(&self.code);
        let mut parser = parser::Parser::new(lex);

        let ast = parser.parse()?;

        for s in ast.statements {
            match s {
                ast::Statement::Request(request) => {
                    let mut req = match request.method {
                        ast::RequestMethod::GET => {
                            ureq::get(&request.endpoint.evaluate(&env, &self.error_factory)?)
                        }
                        ast::RequestMethod::POST => {
                            ureq::post(&request.endpoint.evaluate(&env, &self.error_factory)?)
                        }
                    };

                    let mut body = None;

                    for statement in request.params {
                        match statement {
                            ast::Statement::Request(_) => todo!(),
                            ast::Statement::HeaderStatement { name, value } => {
                                req = req
                                    .set(&name.value, &value.evaluate(&env, &self.error_factory)?);
                            }
                            ast::Statement::BodyStatement { value } => {
                                if let None = body {
                                    body = Some(value.evaluate(&env, &self.error_factory)?);
                                }
                            }
                            ast::Statement::ExpressionStatement(_) => todo!(),
                            ast::Statement::SetStatement { identifier, .. } => {
                                return Err(Box::new(
                                    self.error_factory
                                        .inapropriate_statement(&identifier)
                                        .with_message(
                                            "set statements are not allowed inside requests",
                                        ),
                                ))
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
                ast::Statement::HeaderStatement { name, .. } => {
                    return Err(Box::new(
                        self.error_factory
                            .inapropriate_statement(&name)
                            .with_message("header statements only allowed inside requests"),
                    ));
                }
                ast::Statement::BodyStatement { .. } => todo!(),
                ast::Statement::ExpressionStatement(e) => {
                    e.evaluate(env, &self.error_factory)?;
                }
                ast::Statement::SetStatement { identifier, value } => {
                    if identifier.value != "BASE_URL" {
                        return Err(Box::new(self.error_factory.unknown_constant(&identifier)));
                    }

                    env.base_url = Some(value.evaluate(&env, &self.error_factory)?);
                }
            }
        }

        Ok(())
    }
}
