use crate::lexer::locations::Span;
use tower_lsp::lsp_types::Position;

pub trait ContainsPosition {
    fn contains(&self, position: &Position) -> bool;
}

impl ContainsPosition for Span {
    fn contains(&self, position: &Position) -> bool {
        if self.start.line == self.end.line {
            return (self.start.col..=self.end.col).contains(&(position.character as usize));
        }
        (self.start.line..=self.end.line).contains(&(position.line as usize))
    }
}
