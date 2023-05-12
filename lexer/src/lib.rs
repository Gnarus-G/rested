pub mod locations;
use std::ops::Range;

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
    AttributePrefix,
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
            b'}' if self.inside_multiline_string => self.multiline_string_literal(),
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

        self.step(); //eat the opening quote

        let start_pos = self.position;

        let (s, e) = loop {
            let end_ahead = self.peek_char().is(b'`');
            let dollar_curly_ahead = self.peek_char().is(b'$') && self.peek_n_char(2).is(b'{');

            if dollar_curly_ahead {
                break (start_pos, self.position + 1);
            }

            if end_ahead {
                self.inside_multiline_string = false;
                let end_pos = self.position + 1;
                self.step();
                break (start_pos, end_pos);
            }

            if self.ch().is(b'`') {
                self.inside_multiline_string = false;
                break (start_pos, self.position);
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

        self.step(); //eat the opening quote

        let (s, e) = self.read_while(|&c| c != b'"' && c != b'\n');
        let string = self.input_slice(s..e);

        self.step(); //eat the closing quote or newline character

        match self.ch() {
            Some(b'\n') | None => {
                return Token {
                    kind: TokenKind::UnfinishedStringLiteral,
                    start: location,
                    text: string,
                };
            }
            _ => {}
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    use TokenKind::*;

    macro_rules! assert_lexes {
        ($input:literal, $tokens:expr) => {
            let mut lexer = Lexer::new($input);

            for token in $tokens {
                assert_eq!(lexer.next(), token);
            }
        };
    }

    pub fn at(line: usize, col: usize) -> Location {
        Location { line, col }
    }

    #[test]
    fn lex_string_literals() {
        assert_lexes!(
            r#""hello""#,
            [Token {
                kind: StringLiteral,
                text: "hello",
                start: at(0, 0)
            },]
        );

        assert_lexes!(
            r#""hello"#,
            [Token {
                kind: UnfinishedStringLiteral,
                text: "hello",
                start: at(0, 0)
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
                    start: at(1, 0)
                },
                Token {
                    kind: UnfinishedStringLiteral,
                    text: "world",
                    start: at(2, 0)
                }
            ]
        );

        assert_lexes!(
            r#" "" "" ``"#,
            [
                Token {
                    kind: StringLiteral,
                    text: "",
                    start: at(0, 1)
                },
                Token {
                    kind: StringLiteral,
                    text: "",
                    start: at(0, 4)
                },
                Token {
                    kind: StringLiteral,
                    text: "",
                    start: at(0, 7)
                }
            ]
        );

        assert_lexes!(
            r#" { "Bearer token" } "#,
            [
                Token {
                    kind: LBracket,
                    text: "{",
                    start: at(0, 1)
                },
                Token {
                    kind: StringLiteral,
                    text: "Bearer token",
                    start: at(0, 3)
                },
                Token {
                    kind: RBracket,
                    text: "}",
                    start: at(0, 18)
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
                    start: at(0, 0)
                },
                Token {
                    kind: UnfinishedMultiLineStringLiteral,
                    text: "\nstuff",
                    start: at(5, 0)
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
                    start: at(0, 0)
                },
                Token {
                    kind: Url,
                    text: "http://localhost",
                    start: at(0, 4),
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
                    start: at(0, 0),
                    text: "get"
                },
                Token {
                    kind: Url,
                    start: at(0, 4),
                    text: "http://localhost"
                },
                Token {
                    kind: LBracket,
                    start: at(0, 21),
                    text: "{"
                },
                Token {
                    kind: Header,
                    start: at(0, 23),
                    text: "header"
                },
                Token {
                    kind: StringLiteral,
                    start: at(0, 30),
                    text: "Authorization"
                },
                Token {
                    kind: StringLiteral,
                    start: at(0, 46),
                    text: "Bearer token"
                },
                Token {
                    kind: RBracket,
                    start: at(0, 61),
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
                    start: at(0, 0)
                },
                Token {
                    kind: Url,
                    text: "http://localhost",
                    start: at(1, 0),
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
                    start: at(0, 0)
                },
                Token {
                    kind: Url,
                    text: "http://localhost",
                    start: at(1, 4),
                },
                Token {
                    kind: LBracket,
                    text: "{",
                    start: at(2, 0),
                },
                Token {
                    kind: RBracket,
                    text: "}",
                    start: at(3, 0),
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
                    start: at(1, 0),
                    text: "post"
                },
                Token {
                    kind: Url,
                    start: at(1, 5),
                    text: "http://localhost"
                },
                Token {
                    kind: LBracket,
                    start: at(1, 22),
                    text: "{"
                },
                Token {
                    kind: Header,
                    start: at(2, 4),
                    text: "header"
                },
                Token {
                    kind: StringLiteral,
                    start: at(2, 11),
                    text: "Authorization"
                },
                Token {
                    kind: StringLiteral,
                    start: at(2, 27),
                    text: "Bearer token"
                },
                Token {
                    kind: Body,
                    start: at(3, 4),
                    text: "body"
                },
                Token {
                    kind: StringLiteral,
                    start: at(3, 9),
                    text: "{neet: 1337}"
                },
                Token {
                    kind: RBracket,
                    start: at(4, 0),
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
                    start: at(0, 0)
                },
                Token {
                    kind: LParen,
                    text: "(",
                    start: at(0, 3)
                },
                Token {
                    kind: RParen,
                    text: ")",
                    start: at(0, 4)
                },
                Token {
                    kind: Ident,
                    text: "env",
                    start: at(0, 6)
                },
                Token {
                    kind: LParen,
                    text: "(",
                    start: at(0, 9)
                },
                Token {
                    kind: StringLiteral,
                    text: "stuff",
                    start: at(0, 10)
                },
                Token {
                    kind: RParen,
                    text: ")",
                    start: at(0, 17)
                }
            ]
        );
    }

    #[test]
    fn lex_template_literals() {
        assert_lexes!(
            r#"`stuff${"interpolated"}(things${env("dead_night")}` `dohickeys`"#,
            [
                Token {
                    kind: MultiLineStringLiteral,
                    text: "stuff",
                    start: at(0, 0)
                },
                Token {
                    kind: DollarSignLBracket,
                    text: "${",
                    start: at(0, 6)
                },
                Token {
                    kind: StringLiteral,
                    text: "interpolated",
                    start: at(0, 8)
                },
                Token {
                    kind: MultiLineStringLiteral,
                    text: "(things",
                    start: at(0, 22)
                },
                Token {
                    kind: DollarSignLBracket,
                    text: "${",
                    start: at(0, 30)
                },
                Token {
                    kind: Ident,
                    text: "env",
                    start: at(0, 32)
                },
                Token {
                    kind: LParen,
                    text: "(",
                    start: at(0, 35)
                },
                Token {
                    kind: StringLiteral,
                    text: "dead_night",
                    start: at(0, 36)
                },
                Token {
                    kind: RParen,
                    text: ")",
                    start: at(0, 48)
                },
                Token {
                    kind: MultiLineStringLiteral,
                    text: "",
                    start: at(0, 49)
                },
                Token {
                    kind: MultiLineStringLiteral,
                    text: "dohickeys",
                    start: at(0, 52)
                },
            ]
        );

        assert_lexes!(
            r#"`a${"temp"}` }}"#,
            [
                Token {
                    kind: MultiLineStringLiteral,
                    text: "a",
                    start: at(0, 0)
                },
                Token {
                    kind: DollarSignLBracket,
                    text: "${",
                    start: at(0, 2)
                },
                Token {
                    kind: StringLiteral,
                    text: "temp",
                    start: at(0, 4)
                },
                Token {
                    kind: MultiLineStringLiteral,
                    text: "",
                    start: at(0, 10)
                },
                Token {
                    kind: RBracket,
                    text: "}",
                    start: at(0, 13)
                },
            ]
        );
    }
}
