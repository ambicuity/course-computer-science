# Lesson 22: Phase Capstone — A Self-Hosting Compiler

## Overview

This capstone integrates every concept from the compiler phase: lexing, parsing, AST construction, type checking, intermediate representation, optimization, and code generation. We build **pal** — a compiler for a small Pascal-like language that targets RISC-V assembly. The ultimate test: write pal's compiler in pal itself, then compile itself.

## Architecture

```
pal source (.pal)
    ↓
[Lexer]  → tokens
    ↓
[Parser] → AST
    ↓
[Type Checker] → annotated AST
    ↓
[IR Generator] → 3-address IR
    ↓
[Optimizer] → constant folding + DCE
    ↓
[Code Generator] → RISC-V assembly (.s)
    ↓
[Driver] → assemble + link → executable
```

Every stage is a pure function (or close to it). The AST and IR types are algebraic data types (enums in Rust), which the compiler can exhaustively pattern-match on.

## The Pal Language

Pal is a minimal Pascal-ish language:

```
program example;

function fib(n: int): int;
begin
  if n <= 1 then
    fib := n
  else
    fib := fib(n - 1) + fib(n - 2)
end;

begin
  print(fib(10))
end.
```

### Features

- **Types**: `int`, `bool`
- **Declarations**: `var x: int;`, `function f(a: int): int;`
- **Statements**: assignment (`:=`), `if`/`then`/`else`, `while`/`do`, `begin`/`end`, `print`
- **Expressions**: integer literals, booleans, identifiers, binary ops (`+`, `-`, `*`, `/`, `=`, `<`, `>`, `<=`, `>=`, `and`, `or`), function calls
- **Return**: function name as LHS (`fib := expression`) sets the return value

### Grammar

```
program    → 'program' IDENT ';' declarations compound '.'
declarations → ( var_decl | func_decl )*
var_decl   → 'var' IDENT ':' type ';'
func_decl  → 'function' IDENT '(' params ')' ':' type ';' declarations compound ';'
params     → ( IDENT ':' type ( ';' IDENT ':' type )* )?
type       → 'int' | 'bool'
compound   → 'begin' stmt_list 'end'
stmt_list  → stmt ( ';' stmt )*
stmt       → assign | if_stmt | while_stmt | print_stmt | compound
assign     → IDENT ':=' expr
if_stmt    → 'if' expr 'then' stmt ( 'else' stmt )?
while_stmt → 'while' expr 'do' stmt
print_stmt→ 'print' '(' expr ')'
expr       → comp ( ( 'and' | 'or' ) comp )*
comp       → arith ( ( '=' | '<' | '>' | '<=' | '>=' ) arith )?
arith      → term ( ( '+' | '-' ) term )*
term       → factor ( ( '*' | '/' ) factor )*
factor     → INTEGER | BOOLEAN | IDENT | IDENT '(' args ')' | '(' expr ')'
args       → expr ( ',' expr )*
```

## Implementation

### 1. Lexer (`lexer.rs`)

The lexer is a hand-written scanner that produces a stream of tokens. Each token carries its kind, a string slice into the source, and a source location (line, column).

Key design: the lexer does not allocate strings — tokens reference the source buffer via slices. This makes it zero-copy in the common case.

### 2. Parser (`parser.rs`)

A recursive descent parser that consumes tokens and builds an AST. Each grammar rule maps to a function. Error recovery is minimal — on error, we skip to the next semicolon or keyword.

The parser uses a simple `expect`/`peek`/`advance` interface over the token stream.

### 3. AST (`ast.rs`)

Algebraic data types representing the program structure:

```rust
enum Expr {
    IntLit(i64),
    BoolLit(bool),
    Var(String),
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    Call(String, Vec<Expr>),
}

enum Stmt {
    Assign(String, Expr),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
    Print(Expr),
    Block(Vec<Stmt>),
}
```

### 4. Type Checker (`typecheck.rs`)

Walks the AST, checking that:
- Variables are declared before use
- Function calls match parameter counts and types
- Binary operators receive compatible operands
- Assignments target declared variables or the enclosing function name

The type checker builds a symbol table (stack of scopes) and annotates nodes with resolved types.

### 5. IR (`ir.rs`)

Translates the AST into three-address code:

```
t0 = n
t1 = 1
t2 = t0 <= t1
if t2 goto L1
...
L1:
  fib = n
  ret fib
```

Instructions: `Assign`, `BinOp`, `Call`, `IfGoto`, `Goto`, `Label`, `Return`, `Print`.

### 6. Optimizer (`optimize.rs`)

Two passes:

**Constant folding**: if both operands of a binary op are literals, compute the result at compile time.

**Dead code elimination**: remove instructions whose results are never used. Remove unreachable code after unconditional jumps.

### 7. Code Generator (`codegen.rs`)

Maps IR instructions to RISC-V (RV32I) assembly:

| IR | RISC-V |
|----|--------|
| `x = n` | `li t0, n` |
| `z = x + y` | `add t2, t0, t1` |
| `z = x - y` | `sub t2, t0, t1` |
| `z = x * y` | `mul t2, t0, t1` |
| `if x goto L` | `bnez t0, L` |
| `call f` | `call f` |
| `ret x` | `mv a0, t0` / `ret` |

Functions use the standard RISC-V calling convention: arguments in `a0`–`a7`, return value in `a0`, callee-saved registers for locals.

### 8. Driver (`main.rs`)

Ties everything together:

```
pal compile source.pal → source.s → source.o → source (executable)
```

## Build It

The complete compiler is in `code/`. To build:

```bash
cd code/
cargo build --release
```

## Use It

```bash
# Compile and run a pal program
./target/release/pal compile hello.pal
./hello

# Show each compilation stage
./target/release/pal --verbose hello.pal

# Generate IR only (no codegen)
./target/release/pal --ir-only hello.pal
```

## The Self-Hosting Test

The real milestone: write a simplified version of pal's compiler **in pal itself**. The pal compiler (written in Rust) compiles the pal compiler (written in pal), producing an executable that can compile pal programs. See `outputs/pal/` for the project structure.

## Ship It

The `outputs/pal/` directory contains the pal project with:
- Example programs (fibonacci.pal, factorial.pal, primes.pal)
- Build and run instructions
- A self-hosting test harness

## Exercises

### Level 1 — Extend the Language
Add `for` loops to pal: `for i := 1 to 10 do stmt`. Update the lexer, parser, AST, type checker, IR generator, and code generator. Write a test program that computes the sum 1+2+...+100.

### Level 2 — Add Optimizations
Implement common subexpression elimination (CSE) in the optimizer. Track expressions that have been computed and reuse results. Benchmark the improvement on recursive fibonacci.

### Level 3 — Self-Hosting
Write a minimal compiler for a subset of pal *in pal itself*. It does not need to handle all features — just enough to compile a simple program. Then use the Rust-based compiler to build your pal-based compiler, and verify the pal-based compiler produces correct output.
