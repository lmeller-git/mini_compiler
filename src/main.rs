#![allow(dead_code)]
use frontend::get_ast;

mod backend;
mod frontend;

fn main() {
    let s = "
        let x = 1 + 2;
        print x;
        let y = x * (5 + 1 - 2 / 2);
        ";
    let ast = get_ast(s).unwrap();
    println!("{}", ast);
}
