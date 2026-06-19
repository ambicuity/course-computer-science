# Loop Optimization & Vectorization

> 90% of execution time is spent in loops. Optimize them.

**Type:** Learn
**Languages:** Rust, C
**Prerequisites:** Phase 08 lessons 01–14
**Time:** ~75 minutes

## Learning Objectives

- Explain loop unrolling, interchange, fusion/fission, and strength reduction.
- Analyze loop-carried data dependences to determine vectorizability.
- Implement loop optimization passes in Rust.
- Demonstrate auto-vectorization and manual SIMD intrinsics in C.

## Why This Matters

Profiling consistently shows that hot loops consume the majority of CPU cycles in real programs. The compiler's loop optimizer is where theoretical peak performance meets reality. Understanding these optimizations tells you why your matrix multiply is 10× faster with the right loop order, and why the compiler sometimes fails to vectorize.

## The Concept

### Loop Unrolling

Duplicate the loop body N times to reduce branch overhead and expose instruction-level parallelism:

```
Original:                Unrolled by 2:
  for i in 0..100:         for i in (0..100).step_by(2):
    a[i] = b[i] + c          a[i]   = b[i]   + c
                               a[i+1] = b[i+1] + c
```

Benefits: fewer branches, more independent instructions for the CPU to execute in parallel. Cost: larger code size (I-cache pressure). Compilers typically unroll by 2–8×.

### Loop Interchange

Swap the nesting order of two loops to improve cache locality:

```
Column-major (bad):        Row-major (good):
  for j in 0..N:             for i in 0..N:
    for i in 0..N:             for j in 0..N:
      sum += a[i][j]             sum += a[i][j]
```

If the inner dimension is contiguous in memory, row-major traversal hits cache lines efficiently. Interchange works when there are no loop-carried dependences that would be violated by swapping.

### Loop Fusion and Fission

**Fusion**: combine two adjacent loops with the same iteration space into one (reduces loop overhead):
```
Before:                After:
  for i: f(i)            for i:
    for i: g(i)            f(i)
                            g(i)
```

**Fission**: split one loop into two (improves cache behavior or enables other optimizations):
```
Before:                After:
  for i:                   for i:
    f(a[i])                  f(a[i])
    g(b[i])                for i:
                              g(b[i])
```

### Strength Reduction

Replace expensive operations with cheaper equivalents. The classic case is replacing multiplication with addition in loop induction variables:

```
Before:                    After:
  for i in 0..n:             t = &a[0]
    a[i] = i * 4 + base      for i in 0..n:
                               *t = base
                               t += 4
```

Array indexing `a[i]` internally computes `base + i * element_size`. Strength reduction turns this into a pointer increment.

### Vectorization (SIMD)

Modern CPUs have SIMD units that process 4–16 elements simultaneously (SSE: 128-bit, AVX2: 256-bit, AVX-512: 512-bit). **Auto-vectorization** transforms scalar loops into vector operations:

```
Scalar:                     Vectorized (4-wide):
  for i in 0..1000:           for i in (0..1000).step_by(4):
    a[i] = b[i] + c[i]         v_a = load(&b[i])   // 4 floats
                                 v_b = load(&c[i])
                                 v_r = vadd(v_a, v_b)
                                 store(&a[i], v_r)
```

### Loop-Carried Dependence

A loop can only be vectorized if iterations are **independent** — no iteration reads a value written by a previous iteration. The three dependence types:

| Dependence | Pattern | Vectorizable? |
|-----------|---------|--------------|
| Flow (RAW) | `a[i] = ...; ... = a[i-1]` | No |
| Anti (WAR) | `... = a[i]; a[i+1] = ...` | Usually yes (can reorder) |
| Output (WAW) | `a[i] = ...; a[i+1] = ...` | Usually yes |

### Compiler Vectorization

GCC and Clang auto-vectorize at `-O2`/`-O3`. They check:
1. Trip count is known at compile time.
2. No loop-carried dependences.
3. Alignment: data is properly aligned for SIMD loads/stores.
4. Cost model: vectorized version is actually faster.

You can check with `-fopt-info-vec` (GCC) or `-Rpass=loop-vectorize` (Clang).

## Build It

### Rust — Loop Optimization Passes

`code/main.rs` implements:
- `loop_unroll(body, factor)` — duplicate body N times.
- `strength_reduce(loop_info)` — induction variable analysis.
- `loop_interchange(outer, inner)` — swap nesting.
- `can_vectorize(loop)` — simple dependence check.

### C — Vectorization Demos

`code/main.c` provides:
- A vector addition loop annotated for auto-vectorization.
- Manual AVX2 intrinsics as a comparison.
- A loop unrolling benchmark.
- Compiler flag comments showing how to inspect vectorization.

Compile and inspect:

```bash
cd code && gcc -O2 -fopt-info-vec main.c -o vec_demo && ./vec_demo
# View vectorized assembly:
objdump -d vec_demo | grep -A5 vector_add
```

## Use It

LLVM's loop vectorizer lives in `lib/Transforms/Vectorize/LoopVectorize.cpp`. It:
1. Analyzes loop-carried dependences using SCEV (Scalar Evolution).
2. Builds a VPlan (vectorization plan) with different VF (vectorization factor) candidates.
3. Selects the best VF based on a cost model.
4. Emits vector IR with the chosen VF.

GCC's auto-vectorizer is in `gcc/tree-vect-loop.cc`. It uses the same logic with GCC's internal representation.

Both compilers support pragma hints: `#pragma GCC ivdep` (ignore vector dependences) and `#pragma clang loop vectorize(enable)`.

## Read the Source

- `llvm/lib/Transforms/Vectorize/LoopVectorize.cpp` — LLVM's loop vectorizer. Look at `LoopVectorizationCostModel::computeVectorizationFactor()`.
- `llvm/lib/Analysis/LoopAccessAnalysis.cpp` — Dependence analysis that determines if a loop is safe to vectorize.
- `gcc/tree-vect-loop.cc` — GCC's loop vectorization pass.

## Ship It

The Rust loop optimization passes and the C vectorization benchmarks form your toolbox for understanding and writing high-performance loops. The dependence analysis function is especially reusable: it catches vectorization blockers before you hand-optimize.

## Exercises

1. **Easy** — Write a C loop that the compiler refuses to vectorize because of a flow dependence (`a[i] = a[i-1] + 1`). Verify with `-fopt-info-vec-missed` (GCC) or `-Rpass-missed=loop-vectorize` (Clang).

2. **Medium** — Implement loop interchange in the Rust code. Given a nested loop over a 2D array, detect which order has better cache locality (stride-1 access in the inner loop) and swap if needed.

3. **Hard** — Write a SIMD vector addition using AVX2 intrinsics in C that processes 8 floats at a time. Compare the throughput against the auto-vectorized version using `clock()` benchmarks. Report the speedup ratio.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Unrolling | "Unroll the loop" | Duplicate the loop body N times to reduce branch overhead and expose ILP |
| Strength reduction | "Reduce strength" | Replace expensive operations (multiply) with cheaper ones (add) in induction variables |
| Vectorization | "Vectorize the loop" | Transform scalar operations into SIMD operations that process multiple elements per instruction |
| Loop-carried dependence | "Iteration i depends on i-1" | A data dependence between different iterations that prevents parallel/vector execution |
| SIMD | "Single instruction, multiple data" | CPU instructions that operate on vectors of data simultaneously (SSE, AVX, NEON) |

## Further Reading

- Kennedy, K. and Allen, J. "Optimizing Compilers for Modern Architectures." Ch. 3–7. — Deep treatment of loop transformations.
- Intel Intrinsics Guide: https://www.intel.com/content/www/us/en/docs/intrinsics-guide/ — Reference for SSE/AVX/AVX-512 intrinsics.
- GCC vectorization options: `gcc/doc/invoke.texi` — `-ftree-vectorize`, `-fopt-info-vec`.
