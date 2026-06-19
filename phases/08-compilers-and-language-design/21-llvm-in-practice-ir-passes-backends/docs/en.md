# Lesson 21: LLVM in Practice — IR, Passes, Backends

## Overview

LLVM is a modular compiler infrastructure used by Clang, Rust, Swift, and dozens of other languages. It provides a clean separation between frontend parsing, mid-level optimization, and backend code generation — all connected through a well-defined intermediate representation (LLVM IR). This lesson covers writing IR by hand, building optimization passes, and understanding how backends turn IR into machine code.

## LLVM Architecture

The compiler pipeline has three stages:

```
Source Code → [Frontend] → LLVM IR → [Passes] → LLVM IR → [Backend] → Machine Code
               (Clang)                (opt)                   (llc)
```

- **Frontend**: parses source, performs language-specific analysis, emits LLVM IR.
- **Optimization passes**: transform IR to improve performance without changing semantics.
- **Backend**: selects instructions, allocates registers, emits assembly or object code.

Each stage communicates only through LLVM IR, so you can swap frontends or backends independently.

## LLVM IR Basics

LLVM IR is a typed, SSA-based (static single assignment), three-address instruction set. Every value has a type, and every register is assigned exactly once.

### Types

| Type | Meaning |
|------|---------|
| `i1` | 1-bit boolean |
| `i32` | 32-bit integer |
| `i64` | 64-bit integer |
| `float` | 32-bit IEEE 754 float |
| `double` | 64-bit IEEE 754 float |
| `ptr` | pointer (opaque, typed in older LLVM) |
| `[N x i32]` | array of N i32s |
| `{ i32, float }` | struct |

### Core Instructions

**Memory**: `alloca` (stack), `load`, `store`

**Arithmetic**: `add`, `sub`, `mul`, `sdiv`, `srem`, `fadd`, `fsub`

**Comparison**: `icmp eq/ne/slt/sle/sgt/sge`, `fcmp`

**Control flow**: `br label`, `br i1 %cond, label %then, label %else`, `switch`, `ret`

**Other**: `phi` (SSA merge), `call`, `getelementptr`, `bitcast`, `sext`, `trunc`

### Example: Fibonacci in LLVM IR

```llvm
define i32 @fib(i32 %n) {
entry:
  %cmp = icmp sle i32 %n, 1
  br i1 %cmp, label %base, label %recur

base:
  ret i32 %n

recur:
  %n1 = sub i32 %n, 1
  %n2 = sub i32 %n, 2
  %r1 = call i32 @fib(i32 %n1)
  %r2 = call i32 @fib(i32 %n2)
  %result = add i32 %r1, %r2
  ret i32 %result
}
```

### SSA and Phi Nodes

When control flow merges, you need `phi` to select the right value:

```llvm
entry:
  %a = add i32 1, 2
  br i1 %cond, label %left, label %right

left:
  %b = add i32 %a, 10
  br label %merge

right:
  %c = mul i32 %a, 3
  br label %merge

merge:
  %result = phi i32 [ %b, %left ], [ %c, %right ]
  ret i32 %result
```

## Generating IR with Clang

The easiest way to see LLVM IR from C/C++:

```bash
# Generate LLVM IR text
clang -S -emit-llvm -O0 -o fib.ll fib.c

# Generate IR with optimizations
clang -S -emit-llvm -O2 -o fib_opt.ll fib.c

# Run specific optimization passes
opt -S -passes=mem2reg,simplifycfg fib.ll -o fib_opt.ll
```

The `-O0` IR is verbose (lots of `alloca`/`load`/`store`). The `-O2` IR is in SSA form with optimizations applied.

## Optimization Passes

Passes are organized by scope:

| Scope | Example Passes | Operates On |
|-------|---------------|-------------|
| **Function** | `mem2reg`, `instcombine`, `simplifycfg`, `dce` | One function at a time |
| **Module** | `globalopt`, `strip` | Entire module |
| **Loop** | `loop-unroll`, `loop-vectorize`, `licm` | Individual loops |

### Key Passes Explained

**`mem2reg`**: promotes `alloca`/`load`/`store` to SSA registers. Essential first pass.

**`instcombine`**: peephole optimization — replaces instruction patterns with cheaper equivalents.

**`simplifycfg`**: removes dead blocks, simplifies branches, merges identical blocks.

**`dce`** (dead code elimination): removes instructions whose results are never used.

**`inline`**: inlines small functions, exposing optimization opportunities.

### Writing a Custom Pass

LLVM passes operate on the IR. A function pass visits each function and transforms it:

```cpp
#include "llvm/IR/PassManager.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/Instructions.h"

namespace llvm {
  PreservedAnalyses InstCounterPass::run(Function &F,
                                          FunctionAnalysisManager &AM) {
    int count = 0;
    for (BasicBlock &BB : F) {
      count += std::distance(BB.begin(), BB.end());
    }
    llvm::errs() << "Function " << F.getName()
                 << " has " << count << " instructions\n";
    return PreservedAnalyses::all();
  }
}
```

## Backends: From IR to Machine Code

The backend handles:

1. **Instruction selection**: maps IR operations to target instructions (uses TableGen definitions).
2. **Scheduling**: orders instructions to minimize stalls.
3. **Register allocation**: assigns virtual registers to physical registers.
4. **Emission**: produces assembly or object code.

```bash
# Generate RISC-V assembly from IR
llc -mtriple=riscv64 -O2 fib.ll -o fib.s

# Generate x86-64 assembly
llc -mtriple=x86_64 -O2 fib.ll -o fib_x86.s

# Generate object file directly
llc -filetype=obj -O2 fib.ll -o fib.o
```

## Build It

The `code/main.cpp` demonstrates:
1. Hand-written LLVM IR for fibonacci, factorial, and GCD.
2. A shell script to compile and run passes.
3. A custom pass that counts instructions.

## Use It

- **Clang** (C/C++), **Rust**, **Swift**, **Kotlin/Native**, **Julia** all use LLVM as their backend.
- **MLIR** (Multi-Level IR) extends LLVM for domain-specific compilers.
- **Emscripten** compiles LLVM IR to WebAssembly.

## Ship It

See `code/` for complete examples you can compile and run with LLVM tools.

## Exercises

### Level 1 — Parse and Inspect
Write a C function (e.g., `int square(int x) { return x * x; }`). Compile it with `clang -S -emit-llvm -O0`. Identify the `alloca`, `load`, `store`, `mul`, and `ret` instructions. Then compile with `-O2` and compare.

### Level 2 — Pass Pipeline
Take the IR from Level 1. Run `opt -passes=mem2reg` on it, then `opt -passes=instcombine`. Show the IR after each step and explain the transformations.

### Level 3 — Custom Counter Pass
Write a function pass that counts how many times each instruction opcode appears in a function. Build it against LLVM, load it with `opt -load-pass-plugin`, and run it on a test program.
