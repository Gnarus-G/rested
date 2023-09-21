use crate::{
    interpreter,
    lexer::Token,
    parser::{
        ast::{self, Expression},
        ast_visit::{self, VisitWith},
    },
};
use tower_lsp::lsp_types::*;

use super::IntoPosition;

pub struct EnvVarsNotInAllNamespaces<'env> {
    pub env: &'env interpreter::environment::Environment,
    pub warnings: Vec<tower_lsp::lsp_types::Diagnostic>,
}

impl<'env> EnvVarsNotInAllNamespaces<'env> {
    pub fn new(env: &'env interpreter::environment::Environment) -> Self {
        Self {
            env,
            warnings: vec![],
        }
    }
}

impl<'env> ast_visit::Visitor for EnvVarsNotInAllNamespaces<'env> {
    fn visit_expr(&mut self, expr: &Expression) {
        expr.visit_children_with(self);

        if let Expression::Call {
            arguments,
            identifier: ast::result::ParsedNode::Ok(Token { text: "env", .. }),
        } = expr
        {
            if let Some(Expression::String(value)) = &arguments.parameters.first() {
                let namespaces_from_which_var_is_missing = self
                    .env
                    .namespaced_variables
                    .iter()
                    .filter(|(_, vars)| !vars.contains_key(&value.value.to_string()))
                    .map(|(namespace, _)| namespace)
                    .cloned()
                    .collect::<Vec<_>>();

                if !namespaces_from_which_var_is_missing.is_empty() {
                    self.warnings.push(Diagnostic {
                        range: Range {
                            start: value.span.start.into_position(),
                            end: value.span.end.into_position(),
                        },
                        message: format!(
                            "variable '{}' missing from some namespaces: {}",
                            value.value,
                            namespaces_from_which_var_is_missing.join(", ")
                        ),
                        severity: Some(DiagnosticSeverity::WARNING),
                        ..Default::default()
                    })
                }
            }
        }
    }
}
