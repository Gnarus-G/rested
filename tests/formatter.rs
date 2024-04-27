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

        assert!(!formatter.has_error);

        assert_display_snapshot!(formatter.into_output());
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

[test, 12, {ness: false, wow: [1, 2,3]}]
"#
    );
}
