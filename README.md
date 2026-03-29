# mini_compiler

A minimal example compiler


## Quickstart

Clone this repository and run:

```bash
 cargo run --release -- examples/hello_world.txt
 ./examples/target/hello_world
```


## Usage

write your code into some file, then call

```cargo run --release -- <file>```

this will create `target/` in the parent of your file and fill it with `<file_name>.asm`, `<file_name>.o` and `<file_name>`.

To run the binary simply call `target/<file_name>`

## Syntax

Declare a variable with:

```<ident> = <expr>;```

Define a function with:

```
begin_def <ident> [ <ident> ( ',' <ident> )* ] ;
 <line>*
end_def
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


## Builtin functions

The current supported builtin functions are

- print
- print_str
- exit
- label
- goto
- sqrt

## Supported targets

Currently only x86_64 linux is supported.

All targets depend on gcc and nasm.
