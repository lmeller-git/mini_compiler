#![allow(dead_code)]
use frontend::get_ast;

mod backend;
pub mod frontend;

fn main() {
    let s = "
    x = 1 + 2;
    print x + 42;
    y =  x/ 6 % 5;
        ";
    let ast = get_ast(s).unwrap();
    println!("{}", ast);
    let code = backend::generate(&ast).unwrap();
    println!("{:#?}", code);
    _ = backend::asm_gen(code, "test.asm");
}
