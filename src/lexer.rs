use std::ops::Range;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenKind {
    // keywords
    Get,
    Post,
    Header,
    Body,

    Ident,

    // literals
    StringLiteral,
    Url,

    // operators
    Assign,

    // special characters
    LBracket,
    RBracket,
    End,
}

#[derive(PartialEq, Clone, Copy)]
pub struct Location {
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Debug for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(PartialEq)]
pub struct Token<'t> {
    pub kind: TokenKind,
    pub text: &'t str,
    pub location: Location,
}

impl<'i> std::fmt::Debug for Token<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}({:?}) at {:?}", self.kind, self.text, self.location)
    }
}

#[derive(Debug)]
pub struct Lexer<'i> {
    input: &'i [u8],
    position: usize,
    cursor: Location,
}

impl<'i> Lexer<'i> {
    pub fn new(input: &'i str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
            cursor: Location { line: 0, col: 0 },
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
        if let Some(b'\n') = self.ch() {
            self.cursor.line += 1;
            self.cursor.col = 0;
        } else {
            self.cursor.col += 1;
        };

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
    /// this function is also true for the predicate function given.
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
                    location: self.cursor,
                    text: "",
                }
            }
        };

        let t = match ch {
            b'"' => self.string_literal(),
            b'{' => Token {
                kind: LBracket,
                location: self.cursor,
                text: "{",
            },
            b'}' => Token {
                kind: RBracket,
                location: self.cursor,
                text: "}",
            },
            b'=' => Token {
                kind: Assign,
                location: self.cursor,
                text: "=",
            },
            c if c.is_ascii_alphabetic() => self.keyword_or_identifier(),
            &c => todo!("{}", c as char),
        };

        self.step();

        t
    }

    fn string_literal(&mut self) -> Token<'i> {
        let location = self.cursor;

        self.step(); //eat the opening quote

        if let Some(b'"') = self.ch() {
            return Token {
                kind: TokenKind::StringLiteral,
                location,
                text: "",
            };
        }

        let (s, e) = self.read_while(|&c| c != b'"');
        let string = self.input_slice(s..e);

        self.step(); //eat the closing quote

        Token {
            kind: TokenKind::StringLiteral,
            location,
            text: string,
        }
    }

    fn keyword_or_identifier(&mut self) -> Token<'i> {
        let location = self.cursor;
        let (s, e) = self.read_while(|c| c.is_ascii_alphabetic());
        let string = self.input_slice(s..e);

        use TokenKind::*;

        match string {
            "get" => Token {
                kind: Get,
                location,
                text: string,
            },
            "post" => Token {
                kind: Post,
                location,
                text: string,
            },
            "header" => Token {
                kind: Header,
                location,
                text: string,
            },
            "body" => Token {
                kind: Body,
                location,
                text: string,
            },
            "http" | "https" => {
                let (.., e) = self.read_while(|&c| c != b' ');
                let s = self.input_slice(s..e);
                Token {
                    kind: Url,
                    location,
                    text: s,
                }
            }
            s => Token {
                kind: Ident,
                location,
                text: s,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use TokenKind::*;

    impl Into<Location> for (usize, usize) {
        fn into(self) -> Location {
            Location {
                line: self.0,
                col: self.1,
            }
        }
    }

    macro_rules! assert_lexes {
        ($input:literal, $tokens:expr) => {
            let mut lexer = Lexer::new($input);

            for token in $tokens {
                assert_eq!(lexer.next(), token);
            }
        };
    }

    #[test]
    fn lex_string_literals() {
        assert_lexes!(
            r#""hello""#,
            [Token {
                kind: StringLiteral,
                text: "hello",
                location: (0, 0).into()
            },]
        );

        assert_lexes!(
            r#" "" "" "#,
            [
                Token {
                    kind: StringLiteral,
                    text: "",
                    location: (0, 1).into()
                },
                Token {
                    kind: StringLiteral,
                    text: "",
                    location: (0, 4).into()
                }
            ]
        );

        assert_lexes!(
            r#" { "Bearer token" } "#,
            [
                Token {
                    kind: LBracket,
                    text: "{",
                    location: (0, 1).into()
                },
                Token {
                    kind: StringLiteral,
                    text: "Bearer token",
                    location: (0, 3).into()
                },
                Token {
                    kind: RBracket,
                    text: "}",
                    location: (0, 18).into()
                }
            ]
        );
    }

    #[test]
    fn lex_get_url() {
        assert_lexes!(
            "get http://localhost",
            [
                Token {
                    kind: Get,
                    text: "get",
                    location: (0, 0).into()
                },
                Token {
                    kind: Url,
                    text: "http://localhost",
                    location: (0, 4).into(),
                }
            ]
        );
    }

    #[test]
    fn lex_get_url_with_header() {
        assert_lexes!(
            "get http://localhost { header Authorization = \"Bearer token\" }",
            vec![
                Token {
                    kind: Get,
                    location: (0, 0).into(),
                    text: "get"
                },
                Token {
                    kind: Url,
                    location: (0, 4).into(),
                    text: "http://localhost"
                },
                Token {
                    kind: LBracket,
                    location: (0, 21).into(),
                    text: "{"
                },
                Token {
                    kind: Header,
                    location: (0, 23).into(),
                    text: "header"
                },
                Token {
                    kind: Ident,
                    location: (0, 30).into(),
                    text: "Authorization"
                },
                Token {
                    kind: Assign,
                    location: (0, 44).into(),
                    text: "="
                },
                Token {
                    kind: StringLiteral,
                    location: (0, 46).into(),
                    text: "Bearer token"
                },
                Token {
                    kind: RBracket,
                    location: (0, 61).into(),
                    text: "}"
                },
            ]
        );
    }

    #[test]
    fn lex_get_url_over_many_lines() {
        assert_lexes!(
            "get\nhttp://localhost",
            [
                Token {
                    kind: Get,
                    text: "get",
                    location: (0, 0).into()
                },
                Token {
                    kind: Url,
                    text: "http://localhost",
                    location: (1, 0).into(),
                }
            ]
        );

        assert_lexes!(
            r#"get 
    http://localhost 
{
}"#,
            [
                Token {
                    kind: Get,
                    text: "get",
                    location: (0, 0).into()
                },
                Token {
                    kind: Url,
                    text: "http://localhost",
                    location: (1, 4).into(),
                },
                Token {
                    kind: LBracket,
                    text: "{",
                    location: (2, 0).into(),
                },
                Token {
                    kind: RBracket,
                    text: "}",
                    location: (3, 0).into(),
                }
            ]
        );
    }

    #[test]
    fn lex_get_url_with_header_and_body() {
        assert_lexes!(
            r#"
post http://localhost { 
    header Authorization = "Bearer token" 
    body "{neet: 1337}" 
}"#,
            vec![
                Token {
                    kind: Post,
                    location: (1, 0).into(),
                    text: "post"
                },
                Token {
                    kind: Url,
                    location: (1, 5).into(),
                    text: "http://localhost"
                },
                Token {
                    kind: LBracket,
                    location: (1, 22).into(),
                    text: "{"
                },
                Token {
                    kind: Header,
                    location: (2, 4).into(),
                    text: "header"
                },
                Token {
                    kind: Ident,
                    location: (2, 11).into(),
                    text: "Authorization"
                },
                Token {
                    kind: Assign,
                    location: (2, 25).into(),
                    text: "="
                },
                Token {
                    kind: StringLiteral,
                    location: (2, 27).into(),
                    text: "Bearer token"
                },
                Token {
                    kind: Body,
                    location: (3, 4).into(),
                    text: "body"
                },
                Token {
                    kind: StringLiteral,
                    location: (3, 9).into(),
                    text: "{neet: 1337}"
                },
                Token {
                    kind: RBracket,
                    location: (4, 0).into(),
                    text: "}"
                },
            ]
        );
    }
}
