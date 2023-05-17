pub mod locations;

use locations::Location;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenKind {
    // keywords
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Header,
    Body,
    Set,
    Let,

    Ident,

    // literals
    StringLiteral,
    MultiLineStringLiteral,
    Url,
    Pathname,

    Linecomment,
    Shebang,

    // operators
    Assign,

    // special characters
    DollarSignLBracket,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LSquare,
    RSquare,
    Colon,
    AttributePrefix,
    Comma,
    End,

    //edge cases
    UnfinishedStringLiteral,
    UnfinishedMultiLineStringLiteral,
    IllegalToken,
}

#[derive(PartialEq)]
pub struct Token<'t> {
    pub kind: TokenKind,
    pub text: &'t str,
    pub start: Location,
}

impl<'source> Token<'source> {
    pub fn end_location(&self) -> Location {
        Location {
            line: self.start.line,
            col: self.start.col + self.text.len(),
        }
    }
}

impl<'i> std::fmt::Debug for Token<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}({:?}) at {:?}", self.kind, self.text, self.start)
    }
}

trait CharaterTest {
    fn passes<P: Fn(&u8) -> bool>(&self, predicate: P) -> bool;
    fn is(&self, ch: u8) -> bool {
        self.passes(|&c| ch == c)
    }
}

impl CharaterTest for Option<&u8> {
    fn passes<P: Fn(&u8) -> bool>(&self, predicate: P) -> bool {
        match self {
            Some(c) => predicate(c),
            None => false,
        }
    }
}

#[derive(Debug)]
pub struct Lexer<'i> {
    input: &'i [u8],
    position: usize,
    cursor: Location,
    inside_multiline_string: bool,
}

impl<'i> Lexer<'i> {
    pub fn new(input: &'i str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
            cursor: Location { line: 0, col: 0 },
            inside_multiline_string: false,
        }
    }

    pub fn input(&self) -> &'i str {
        std::str::from_utf8(&self.input).expect("input should only contain utf-8 characters")
    }

    fn input_slice(&self, range: impl std::slice::SliceIndex<[u8], Output = [u8]>) -> &'i str {
        std::str::from_utf8(&self.input[range]).expect("input should only contain utf-8 characters")
    }

    fn char_at(&self, position: usize) -> Option<&u8> {
        if position < self.input.len() {
            return Some(&self.input[position]);
        }
        return None;
    }

    fn ch(&self) -> Option<&u8> {
        self.char_at(self.position)
    }

    fn step(&mut self) {
        if self.peek_char().is_none() {
            return self.position += 1;
        }
        self.check_and_bump_new_line();
        self.position += 1;
    }

    fn check_and_bump_new_line(&mut self) {
        if let Some(b'\n') = self.ch() {
            self.cursor.line += 1;
            self.cursor.col = 0;
        } else {
            self.cursor.col += 1;
        };
    }

    fn peek_n_char(&self, n: usize) -> Option<&u8> {
        self.char_at(self.position + n)
    }

    fn peek_char(&self) -> Option<&u8> {
        self.peek_n_char(1)
    }

    /// Assumes that the character at the current position, immediately before calling
    /// this function is also true for the predicate function given.
    fn read_while<P: Fn(&u8) -> bool>(&mut self, predicate: P) -> (usize, usize) {
        let start_pos = self.position;

        while self.peek_char().passes(&predicate) {
            self.step();
        }

        return (start_pos, self.position + 1);
    }

    fn skip_whitespace(&mut self) {
        while self.ch().passes(|c| c.is_ascii_whitespace()) {
            self.step();
        }
    }

    pub fn next(&mut self) -> Token<'i> {
        use TokenKind::*;

        self.skip_whitespace();

        let ch = match self.ch() {
            Some(ch) => ch,
            None => {
                return Token {
                    kind: TokenKind::End,
                    start: self.cursor,
                    text: "",
                }
            }
        };

        let t = match ch {
            b'"' if self.peek_char().is(b'"') => self.empty_string_literal(),
            b'"' => self.string_literal(),
            b'`' if self.peek_char().is(b'`') => self.empty_string_literal(),
            b'`' => self.multiline_string_literal(),
            b'$' if self.peek_char().is(b'{') => {
                let token = Token {
                    kind: DollarSignLBracket,
                    text: "${",
                    start: self.cursor,
                };
                self.step();
                token
            }
            b'}' if self.peek_char().is(b'`') && self.inside_multiline_string => {
                self.step(); // eat the curly
                self.step(); // eat the backtick
                self.inside_multiline_string = false;
                self.next()
            }
            b'}' if self.inside_multiline_string => {
                self.step(); // eat the curly
                self.multiline_string_literal()
            }
            b'(' => Token {
                kind: LParen,
                start: self.cursor,
                text: "(",
            },
            b')' => Token {
                kind: RParen,
                start: self.cursor,
                text: ")",
            },
            b'{' => Token {
                kind: LBracket,
                start: self.cursor,
                text: "{",
            },
            b'}' => Token {
                kind: RBracket,
                start: self.cursor,
                text: "}",
            },
            b'[' => Token {
                kind: LSquare,
                start: self.cursor,
                text: "[",
            },
            b']' => Token {
                kind: RSquare,
                start: self.cursor,
                text: "]",
            },
            b',' => Token {
                kind: Comma,
                text: ",",
                start: self.cursor,
            },
            b':' => Token {
                kind: Colon,
                text: ":",
                start: self.cursor,
            },
            b'=' => Token {
                kind: Assign,
                text: "=",
                start: self.cursor,
            },
            b'@' => Token {
                kind: AttributePrefix,
                text: "@",
                start: self.cursor,
            },
            b'/' if self.peek_char().is(b'/') => self.line_comment(),
            b'/' => self.pathname(),
            b'#' if self.peek_char().is(b'!') => self.shebang(),
            c if c.is_ascii_alphabetic() => self.keyword_or_identifier(),
            _ => Token {
                kind: IllegalToken,
                text: std::str::from_utf8(&self.input[self.position..self.position + 1]).unwrap(),
                start: self.cursor,
            },
        };

        self.step();

        t
    }

    fn multiline_string_literal(&mut self) -> Token<'i> {
        self.inside_multiline_string = true;
        let location = self.cursor;
        let start_pos = self.position;

        self.step();

        let (s, e) = loop {
            let is_at_end = self.ch().is(b'`');
            let dollar_curly_ahead = self.peek_char().is(b'$') && self.peek_n_char(2).is(b'{');

            if dollar_curly_ahead {
                break (start_pos, self.position + 1);
            }

            if is_at_end {
                self.inside_multiline_string = false;
                break (start_pos, self.position + 1);
            }

            if let None = self.ch() {
                return Token {
                    kind: TokenKind::UnfinishedMultiLineStringLiteral,
                    start: location,
                    text: self.input_slice(start_pos..self.position),
                };
            }

            self.step();
        };

        let string = self.input_slice(s..e);

        Token {
            kind: TokenKind::MultiLineStringLiteral,
            start: location,
            text: string,
        }
    }

    fn string_literal(&mut self) -> Token<'i> {
        let location = self.cursor;

        let (s, e) = self.read_while(|&c| c != b'"' && c != b'\n');

        match self.peek_char() {
            Some(b'\n') | None => {
                return Token {
                    kind: TokenKind::UnfinishedStringLiteral,
                    start: location,
                    text: self.input_slice(..e),
                };
            }
            _ => {}
        }

        let string = self.input_slice(s..=e);

        self.step();

        Token {
            kind: TokenKind::StringLiteral,
            start: location,
            text: string,
        }
    }

    fn empty_string_literal(&mut self) -> Token<'i> {
        let location = self.cursor;
        self.step();
        Token {
            kind: TokenKind::StringLiteral,
            start: location,
            text: "",
        }
    }

    fn keyword_or_identifier(&mut self) -> Token<'i> {
        let location = self.cursor;
        let (s, e) = self.read_while(|&c| c.is_ascii_alphabetic() || c == b'_');
        let string = self.input_slice(s..e);

        use TokenKind::*;

        match string {
            "let" => Token {
                kind: Let,
                start: location,
                text: string,
            },
            "get" => Token {
                kind: Get,
                start: location,
                text: string,
            },
            "post" => Token {
                kind: Post,
                start: location,
                text: string,
            },
            "put" => Token {
                kind: Put,
                start: location,
                text: string,
            },
            "patch" => Token {
                kind: Patch,
                start: location,
                text: string,
            },
            "delete" => Token {
                kind: Delete,
                start: location,
                text: string,
            },
            "header" => Token {
                kind: Header,
                start: location,
                text: string,
            },
            "set" => Token {
                kind: Set,
                start: location,
                text: string,
            },
            "body" => Token {
                kind: Body,
                start: location,
                text: string,
            },
            "http" | "https" => {
                let (.., e) = self.read_while(|&c| !c.is_ascii_whitespace());
                let s = self.input_slice(s..e);
                Token {
                    kind: Url,
                    start: location,
                    text: s,
                }
            }
            s => Token {
                kind: Ident,
                start: location,
                text: s,
            },
        }
    }

    fn pathname(&mut self) -> Token<'i> {
        let location = self.cursor;
        let (s, e) = self.read_while(|&c| !c.is_ascii_whitespace());
        let string = self.input_slice(s..e);

        Token {
            kind: TokenKind::Pathname,
            start: location,
            text: string,
        }
    }

    fn shebang(&mut self) -> Token<'i> {
        let location = self.cursor;
        let (s, e) = self.read_while(|&c| c != b'\n');
        let string = self.input_slice(s..e);
        Token {
            text: string,
            start: location,
            kind: TokenKind::Shebang,
        }
    }

    fn line_comment(&mut self) -> Token<'i> {
        let location = self.cursor;
        let (s, e) = self.read_while(|&c| c != b'\n');
        let string = self.input_slice(s..e);
        Token {
            kind: TokenKind::Linecomment,
            text: string,
            start: location,
        }
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = Token<'source>;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next();

        if let TokenKind::End = token.kind {
            return None;
        }

        Some(token)
    }
}
