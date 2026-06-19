# Lesson 01: The Compilation Pipeline — End to End

## Overview

Every program you write — from a "Hello, World" to the Linux kernel — passes through a series of transformations before a computer can execute it. These transformations form the **compilation pipeline**: a sequence of stages where each stage consumes one representation and produces another, progressively lowering human intent toward machine instructions.

This lesson maps the full pipeline. You will learn what each stage does, what it produces, and how real compilers like GCC, Clang, and Rust are organized.

---

## The Pipeline at a Glance

```
  ┌─────────────┐
  │ Source Code  │   hello.c / hello.rs
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │ Preprocessor │   #include, #define, #ifdef
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │    Lexer     │   Character stream → Token stream
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │    Parser    │   Token stream → AST
  └──────┬──────┘
         ▼
  ┌─────────────────┐
  │ Semantic Analysis │  Name binding, type checking
  └──────┬──────────┘
         ▼
  ┌─────────────┐
  │   IR Gen     │   AST → Intermediate Representation
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │ Optimization │   IR → optimized IR (multiple passes)
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │ Code Gen     │   IR → Assembly / Machine code
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │  Assembler   │   Assembly → Object file (.o)
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │   Linker     │   Object files → Executable
  └──────┬──────┘
         ▼
  ┌─────────────┐
  │  Executable  │   a.out / hello.exe
  └─────────────┘
```

Each box is a **stage**. The arrows are the **interfaces** — the representations passed between stages. A compiler is a pipeline of such transformations.

---

## Stage 1: Source Code

The input is text — source code written by a human. It lives in a file: `hello.c`, `main.rs`, `app.py`. The compiler reads this file as a stream of characters.

---

## Stage 2: Preprocessor

Languages like C and C++ have a **preprocessor** that runs before the compiler proper. It handles:

- **File inclusion** (`#include <stdio.h>`) — pastes header contents into the source.
- **Macro expansion** (`#define PI 3.14159`) — textual substitution.
- **Conditional compilation** (`#ifdef DEBUG`) — includes or excludes code blocks.

The output is a **translation unit** — a single expanded source file. Languages like Rust and Python do not have a preprocessor in this sense; they use module systems instead.

---

## Stage 3: Lexer (Scanner)

The **lexer** converts the character stream into a **token stream**. A token is a categorized chunk of text:

| Text | Token |
|------|-------|
| `int` | `KEYWORD_INT` |
| `main` | `IDENTIFIER("main")` |
| `(` | `LPAREN` |
| `42` | `INTEGER(42)` |
| `+` | `PLUS` |
| `"`hello`"` | `STRING("hello")` |

Whitespace and comments are typically discarded at this stage. The lexer attaches **source locations** (line, column) to each token for error reporting.

---

## Stage 4: Parser

The **parser** consumes the token stream and builds an **Abstract Syntax Tree (AST)** — a tree structure that represents the grammatical structure of the program.

For `int x = 3 + 4;`, the AST looks like:

```
VarDecl(type: "int", name: "x")
  └── BinExpr(op: "+")
        ├── Literal(3)
        └── Literal(4)
```

Parsers are built from a **grammar** — a formal description of the language's syntax. Common parser types include recursive descent (hand-written), LR, LALR, and PEG.

---

## Stage 5: Semantic Analysis

The AST is syntactically correct but may be semantically wrong. **Semantic analysis** catches issues the grammar cannot:

- **Name resolution** — does `x` refer to a declared variable?
- **Type checking** — is `3 + "hello"` valid?
- **Scope checking** — is `x` visible at this point?
- **Definite assignment** — is `x` used before it is assigned?

The compiler builds **symbol tables** to track declarations and scopes. Type information is attached to AST nodes.

---

## Stage 6: Intermediate Representation (IR) Generation

The AST is transformed into an **Intermediate Representation** — a lower-level form closer to machine code but still platform-independent. Common IR forms include:

- **Three-Address Code (TAC)** — `t1 = 3 + 4; x = t1;`
- **Static Single Assignment (SSA)** — each variable assigned exactly once: `x₁ = 3 + 4`
- **Control Flow Graphs (CFG)** — basic blocks connected by branches

**LLVM IR** is the most widely used IR today. It is a typed, SSA-based, RISC-like instruction set that serves as the universal middle representation for many compilers.

---

## Stage 7: Optimization

The optimizer transforms IR to produce faster, smaller, or more efficient code without changing observable behavior. Classic optimizations include:

- **Constant folding** — `3 + 4` → `7` at compile time.
- **Dead code elimination** — remove code that never executes.
- **Loop unrolling** — replicate loop body to reduce branch overhead.
- **Inlining** — replace function call with the function body.
- **Register allocation** — assign variables to CPU registers.

Optimization runs in multiple **passes** over the IR. Each pass is a separate transformation.

---

## Stage 8: Code Generation

The optimized IR is translated into **target-specific** assembly or machine code. This stage must handle:

- **Instruction selection** — map IR operations to CPU instructions.
- **Register allocation** — limited physical registers must be assigned.
- **Instruction scheduling** — reorder instructions to exploit pipeline parallelism.

This is where the compiler "speaks" a specific architecture: x86-64, ARM, RISC-V.

---

## Stage 9: Assembler

The **assembler** converts human-readable assembly (`.s` files) into **object files** (`.o` files) — binary encodings of machine instructions plus metadata (symbol tables, relocation records).

---

## Stage 10: Linker

The **linker** combines multiple object files and libraries into a single **executable**. It resolves **symbol references** — when `main.o` calls `printf`, the linker finds `printf` in `libc` and patches the call site.

Static linking copies library code into the executable. Dynamic linking defers resolution to load time via shared libraries (`.so` / `.dll`).

---

## Frontend vs Backend

The pipeline splits naturally into two halves:

```
  FRONTEND                          BACKEND
  ────────                          ───────
  Source → Lexer → Parser → AST    IR → Optimize → CodeGen → Assembly
  (language-specific)              (target-specific)
```

The **frontend** handles syntax and semantics of a source language. The **backend** handles code generation for a target architecture. The **IR** is the contract between them.

This separation is why Clang can compile C, C++, and Objective-C to x86, ARM, and RISC-V — one frontend per language, one backend per target, shared optimizer.

---

## LLVM IR: The Universal Middle

LLVM revolutionized compiler design by providing a high-quality, well-documented IR that many compilers target:

- **Clang** (C/C++) → LLVM IR → x86/ARM/RISC-V
- **Rust** (rustc) → LLVM IR → x86/ARM/RISC-V
- **Swift** → LLVM IR → x86/ARM
- **Julia** → LLVM IR → x86/ARM

A language author writes a frontend that emits LLVM IR and gets world-class optimization and code generation for free.

---

## Interpreters, Compilers, and JIT

Not all languages follow the full pipeline:

- **Interpreters** (CPython, Ruby MRI) — execute the AST or bytecode directly, line by line. No machine code generation.
- **Compilers** (GCC, rustc) — the full pipeline described above. Output is machine code.
- **JIT compilers** (V8, HotSpot, PyPy) — start interpreting, then compile hot paths to machine code at runtime. Best of both worlds.
- **Transpilers** (TypeScript, Babel) — translate source to source, then compile the output.

---

## Single-Pass vs Multi-Pass

A **single-pass compiler** reads source once and emits code immediately (early Pascal compilers). It is fast but limited — no optimization, restricted language features.

A **multi-pass compiler** processes the source multiple times: one pass for lexing, one for parsing, one for semantic analysis, etc. Modern compilers are multi-pass and spend most of their time in optimization.

---

## Incremental Compilation

Recompiling an entire project after changing one line is wasteful. **Incremental compilation** tracks dependencies between compilation units and only recompiles what changed.

Rust's `rustc` supports incremental compilation: it caches intermediate results and reuses them when inputs have not changed.

---

## Build It: Compilation Pipeline Overview

A minimal compilation pipeline has three components:

1. **Frontend** — lex, parse, analyze.
2. **Optimizer** — transform IR.
3. **Backend** — generate target code.

Design a pipeline for a toy language that adds two integers. Define the token types, the AST node types, and the IR instructions.

---

## Use It: Real Compiler Pipelines

### GCC

GCC's pipeline: `cpp` (preprocessor) → `cc1` (compiler: lex, parse, optimize, codegen) → `as` (assembler) → `ld` (linker). GCC uses its own internal IR (GIMPLE, RTL) and targets dozens of architectures.

### Clang / LLVM

Clang is the frontend. It emits LLVM IR. LLVM's optimizer runs dozens of passes. LLVM's backend generates assembly. `lld` is the linker. This modular design allows any language to plug into the LLVM ecosystem.

### Rust

`rustc` has a unique pipeline: Source → Tokens → AST → HIR (high-level IR) → MIR (mid-level IR, for borrow checking) → LLVM IR → machine code. The MIR stage is where Rust's ownership and borrowing rules are enforced.

---

## Ship It: Compilation Pipeline Reference Card

| Stage | Input | Output | Example Tool |
|-------|-------|--------|-------------|
| Preprocessor | Source | Expanded source | `cpp` |
| Lexer | Characters | Tokens | `flex`, `logos` |
| Parser | Tokens | AST | `bison`, hand-written |
| Semantic Analysis | AST | Typed AST | Hand-written |
| IR Generation | Typed AST | IR | LLVM API |
| Optimization | IR | Optimized IR | LLVM `opt` |
| Code Generation | IR | Assembly | LLVM `llc` |
| Assembler | Assembly | Object file | `as`, `nasm` |
| Linker | Object files | Executable | `ld`, `lld` |

---

## Exercises

### Level 1: Identify

List all ten stages of the compilation pipeline in order. For each stage, write one sentence describing its purpose.

### Level 2: Classify

For each of the following, state whether it is a single-pass compiler, multi-pass compiler, interpreter, JIT compiler, or transpiler:

1. CPython
2. GCC with `-O2`
3. TypeScript compiler (`tsc`)
4. V8 JavaScript engine
5. Early Turbo Pascal

Explain your reasoning for each.

### Level 3: Design

You are designing a compiler for a new language called Flint. The language has variables, functions, `if/else`, `while` loops, and integer arithmetic. Describe:

1. What token types your lexer will produce (list at least 10).
2. What AST node types your parser will produce (list at least 8).
3. What IR instructions your code generator will consume (list at least 6).
4. Whether you would use LLVM IR or design your own, and why.
