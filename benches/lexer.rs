use criterion::{criterion_group, criterion_main, Criterion};
use rested::lexer;

const SOURCE: &str = r#"
set BASE_URL env("base")

[env("token")]

let string = "death"

post /todos {
   header "Authorization" env("token")
   body {
    userId: 1,
    id: 999,
    "title": `delectus aut ${string}`,
    "completed": env("base")
  }
}
"#;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("tokenize", |b| {
        b.iter(|| {
            let lexer = lexer::Lexer::new(SOURCE);
            let _: Vec<_> = lexer.into_iter().collect();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
