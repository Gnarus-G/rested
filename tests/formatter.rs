use insta::assert_display_snapshot;
use rested::{
    fmt,
    parser::{ast::Program, ast_visit::VisitWith},
};

macro_rules! assert_fmt {
    ($input:literal) => {
        let program = Program::from($input);

        let mut formatter = fmt::FormattedPrinter::new();

        program.visit_with(&mut formatter);

        assert!(
            formatter.error.is_none(),
            "formatter has errors {:?}",
            formatter.error
        );

        assert_display_snapshot!(formatter.into_output());
    };
}

macro_rules! assert_error {
    ($input:literal) => {
        let program = Program::from($input);

        let mut formatter = fmt::FormattedPrinter::new();

        program.visit_with(&mut formatter);

        assert!(
            formatter.error.is_some(),
            "we should have collected an error"
        );
    };
}

#[test]
fn it_works() {
    assert_fmt!(
        r#"
set BASE_URL 
  env("hi")

let t = {
  value: 23,
  love: "you",
  hello: {
    world: true, test: {
      ing: "true", wow: { b: 122 }
    }
  }
}


let l 
= t

        let l = [null, t]

let l = { l: l,
   k: "asdf"
}

let m = json(
l
)

let a = [m]

let c = json(
[
{ "asdf": true, test: 12434,
a: 124, b: [
{
we: 123
}
]
}]
)

get /admin {
   header "Content-Type" "application/json"
   body json({"a": 12, t: true}) }

[test, 12, {ness: false, wow: [1, 2,3]}]

post `${env
    ("b_url")}/asdf${}` {
   header "Content-Type" "application/json"
   body m }
"#
    );
}

#[test]
fn it_echos_line_comments() {
    assert_fmt!(
        r#"
set BASE_URL 
  env("hi")

// let t = {
//   value: 23,
//   love: "you",
//   hello: {
//     world: true, test: {
//       ing: "true", wow: { b: 122 }
//     }
//   }
// }


// let l 
// = t
//
//         let l = [null, t]

post `${env
    ("b_url")}/asdf${}` {
   header "Content-Type" "application/json"
       // This a line comment
       // And this is another
   body m }
"#
    );
}

#[test]
fn it_collect_an_error_on_bad_syntax() {
    assert_error!(
        r#"
set BASE_URL 
  env("hi")

let a = [m]

get /admin {
   header "Content-Type" "application/json"
   body json({'a': 12, t: true}) }

[test, 12, {ness: false, wow: [1, 2,3]}]

post `${env
    ("b_url")}/asdf${}` {
   header "Content-Type" "application/json"
   body m }
"#
    );
}
