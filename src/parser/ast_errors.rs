use super::{
    ast::{self, Expression, Item, Statement},
    error::ParseError,
};
use crate::error_meta::ContextualError;

pub trait GetErrors<'source> {
    fn errors(&self) -> Vec<ContextualError<ParseError<'source>>>;
}

impl<'source> GetErrors<'source> for Item<'source> {
    fn errors(&self) -> Vec<ContextualError<ParseError<'source>>> {
        let mut errors = vec![];

        match &self {
            Item::Set { identifier, value } => {
                if let ast::result::ParsedNode::Error(error) = identifier {
                    errors.push(*error.clone());
                }
                errors.extend(value.errors());
            }
            Item::Let { value, identifier } => {
                if let ast::result::ParsedNode::Error(error) = identifier {
                    errors.push(*error.clone());
                }
                errors.extend(value.errors())
            }
            Item::LineComment(_) => {}
            Item::Request {
                block: Some(block), ..
            } => errors.extend(block.statements.iter().flat_map(|s| s.errors())),
            Item::Expr(expr) => errors.extend(expr.errors()),
            Item::Attribute { parameters, .. } => {
                if let Some(args) = parameters.as_ref() {
                    errors.extend(args.parameters.iter().flat_map(|expr| expr.errors()))
                }
            }
            Item::Error(e) => errors.push(*e.clone()),
            _ => {}
        }

        errors
    }
}

impl<'source> GetErrors<'source> for Statement<'source> {
    fn errors(&self) -> Vec<ContextualError<ParseError<'source>>> {
        let mut errors = vec![];

        match &self {
            Statement::Header { value, name } => {
                if let ast::result::ParsedNode::Error(error) = name {
                    errors.push(*error.clone());
                }
                errors.extend(value.errors())
            }
            Statement::Body { value, .. } => errors.extend(value.errors()),
            Statement::LineComment(_) => {}
            Statement::Error(e) => errors.push(*e.clone()),
        }

        errors
    }
}

impl<'source> GetErrors<'source> for Expression<'source> {
    fn errors(&self) -> Vec<ContextualError<ParseError<'source>>> {
        let mut errors = vec![];
        match &self {
            Expression::Call { arguments, .. } => {
                errors.extend(arguments.iter().flat_map(|e| e.errors()))
            }
            Expression::Array((_, arr)) => errors.extend(arr.iter().flat_map(|e| e.errors())),
            Expression::Object((_, o)) => errors.extend(o.values().flat_map(|e| e.errors())),
            Expression::TemplateSringLiteral { parts, .. } => {
                parts.iter().for_each(|expr| errors.extend(expr.errors()))
            }
            Expression::Error(e) => errors.push(*e.clone()),
            _ => {}
        }

        errors
    }
}
