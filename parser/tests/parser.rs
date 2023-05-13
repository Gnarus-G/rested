use insta::assert_ron_snapshot;

use parser::Parser;

macro_rules! assert_ast {
    ($input:literal) => {
        let mut parser = Parser::new($input);
        let ast = parser.parse().unwrap();

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

use lexer::locations::Location;

pub fn at(line: usize, col: usize) -> Location {
    Location { line, col }
}

#[test]
fn parse_get_urls() {
    assert_ast!(
        r#"get http://localhost:8080
        get http://localhost:8080 {}"#
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
}
