use crate::{error_meta::ContextualError, lexer::locations::GetSpan};

use super::{
    ast::{
        result::ParsedNode, CallExpr, Endpoint, Expression, ExpressionList, Item, Request,
        Statement, StringLiteral,
    },
    error::ParseError,
};

pub trait Visitor<'source>
where
    Self: std::marker::Sized,
{
    fn visit_item(&mut self, item: &Item<'source>) {
        item.visit_children_with(self);
    }

    fn visit_request(&mut self, request: &Request<'source>) {
        request.visit_children_with(self);
    }

    fn visit_statement(&mut self, statement: &Statement<'source>) {
        statement.visit_children_with(self);
    }

    fn visit_endpoint(&mut self, endpoint: &Endpoint<'source>) {
        endpoint.visit_children_with(self);
    }

    fn visit_expr(&mut self, expr: &Expression<'source>) {
        expr.visit_children_with(self);
    }

    fn visit_string(&mut self, stringlit: &StringLiteral<'source>) {
        stringlit.visit_children_with(self);
    }

    fn visit_call_expr(&mut self, expr: &CallExpr<'source>) {
        expr.visit_children_with(self);
    }

    fn visit_parsed_node<T: GetSpan>(&mut self, token: &ParsedNode<'source, T>) {
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
                visitor.visit_parsed_node(identifier);
                visitor.visit_expr(value);
            }
            Item::Let { identifier, value } => {
                visitor.visit_parsed_node(identifier);
                visitor.visit_expr(value)
            }
            Item::Request(req) => {
                visitor.visit_request(req);
            }
            Item::Expr(expr) => visitor.visit_expr(expr),
            Item::Attribute {
                arguments: Some(arguments),
                identifier,
                ..
            } => {
                visitor.visit_parsed_node(identifier);
                for arg in arguments.exprs.iter() {
                    visitor.visit_expr(arg);
                }
            }
            Item::Error(e) => visitor.visit_error(e),
            _ => {}
        }
    }
}

impl<'source> VisitWith<'source> for Request<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_request(self);
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        match self {
            Request {
                endpoint,
                block: Some(block),
                ..
            } => {
                visitor.visit_endpoint(endpoint);
                for statement in block.statements.iter() {
                    visitor.visit_statement(statement)
                }
            }
            Request { endpoint, .. } => visitor.visit_endpoint(endpoint),
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
                visitor.visit_parsed_node(name);
                visitor.visit_expr(value);
            }
            Statement::Body { value, .. } => visitor.visit_expr(value),
            Statement::Error(e) => visitor.visit_error(e),
            Statement::LineComment(_) => {}
        }
    }
}

impl<'source> VisitWith<'source> for Endpoint<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_endpoint(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        if let Endpoint::Expr(e) = self {
            visitor.visit_expr(e)
        }
    }
}

impl<'source> VisitWith<'source> for Expression<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_expr(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        match self {
            Expression::Call(expr) => visitor.visit_call_expr(expr),
            Expression::Array(ExpressionList { exprs, .. }) => {
                for expr in exprs.iter() {
                    visitor.visit_expr(expr)
                }
            }
            Expression::Object((_, entries)) => {
                for entry in entries.iter() {
                    match entry {
                        ParsedNode::Ok(entry) => {
                            visitor.visit_parsed_node(&entry.key);
                            visitor.visit_expr(&entry.value)
                        }
                        ParsedNode::Error(e) => visitor.visit_error(e),
                    }
                }
            }
            Expression::TemplateSringLiteral { parts, .. } => {
                for expr in parts.iter() {
                    visitor.visit_expr(expr)
                }
            }
            Expression::Error(e) => visitor.visit_error(e),
            Expression::Identifier(ident) => visitor.visit_parsed_node(ident),
            Expression::String(s) => visitor.visit_string(s),
            _ => {}
        };
    }
}

impl<'source> VisitWith<'source> for StringLiteral<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_string(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, _visitor: &mut V) {}
}

impl<'source> VisitWith<'source> for CallExpr<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_call_expr(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_parsed_node(&self.identifier);
        for arg in self.arguments.exprs.iter() {
            visitor.visit_expr(arg)
        }
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
        visitor.visit_parsed_node(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        if let ParsedNode::Error(e) = self {
            visitor.visit_error(e)
        }
    }
}
