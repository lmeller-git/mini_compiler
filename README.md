# mini_compiler

A minimal example compiler


## Quickstart

Clone this repository and run:

```bash
 cargo run --release -- examples/the_answer.txt
 ./examples/target/the_answer
```


## Usage

write your code into some file, then call

```cargo run --release -- <file>```

this will create `target/` in the parent of your file and fill it with `<file_name>.asm`, `<file_name>.o` and `<file_name>`.

To run the binary simply call `target/<file_name>`

## Syntax

Declare a variable with:

```<ident> = <expr>;```

Call a function with:

```<func> <expr>;```

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

```(<var1> + <var2>) - (<var3> + <var4>);```


## Supported targets

Currently only x86_64 linux is supported.

All targets depend on gcc and nasm.
