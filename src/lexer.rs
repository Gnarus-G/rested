use std::ops::Range;

#[derive(Debug, PartialEq)]
pub enum TokenKind {
    // keywords
    Get,
    Header,

    Ident,

    // literals
    StringLiteral,
    Url,

    // operators
    Assign,

    // special characters
    Quote,
    LBracket,
    RBracket,
    End,
}

#[derive(Debug, PartialEq)]
pub struct Token<'t> {
    pub kind: TokenKind,
    pub text: &'t str,
}

#[derive(Debug)]
pub struct Lexer<'i> {
    input: &'i [u8],
    position: usize,
}

impl<'i> Lexer<'i> {
    pub fn new(input: &'i str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
        }
    }

    pub fn input(&self) -> &'i str {
        std::str::from_utf8(&self.input).expect("input should only contain utf-8 characters")
    }

    fn input_slice(&self, range: Range<usize>) -> &'i str {
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
        self.position += 1;
    }

    fn peek_char(&self) -> Option<&u8> {
        self.char_at(self.position + 1)
    }

    fn if_peek(&self, ch: u8) -> bool {
        match self.peek_char() {
            Some(c) => *c == ch,
            None => false,
        }
    }

    fn if_previous(&self, ch: u8) -> bool {
        if self.position == 0 {
            return false;
        }
        match self.char_at(self.position - 1) {
            Some(c) => *c == ch,
            None => false,
        }
    }

    /// Assumes that the character at the current position, immediately before calling
    /// this function is also true the predicate function given.
    fn read_while<P: Fn(&u8) -> bool>(&mut self, predicate: P) -> (usize, usize) {
        let start_pos = self.position;

        while match self.peek_char() {
            Some(c) => predicate(c),
            None => false,
        } {
            self.step();
        }

        return (start_pos, self.position + 1);
    }

    fn skip_whitespace(&mut self) {
        while match self.ch() {
            Some(c) => c.is_ascii_whitespace(),
            None => false,
        } {
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
                    text: "",
                }
            }
        };

        let t = match ch {
            b'"' => Token {
                kind: Quote,
                text: "\"",
            },
            b'{' => Token {
                kind: LBracket,
                text: "{",
            },
            b'}' => Token {
                kind: RBracket,
                text: "}",
            },
            b'=' => Token {
                kind: Assign,
                text: "=",
            },
            _ if self.if_previous(b'"') => self.string_literal(),
            c if c.is_ascii_alphabetic() => self.keyword_or_identifier(),
            &c => todo!("{}", c as char),
        };

        self.step();

        t
    }

    fn string_literal(&mut self) -> Token<'i> {
        let (s, e) = self.read_while(|&c| c != b'"');
        let string = self.input_slice(s..e);

        Token {
            kind: TokenKind::StringLiteral,
            text: string,
        }
    }

    fn keyword_or_identifier(&mut self) -> Token<'i> {
        let (s, e) = self.read_while(|c| c.is_ascii_alphabetic());
        let string = self.input_slice(s..e);

        use TokenKind::*;

        match string {
            "get" => Token {
                kind: Get,
                text: string,
            },
            "header" => Token {
                kind: Header,
                text: string,
            },
            "http" | "https" => {
                let (.., e) = self.read_while(|&c| c != b' ');
                let s = self.input_slice(s..e);
                Token { kind: Url, text: s }
            }
            s => Token {
                kind: Ident,
                text: s,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use TokenKind::*;

    impl<'i> Iterator for Lexer<'i> {
        type Item = Token<'i>;

        fn next(&mut self) -> Option<Self::Item> {
            let token = self.next();
            if let TokenKind::End = token.kind {
                return None;
            }
            return Some(token);
        }
    }

    #[test]
    fn lex_get_url() {
        let mut s = Lexer::new("get http://localhost");

        assert_eq!(
            s.next(),
            Token {
                kind: Get,
                text: "get"
            }
        );

        assert_eq!(
            s.next(),
            Token {
                kind: Url,
                text: "http://localhost"
            }
        )
    }

    #[test]
    fn lex_get_url_with_header() {
        let lexer = Lexer::new("get http://localhost { header Authorization = \"Bearer token\" }");

        let tokens: Vec<_> = lexer.into_iter().collect();

        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: Get,
                    text: "get"
                },
                Token {
                    kind: Url,
                    text: "http://localhost"
                },
                Token {
                    kind: LBracket,
                    text: "{"
                },
                Token {
                    kind: Header,
                    text: "header"
                },
                Token {
                    kind: Ident,
                    text: "Authorization"
                },
                Token {
                    kind: Assign,
                    text: "="
                },
                Token {
                    kind: Quote,
                    text: "\""
                },
                Token {
                    kind: StringLiteral,
                    text: "Bearer token"
                },
                Token {
                    kind: Quote,
                    text: "\""
                },
                Token {
                    kind: RBracket,
                    text: "}"
                },
            ]
        )
    }
}
