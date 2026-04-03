# Statically Allocated Language Examples

This directory contains a suite of example programs. These examples demonstrate the core features of the compiler and std library.

## Running the Examples

All examples assume the cwd to be mini_compiler.

---

### Hello World (`hello_world.lang`)
basic string literals and standard output.

```bash
cargo run --release -- examples/hello_world.lang
./target/a.out
```

### The Answer (`the_answer.lang`)
basic expr evaluation.

```bash
cargo run --release -- examples/the_answer.lang
./target/a.out
```

### Loops (`loop.lang`)
basic control flow

```bash
cargo run --release -- examples/loop.lang
./target/a.out
```

### Bitwise Operations (`bit_wise.lang`)
basic bitwise operation

```bash
cargo run --release -- examples/bit_wise.lang
./target/a.out
```

### Prime Number (`prime.lang`)
small cli script that tells you wether a number is prime or not

```bash
cargo run --release -- examples/prime.lang lib/std/utils.lang
./target/a.out
```

### Pointers and Memory (`ptrs.lang`)
basic pointer operations

```bash
cargo run --release -- examples/ptrs.lang
./target/a.out
```

### Static linking (`statically`)
basic static linking of multiple files

```bash
cargo run --release --examples/statically
./target/a.out
```

### Malloc and C FFI (`malloc.lang`)
ffi calls into glibc + heap usage

```bash
cargo run --release -- examples/malloc.lang lib/std/array.lang lib/std/utils.lang lib/std/ffi.asm
./target/a.out
```

### Vector (`vec.lang`)
basic vector usage

```bash
cargo run --release -- lib/std/collections/vec.lang lib/std/mem.lang lib/std/utils.lang examples/vec.lang lib/std/ffi.asm
./target/a.out
```

### String (`string.lang`)
basic dynamic strings

```bash
cargo run --release -- examples/string.lang lib
./target/a.out
```

### Linked List (`linked_list.lang`)
basic linked_list usage

```bash
cargo run --release -- examples/linked_list.lang lib
./target/a.out
```

### Test (`test.lang`)
defining and running tests

```bash
cargo run --release -- examples/test.lang lib --test
./target/a.out
```

Of course this may also be run as a normal executable:

```bash
cargo run --release -- examples/test.lang lib
./target/a.out
```
