use crate::{
    error_meta::ContextualError,
    lexer::{self, locations::GetSpan},
};

use super::{
    ast::{
        result::ParsedNode, Attribute, CallExpr, ConstantDeclaration, Endpoint, Expression,
        ExpressionList, Item, Literal, ObjectEntry, Program, Request, Statement, StringLiteral,
        TemplateSringPart, VariableDeclaration,
    },
    error::ParseError,
};

pub trait Visitor<'source>
where
    Self: std::marker::Sized,
{
    fn visit_program(&mut self, program: &Program<'source>) {
        program.visit_children_with(self);
    }

    fn visit_item(&mut self, item: &Item<'source>) {
        item.visit_children_with(self);
    }

    fn visit_variable_declaration(&mut self, declaration: &VariableDeclaration<'source>) {
        declaration.visit_children_with(self);
    }

    fn visit_constant_declaration(&mut self, declaration: &ConstantDeclaration<'source>) {
        declaration.visit_children_with(self);
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

    fn visit_template_string_part(&mut self, part: &TemplateSringPart<'source>) {
        part.visit_children_with(self);
    }

    fn visit_expr(&mut self, expr: &Expression<'source>) {
        expr.visit_children_with(self);
    }

    fn visit_object_entry(&mut self, entry: &ObjectEntry<'source>) {
        entry.visit_children_with(self);
    }

    fn visit_attribute(&mut self, attribute: &Attribute<'source>) {
        attribute.visit_children_with(self);
    }

    fn visit_line_comment(&mut self, comment: &Literal<'source>) {
        comment.visit_children_with(self);
    }

    fn visit_expr_list(&mut self, expr_list: &ExpressionList<'source>) {
        expr_list.visit_children_with(self);
    }

    fn visit_literal(&mut self, stringlit: &Literal<'source>) {
        stringlit.visit_children_with(self);
    }

    fn visit_string(&mut self, stringlit: &StringLiteral<'source>) {
        stringlit.visit_children_with(self);
    }

    fn visit_call_expr(&mut self, expr: &CallExpr<'source>) {
        expr.visit_children_with(self);
    }

    fn visit_token(&mut self, token: &lexer::Token<'source>) {
        token.visit_children_with(self);
    }

    fn visit_parsed_node<T: GetSpan + VisitWith<'source>>(
        &mut self,
        node: &ParsedNode<'source, T>,
    ) {
        node.visit_children_with(self);
    }

    fn visit_error(&mut self, err: &ContextualError<ParseError<'source>>) {
        err.visit_children_with(self);
    }
}

pub trait VisitWith<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V);
    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V);
}

impl<'source> VisitWith<'source> for Program<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_program(self);
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        for item in self.items.iter() {
            visitor.visit_item(item);
        }
    }
}

impl<'source> VisitWith<'source> for Item<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_item(self);
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        match self {
            Item::Set(set_d) => {
                visitor.visit_constant_declaration(set_d);
            }
            Item::Let(let_d) => {
                visitor.visit_variable_declaration(let_d);
            }
            Item::Request(req) => {
                visitor.visit_request(req);
            }
            Item::Expr(expr) => visitor.visit_expr(expr),
            Item::Attribute(att) => visitor.visit_attribute(att),
            Item::Error(e) => visitor.visit_error(e),
            Item::LineComment(comment) => visitor.visit_line_comment(comment),
        }
    }
}

impl<'source> VisitWith<'source> for ConstantDeclaration<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_constant_declaration(self);
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        let ConstantDeclaration { identifier, value } = self;

        visitor.visit_parsed_node(identifier);
        visitor.visit_expr(value);
    }
}

impl<'source> VisitWith<'source> for VariableDeclaration<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_variable_declaration(self);
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        let VariableDeclaration { identifier, value } = self;

        visitor.visit_parsed_node(identifier);
        visitor.visit_expr(value);
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

impl<'source> VisitWith<'source> for ExpressionList<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_expr_list(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        for expr in self.expressions() {
            visitor.visit_expr(expr);
        }
    }
}

impl<'source> VisitWith<'source> for TemplateSringPart<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_template_string_part(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        match self {
            TemplateSringPart::ExpressionPart(expr) => visitor.visit_expr(expr),
            TemplateSringPart::StringPart(s) => visitor.visit_string(s),
        };
    }
}

impl<'source> VisitWith<'source> for Expression<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_expr(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        match self {
            Expression::Call(expr) => visitor.visit_call_expr(expr),
            Expression::Array(list) => {
                for expr in list.expressions() {
                    visitor.visit_expr(expr)
                }
            }
            Expression::Object(entry_list) => {
                for entry in entry_list.entries() {
                    visitor.visit_object_entry(entry)
                }
            }
            Expression::TemplateSringLiteral { parts, .. } => {
                for expr in parts.iter() {
                    visitor.visit_template_string_part(expr)
                }
            }
            Expression::Error(e) => visitor.visit_error(e),
            Expression::Identifier(ident) => visitor.visit_parsed_node(ident),
            Expression::String(s) => visitor.visit_string(s),
            _ => {}
        };
    }
}

impl<'source> VisitWith<'source> for ObjectEntry<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_object_entry(self);
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_parsed_node(&self.key);
        visitor.visit_expr(&self.value)
    }
}

impl<'source> VisitWith<'source> for Attribute<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_attribute(self);
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_parsed_node(&self.identifier);

        if let Attribute {
            arguments: Some(arguments),
            ..
        } = self
        {
            for arg in arguments.expressions() {
                visitor.visit_expr(arg);
            }
        }
    }
}

impl<'source> VisitWith<'source> for Literal<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_literal(self);
    }

    fn visit_children_with<V: Visitor<'source>>(&self, _visitor: &mut V) {}
}

impl<'source> VisitWith<'source> for StringLiteral<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_string(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, _visitor: &mut V) {}
}

impl<'source> VisitWith<'source> for lexer::Token<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_token(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, _visitor: &mut V) {}
}

impl<'source> VisitWith<'source> for CallExpr<'source> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_call_expr(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_parsed_node(&self.identifier);
        for arg in self.arguments.expressions() {
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

impl<'source, T: GetSpan + VisitWith<'source>> VisitWith<'source> for ParsedNode<'source, T> {
    fn visit_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        visitor.visit_parsed_node(self)
    }

    fn visit_children_with<V: Visitor<'source>>(&self, visitor: &mut V) {
        match self {
            ParsedNode::Ok(node) => node.visit_with(visitor),
            ParsedNode::Error(e) => {
                visitor.visit_error(e);
            }
        }
    }
}
