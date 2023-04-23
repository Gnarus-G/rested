use insta::assert_ron_snapshot;

use rested::parser::Parser;

#[test]
fn it_works() {
    let code = r#"
set BASE_URL "httas..."
post http://lasdf.. {}
// asdfasdf
@log("output/file.json")
get /asd {
  // asdfasd
  header "Authorization" "Bearer token"
  body `{"neet": "${env("var")}"}`
}"#;

    let p = Parser::new(code).parse().unwrap();
    insta::with_settings!({
         description => code
    }, {
        assert_ron_snapshot!(p);
    })
}
