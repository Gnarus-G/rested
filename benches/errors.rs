use criterion::{criterion_group, criterion_main, Criterion};
use rested::parser;

const SOURCE: &str = r#"
set BASE_URL env("base"

[env("token")]

let string "death"

post a/ {
   header "Authorization" env("token")
   body {
    userId 1,
    id: 999,
    "title": delectus aut ${string}`,
    "completed": env("bas
  }
}
"#;

fn getting_errors_benchmark(c: &mut Criterion) {
    let mut parser = parser::Parser::new(SOURCE);
    let program = parser.parse();

    c.bench_function("collecting errors", |b| {
        b.iter(|| {
            let _ = program.errors();
        })
    });
}

criterion_group!(benches, getting_errors_benchmark);
criterion_main!(benches);
