use std::{
    error::Error,
    io::{stdin, stdout, Write},
};

mod ast;
mod lexer;
mod parser;

fn main() -> Result<(), Box<dyn Error>> {
    print!(":>> ");
    stdout().flush()?;

    for line in stdin().lines() {
        let code = line?;
        let lex = lexer::Lexer::new(&code);
        let mut parser = parser::Parser::new(lex);

        let ast = parser.parse();

        for s in ast.requests {
            match s {
                ast::Request::Get(get) => {
                    let res = ureq::get(get.url).call()?.into_string()?;
                    println!("{res}");
                }
            }
        }

        print!(":>> ");
        stdout().flush()?;
    }
    Ok(())
}
