#![allow(dead_code)]
use std::{
    fs::{File, create_dir},
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU8, Ordering},
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

pub static VERBOSITY: AtomicU8 = AtomicU8::new(0);

fn main() {
    let args = ParserImpl::parse();
    VERBOSITY.store(args.verbosity, Ordering::Relaxed);
    let mut contents = File::open(&args.file).unwrap();
    let mut s = String::new();
    contents.read_to_string(&mut s).unwrap();
    let ast = get_ast(&s).unwrap();

    print_if!(2, "{}", ast);

    let mut dir = args.file.parent().unwrap_or(Path::new(".")).to_path_buf();
    dir.push("target");

    print_if!(1, "generating code in {}", dir.display());

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

    print_if!(2, "{:#?}", code);

    _ = backend::asm_gen(code, &dir.join(format!("{}.asm", f_name)));

    print_if!(
        1,
        "generating {0}.o from {0}.asm",
        dir.join(format!("{}", f_name)).display()
    );

    assemble(&dir.join(format!("{}.asm", f_name)), f_name);

    print_if!(
        1,
        "{} succesfully generated",
        dir.join(format!("{}.o", f_name)).display()
    );

    link_with_gcc(&dir.join(format!("{}.o", f_name)), f_name);

    print_if!(
        0,
        "code succesfully generated in {}/{}",
        dir.display(),
        f_name
    );
}

fn assemble(f: &Path, f_name: &str) {
    let status = Command::new("nasm")
        .args([
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
        .args([
            "-no-pie",
            f.to_str().unwrap(),
            "-o",
            &format!("{}/{}", f.parent().unwrap().display(), f_name),
        ])
        .status()
        .expect("failed to run gcc");
    assert!(status.success(), "gcc failed");
}

#[macro_export]
macro_rules! print_if {
    ($min_verbosity:expr) => {
        if $crate::VERBOSITY.load(Ordering::Relaxed) > $min_verbosity {
            println!();
        }
    };
    ($min_verbosity:expr, $($arg:tt)*) => {
        if $crate::VERBOSITY.load(Ordering::Relaxed) > $min_verbosity {
            println!("{}", format_args!($($arg)*))
        }
    };
}
