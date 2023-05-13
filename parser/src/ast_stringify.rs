use crate::ast::{Endpoint, Expression, Item, Statement};

impl<'source> ToString for Expression<'source> {
    fn to_string(&self) -> String {
        match self {
            Expression::Identifier(i) => i.name.to_string(),
            Expression::String(l) => l.raw.to_string(),
            Expression::Call {
                identifier,
                arguments,
            } => format!(
                "{}({})",
                identifier.name,
                arguments
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            Expression::TemplateSringLiteral { parts, .. } => parts
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<String>>()
                .join(" "),
        }
    }
}

impl<'source> ToString for Statement<'source> {
    fn to_string(&self) -> String {
        match self {
            Statement::Header { name, value } => {
                format!("header {} {}", name.raw, value.to_string())
            }
            Statement::Body { value, .. } => format!("body {}", value.to_string()),
            Statement::LineComment(l) => l.value.to_string(),
        }
    }
}

impl<'source> ToString for Endpoint<'source> {
    fn to_string(&self) -> String {
        match self {
            Endpoint::Url(l) => l.value.to_string(),
            Endpoint::Pathname(l) => l.value.to_string(),
        }
    }
}

impl<'source> ToString for Item<'source> {
    fn to_string(&self) -> String {
        match self {
            Item::Set { identifier, value } => {
                format!("set {} {}", identifier.name, value.to_string())
            }
            Item::Let { identifier, value } => {
                format!("let {} {}", identifier.name, value.to_string())
            }
            Item::LineComment(l) => l.value.to_string(),
            Item::Request {
                method,
                endpoint,
                block,
                ..
            } => format!(
                "{} {} {{ {} }}",
                method,
                endpoint.to_string(),
                block
                    .as_ref()
                    .map(|b| b
                        .statements
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<String>>()
                        .join(" "))
                    .unwrap_or(String::from(""))
            ),
            Item::Attribute {
                identifier,
                parameters,
                ..
            } => format!(
                "@{}({})",
                identifier.name,
                parameters
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
        }
    }
}
