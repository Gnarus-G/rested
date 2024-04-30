pub mod config;
pub mod error;
pub mod error_meta;
pub mod fmt;
pub mod interpreter;
pub mod language_server;
pub mod lexer;
pub mod parser;

mod utils {
    use std::sync::Arc;

    // Rc -> Because this is very cheap to clone
    // Arc -> Because we implement a language_server with an async runtime
    pub type String = Arc<str>;

    use serde::Serialize;

    #[derive(Debug, PartialEq, Serialize)]
    pub enum OneOf<L, R> {
        This(L),
        That(R),
    }

    impl<L, R> OneOf<L, R> {
        pub fn this(&self) -> Option<&L> {
            match self {
                OneOf::This(t) => Some(t),
                OneOf::That(_) => None,
            }
        }

        pub fn that(&self) -> Option<&R> {
            match self {
                OneOf::That(t) => Some(t),
                OneOf::This(_) => None,
            }
        }
    }
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
