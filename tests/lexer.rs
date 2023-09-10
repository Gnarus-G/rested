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
    assert_lexes!("123124 1.0");
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
}
