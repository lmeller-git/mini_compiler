use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::Ordering,
};

use clap::Parser;
use mini_compiler::{VERBOSITY, backend, frontend::get_ast, print_if};

const EXT: &str = "lang";

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct ParserImpl {
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    #[arg(short, long, default_value = "a.out")]
    output: String,

    #[arg(short, long, default_value = "./target")]
    target: String,

    #[arg(short, long, default_value_t = 1)]
    verbosity: u8,
}

fn main() {
    let args = ParserImpl::parse();
    VERBOSITY.store(args.verbosity, Ordering::Relaxed);

    let mut files = Vec::new();
    for path in &args.inputs {
        if path.is_dir() {
            files.extend(recursive_collect(path));
        } else if path.is_file() {
            // ignore EXT for manually added files
            files.push(path.clone());
        } else {
            panic!("Input file not found: {}", path.display());
        }
    }

    if files.is_empty() {
        println!("No files to compile");
        return;
    }

    let target_dir = PathBuf::from(args.target);
    if let Err(e) = fs::create_dir_all(&target_dir) {
        panic!(
            "could not create target directory {}, {:#?}",
            target_dir.display(),
            e
        );
    }

    let mut obj_files = Vec::new();

    print_if!(1, "compiling {} files...", files.len());

    for file in &files {
        let f_name = file.file_stem().unwrap().to_str().unwrap();
        print_if!(1, "Compiling {}", file.display());

        let mut s = String::new();
        File::open(file).unwrap().read_to_string(&mut s).unwrap();

        let ast = get_ast(&s).unwrap();
        print_if!(2, "AST for {}: {}", f_name, ast);

        let code = backend::generate(&ast).unwrap();
        print_if!(2, "IR for {}: {:#?}", f_name, code);

        let asm_path = target_dir.join(format!("{}.asm", f_name));
        let obj_path = target_dir.join(format!("{}.o", f_name));

        backend::asm_gen(code, &asm_path).unwrap();

        print_if!(
            1,
            "Assembling {} to {}",
            asm_path.display(),
            obj_path.display()
        );
        assemble(&asm_path, &obj_path);

        obj_files.push(obj_path);
    }

    let final_binary = target_dir.join(&args.output);
    print_if!(
        1,
        "Linking {} objects into {}",
        obj_files.len(),
        final_binary.display()
    );

    link_with_gcc(&obj_files, &final_binary);

    print_if!(0, "Compiled files into: {}", final_binary.display());
}

fn assemble(asm_path: &Path, obj_path: &Path) {
    let status = Command::new("nasm")
        .args([
            "-f",
            "elf64",
            asm_path.to_str().unwrap(),
            "-o",
            obj_path.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run nasm");
    assert!(status.success(), "nasm failed for {}", asm_path.display());
}

fn link_with_gcc(obj_files: &[PathBuf], out_path: &Path) {
    let mut args = vec!["-no-pie".to_string()];

    for obj in obj_files {
        args.push(obj.display().to_string());
    }

    args.push("-o".to_string());
    args.push(out_path.display().to_string());

    let status = Command::new("gcc")
        .args(&args)
        .status()
        .expect("failed to run gcc");
    assert!(status.success(), "gcc linking failed");
}

fn recursive_collect(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some(EXT) {
                files.push(p);
            } else if p.is_dir() {
                files.extend(recursive_collect(&p));
            }
        }
    }

    files
}
