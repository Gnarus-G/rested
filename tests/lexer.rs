use insta::assert_debug_snapshot;

macro_rules! assert_lexes {
    ($input:literal) => {
        let lexer = rested::lexer::Lexer::new($input);

        insta::with_settings!({
             description => $input
        }, {
            assert_debug_snapshot!(lexer.into_iter().collect::<Vec<rested::lexer::Token>>());
        })
    }
}

#[test]
fn lex_put_patch_delete() {
    assert_lexes!(
        r#"
put /api {}
patch /api {}
delete /api {}
"#
    );
}

#[test]
fn lex_string_literals() {
    assert_lexes!(r#""hello""#);

    assert_lexes!(r#""hello"#);

    assert_lexes!(
        r#"
"hello
"world
"#
    );

    assert_lexes!(r#" "" "" ``"#);

    assert_lexes!(r#" { "Bearer token" } "#);

    assert_lexes!(
        r#"`
{
    stuff
}`

`
stuff"#
    );
}

#[test]
fn lex_bools() {
    assert_lexes!("true false");
}

#[test]
fn lex_numbers() {
    assert_lexes!("123124 1.0 23.8635");
}

#[test]
fn lex_get_url() {
    assert_lexes!("get http://localhost");
}

#[test]
fn lex_get_url_with_header() {
    assert_lexes!("get http://localhost { header \"Authorization\" \"Bearer token\" }");
}

#[test]
fn lex_get_url_over_many_lines() {
    assert_lexes!("get\nhttp://localhost");

    assert_lexes!(
        r#"get 
    http://localhost 
{
}"#
    );
}

#[test]
fn lex_get_url_with_header_and_body() {
    assert_lexes!(
        r#"
post http://localhost { 
    header "Authorization" "Bearer token" 
    body "{neet: 1337}" 
}"#
    );
}

#[test]
fn lex_call_expression() {
    assert_lexes!(r#"env() env("stuff")"#);
    assert_lexes!(
        r#"
let a = env(")
"#
    );

    assert_lexes!(
        r#"get `http://localhost:8080/api?sort=${sort}&filter=${filter}`
let a = read("testasdf.rd")"#
    );
}

#[test]
fn lex_template_literals() {
    assert_lexes!(r#"`stuff${"interpolated"}(things${env("dead_night")}` `dohickeys`"#);

    assert_lexes!(r#"`a${"temp"}` }}"#);

    assert_lexes!(
        r#"
let b = `asdf ${}`
"#
    );

    assert_lexes!(r#"` ${} ${} `"#);
    assert_lexes!(r#"`head ${} ${} tail`"#);

    assert_lexes!(
        r#"
let b = `asdf ${test} ${} ${"word"}`
"#
    );

    assert_lexes!(
        r#"
let b = `asdf ${}` 
let c = {}
"#
    );

    assert_lexes!(r#"`${test} ${df} `"#);
    assert_lexes!(r#"`asdf ${test} ${df} `"#);

    assert_lexes!(r#"`asdf ${`hello${"world"}`} ${asdf} jkl`"#);

    assert_lexes!(
        r#"
        post /api {
            body `{"neet": ${env("love")}, 2: ${"two"}}`
        }

        post http://localhost {
          header "Authorization" "Bearer token"
          body `{"neet": 1337}`
        }

        "#
    );

    assert_lexes!(r#"`string${env("base")}morestring${true}evenmore${"expr string"}`"#);

    assert_lexes!(r#"`asdf ${`hello${"world"}`} jkl`"#);
}

#[test]
fn lex_json_object() {
    assert_lexes!(
        r#"
let obj = {
    key: "value",
    otherKey: ["val", "vlue"]
}
"#
    );

    assert_lexes!(
        r#"
let o = {
    key: "value",
    post: {
        author: "Me"
    }
}"#
    );
}

#[test]
fn lex_eof_position() {
    let src = "let varname = ";

    let mut lexer = Lexer::new(src);

    use rested::lexer::*;

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::Let,
            text: "let",
            start: rested::lexer::locations::Position::default()
        }
    );

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::Ident,
            text: "varname",
            start: locations::Position {
                line: 0,
                col: 4,
                value: 4
            }
        }
    );

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::Assign,
            text: "=",
            start: locations::Position {
                line: 0,
                col: 12,
                value: 12
            }
        }
    );

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::End,
            text: "",
            start: locations::Position {
                line: 0,
                col: 13,
                value: 13
            }
        }
    );
}

#[test]
fn lex_eof_position_with_newline() {
    let src = "let varname = \n";
    let mut lexer = Lexer::new(src);

    use rested::lexer::*;

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::Let,
            text: "let",
            start: rested::lexer::locations::Position::default()
        }
    );

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::Ident,
            text: "varname",
            start: locations::Position {
                line: 0,
                col: 4,
                value: 4
            }
        }
    );

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::Assign,
            text: "=",
            start: locations::Position {
                line: 0,
                col: 12,
                value: 12
            }
        }
    );

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::End,
            text: "",
            start: locations::Position {
                line: 1,
                col: 0,
                value: 14
            }
        }
    );
}

#[test]
fn lex_eof_position_with_newlines() {
    let src = r#"let varname = "value"
        let
"#;
    let mut lexer = Lexer::new(src);

    use rested::lexer::*;

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::Let,
            text: "let",
            start: rested::lexer::locations::Position::default()
        }
    );

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::Ident,
            text: "varname",
            start: locations::Position {
                line: 0,
                col: 4,
                value: 4
            }
        }
    );

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::Assign,
            text: "=",
            start: locations::Position {
                line: 0,
                col: 12,
                value: 12
            }
        }
    );

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::StringLiteral,
            text: "\"value\"",
            start: locations::Position {
                line: 0,
                col: 14,
                value: 14
            }
        }
    );

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::Let,
            text: "let",
            start: locations::Position {
                line: 1,
                col: 8,
                value: 30
            }
        }
    );

    assert_eq!(
        lexer.next_token(),
        Token {
            kind: TokenKind::End,
            text: "",
            start: locations::Position {
                line: 2,
                col: 0,
                value: 33
            }
        }
    );
}
