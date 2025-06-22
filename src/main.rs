#![allow(dead_code)]
use frontend::get_ast;

mod backend;
mod frontend;

fn main() {
    let s = "1 + 2";
    let _ = get_ast(s);
}
