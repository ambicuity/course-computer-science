# Cache-Aware Algorithm Design

> The gap between "correct on paper" and "fast in practice" is measured in cache misses.

**Type:** Learn
**Languages:** C++, Rust
**Prerequisites:** Phase 15 lessons 01–04
**Time:** ~75 minutes

## Learning Objectives

- Explain the CPU cache hierarchy and quantify the cost of cache misses vs hits.
- Implement loop tiling (blocking) for matrix multiply and measure the speedup.
- Implement a cache-oblivious recursive matrix multiply that auto-tiles.
- Choose between AoS and SoA layouts based on access patterns.
- Recognize why linked lists are catastrophically slow vs arrays for traversal.
- Avoid false sharing in multi-threaded code (preview; full treatment in L06).

## The Problem

You wrote an O(n³) matrix multiply. Your colleague wrote the same O(n³) matrix multiply. Yours runs 10× slower. Same algorithm. Same complexity. Same machine.

The difference? Your code strides through memory in the worst possible order for the cache. Her code respects locality. The CPU spends most of its time stalled waiting for data from DRAM, not doing math.

This lesson teaches you how to design algorithms and data structures that work *with* the cache hierarchy instead of against it. Without this knowledge, profiling will show mysterious stalls you can't explain and "optimizations" that make things slower.

## The Concept

### The Cache Hierarchy

Modern CPUs have a hierarchy of caches, each smaller and faster than the next:

| Level | Typical Size | Latency (cycles) | Latency (ns @ 3 GHz) |
|-------|-------------|-------------------|----------------------|
| L1    | 32 KB       | 4                 | ~1.3                 |
| L2    | 256 KB      | 12                | ~4                   |
| L3    | 6–32 MB     | 40                | ~13                  |
| DRAM  | GBs         | 200+              | ~67                  |

A cache miss that falls through to DRAM costs ~50× an L1 hit. An algorithm that fits its working set in L1 can be 50× faster than one that spills to DRAM on every access.

### Cache Lines

Caches don't move individual bytes. They move **cache lines** — 64-byte blocks. When you read address 0x100, the CPU loads bytes 0x100–0x13F into a cache line. The next read from 0x108 is essentially free.

This is **spatial locality**: accessing nearby addresses soon after each other benefits from the cache line already being loaded. This is why sequential array traversal is fast and random pointer chasing is slow.

### Temporal vs Spatial Locality

- **Temporal locality**: If you accessed it recently, you'll likely access it again soon. Re-use data while it's hot in cache.
- **Spatial locality**: If you accessed address X, you'll likely access X+1, X+2, etc. Organize data so nearby accesses are nearby in memory.

Good cache-aware design exploits both. Bad code violates both.

### The Matrix Multiply Problem

Consider C = A × B, where all three are N×N matrices stored in row-major order.

Naive triple loop:

```
for i in 0..N:
    for j in 0..N:
        for k in 0..N:
            C[i][j] += A[i][k] * B[k][j]
```

The inner loop reads A[i][k] sequentially (good spatial locality in A's row). But B[k][j] strides across B's rows — each access touches a different cache line. With N=1024 and 8-byte doubles, each B row is 8 KB. L1 is only 32 KB. Every B access evicts a useful line.

This is the canonical cache disaster.

### Loop Tiling (Blocking)

The fix: break the matrices into small **tiles** that fit in cache. Instead of processing entire rows, process small blocks:

```
for i in 0..N step TILE:
    for j in 0..N step TILE:
        for k in 0..N step TILE:
            for ii in i..min(i+TILE, N):
                for jj in j..min(j+TILE, N):
                    for kk in k..min(k+TILE, N):
                        C[ii][jj] += A[ii][kk] * B[kk][jj]
```

A tile of size TILE×TILE holds TILE² doubles = 8·TILE² bytes. For TILE=32, that's 8 KB per tile — three tiles (A-tiling, B-tiling, C-tiling) need ~24 KB, which fits comfortably in L1.

Result: 10–30× speedup for large matrices. Same O(n³). Same arithmetic. Just better memory access.

### Cache-Oblivious Algorithms

Tiled code is **cache-aware**: you hardcode the tile size for a specific cache. Change the machine, re-tune.

A **cache-oblivious** algorithm achieves near-optimal cache performance *without* knowing cache parameters. The idea: recursively divide the problem until sub-problems fit in cache, whatever size that cache is.

For matrix multiply, recursively split each N×N multiply into four (N/2)×(N/2) multiplies:

```
mat_mul(A, B, C, n):
    if n <= BASE:
        naive_multiply(A, B, C, n)   // base case fits in cache
        return
    // Split A, B, C into quadrants
    C11 = A11*B11 + A12*B21
    C12 = A11*B12 + A12*B22
    C21 = A21*B11 + A22*B21
    C22 = A21*B12 + A22*B22
```

At the base case (e.g., n=32 or n=64), the sub-matrices fit in.cache. The recursion naturally adapts to any cache size — no tuning required.

### AoS vs SoA

Consider an array of particles:

```cpp
struct Particle { double x, y, z, vx, vy, vz, mass, charge; };  // 56 bytes
Particle particles[N];
```

This is **Array of Structures (AoS)**. If you compute `particles[i].x` for all i, you load a full 56-byte struct per cache line but use only 8 bytes. 85% of each cache line is wasted.

**Structure of Arrays (SoA)** layout:

```cpp
struct Particles {
    double x[N], y[N], z[N], vx[N], vy[N], vz[N], mass[N], charge[N];
};
```

Now `x[0]` and `x[1]` are adjacent. A cache line holds 8 x-values. Spatial locality is maximized.

Rule of thumb: if your hot loop touches 1–2 fields per element, SoA wins. If it touches most fields, AoS is fine.

### Pointer Chasing vs Sequential Access

Traversing a linked list:

```cpp
for (auto* p = head; p; p = p->next) { sum += p->value; }
```

Each `p->next` reads a pointer that could be anywhere in memory. Every iteration is a potential cache miss. With 200-cycle DRAM latency, traversing 1M nodes takes ~200ms just on stalls.

Traversing an array:

```cpp
for (int i = 0; i < N; i++) { sum += arr[i].value; }
```

The hardware prefetcher detects the sequential pattern and loads cache lines ahead. Latency is hidden. 1M elements in ~1ms.

This is why B-trees beat BSTs for databases: B-tree nodes hold hundreds of keys in one cache-line-friendly block, while BST nodes are scattered individual allocations.

### Cache-Friendly Patterns

1. **Sequential access** — Always prefer. Hardware prefetchers love it.
2. **Avoid power-of-two strides** — Stride 256 on a 4-way set-associative cache hits only 4 sets. All other sets sit idle. This is called *cache conflict*. Use prime strides or pad arrays.
3. **Working set fits in cache** — If your hot data fits in L1 (32 KB) or L2 (256 KB), you win. Profile and resize.
4. **Prefetching** — Modern CPUs prefetch sequential patterns automatically. For irregular access, `__builtin_prefetch()` gives a hint, but use sparingly.
5. **Hot/cold splitting** — Split structs so frequently accessed ("hot") fields are together, rarely accessed ("cold") fields are elsewhere.

### False Sharing (Preview)

When two threads write to different fields on the **same cache line**, each write invalidates the other's copy. The line bounces between cores at DRAM latency. This is *false sharing* — the threads don't share data, but they share a cache line.

The full treatment is in Lesson 06 (False Sharing & Memory Alignment). For now: pad hot struct fields to 64-byte boundaries so independent writes land on independent lines.

### Why Quicksort is Cache-Friendly

Quicksort's partition step scans an array linearly from both ends — perfect sequential access. Mergesort, while O(n log n) like quicksort, needs a temporary buffer and accesses it in a less predictable pattern. For realistic data sizes, quicksort's cache behavior often makes it faster despite worse worst-case complexity.

### Why B-Trees Beat BSTs

A BST node holds one key and two child pointers (~24 bytes). Traversing a BST with 10M keys requires ~23 levels, each a likely cache miss on a different node. Total: ~4,600 cycles just in stalls.

A B-tree node holds hundreds of keys in a contiguous block. Traversing a B-tree with 10M keys requires ~3 levels, each loading one cache-line-friendly block. Total: ~120 cycles of cache latency. 40× faster for lookups, and this is why every database uses B-trees.

## Build It

### Step 1: Naive Matrix Multiply

The baseline — O(n³) with terrible cache behavior:

```cpp
void mat_mul_naive(const double* A, const double* B, double* C, int N) {
    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++) {
            double sum = 0;
            for (int k = 0; k < N; k++)
                sum += A[i * N + k] * B[k * N + j];
            C[i * N + j] = sum;
        }
}
```

### Step 2: Tiled Matrix Multiply

The same O(n³), but cache-friendly:

```cpp
void mat_mul_tiled(const double* A, const double* B, double* C, int N, int TILE) {
    for (int i = 0; i < N; i += TILE)
        for (int j = 0; j < N; j += TILE)
            for (int k = 0; k < N; k += TILE)
                for (int ii = i; ii < std::min(i + TILE, N); ii++)
                    for (int jj = j; jj < std::min(j + TILE, N); jj++) {
                        double sum = C[ii * N + jj];
                        for (int kk = k; kk < std::min(k + TILE, N); kk++)
                            sum += A[ii * N + kk] * B[kk * N + jj];
                        C[ii * N + jj] = sum;
                    }
}
```

### Step 3: Cache-Oblivious Matrix Multiply

Recursive divide-and-conquer, no explicit tile size:

```cpp
void mat_mul_recursive(const double* A, const double* B, double* C,
                       int N, int ldA, int ldB, int ldC) {
    if (N <= 64) {  // base case: fits in L1
        for (int i = 0; i < N; i++)
            for (int j = 0; j < N; j++) {
                double sum = C[i * ldC + j];
                for (int k = 0; k < N; k++)
                    sum += A[i * ldA + k] * B[k * ldB + j];
                C[i * ldC + j] = sum;
            }
        return;
    }
    int h = N / 2;
    // C11 += A11*B11 + A12*B21  (and similar for other quadrants)
    mat_mul_recursive(A, B, C, h, ldA, ldB, ldC);
    mat_mul_recursive(A + h, B + h * ldB, C, h, ldA, ldB, ldC);
    mat_mul_recursive(A, B + h, C + h, h, ldA, ldB, ldC);
    mat_mul_recursive(A + h, B + h * ldB + h, C + h, h, ldA, ldB, ldC);
    mat_mul_recursive(A + h * ldA, B, C + h * ldC, h, ldA, ldB, ldC);
    mat_mul_recursive(A + h * ldA + h, B + h * ldB, C + h * ldC, h, ldA, ldB, ldC);
    mat_mul_recursive(A + h * ldA, B + h, C + h * ldC + h, h, ldA, ldB, ldC);
    mat_mul_recursive(A + h * ldA + h, B + h * ldB + h, C + h * ldC + h, h, ldA, ldB, ldC);
}
```

## Use It

### Production Matrix Libraries

- **BLAS** (`dgemm`): The gold standard. Uses architecture-specific tile sizes, SIMD, and multi-threading. Your tiled code might be 5–10× slower than OpenBLAS or Intel MKL, which tune for specific CPUs.
- **Eigen** (C++): Expression templates that generate tiled code at compile time. See `Eigen/src/Core/products/GeneralMatrixMatrix.h`.
- **NumPy**: Calls BLAS under the hood. `numpy.dot` is already tiled.

### How BLAS Does It Better

1. **Micro-kernels**: Hand-written assembly for inner loops (e.g., 4×8 AVX-512 kernel).
2. **Packing**: Copies tiles into contiguous buffers aligned to cache boundaries.
3. **Auto-tuning**: ATLAS empirically finds optimal tile sizes. BLIS uses analytical models.

Your tiled version captures 80% of the benefit. BLAS gets the last 20% with machine-specific tricks.

## Read the Source

- **OpenBLAS**: `kernel/x86_64/dgemm_kernel_4x8_haswell.c` — the micro-kernel that processes a 4×8 tile using AVX2 instructions.
- **BLIS**: `frame/3/gemm/haswell/1/bli_gemm_haswell_ref.c` — reference implementation of the register-level kernel.
- **glibc malloc**: `malloc/malloc.c` — look at how `ptmalloc2` manages small vs large allocations and how free lists are organized (linked lists that cause pointer chasing!)

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. It is:

- **Cache Reference Card** (`cache_reference.md`) — A one-page reference with cache hierarchy numbers, tiling algorithm, AoS vs SoA decision guide, and common cache-friendly patterns. Keep it next to your desk.

## Exercises

1. **Easy** — Run the benchmarks. Confirm that tiled beats naive by 5–10× for N≥512. Measure the crossover point where tiling starts to matter.
2. **Medium** — Implement SoA particle layout in Rust. Benchmark a gravity computation (accessing x, y, z fields only) with AoS vs SoA. Show a 3–5× speedup.
3. **Hard** — Implement a Morton-order (Z-order) matrix traversal. Compare its cache behavior against row-major and column-major for matrix transpose. Measure L1 cache misses using `perf stat -e L1-dcache-load-misses`.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Cache line | "The cache moves data in 64-byte chunks" | The minimum unit of transfer between cache levels. You always load 64 bytes even if you read 1 byte. |
| Tiling / Blocking | "Process data in tiles" | Restructure loops so the inner working set fits in a cache level, reusing data before it's evicted. |
| Cache-oblivious | "No tile size to tune" | An algorithm that recursively divides until sub-problems fit in cache, without needing to know cache parameters. |
| AoS vs SoA | "Array of structs vs struct of arrays" | Memory layout choice: interleaved fields (AoS) vs grouped fields (SoA). SoA wins when hot loops touch few fields. |
| False sharing | "Threads fighting over a cache line" | Two cores writing different variables on the same 64-byte line, causing it to bounce between caches at DRAM latency. |
| Spatial locality | "Nearby addresses are accessed together" | If you touch address X, you'll probably touch X+8 soon. Cache lines exploit this by loading 64 continuous bytes. |
| Temporal locality | "Recently used data is reused soon" | If you touch address X now, you'll likely touch it again before it's evicted. Tiling keeps data hot. |
| Working set | "Hot data that fits in cache" | The subset of data accessed in the tight inner loop. If it fits in L1, you win. |

## Further Reading

- *What Every Programmer Should Know About Memory* — Ulrich Drepper (2007). The definitive guide. Still relevant.
- *Cache-Oblivious Algorithms* — Frigo, Leiserson, Prokop, Ramachandran (1999). The original paper.
- *The Anatomy of a High-Performance Matrix Multiply* — Goto, van de Geijn (2008). How BLAS actually works.
- *Is Parallel Programming Hard?* — Paul McKenney (2021). Chapter 5 covers cache effects in detail.
- Matt Godbolt's *Compiler Explorer* (godbolt.org) — Compile the benchmark code and inspect the assembly. See how the compiler generates SIMD for the inner loop.