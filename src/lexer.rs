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
    MultiLineStringLiteral,
    Url,

    // special characters
    LParen,
    RParen,
    LBracket,
    RBracket,
    End,

    //edge cases
    UnfinishedStringLiteral,
    UnfinishedMultiLineStringLiteral,
    IllegalToken,
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
                    location: self.cursor,
                    text: "",
                }
            }
        };

        let t = match ch {
            b'"' if self.peek_char().is(b'"') => self.empty_string_literal(),
            b'"' => self.string_literal(),
            b'`' if self.peek_char().is(b'`') => self.empty_string_literal(),
            b'`' => self.multiline_string_literal(),
            b'(' => Token {
                kind: LParen,
                location: self.cursor,
                text: "(",
            },
            b')' => Token {
                kind: RParen,
                location: self.cursor,
                text: ")",
            },
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
            c if c.is_ascii_alphabetic() => self.keyword_or_identifier(),
            _ => Token {
                kind: IllegalToken,
                text: std::str::from_utf8(&self.input[self.position..self.position + 1]).unwrap(),
                location: self.cursor,
            },
        };

        self.step();

        t
    }

    fn multiline_string_literal(&mut self) -> Token<'i> {
        let location = self.cursor;

        self.step(); //eat the opening quote

        let (s, e) = self.read_while(|&c| c != b'`');
        let string = self.input_slice(s..e);

        self.step(); //eat the closing quote

        if let None = self.ch() {
            return Token {
                kind: TokenKind::UnfinishedMultiLineStringLiteral,
                location,
                text: string,
            };
        }

        Token {
            kind: TokenKind::MultiLineStringLiteral,
            location,
            text: string,
        }
    }

    fn string_literal(&mut self) -> Token<'i> {
        let location = self.cursor;

        self.step(); //eat the opening quote

        let (s, e) = self.read_while(|&c| c != b'"' && c != b'\n');
        let string = self.input_slice(s..e);

        self.step(); //eat the closing quote or newline character

        match self.ch() {
            Some(b'\n') | None => {
                return Token {
                    kind: TokenKind::UnfinishedStringLiteral,
                    location,
                    text: string,
                };
            }
            _ => {}
        }

        Token {
            kind: TokenKind::StringLiteral,
            location,
            text: string,
        }
    }

    fn empty_string_literal(&mut self) -> Token<'i> {
        let location = self.cursor;
        self.step();
        Token {
            kind: TokenKind::StringLiteral,
            location,
            text: "",
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
                let (.., e) = self.read_while(|&c| !c.is_ascii_whitespace());
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
            r#""hello"#,
            [Token {
                kind: UnfinishedStringLiteral,
                text: "hello",
                location: (0, 0).into()
            },]
        );

        assert_lexes!(
            r#"
"hello
"world
"#,
            [
                Token {
                    kind: UnfinishedStringLiteral,
                    text: "hello",
                    location: (1, 0).into()
                },
                Token {
                    kind: UnfinishedStringLiteral,
                    text: "world",
                    location: (2, 0).into()
                }
            ]
        );

        assert_lexes!(
            r#" "" "" ``"#,
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
                },
                Token {
                    kind: StringLiteral,
                    text: "",
                    location: (0, 7).into()
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

        assert_lexes!(
            r#"`
{
    stuff
}`

`
stuff"#,
            [
                Token {
                    kind: MultiLineStringLiteral,
                    text: "\n{\n    stuff\n}",
                    location: (0, 0).into()
                },
                Token {
                    kind: UnfinishedMultiLineStringLiteral,
                    text: "\nstuff",
                    location: (5, 0).into()
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
            "get http://localhost { header \"Authorization\" \"Bearer token\" }",
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
                    kind: StringLiteral,
                    location: (0, 30).into(),
                    text: "Authorization"
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
    header "Authorization" "Bearer token" 
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
                    kind: StringLiteral,
                    location: (2, 11).into(),
                    text: "Authorization"
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

    #[test]
    fn lex_call_expression() {
        assert_lexes!(
            r#"env() env("stuff")"#,
            [
                Token {
                    kind: Ident,
                    text: "env",
                    location: (0, 0).into()
                },
                Token {
                    kind: LParen,
                    text: "(",
                    location: (0, 3).into()
                },
                Token {
                    kind: RParen,
                    text: ")",
                    location: (0, 4).into()
                },
                Token {
                    kind: Ident,
                    text: "env",
                    location: (0, 6).into()
                },
                Token {
                    kind: LParen,
                    text: "(",
                    location: (0, 9).into()
                },
                Token {
                    kind: StringLiteral,
                    text: "stuff",
                    location: (0, 10).into()
                },
                Token {
                    kind: RParen,
                    text: ")",
                    location: (0, 17).into()
                }
            ]
        );
    }
}
