pub mod error;
pub mod error_meta;
pub mod interpreter;
pub mod language_server;
pub mod lexer;
pub mod parser;

mod utils {
    use std::sync::Arc;
    pub type Array<T> = Arc<[T]>;
}
