#![allow(dead_code)]
use std::{
    fs::{File, create_dir},
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
    process::Command,
};

use clap::Parser;
use frontend::get_ast;

mod backend;
pub mod frontend;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct ParserImpl {
    file: PathBuf,
    #[arg(short, long, default_value_t = 1)]
    verbosity: u8,
}

fn main() {
    let args = ParserImpl::parse();
    let mut contents = File::open(&args.file).unwrap();
    let mut s = String::new();
    contents.read_to_string(&mut s).unwrap();
    let ast = get_ast(&s).unwrap();
    if args.verbosity > 1 {
        println!("{}", ast);
    }

    let mut dir = args.file.parent().unwrap_or(Path::new(".")).to_path_buf();
    dir.push("./target");

    let f_name = args
        .file
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .split(".")
        .next()
        .unwrap();

    match create_dir(&dir) {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
        e => panic!("could not create target/, {:#?}", e),
    }
    let code = backend::generate(&ast).unwrap();
    if args.verbosity > 1 {
        println!("{:#?}", code);
    }
    _ = backend::asm_gen(code, &dir.join(format!("{}.asm", f_name)));

    assemble(&dir.join(format!("{}.asm", f_name)), f_name);
    link_with_gcc(&dir.join(format!("{}.o", f_name)), f_name);

    if args.verbosity > 0 {
        println!("code succesfully generated in {}/{}", dir.display(), f_name);
    }
}

fn assemble(f: &Path, f_name: &str) {
    let status = Command::new("nasm")
        .args(&[
            "-f",
            "elf64",
            f.to_str().unwrap(),
            "-o",
            &format!("{}/{}.o", f.parent().unwrap().display(), f_name),
        ])
        .status()
        .expect("failed to run nasm");
    assert!(status.success(), "nasm failed");
}

fn link_with_gcc(f: &Path, f_name: &str) {
    let status = Command::new("gcc")
        .args(&[
            "-no-pie",
            f.to_str().unwrap(),
            "-o",
            &format!("{}/{}", f.parent().unwrap().display(), f_name),
        ])
        .status()
        .expect("failed to run gcc");
    assert!(status.success(), "gcc failed");
}
