# mini_compiler

A minimal example compiler

## Syntax

declare a variable with:  
```<var> = <expr>;```

everything else are exprs

valid exprs use basic math operators and parentheses:

```(<var1> + <var2>) - (<var3> + <var4>);```

print stuff with:  
```print <expr>;```

## Usage

write your code into some file, then call

```cargo run --release -- <file>```

this will create target/ in the parent of <file> and fill it with <file_name>.asm, <file_name>.o and <file_name>.

To run the binary simply call target/<file_name>

## Supported targets

currently only x86_64 linux is supported.

All targets depend on gcc and nasm.
