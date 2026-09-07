<img src="docs/logo.svg" alt="Alchemist logo" width="96">

# Alchemist: a compiler in Rust with a bundled bytecode VM

Alchemist is a from-scratch compiler pipeline for a small language. It does not stop at an interpreter. Source text goes through a hand-written lexer, a recursive-descent parser, and a code generator that lowers the AST into a compact stack bytecode, then a small virtual machine executes that bytecode. You can watch source become instructions, and then watch the instructions run.

## The language

Alchemist compiles a small but real language:

- Integer, boolean, and string values
- `let` bindings and assignment
- Arithmetic (`+ - * / %`), comparison (`== != < > <= >=`), and boolean (`&& || !`) operators with correct precedence
- `if` / `else`
- `while` loops
- Functions with parameters, `return`, and recursion
- A `print` builtin

```
fn fact(n) {
    if (n <= 1) {
        return 1;
    } else {
        return n * fact(n - 1);
    }
}

print(fact(5));
```

## The pipeline

1. **Lexer** turns source text into tokens, tracking line and column for every error.
2. **Parser** builds an AST with recursive descent and precedence climbing, guarded against deep nesting so a malformed file returns a parse error instead of overflowing the stack.
3. **Codegen** walks the AST once and lowers it into a stack bytecode: `PUSH`, arithmetic and comparison ops, `LOAD` / `STORE` for locals, `JUMP` / `JUMP_IF_FALSE`, `CALL` / `RET`, `PRINT`, `HALT`.
4. **VM** is a stack machine with call frames that executes the bytecode directly, with clean errors instead of panics for stack underflow and division by zero.

Compile errors are typed and reported before anything runs: undefined variable, arity mismatch, and parse errors. Runtime errors are caught by the VM: stack underflow and division by zero.

## Usage

```
# compile and run a program
alchemist run prog.al

# emit bytecode to a file
alchemist build prog.al -o prog.abc

# show the instructions inside a bytecode file
alchemist disasm prog.abc
```

## Building from source

```
cargo build --release
cargo test
```

Written from scratch in Rust with `clap` for the CLI. No parser-generator or compiler crate anywhere in the pipeline.

By Pavan Nallamothu.
