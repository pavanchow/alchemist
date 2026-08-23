# Alchemist design notes

This document describes the language grammar and the four stages of the compiler: lexer, parser, code generator, and virtual machine.

## Language grammar

Informal grammar, `*` for zero or more, `?` for optional.

```
program    := stmt*

stmt       := "let" IDENT "=" expr ";"
            | IDENT "=" expr ";"
            | "print" "(" expr ")" ";"
            | "if" "(" expr ")" block ("else" (block | stmt))?
            | "while" "(" expr ")" block
            | "return" expr? ";"
            | "fn" IDENT "(" params? ")" block
            | expr ";"

block      := "{" stmt* "}"
params     := IDENT ("," IDENT)*

expr       := or_expr
or_expr    := and_expr ("||" and_expr)*
and_expr   := equality ("&&" equality)*
equality   := comparison (("==" | "!=") comparison)*
comparison := term (("<" | ">" | "<=" | ">=") term)*
term       := factor (("+" | "-") factor)*
factor     := unary (("*" | "/" | "%") unary)*
unary      := ("-" | "!") unary | primary
primary    := INT | "true" | "false" | STRING | IDENT
            | IDENT "(" args? ")"
            | "(" expr ")"
args       := expr ("," expr)*
```

Precedence from loosest to tightest: `||`, `&&`, equality, comparison, additive, multiplicative, unary. Grouping with parentheses overrides all of it. `&&` and `||` evaluate both sides eagerly; there is no short circuit, which keeps the code generator and the VM simple since every binary operator has the same "evaluate left, evaluate right, apply" shape.

Functions can only be declared at the top level, not nested inside another function or a block. A function's own statements can call any other top-level function, including itself, since function signatures are collected in a pass over the top-level statements before any function body is compiled.

## Lexer

`src/lexer.rs` is a hand-written character scanner. It walks the source once, classifying runs of digits as integers, runs of letters and underscores as identifiers or keywords, and quoted text as strings with `\n`, `\t`, `\"`, and `\\` escapes. Two-character operators (`==`, `!=`, `<=`, `>=`, `&&`, `||`) are recognized by looking one character ahead before falling back to the single-character token. Every token carries its line and column so later errors can point at the right place. `//` starts a line comment.

## Parser

`src/parser.rs` is recursive descent with precedence climbing for expressions, one function per precedence level, from `or_expr` down to `primary`. Each level calls the next tighter level and then loops while it sees an operator at its own precedence, which naturally left-associates binary operators.

Every recursive expression function increments a depth counter on entry and decrements it on exit. If the counter passes a fixed limit the parser returns a parse error instead of recursing further, so a source file built entirely of nested parentheses cannot blow the native call stack. This is checked directly: a test file with five thousand nested parens is rejected with a clean error rather than crashing.

Statements are parsed with one function that dispatches on the leading keyword. An identifier followed by `=` is disambiguated from an identifier that starts an expression statement by looking one token ahead before committing.

## Code generator

`src/codegen.rs` walks the AST once and emits `Instr` values from `src/bytecode.rs`. Two passes make this work:

1. A pre-pass collects every top-level `fn` into a signature table (name, parameter count, and a stable function index) before compiling anything. This is what lets a function call another function defined later in the file, and lets a function call itself.
2. The main pass compiles the top-level statements first (this is the implicit "main" routine), followed by a `HALT`, then compiles each function body in turn, recording where its code starts.

Variables are resolved at compile time to numbered local slots, not looked up by name at runtime. Each function (including the implicit main routine) has its own flat slot space; `let` inside an `if` or `while` body still gets a fresh slot in the same space, since the VM does not need per-block frames, only per-function frames. Referencing a name that was never declared, or calling a function with the wrong number of arguments, is a typed `CompileError` raised before code generation finishes, with the offending line number attached.

`if` / `else` and `while` compile down to `JUMP_IF_FALSE` and `JUMP` with the target addresses patched in after the body they jump over has been emitted, since the target address is not known until then.

## Bytecode

A `Chunk` is a flat instruction stream plus a function table (name, arity, local count, start address) and the number of locals used by the top-level routine. The instruction set:

```
PUSH_INT <n>          push an integer literal
PUSH_BOOL <b>          push a boolean literal
PUSH_STR <s>           push a string literal
POP                    discard the top of the stack
ADD SUB MUL DIV MOD    binary arithmetic (ADD also concatenates two strings)
NEG                    unary negation
EQ NEQ LT GT LE GE     comparisons, push a bool
AND OR NOT             boolean operators
LOAD <slot>            push the current frame's local at slot
STORE <slot>           pop into the current frame's local at slot
JUMP <addr>            unconditional jump
JUMP_IF_FALSE <addr>   pop a bool, jump if false
CALL <func_idx> <argc> pop argc values, call the function at that index
RET                    return to the caller, return value stays on the stack
PRINT                  pop and print
HALT                   stop the VM
```

`alchemist build` writes a `Chunk` to disk as `.abc` in a small line-based text format (locals count, function table, then one line per instruction). `alchemist disasm` reads that format back and prints it, which is also exactly what a plain `alchemist build` followed by a look at the file gives you: there is no separate binary format to reverse engineer.

## Virtual machine

`src/vm.rs` is a stack machine. There is one shared operand stack for expression evaluation across the whole program, and a separate stack of call frames. Each frame holds a return address and a `Vec` of local values sized to that function's local count.

A function call pops its arguments off the shared operand stack, builds a new frame with those arguments as the first locals, pushes the frame, and jumps to the function's start address. The callee evaluates expressions on the same shared operand stack, which works because the callee only ever pushes and pops values above the point where it started: by the time it hits `RET` it has left exactly one value behind, its return value, in the same place a native stack machine would. `RET` pops the current frame and resumes at its return address; the return value is already sitting on top of the shared stack for the caller to use.

Every value-producing instruction that expects a particular type (arithmetic expects integers, `AND` and `OR` expect booleans) checks the type of what it pops and returns a `RuntimeError::TypeError` instead of panicking if it does not match. Popping from an empty stack returns `StackUnderflow` rather than panicking. Division and modulo by zero return `DivisionByZero` before the divide happens. None of these paths call `panic!`, `unwrap`, or index out of bounds on attacker-controlled input; they return a typed `Result` that the CLI prints and exits non-zero on.
