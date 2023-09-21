use super::ast::{Expression, Item, Statement};

pub trait Visitor
where
    Self: std::marker::Sized,
{
    fn visit_item(&mut self, item: &Item) {
        item.visit_children_with(self);
    }
    fn visit_statement(&mut self, statement: &Statement) {
        statement.visit_children_with(self);
    }
    fn visit_expr(&mut self, expr: &Expression) {
        expr.visit_children_with(self);
    }
}

pub trait VisitWith {
    fn visit_with<V: Visitor>(&self, visitor: &mut V);
    fn visit_children_with<V: Visitor>(&self, visitor: &mut V);
}

impl<'source> VisitWith for Item<'source> {
    fn visit_with<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_item(self);
    }

    fn visit_children_with<V: Visitor>(&self, visitor: &mut V) {
        match self {
            Item::Set { identifier, value } => visitor.visit_expr(value),
            Item::Let { identifier, value } => visitor.visit_expr(value),
            Item::Request {
                method,
                endpoint,
                block: Some(block),
                span,
            } => {
                for statement in &block.statements {
                    visitor.visit_statement(statement)
                }
            }
            Item::Expr(expr) => visitor.visit_expr(expr),
            Item::Attribute {
                parameters: Some(arguments),
                ..
            } => {
                for arg in &arguments.parameters {
                    visitor.visit_expr(arg);
                }
            }

            _ => {}
        }
    }
}

impl<'source> VisitWith for Statement<'source> {
    fn visit_with<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_statement(self)
    }

    fn visit_children_with<V: Visitor>(&self, visitor: &mut V) {
        match self {
            Statement::Header { value, .. } => visitor.visit_expr(value),
            Statement::Body { value, start } => visitor.visit_expr(value),
            _ => {}
        }
    }
}

impl<'source> VisitWith for Expression<'source> {
    fn visit_with<V: Visitor>(&self, visitor: &mut V) {
        visitor.visit_expr(self)
    }

    fn visit_children_with<V: Visitor>(&self, visitor: &mut V) {
        match self {
            Expression::Call {
                identifier,
                arguments,
            } => {
                for arg in &arguments.parameters {
                    visitor.visit_expr(arg)
                }
            }
            Expression::Array((_, elements)) => {
                for ele in elements {
                    visitor.visit_expr(&ele.expr)
                }
            }
            Expression::Object((_, entries)) => {
                for entry in entries {
                    visitor.visit_expr(&entry.value)
                }
            }
            Expression::TemplateSringLiteral { span, parts } => {
                for expr in parts {
                    visitor.visit_expr(expr)
                }
            }
            _ => {}
        };
    }
}
