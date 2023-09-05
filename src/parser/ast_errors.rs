use super::{
    ast::{Expression, Item, Statement},
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
            Item::Set { value, .. } | Item::Let { value, .. } => errors.extend(value.errors()),
            Item::LineComment(_) => {}
            Item::Request {
                block: Some(block), ..
            } => errors.extend(block.statements.iter().flat_map(|s| s.errors())),
            Item::Expr(expr) => errors.extend(expr.errors()),
            Item::Attribute { parameters, .. } => {
                errors.extend(parameters.into_iter().flat_map(|expr| expr.errors()))
            }
            Item::Error(e) => errors.push(e.to_owned()),
            _ => {}
        }

        errors
    }
}

impl<'source> GetErrors<'source> for Statement<'source> {
    fn errors(&self) -> Vec<ContextualError<ParseError<'source>>> {
        let mut errors = vec![];

        match &self {
            Statement::Header { value, .. } => errors.extend(value.errors()),
            Statement::Body { value, .. } => errors.extend(value.errors()),
            Statement::LineComment(_) => {}
            Statement::Error(e) => errors.push(e.to_owned()),
        }

        errors
    }
}

impl<'source> GetErrors<'source> for Expression<'source> {
    fn errors(&self) -> Vec<ContextualError<ParseError<'source>>> {
        let mut errors = vec![];
        match &self {
            Expression::Call { arguments, .. } => {
                errors.extend(arguments.into_iter().map(|e| e.errors()).flatten())
            }
            Expression::Array((_, arr)) => {
                errors.extend(arr.into_iter().map(|e| e.errors()).flatten())
            }
            Expression::Object((_, o)) => errors.extend(o.values().map(|e| e.errors()).flatten()),
            Expression::TemplateSringLiteral { parts, .. } => parts
                .into_iter()
                .for_each(|expr| errors.extend(expr.errors())),
            Expression::Error(e) => errors.push(e.to_owned()),
            _ => {}
        }

        errors
    }
}
