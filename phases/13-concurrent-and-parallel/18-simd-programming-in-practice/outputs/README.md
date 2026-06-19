# SIMD Programming in Practice — Outputs

## Artifact

The reusable artifact is a set of SIMD-accelerated vector primitives:

- **Aligned allocation** (`posix_memalign` / Rust `Layout`)
- **Element-wise f32 add** — scalar, auto-vectorised, and AVX2 / SIMD
- **Dot product** — with horizontal reduction
- **4×4 matrix multiply** — via column-gather + broadcast-mul

These primitives are self-contained and reusable in later phases
(especially the Phase 13 capstone: work-stealing scheduler).

## Benchmarks

Compile and run the C++ version:

```bash
cd code
g++ -std=c++17 -mavx2 -O3 -fopenmp main.cpp -o simd_bench
./simd_bench
```

Compile and run the Rust version (requires nightly):

```bash
cd code
rustup default nightly
cargo build --release
cargo run --release
```

### Expected Output (representative)

Output is printed to stdout.  Typical results on a Haswell-or-later
x86-64 CPU with AVX2 at ~3 GHz:

```
--- Element-wise Add ---
  scalar   : 45000 us
  autovec  : 12000 us  (3.8x)
  avx2     : 11000 us  (4.1x)

--- Dot Product ---
  scalar   : 56000 us
  autovec  : 14000 us  (4.0x)
  avx2     : 13000 us  (4.3x)

--- 4x4 Matrix Multiply ---
  scalar   : 0.050 us
  avx2     : 0.015 us  (3.3x)
```

**Important:** Actual numbers depend on CPU model, memory bandwidth,
and whether data fits in cache.  The add/dot-product benchmarks operate
on 32 MiB of data (exceeds L2 cache on most CPUs), so they are
memory-bound.  Speedups for cache-resident data will be closer to 6–8×.

## Files

| File              | Purpose                                           |
|-------------------|---------------------------------------------------|
| `simd_bench`      | Compiled C++ binary (after `make`)                |
| `(target/release/simd_bench_rs)` | Compiled Rust binary (after `cargo build`) |

## Key Metric

The ratio `scalar_time / simd_time` measures how much of the
**theoretical 8× SIMD peak** you achieve.  A ratio < 8× usually
indicates memory bandwidth or reduction overhead is the bottleneck.
