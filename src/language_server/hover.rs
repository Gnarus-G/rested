use tower_lsp::lsp_types::Position;
use tracing::warn;

use crate::{
    interpreter::{environment::Environment, ir},
    lexer::locations::GetSpan,
    parser::{
        ast,
        ast_visit::{self, VisitWith},
    },
};

use super::position::ContainsPosition;

pub struct HoverDocsResolver<'source> {
    program: Option<ir::Program<'source>>,
    position: Position,
    pub docs: Option<String>,
    is_in_env_call: bool,
    env: Environment,
}

impl<'source> HoverDocsResolver<'source> {
    pub fn new(
        program: Option<ir::Program<'source>>,
        position: Position,
        env: Environment,
    ) -> Self {
        Self {
            program,
            position,
            docs: None,
            is_in_env_call: false,
            env,
        }
    }
}

impl<'source> ast_visit::Visitor<'source> for HoverDocsResolver<'source> {
    fn visit_call_expr(&mut self, expr: &ast::CallExpr<'source>) {
        if let ast::result::ParsedNode::Ok(ident) = &expr.identifier {
            if ident.text == "env" {
                self.is_in_env_call = true
            }
        };

        if expr.identifier.span().contains(&self.position) {
            if let ast::result::ParsedNode::Ok(ident) = &expr.identifier {
                let docs = match ident.text {
                    "env" => [
                        "Read env file to grab values.",
                        "Read `.env.rd.json` from the current workspace if there is one,",
                        "otherwise read that in the home directory.",
                        "```typescript",
                        "(builtin) env(variable: string): string",
                        "```",
                    ]
                    .join("\n"),
                    "json" => [
                        "Convert any value to a json string.",
                        "```typescript",
                        "(builtin) json(value: any): string",
                        "```",
                    ]
                    .join("\n"),
                    "read" => [
                        "Read file contents into a string and returns that string.",
                        "```typescript",
                        "(builtin) read(filename: string): string",
                        "```",
                    ]
                    .join("\n"),
                    "escape_new_lines" => [
                        "Escape the '\\n' characters in a string.",
                        "```typescript",
                        "(builtin) escape_new_lines(value: string): string",
                        "```",
                    ]
                    .join("\n"),
                    _ => "".to_string(),
                };

                self.docs = Some(docs);
                return;
            };
        }

        expr.visit_children_with(self);
    }

    fn visit_string(&mut self, stringlit: &ast::StringLiteral<'source>) {
        if stringlit.span.contains(&self.position) && self.is_in_env_call {
            let var = &stringlit.value.to_string();
            let values: Vec<String> = self
                .env
                .get_variable_value_per_namespace(var)
                .iter()
                .map(|&(ns, value)| {
                    let suffix = if self.env.selected_namespace() == *ns {
                        Some("(current)")
                    } else {
                        None
                    };

                    let doc = format!("{ns}: {value:?} {}", suffix.unwrap_or_default());

                    doc
                })
                .collect::<Vec<_>>();

            if values.is_empty() {
                warn!("didn't get a value for the variable {var}")
            } else {
                let current_value = self
                    .env
                    .get_variable_value(var)
                    .map(|value| format!("```json\n{value:?}\n```"))
                    .unwrap_or_default();

                let values = values.join("\n");
                let docs = [
                    &current_value,
                    "Resolved from env file:",
                    "```sh",
                    &self.env.env_file_name.to_string_lossy(),
                    "```",
                    "```js",
                    &values,
                    "```",
                ];
                self.docs = Some(docs.join("\n"));
            }
        }
    }

    fn visit_endpoint(&mut self, endpoint: &ast::Endpoint<'source>) {
        if endpoint.span().contains(&self.position) {
            let item_at_position = self
                .program
                .as_ref()
                .and_then(|p| p.items.iter().find(|i| i.span.contains(&self.position)));

            match item_at_position {
                Some(item) => {
                    self.docs = Some(item.request.url.clone());
                    return;
                }
                None => {
                    warn!("didn't find a evaluated request item for endpoint on cursor")
                }
            };
        }
        endpoint.visit_children_with(self);
    }
}
