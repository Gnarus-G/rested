use insta::assert_ron_snapshot;

use rested::parser::Parser;

macro_rules! assert_ast {
    ($input:literal) => {
        let mut parser = Parser::new($input);
        let ast = parser.parse();

        insta::with_settings!({
             description => $input
        }, {
            assert_ron_snapshot!(ast);
        })
    };
}

#[test]
fn it_works() {
    assert_ast!(
        r#"
set BASE_URL "httas..."
post http://lasdf.. {}
// asdfasdf

let output_file = "output/file.json"
let token = env("auth-token")

@log(output_file)
post /asd {
  // asdfasd
  header "Authorization" token
  body `{
      "neet": "${env("var")}",
      "nerd": "${output_file}",
  }`
}

@skip
get /api {}

put /api {}
patch /api {}
delete /api {}
"#
    );
}

#[test]
fn parse_get_urls() {
    assert_ast!(
        r#"get http://localhost:8080
        get http://localhost:8080 {}"#
    );

    assert_ast!(
        r#"get "http://localhost:8080"
        get env("url")"#
    );

    assert_ast!(
        r#"get `${url}/api`
        get `${env(`${uri}:8080`)}/api`
        get `${env("url")}/api`"#
    );
}

#[test]
fn parse_post_url() {
    assert_ast!("post http://localhost");

    assert_ast!("post /api/v2");
}

#[test]
fn parse_attributes() {
    assert_ast!(r#"@log("path/to/file") get /api"#);
}

#[test]
fn parse_attributes_ignoring_comments_after_them() {
    assert_ast!(
        r#"@log("path/to/file") 
                // ignored
                get /api"#
    );
}

#[test]
fn parse_get_with_headers() {
    assert_ast!(
        r#"
        get http://localhost {
        header "Authorization" "Bearer token"
        header "random" "tokener Bear"
        }"#
    );
}

#[test]
fn parse_post_with_headers() {
    assert_ast!(
        r#"
        post http://localhost {
        header "Authorization" "Bearer token"
        header "random" "tokener Bear"
        }"#
    );
}
#[test]
fn parse_post_with_headers_and_body() {
    assert_ast!(
        r#"
        post http://localhost {
        header "Authorization" "Bearer token"
        body "{neet: 1337}"
        }"#
    );
}

#[test]
fn parse_post_with_headers_and_body_as_json_string() {
    assert_ast!(
        r#"
        post http://localhost {
        header "Authorization" "Bearer token"
        body `
        {"neet": 1337}
        `
        }"#
    );
}

#[test]
fn parse_env_call_expression() {
    assert_ast!(r#"post http://localhost { header "name" env("auth") body env("data") }"#);
    assert_ast!("let a = env(\"\")");
    assert_ast!(
        r#"env("base",
            //aasdf
            "url")"#
    );
    assert_ast!(
        r#"env("base",
            //aasdf
            // asdf
            "url"
            //trailing comment
            )"#
    );
}

#[test]
fn parse_global_constant_setting() {
    assert_ast!("set BASE_URL \"stuff\"");
}

#[test]
fn parse_template_string_literal() {
    assert_ast!(
        r#"
        post /api {
            body `{"neet": ${env("love")}, 2: ${"two"}}`
        }"#
    );

    assert_ast!(
        r#"
let b = `asdf ${}` 
"#
    );

    assert_ast!(
        r#"
let b = `asdf ${test} ${} ${"word"}`
"#
    );

    assert_ast!(
        r#"
let b = `asdf ${}` 
let c = {}
"#
    );

    assert_ast!(
        r#"
`asdf ${test} ${a}`
"#
    );

    assert_ast!(r#"`asdf ${`hello${"world"}`} jkl`"#);
}

#[test]
fn parse_json_object() {
    assert_ast!(
        r#"
let o = {
    key: "value",
    akey: 123,
    love: "me"
}"#
    );

    assert_ast!(
        r#"
let o = {
    akesy: 1323,
    love: "you"
        //trailing comment
}"#
    );

    assert_ast!(
        r#"
let o = {
    get: "value",
    post: 123,
    put: {
        author: "Me"
    },
    patch: true,
    delete: false,
    set: "true",
    let: view,
    null: null,
    header: "",
    body: "",
}"#
    );
}

#[test]
fn parse_json_object_ignoring_comments() {
    assert_ast!(
        r#"
let o = {
    key: "value",
    // akey: 123,
    love: [
        "asdf",
        // asdf,
        12
    ]
}"#
    );
}

#[test]
fn parse_json_object_allowing_trailing_comma() {
    assert_ast!(
        r#"
let o = {
    key: "value",
    akey: [1, 2, 3,],
    love: "me",
    "test": {a: "asdf", b: 1, c: 3,},
}"#
    );
}

#[test]
fn parse_json_object_deep() {
    assert_ast!(
        r#"
let o = {
    key: "value",
    akey: false,
    love: {
        hello: {
            w: "1",
            o: {
                two: 2.123,
                and: {}
            }
        }
    }
}"#
    );
}

#[test]
fn parse_json_object_with_array_keys() {
    assert_ast!(
        r#"
let o = {
    key: "value",
    akey: "234va",
    oKey: ["val", "val2"],
    aoKay: ["val", "123", {
        hey: "yo!",
        hello: "world"
    }]
}"#
    );
}

#[test]
fn parse_json_object_with_call_expressions() {
    assert_ast!(
        r#"
let o = {
    key: read("test"),
    akey: env("url")
}"#
    );
}
