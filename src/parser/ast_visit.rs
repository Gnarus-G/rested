use crate::{error_meta::ContextualError, lexer::locations::GetSpan};

use super::{
    ast::{result::ParsedNode, Expression, Item, Statement},
    error::ParseError,
};

pub trait Visitor<'source>
where
    Self: std::marker::Sized,
{
    fn visit_item(&mut self, item: &Item<'source>) {
        item.visit_children_with(self);
    }

    fn visit_statement(&mut self, statement: &Statement<'source>) {
        statement.visit_children_with(self);
    }

    fn visit_expr(&mut self, expr: &Expression<'source>) {
        expr.visit_children_with(self);
    }

    fn visit_token<T: GetSpan>(&mut self, token: &ParsedNode<'source, T>) {
        token.visit_children_with(self);
    }

    fn visit_error(&mut self, err: &ContextualError<ParseError<'source>>) {
        err.visit_children_with(self);
    }
}

pub trait VisitWith<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V);
    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V);
}

impl<'source> VisitWith<'source> for Item<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_item(self);
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        match self {
            Item::Set { identifier, value } => {
                visitor.visit_token(identifier);
                visitor.visit_expr(value);
            }
            Item::Let { identifier, value } => {
                visitor.visit_token(identifier);
                visitor.visit_expr(value)
            }
            Item::Request {
                block: Some(block), ..
            } => {
                for statement in &block.statements {
                    visitor.visit_statement(statement)
                }
            }
            Item::Expr(expr) => visitor.visit_expr(expr),
            Item::Attribute {
                parameters: Some(arguments),
                identifier,
                ..
            } => {
                visitor.visit_token(identifier);
                for arg in &arguments.parameters {
                    visitor.visit_expr(arg);
                }
            }
            Item::Error(e) => visitor.visit_error(e),
            _ => {}
        }
    }
}

impl<'source> VisitWith<'source> for Statement<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_statement(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        match self {
            Statement::Header { name, value } => {
                visitor.visit_token(name);
                visitor.visit_expr(value);
            }
            Statement::Body { value, .. } => visitor.visit_expr(value),
            Statement::Error(e) => visitor.visit_error(e),
            Statement::LineComment(_) => {}
        }
    }
}

impl<'source> VisitWith<'source> for Expression<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_expr(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        match self {
            Expression::Call {
                identifier,
                arguments,
            } => {
                if let ParsedNode::Error(e) = identifier {
                    visitor.visit_error(e)
                }
                for arg in &arguments.parameters {
                    visitor.visit_expr(arg)
                }
            }
            Expression::Array((_, elements)) => {
                for ele in elements {
                    for e in &ele.errors {
                        visitor.visit_error(e);
                    }
                    visitor.visit_expr(&ele.expr)
                }
            }
            Expression::Object((_, entries)) => {
                for entry in entries {
                    visitor.visit_token(&entry.key);
                    for e in &entry.errors {
                        visitor.visit_error(e);
                    }
                    visitor.visit_expr(&entry.value)
                }
            }
            Expression::TemplateSringLiteral { parts, .. } => {
                for expr in parts {
                    visitor.visit_expr(expr)
                }
            }
            Expression::Error(e) => visitor.visit_error(e),
            Expression::Identifier(ident) => visitor.visit_token(ident),
            _ => {}
        };
    }
}

impl<'source> VisitWith<'source> for ContextualError<ParseError<'source>> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_error(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, _visitor: &mut V) {}
}

impl<'source, T: GetSpan> VisitWith<'source> for ParsedNode<'source, T> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_token(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        if let ParsedNode::Error(e) = self {
            visitor.visit_error(e)
        }
    }
}
