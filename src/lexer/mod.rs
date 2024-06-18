mod display;
pub mod locations;

use std::collections::VecDeque;

use locations::Location;

use self::locations::Position;

#[derive(Debug, PartialEq, Clone, Copy, serde::Serialize)]
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
    Null,

    Ident,

    // literals
    Boolean,
    Number,
    StringLiteral,
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
    OpeningBackTick,
    ClosingBackTick,
    Comma,
    End,

    //edge cases
    UnfinishedStringLiteral,
    UnfinishedMultiLineStringLiteral,
    IllegalToken,
}

#[derive(PartialEq, Clone, serde::Serialize)]
pub struct Token<'t> {
    pub kind: TokenKind,
    pub text: &'t str,
    pub start: Position,
}

impl<'source> Token<'source> {
    pub fn end_location(&self) -> Location {
        Location {
            line: self.start.line,
            col: self.start.col + self.text.len(),
        }
    }

    pub fn end_position(&self) -> Position {
        let distance_to_last_char_inclusive = self.text.len().saturating_sub(1);
        Position {
            line: self.start.line,
            // Columns are zero-indexed so we must subtract one for each
            value: self.start.value + distance_to_last_char_inclusive,
            col: self.start.col + distance_to_last_char_inclusive,
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
    position: Position,
    eof_pos: Position,
    template_str_depth: u8,
    template_str_token_buffer: VecDeque<Token<'i>>,
}

impl<'i> Lexer<'i> {
    pub fn new(input: &'i str) -> Self {
        Self {
            input: input.as_bytes(),
            position: Default::default(),
            eof_pos: Default::default(),
            template_str_depth: 0,
            template_str_token_buffer: VecDeque::new(),
        }
    }

    pub fn input(&self) -> &'i str {
        std::str::from_utf8(self.input).expect("input should only contain utf-8 characters")
    }

    fn input_slice(&self, range: impl std::slice::SliceIndex<[u8], Output = [u8]>) -> &'i str {
        std::str::from_utf8(&self.input[range]).expect("input should only contain utf-8 characters")
    }

    fn char_at(&self, position: usize) -> Option<&u8> {
        if position < self.input.len() {
            return Some(&self.input[position]);
        }
        None
    }

    fn ch(&self) -> Option<&u8> {
        self.char_at(self.position.value)
    }

    fn step(&mut self) {
        self.check_and_bump_new_line();
        self.position.value += 1;
    }

    fn check_and_bump_new_line(&mut self) {
        if let Some(b'\n') = self.ch() {
            self.position.line += 1;
            self.position.col = 0;
            if self.peek_char().is_none() {
                self.eof_pos = self.position;
            }
        } else {
            if self.peek_char().is_none() {
                self.eof_pos = self.position;
            }
            self.position.col += 1;
        };
    }

    fn peek_n_char(&self, n: usize) -> Option<&u8> {
        self.char_at(self.position.value + 1 + n)
    }

    fn peek_char(&self) -> Option<&u8> {
        self.peek_n_char(0)
    }

    /// Assumes that the character at the current position, immediately before calling
    /// this function is also true for the predicate function given.
    fn read_while<P: Fn(&u8) -> bool>(&mut self, predicate: P) -> (usize, usize) {
        let start_pos = self.position.value;

        while self.peek_char().passes(&predicate) {
            self.step();
        }

        (start_pos, self.position.value + 1)
    }

    fn skip_whitespace(&mut self) {
        while self.ch().passes(|c| c.is_ascii_whitespace()) {
            self.step();
        }
    }

    pub fn next_token(&mut self) -> Token<'i> {
        if !self.template_str_token_buffer.is_empty() {
            let t = self
                .template_str_token_buffer
                .pop_front()
                .expect("we checked that isn't empty");

            return t;
        }

        use TokenKind::*;

        self.skip_whitespace();

        let ch = match self.ch() {
            Some(ch) => ch,
            None => {
                return Token {
                    kind: TokenKind::End,
                    start: self.eof_pos,
                    text: "",
                }
            }
        };

        let t = match ch {
            b'"' if self.peek_char().is(b'"') => self.empty_string_literal(),
            b'"' => self.string_literal(),
            b'`' => self.multiline_string_literal(),
            b'$' if self.peek_char().is(b'{') => {
                let token = Token {
                    kind: DollarSignLBracket,
                    text: "${",
                    start: self.position,
                };
                self.step();
                token
            }
            b'}' if self.template_str_depth > 0 => self.multiline_string_literal(),
            b'(' => Token {
                kind: LParen,
                start: self.position,
                text: "(",
            },
            b')' => Token {
                kind: RParen,
                start: self.position,
                text: ")",
            },
            b'{' => Token {
                kind: LBracket,
                start: self.position,
                text: "{",
            },
            b'}' => Token {
                kind: RBracket,
                start: self.position,
                text: "}",
            },
            b'[' => Token {
                kind: LSquare,
                start: self.position,
                text: "[",
            },
            b']' => Token {
                kind: RSquare,
                start: self.position,
                text: "]",
            },
            b',' => Token {
                kind: Comma,
                text: ",",
                start: self.position,
            },
            b':' => Token {
                kind: Colon,
                text: ":",
                start: self.position,
            },
            b'=' => Token {
                kind: Assign,
                text: "=",
                start: self.position,
            },
            b'@' => Token {
                kind: AttributePrefix,
                text: "@",
                start: self.position,
            },
            b'/' if self.peek_char().is(b'/') => self.line_comment(),
            b'/' => self.pathname(),
            b'#' if self.peek_char().is(b'!') => self.shebang(),
            c if c.is_ascii_alphabetic() => self.keyword_or_identifier(),
            c if c.is_ascii_digit() => self.number(),
            _ => Token {
                kind: IllegalToken,
                text: std::str::from_utf8(
                    &self.input[self.position.value..self.position.value + 1],
                )
                .unwrap(),
                start: self.position,
            },
        };

        self.step();

        t
    }

    fn multiline_string_literal(&mut self) -> Token<'i> {
        match self.ch() {
            Some(b'`') if self.peek_char().is(b'`') => return self.empty_string_literal(),
            Some(b'`') if self.peek_char().is(b'$') && self.peek_n_char(1).is(b'{') => {
                self.template_str_depth += 1;

                self.template_str_token_buffer.push_back(Token {
                    start: self.position,
                    text: "`",
                    kind: TokenKind::OpeningBackTick,
                });

                self.step();

                self.template_str_token_buffer.push_back(Token {
                    kind: TokenKind::DollarSignLBracket,
                    text: "${",
                    start: self.position,
                });

                self.step();

                return self.template_str_token_buffer.pop_front().expect(
                    "there must be a token in the template_str_token_buffer at this point",
                );
            }
            Some(b'`') => {
                self.template_str_depth += 1;
                let p = self.position;
                self.template_str_token_buffer.push_back(Token {
                    start: p,
                    text: "`",
                    kind: TokenKind::OpeningBackTick,
                });
                self.step();
            }
            // End of template string
            Some(b'}') if self.peek_char().is(b'`') && self.template_str_depth > 0 => {
                self.template_str_depth -= 1;

                self.template_str_token_buffer.push_back(Token {
                    kind: TokenKind::RBracket,
                    start: self.position,
                    text: "}",
                });

                self.step(); // eat the curly

                self.template_str_token_buffer.push_back(Token {
                    kind: TokenKind::ClosingBackTick,
                    start: self.position,
                    text: "`",
                });

                return self.template_str_token_buffer.pop_front().expect(
                    "there must be a token in the template_str_token_buffer at this point",
                );
            }
            // End of expression part. Here we know that we've tokenized an expression
            // and are proceeding to the rest of the template string
            Some(b'}') => {
                self.template_str_token_buffer.push_back(Token {
                    kind: TokenKind::RBracket,
                    start: self.position,
                    text: "}",
                });

                self.step();
            }
            _ => unreachable!("should only start tokenizing template strings on a '`' or a '}}'"),
        };

        let start_pos = self.position;

        let (s, e) = loop {
            match self.ch() {
                _ if self.peek_char().is(b'$') && self.peek_n_char(1).is(b'{') => {
                    break (start_pos, self.position.value + 1);
                }
                // End of template string
                Some(b'`') => {
                    self.template_str_depth -= 1;

                    let string = self.input_slice(start_pos.value..self.position.value);

                    self.template_str_token_buffer.push_back(Token {
                        kind: TokenKind::StringLiteral,
                        start: start_pos,
                        text: string,
                    });

                    self.template_str_token_buffer.push_back(Token {
                        kind: TokenKind::ClosingBackTick,
                        text: "`",
                        start: self.position,
                    });

                    return self.template_str_token_buffer.pop_front().expect(
                        "there must be a token in the template_str_token_buffer at this point",
                    );
                }
                None => {
                    self.template_str_token_buffer.push_back(Token {
                        kind: TokenKind::UnfinishedMultiLineStringLiteral,
                        start: start_pos,
                        text: self.input_slice(start_pos.value..self.position.value),
                    });

                    return self.template_str_token_buffer.pop_front().expect(
                        "there must be a token in the template_str_token_buffer at this point",
                    );
                }
                _ => self.step(),
            }
        };

        let string = self.input_slice(s.value..e);

        self.template_str_token_buffer.push_back(Token {
            kind: TokenKind::StringLiteral,
            start: start_pos,
            text: string,
        });

        return self
            .template_str_token_buffer
            .pop_front()
            .expect("there must be a token in the template_str_token_buffer at this point");
    }

    fn string_literal(&mut self) -> Token<'i> {
        let start_pos = self.position;

        let (s, e) = self.read_while(|&c| c != b'"' && c != b'\n');

        match self.peek_char() {
            Some(b'\n') | None => {
                return Token {
                    kind: TokenKind::UnfinishedStringLiteral,
                    start: start_pos,
                    text: self.input_slice(start_pos.value..e),
                };
            }
            _ => {}
        }

        let string = self.input_slice(s..=e);

        self.step();

        Token {
            kind: TokenKind::StringLiteral,
            start: start_pos,
            text: string,
        }
    }

    fn empty_string_literal(&mut self) -> Token<'i> {
        let location = self.position;
        self.step();
        Token {
            kind: TokenKind::StringLiteral,
            start: location,
            text: "\"\"",
        }
    }

    fn keyword_or_identifier(&mut self) -> Token<'i> {
        let location = self.position;
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
            "false" => Token {
                kind: Boolean,
                start: location,
                text: string,
            },
            "true" => Token {
                kind: Boolean,
                start: location,
                text: string,
            },
            "null" => Token {
                kind: Null,
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
        let location = self.position;
        let (s, e) = self.read_while(|&c| !c.is_ascii_whitespace());
        let string = self.input_slice(s..e);

        Token {
            kind: TokenKind::Pathname,
            start: location,
            text: string,
        }
    }

    fn number(&mut self) -> Token<'i> {
        let location = self.position;
        let (s, e) = self.read_while(|&c| c.is_ascii_digit());
        let string = self.input_slice(s..e);

        if self.peek_char().is(b'.') {
            self.step();
            if self.peek_char().passes(|c| c.is_ascii_digit()) {
                self.step();
                let (.., e) = self.read_while(|&c| c.is_ascii_digit());
                let string = self.input_slice(s..e);

                return Token {
                    kind: TokenKind::Number,
                    start: location,
                    text: string,
                };
            }
        }

        Token {
            kind: TokenKind::Number,
            start: location,
            text: string,
        }
    }

    fn shebang(&mut self) -> Token<'i> {
        let location = self.position;
        let (s, e) = self.read_while(|&c| c != b'\n');
        let string = self.input_slice(s..e);
        Token {
            text: string,
            start: location,
            kind: TokenKind::Shebang,
        }
    }

    fn line_comment(&mut self) -> Token<'i> {
        let location = self.position;
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
        let token = self.next_token();

        if let TokenKind::End = token.kind {
            return None;
        }

        Some(token)
    }
}
