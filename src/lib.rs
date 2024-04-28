pub mod config;
pub mod error;
pub mod error_meta;
pub mod interpreter;
pub mod language_server;
pub mod lexer;
pub mod parser;

mod utils {
    use std::sync::Arc;

    // Rc -> Because this is very cheap to clone
    // Arc -> Because we implement a language_server with an async runtime
    pub type String = Arc<str>;
}

pub mod editing {

    pub fn edit<P: AsRef<std::path::Path>>(file_name: P) -> anyhow::Result<()> {
        let default_editor = std::env::var("EDITOR")?;

        std::process::Command::new(default_editor)
            .arg(file_name.as_ref())
            .spawn()?
            .wait()?;

        Ok(())
    }
}

pub mod fmt {
    use crate::parser::{
        ast::{self, ConstantDeclaration, Expression, Item, ObjectEntry, VariableDeclaration},
        ast_visit::{VisitWith, Visitor},
    };

    pub struct FormattedPrinter<'source> {
        pub error:
            Option<crate::error_meta::ContextualError<crate::parser::error::ParseError<'source>>>,
        tab_size: u8,
        indent: usize,
        output: String,
        is_first_item: bool,
        let_statement_streak: u16,
        line_comment_streak: u16,
        is_after_attribute: bool,
    }

    impl<'source> FormattedPrinter<'source> {
        pub fn new() -> Self {
            Self {
                error: None,
                tab_size: 4,
                indent: 0,
                output: String::new(),
                is_first_item: true,
                let_statement_streak: 0,
                line_comment_streak: 0,
                is_after_attribute: false,
            }
        }

        fn push(&mut self, s: char) {
            self.output.push(s)
        }

        fn push_str(&mut self, s: &str) {
            self.output.push_str(s)
        }

        fn new_line(&mut self) {
            self.output.push('\n');
        }

        fn push_indent(&mut self) {
            self.indent += 1;
            self.put_indentation();
        }

        fn put_indentation(&mut self) {
            self.output
                .push_str(&" ".repeat(self.tab_size as usize * self.indent));
        }

        fn pop_indent(&mut self) {
            self.indent -= 1;
        }

        pub fn into_output(self) -> String {
            self.output
        }

        /// Prints one or two new lines when applicable.
        fn hanle_new_line_before_item(&mut self, item: &Item) {
            match item {
                Item::LineComment(_) => {
                    self.let_statement_streak = 0;

                    self.line_comment_streak += 1;

                    if self.line_comment_streak == 1 {
                        self.new_line();
                    }

                    self.new_line();
                }
                Item::Let(_) => {
                    self.line_comment_streak = 0;

                    self.let_statement_streak += 1;

                    if self.let_statement_streak == 1 {
                        self.new_line();
                    }

                    self.new_line();
                }
                _ => {
                    self.line_comment_streak = 0;
                    self.let_statement_streak = 0;

                    if !self.is_first_item {
                        if !self.is_after_attribute {
                            self.new_line();
                        }

                        self.new_line();
                    } else {
                        self.is_first_item = false;
                    }
                }
            }
        }
    }

    impl<'source> Visitor<'source> for FormattedPrinter<'source> {
        fn visit_item(&mut self, item: &crate::parser::ast::Item<'source>) {
            self.hanle_new_line_before_item(item);

            item.visit_children_with(self);

            if let Item::Attribute(_) = item {
                self.is_after_attribute = true;
            } else {
                self.is_after_attribute = false;
            }
        }

        fn visit_line_comment(&mut self, comment: &ast::Literal<'source>) {
            self.push_str(comment.value);
        }

        fn visit_request(&mut self, request: &crate::parser::ast::Request<'source>) {
            self.push_str(&request.method.to_string().to_lowercase());
            self.push(' ');

            match &request.endpoint {
                ast::Endpoint::Expr(expr) => self.visit_expr(expr),
                ast::Endpoint::Url(url) => self.push_str(url.value),
                ast::Endpoint::Pathname(path) => self.push_str(path.value),
            }

            self.push(' ');

            if let Some(block) = &request.block {
                self.push('{');
                self.new_line();

                let len = block.statements.len();
                let mut i = 0;
                for statement in block.statements.iter() {
                    self.push_indent();

                    self.visit_statement(statement);
                    i += 1;

                    if i < len {
                        self.new_line();
                    }

                    self.pop_indent();
                }

                self.new_line();
                self.push('}');
            }
        }

        fn visit_constant_declaration(
            &mut self,
            ConstantDeclaration { identifier, value }: &ConstantDeclaration<'source>,
        ) {
            self.push_str("set ");

            self.visit_parsed_node(identifier);

            self.push_str(" = ");
            self.visit_expr(value);
        }

        fn visit_variable_declaration(
            &mut self,
            VariableDeclaration { identifier, value }: &VariableDeclaration<'source>,
        ) {
            self.push_str("let ");

            self.visit_parsed_node(identifier);

            self.push_str(" = ");
            self.visit_expr(value);
        }

        fn visit_statement(&mut self, statement: &crate::parser::ast::Statement<'source>) {
            match statement {
                ast::Statement::Header { value, .. } => {
                    self.push_str("header ");
                    self.visit_expr(value);
                }
                ast::Statement::Body { value, .. } => {
                    self.push_str("body ");
                    self.visit_expr(value);
                }
                ast::Statement::LineComment(comment) => self.push_str(comment.value),
                ast::Statement::Error(error) => self.visit_error(error),
            }
        }

        fn visit_attribute(&mut self, attribute: &ast::Attribute<'source>) {
            self.push('@');

            self.visit_parsed_node(&attribute.identifier);

            if let Some(args) = &attribute.arguments {
                self.push('(');
                self.visit_expr_list(args);
                self.push(')');
            }
        }

        fn visit_token(&mut self, token: &crate::lexer::Token<'source>) {
            self.push_str(token.text);
        }

        fn visit_expr(&mut self, expr: &crate::parser::ast::Expression<'source>) {
            match expr {
                Expression::String(s) => self.push_str(s.raw),
                Expression::Number((_, n)) => self.push_str(&n.to_string()),
                Expression::Bool((_, b)) => self.push_str(&b.to_string()),
                Expression::Null(_) => self.push_str("null"),
                Expression::Identifier(node) => {
                    self.visit_parsed_node(node);
                }
                Expression::Array(list) => {
                    self.push_str("[");

                    self.visit_expr_list(list);

                    self.push_str("]")
                }
                Expression::Object((_, entries)) => {
                    self.push_str("{");

                    self.new_line();

                    for (i, node) in entries.iter().enumerate() {
                        self.push_indent();

                        self.visit_parsed_node(node);

                        if i != entries.len() - 1 {
                            self.push_str(",");
                        }

                        self.new_line();

                        self.pop_indent();
                    }

                    self.put_indentation();
                    self.push_str("}")
                }
                Expression::Error(e) => self.visit_error(e),
                Expression::Call(call) => {
                    self.visit_parsed_node(&call.identifier);

                    self.push_str("(");

                    self.visit_expr_list(&call.arguments);

                    self.push_str(")")
                }
                Expression::EmptyArray(_) => self.push_str("[]"),
                Expression::EmptyObject(_) => self.push_str("{}"),
                Expression::TemplateSringLiteral { parts, .. } => {
                    for expr in parts.iter() {
                        match expr {
                            Expression::String(s) => self.push_str(s.raw),
                            _ => {
                                self.push_str("${");
                                self.visit_expr(expr);
                                self.push_str("}");
                            }
                        }
                    }
                }
            }
        }

        fn visit_object_entry(&mut self, entry: &ObjectEntry<'source>) {
            let ObjectEntry { key, value } = entry;

            self.visit_parsed_node(key);

            self.push_str(": ");

            self.visit_expr(value)
        }

        fn visit_string(&mut self, stringlit: &ast::StringLiteral<'source>) {
            self.push_str(stringlit.raw);
        }

        fn visit_expr_list(&mut self, expr_list: &crate::parser::ast::ExpressionList<'source>) {
            for (i, expr) in expr_list.exprs.iter().enumerate() {
                self.visit_expr(expr);
                if i != expr_list.exprs.len() - 1 {
                    self.push_str(", ");
                }
            }
        }

        fn visit_error(
            &mut self,
            err: &crate::error_meta::ContextualError<crate::parser::error::ParseError<'source>>,
        ) {
            self.error = Some(err.clone());
            err.visit_children_with(self);
        }
    }
}
