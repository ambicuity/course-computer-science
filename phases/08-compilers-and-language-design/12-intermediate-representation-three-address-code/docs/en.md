# Lesson 12: Intermediate Representation — Three-Address Code

## Overview

After type checking, the compiler has a valid AST. Before generating machine code, most compilers translate the AST into an **intermediate representation (IR)** — a simplified, flat instruction set that is easier to optimize and to target multiple hardware backends. This lesson builds an IR emitter that lowers AST expressions into **three-address code**.

---

## The Problem

A nested expression like `a * b + c * d` is natural for parsing but awkward for code generation. Machine instructions typically operate on registers holding one or two operands and producing one result. IR bridges this gap: it flattens nested expressions into a linear sequence of simple instructions, each touching at most three addresses (two sources, one destination).

---

## Why an IR?

| Reason | Explanation |
|--------|-------------|
| Machine independence | IR can be optimized without knowing the target CPU |
| Multiple backends | One IR, many code generators (x86, ARM, WASM) |
| Optimization | Simpler instructions make data-flow analysis tractable |
| Debugging | IR is human-readable; easier to inspect than AST or assembly |

Most real compilers have more than one IR (e.g., Clang: AST → LLVM IR → SelectionDAG → MachineInstr → MCInst).

---

## Three-Address Code

Each instruction has at most **three** addresses: two operands and one result.

**Instruction set:**

| Instruction | Format | Meaning |
|-------------|--------|---------|
| Assign | `x = y` | Copy value |
| Binary op | `t = x op y` | Arithmetic/logical operation |
| Unary op | `t = op x` | Negation, not |
| Label | `L:` | Jump target |
| Goto | `goto L` | Unconditional jump |
| If-goto | `if x goto L` | Conditional jump |
| Param | `param x` | Pass argument |
| Call | `t = call f, n` | Call function with n args |
| Return | `return x` | Return from function |
| Load | `t = load x` | Dereference pointer |
| Store | `store x, y` | Write y to address x |

### Quadruples

A common concrete representation is **quadruples** — a 4-tuple `(op, arg1, arg2, result)`:

```
(*, a, b, t1)      // t1 = a * b
(*, c, d, t2)      // t2 = c * d
(+, t1, t2, t3)    // t3 = t1 + t2
```

### Temporaries

Every intermediate result gets a fresh **temporary** name (`t1`, `t2`, …). The expression `a + b * c` becomes:

```
t1 = b * c
t2 = a + t1
```

---

## Lowering AST to IR

The translation walks the AST. For each expression, `gen_expr` returns the name of the temporary holding its result:

```
gen_expr(e):
    e is integer literal n  →  t = fresh_temp(); emit(t = n); return t
    e is variable x         →  return x           (no instruction needed)
    e is a + b              →  t1 = gen_expr(a)
                               t2 = gen_expr(b)
                               t3 = fresh_temp(); emit(t3 = t1 + t2); return t3
```

For statements:

```
gen_stmt(s):
    s is assign x = e       →  t = gen_expr(e); emit(x = t)
    s is return e           →  t = gen_expr(e); emit(return t)
    s is if(e) s1 else s2   →  t = gen_expr(e)
                               L_else = fresh_label(); L_end = fresh_label()
                               emit(if t == 0 goto L_else)
                               gen_stmt(s1); emit(goto L_end)
                               emit(L_else:); gen_stmt(s2)
                               emit(L_end:)
```

---

## Build It

See `code/main.rs` for a complete IR generator that handles:

- Integer literals, variables, binary expressions → flattened temporaries
- Assignments, if/else with labels and gotos
- Function calls with param instructions
- Pretty-printed IR output

---

## Use It

Production IRs:

- **LLVM IR** — SSA-based, typed, infinite virtual registers. Used by Clang, Rust, Swift, and many others. Files: `llvm/lib/IR/`, documentation: https://llvm.org/docs/LangRef.html
- **GCC GIMPLE** — three-address code with tuples. GCC's optimization passes operate on GIMPLE. Files: `gcc/gimple*.c`
- **WebAssembly** — stack-based IR designed as a portable compilation target
- **JVM bytecode** — stack-based IR used by Java, Kotlin, Scala

All of these share the core insight from this lesson: flatten nested structure into simple instructions, name every intermediate result, make data flow explicit.

### Read the Source

- `llvm/lib/IR/Instruction.cpp` — base class for all LLVM IR instructions
- `llvm/lib/IR/IRBuilder.h` — the API used to construct LLVM IR programmatically
- GCC `gcc/tree.h` and `gcc/gimple.h` — GIMPLE tuple definitions

---

## Ship It

The reusable artifact from this lesson is an IR library that provides:

- An instruction data type covering all three-address instruction forms
- A generator that walks an AST and emits a linear instruction list
- Temporary and label name generation
- Pretty-printing of the generated IR

---

## Exercises

### Level 1: While Loops

Extend the IR generator with `while (cond) body`. Generate the appropriate label, conditional jump, and goto for a loop.

### Level 2: Short-Circuit Evaluation

Add `&&` and `||` operators. Instead of emitting a single binary instruction, generate control flow that evaluates the left operand and only evaluates the right if necessary (short-circuit semantics).

### Level 3: Basic Blocks

Group the flat instruction list into **basic blocks** — maximal sequences of instructions with one entry point and one exit point. Build a control-flow graph (CFG) where nodes are basic blocks and edges represent jumps. This is the prerequisite for most data-flow optimizations.

---

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| IR | "The compiler's middle language" | Intermediate representation — a program representation between AST and machine code |
| Three-address code | "Each instruction touches 3 things" | An IR where each instruction has at most two operands and one result |
| Temporary | "A compiler-generated variable" | A fresh name for an intermediate computation result |
| Basic block | "A straight-line chunk" | A maximal sequence of instructions with single entry, single exit |
| CFG | "The flow graph" | Control-flow graph — nodes are basic blocks, edges are jumps |
| Quadruple | "A 4-tuple instruction" | (op, arg1, arg2, result) representation of a three-address instruction |

## Further Reading

- Aho, Lam, Sethi, Ullman — *Compilers: Principles, Techniques, and Tools*, Chapter 6: Intermediate Code Generation
- LLVM Language Reference: https://llvm.org/docs/LangRef.html
- Chris Lattner — "LLVM: An Infrastructure for Multi-Stage Optimization" (2002 MSc thesis)
