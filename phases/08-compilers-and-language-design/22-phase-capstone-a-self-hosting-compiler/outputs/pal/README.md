# pal — a self-hosting compiler

A compiler for a small Pascal-like language, targeting RISC-V assembly.

## Features

- Hand-written lexer and recursive descent parser
- AST via algebraic data types (Rust enums)
- Type checker with scoped symbol tables
- Three-address IR
- Optimizations: constant folding + dead code elimination
- RISC-V (RV32I) code generator

## Build

```bash
cargo build --release
```

Requires Rust 1.70+.

## Run

```bash
# Compile a .pal file
./target/release/pal compile hello.pal

# Show all compilation stages
./target/release/pal --verbose hello.pal

# Output IR only (no codegen)
./target/release/pal --ir-only hello.pal
```

## Example Programs

- `hello.pal` — recursive fibonacci
- `factorial.pal` — iterative factorial
- `gcd_test.pal` — Euclidean GCD

## Language

Pal is a minimal Pascal-like language with:
- Types: `int`, `bool`
- Declarations: `var`, `function`
- Control flow: `if`/`then`/`else`, `while`/`do`, `begin`/`end`
- Functions return values via `funcname := expr`
- I/O: `print(expr)`

## Self-Hosting

The ultimate test: write a simplified pal compiler in pal itself.
See the Lesson 22 exercises for details.

## Architecture

```
.pal → [Lexer] → [Parser] → [TypeCheck] → [IR] → [Optimize] → [CodeGen] → .s
```

Each stage is a pure function operating on algebraic data types.
