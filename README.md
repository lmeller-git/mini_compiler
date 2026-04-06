# mini_compiler

A minimal example compiler


## Quickstart

Clone this repository and run:

```bash
 cargo run --release -- examples/hello_world.lang
 ./target/a.out
```


## Usage

Write your code into some file, then call

```bash
cargo run --release -- <file>
```

this will create `target/` in your wd and fill it with `<file_name>.asm`, `<file_name>.o` and `a.out`.

To run the binary simply call `target/a.out`

For more options refer to

```bash
cargo run --release -- --help
```

The compiler accepts a list of files and directories. Passed assembly files will not be compiled, but directly assembled and included.
Passed object files will not be compiled or assembled, but included.
All children of directories with extension `ext` will be compiled.

Run tests easily with `cargo run --release -- <files> --test`

To call functions via FFI, use `c_call` or `c_call_arr` in `lib/std/ffi.asm`

## Syntax

A simple hello world with mini_compiler could look like this:

```
public begin_def main argc, argv;
 name = *argv;
 print_str "hello world from ";
 print_str name;
 print_str "\n";
end_def
```


Declare a variable with:

```<ident> = <expr>;```

All variables are qwords.

Define a function with:

```
begin_def <ident> [ <ident> ( ',' <ident> )* ] ;
 <line>*
end_def
```

Referance an external function with:

```
 extern_def <ident> [ <ident> ( ',' <ident> )* ] ;
```

Call a function with:

```<ident> [ <expr> ( ',' <expr> )* ];```

Execute some code conditionally with:

```if <expr>; <line>```

A line is any valid line of code ending in a semicolon. This inculdes if stmts:

```if <expr>; if <expr>; <line>```

To create a loop use builtin functions

```
label <ident>;
goto <ident>;
```

Everything else are exprs.

Valid exprs use basic math operators, parentheses, strlits and pointer derefs/refs:

```(<expr> + <expr>) - (<expr> + <expr>);```

```
ptr = &0;
print *ptr + 42;
```


Inline assembly may be written with

```
 asm "
 <asm here>
 ";
```

Linker attributes for functions may be defined with

```
 link_attr <attribute1>;
 link_attr <attribute2>;
 <function def / extern def>
```

Linker section tests is used for test functions, i.e. functions annotated with

```
 link_attr section tests;
```

will be run in a test run.

Condtional compilation of function declarations or lines can be achieved with

```
 cfg <expr>;
 <func/line>
```

Test runs will automatically inject --cfg test, thus functions annotated with `cfg test;` will only be compiled in test runs (unless explicilty added).

## Builtin functions

The current supported builtin functions are

- print
- print_str
- exit
- label
- goto
- addr_of

## Supported targets

Currently only x86_64 linux is supported.

All targets depend on gcc and nasm.
