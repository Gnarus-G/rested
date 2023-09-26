use criterion::{criterion_group, criterion_main, Criterion};
use rested::parser;

const SOURCE: &str = r#"
set BASE_URL env("base")

[env("token")]

let string = "death"

post /todos {
   header "Authorization" env("token")
   body json({
    userId: 1,
    id: 999,
    "title": `delectus aut ${string}`,
    "completed": env("base")
  })
}
"#;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("parse ast", |b| {
        b.iter(|| {
            let mut parser = parser::Parser::new(SOURCE);
            let _ = parser.parse();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
