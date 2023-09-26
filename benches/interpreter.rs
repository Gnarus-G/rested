use criterion::{criterion_group, criterion_main, Criterion};
use rested::{interpreter::environment::Environment, parser};

const SOURCE: &str = r#"
set BASE_URL env("base")

[env("tk")]

let string = "life"

post /todos {
   header "Authorization" env("tk")
   body json({
    userId: 1,
    id: 999,
    "title": `delectus aut ${string}`,
    "completed": env("base")
  })
}

set BASE_URL "httas..."
post http://lasdf.. {}
// asdfasdf

let output_file = "output/file.json"
let t = env("fake-token")

@log(output_file)
post /asd {
  // asdfasd
  header "Authorization" t
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
"#;

fn criterion_benchmark(c: &mut Criterion) {
    let mut parser = parser::Parser::new(SOURCE);
    let program = parser.parse();

    let env = Environment::new("./benches/vars.bench.rd.json").unwrap();

    c.bench_function("interpret ast", |b| {
        b.iter(|| {
            let _ = program.interpret(&env).unwrap();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
