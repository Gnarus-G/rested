use insta::assert_ron_snapshot;

use rested::parser::Parser;

#[test]
fn it_works() {
    let code = r#"
set BASE_URL "httas..."
post http://lasdf.. {}
// asdfasdf

let output_file = "output/file.json"
let token = env("auth-token")

@log(output_file)
get /asd {
  // asdfasd
  header "Authorization" token
  body `{
      "neet": "${env("var")}",
      "nerd": "${output_file}",
  }`
}"#;

    let p = Parser::new(code).parse().unwrap();
    insta::with_settings!({
         description => code
    }, {
        assert_ron_snapshot!(p);
    })
}
