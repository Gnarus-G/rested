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
        ast::{
            result::ParsedNode, ConstantDeclaration, Expression, Item, ObjectEntry,
            VariableDeclaration,
        },
        ast_visit::{VisitWith, Visitor},
    };

    pub struct FormattedPrinter {
        pub has_error: bool,
        tab_size: u8,
        indent: usize,
        output: String,
    }

    impl FormattedPrinter {
        pub fn new() -> Self {
            Self {
                has_error: false,
                tab_size: 4,
                indent: 0,
                output: String::new(),
            }
        }

        fn push_char(&mut self, s: char) {
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
    }

    impl<'source> Visitor<'source> for FormattedPrinter {
        fn visit_item(&mut self, item: &crate::parser::ast::Item<'source>) {
            match item {
                Item::Let(d) => self.visit_variable_declaration(d),
                Item::Error(e) => self.visit_error(e),
                Item::Set(s) => self.visit_constant_declaration(s),
                Item::LineComment(lit) => self.push_str(lit.value),
                Item::Request(_) => todo!(),
                Item::Expr(_) => todo!(),
                Item::Attribute {
                    location,
                    identifier,
                    arguments,
                } => todo!(),
            }

            self.push_str("\n");
        }

        fn visit_constant_declaration(
            &mut self,
            ConstantDeclaration { identifier, value }: &ConstantDeclaration<'source>,
        ) {
            self.push_str("set ");

            self.visit_parsed_node(identifier);

            if let ParsedNode::Ok(ident) = identifier {
                self.push_str(ident.text);
            }

            self.push_str(" = ");
            self.visit_expr(value);
            self.new_line();
        }

        fn visit_variable_declaration(
            &mut self,
            VariableDeclaration { identifier, value }: &VariableDeclaration<'source>,
        ) {
            self.push_str("let ");

            self.visit_parsed_node(identifier);

            if let ParsedNode::Ok(ident) = identifier {
                self.push_str(ident.text);
            }

            self.push_str(" = ");
            self.visit_expr(value);
            self.new_line();
        }

        fn visit_expr(&mut self, expr: &crate::parser::ast::Expression<'source>) {
            match expr {
                Expression::String(s) => self.push_str(s.raw),
                Expression::Number((_, n)) => self.push_str(&n.to_string()),
                Expression::Bool((_, b)) => self.push_str(&b.to_string()),
                Expression::Null(_) => self.push_str("null"),
                Expression::Identifier(node) => {
                    self.visit_parsed_node(node);

                    if let ParsedNode::Ok(ident) = node {
                        self.push_str(ident.text);
                    }
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

                        if let ParsedNode::Ok(ObjectEntry { key, value }) = node {
                            self.visit_parsed_node(key);

                            if let ParsedNode::Ok(ident) = key {
                                self.push_str(ident.raw);
                            }

                            self.push_str(": ");

                            self.visit_expr(value)
                        }

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

                    if let ParsedNode::Ok(ident) = &call.identifier {
                        self.push_str(ident.text)
                    }

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
            self.has_error = true;
            err.visit_children_with(self);
        }
    }
}
