use super::{locations::Location, TokenKind};

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TokenKind::*;
        let str = match self {
            Get => "get",
            Post => "post",
            Put => "put",
            Patch => "patch",
            Delete => "delete",
            Header => "header",
            Body => "body",
            Set => "set",
            Let => "let",
            Ident => "identifier",
            Boolean => "boolean",
            Number => "number",
            StringLiteral => "string",
            MultiLineStringLiteral => "string",
            Pathname => "pathname",
            Url => "url",
            Linecomment => "comment",
            Shebang => "#!...",
            Assign => "=",
            DollarSignLBracket => "${",
            LParen => "(",
            RParen => ")",
            LBracket => "{",
            RBracket => "}",
            LSquare => "[",
            RSquare => "]",
            Colon => ":",
            AttributePrefix => "@",
            Comma => ",",
            End => "Eof",
            UnfinishedStringLiteral => "\"...",
            UnfinishedMultiLineStringLiteral => "`...",
            IllegalToken => "illegal",
            Null => "null",
        };

        f.write_str(str)
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}:{}]", self.line + 1, self.col + 1)
    }
}
