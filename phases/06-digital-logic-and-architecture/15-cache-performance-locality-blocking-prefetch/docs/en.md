# Cache Performance — Locality, Blocking, Prefetch

A CPU can execute an instruction every cycle, but if the data isn't in cache, that instruction stalls for 100+ cycles while DRAM responds. Cache performance — not clock speed — is the dominant factor in real-world program throughput.

## The Two Types of Locality

The cache exploits two patterns in how programs access memory.

### Temporal Locality

If an address was recently accessed, it will likely be accessed again soon. Examples:

- **Loop variables** (`i`, `j`, `sum`) — reloaded every iteration
- **Stack frames** — same locals accessed repeatedly within a function call
- **Frequently called functions** — the instruction cache reuses them

The cache keeps recently accessed lines by LRU eviction. Temporal locality means those lines get used again before eviction.

### Spatial Locality

If address *A* was accessed, nearby address *A + offset* will likely be accessed soon. This is why caches fetch entire **cache lines** (typically 64 bytes), not individual bytes:

- **Array traversal** — `A[0], A[1], A[2], ...` each fetch brings in the next 16 integers
- **Struct fields** — accessing `node->data` often precedes `node->next`
- **Sequential code** — instructions execute in address order, so one fetch covers 16 instructions

## Matrix Multiplication: The Classic Cache Killer

Consider multiplying two 512 × 512 `double` matrices (8 bytes each):

```c
for (i = 0; i < N; i++)
    for (j = 0; j < N; j++)
        for (k = 0; k < N; k++)
            C[i][j] += A[i][k] * B[k][j];
```

C stores arrays in **row-major** order: `A[i][k]` increments by 8 bytes per `k` step (good spatial locality). But `B[k][j]` jumps by `N × 8 = 4096` bytes per `k` step — each access lands in a **different** cache line.

Result: ~90% of `B` accesses are cache misses. On a typical machine this means **10× slower** than the same computation with transposed `B`.

### Transpose Fix

```c
// Transpose B so that B[j][k] in memory matches access pattern
for (i = 0; i < N; i++)
    for (j = 0; j < N; j++)
        BT[j][i] = B[i][j];

for (i = 0; i < N; i++)
    for (j = 0; j < N; j++)
        for (k = 0; k < N; k++)
            C[i][j] += A[i][k] * BT[j][k];
```

Now all three array accesses (`A[i][k]`, `BT[j][k]`, `C[i][j]`) advance row-by-row. Cache miss rate drops from ~90% to ~6%.

## Blocking (Tiling)

Transpose wastes memory (need `BT`). Blocking gives the same benefit without extra storage: process the matrix in small square blocks that fit in cache.

```
Matrix 512 × 512,  block size 64
  ┌────────────────────┐
  │ B00  B01  B02  B03 │
  │ B10  B11  B12  B13 │    Each block: 64 × 64 doubles = 32 KB
  │ B20  B21  B22  B23 │    Three blocks in cache at once: 96 KB
  │ B30  B31  B32  B33 │    Fits in a 256 KB L2 cache
  └────────────────────┘
```

```c
for (ii = 0; ii < N; ii += B)
    for (jj = 0; jj < N; jj += B)
        for (kk = 0; kk < N; kk += B)
            for (i = ii; i < min(ii+B, N); i++)
                for (j = jj; j < min(jj+B, N); j++)
                    for (k = kk; k < min(kk+B, N); k++)
                        C[i][j] += A[i][k] * B[k][j];
```

**Choosing block size**: the working set (three blocks of A, B, C) must fit in L1 or L2 cache. Rule of thumb:

```
block_size = sqrt(cache_size / (3 * element_size))
```

For 32 KB L1 with 8-byte doubles: `sqrt(32768 / 24) ≈ 37`. Use 32 as the nearest power of 2.

## Prefetch

Modern hardware prefetchers detect **sequential** and **stride-based** access patterns and fetch upcoming cache lines before the CPU requests them. This hides DRAM latency for streaming workloads.

**Software prefetch** (`__builtin_prefetch` in GCC/Clang) can help when the access pattern is irregular:

```c
for (i = 0; i < N; i++) {
    __builtin_prefetch(&array[linked_list[i]->next], 0, 1);
    process(linked_list[i]);
}
```

Arguments: pointer, `rw` (0 = read, 1 = write), `locality` (0 = no temporal, 3 = high temporal).

Prefetching helps most when:
- Access pattern is predictable but not sequential (linked lists, tree traversal)
- DRAM latency is long relative to compute time
- The prefetch distance is tuned to hide the latency

## False Sharing

Two cores write to **different variables** that happen to reside in the **same cache line**. Each write invalidates the other core's cached copy, causing a ping-pong of coherence traffic even though there's no actual data dependency.

```
Core 0: counter_a  [ 8 bytes | 56 bytes padding ]  ← same 64-byte cache line
Core 1: counter_b  [ 8 bytes | 56 bytes padding ]  ← BROKEN: shares line
```

Fix: **pad** or **align** each variable to its own cache line.

```c
struct {
    int count;
    char pad[60];  // pad to 64 bytes (cache line size)
} counters[NUM_CORES] __attribute__((aligned(64)));
```

False sharing can reduce multi-threaded throughput by 5–10× with no correctness bug — just silent performance degradation.

## Build It

See `code/main.c` for matrix multiply benchmarks comparing naive, transposed, and blocked implementations with a timing harness. See `code/main.cpp` for cache line size detection, false sharing demonstration, and prefetch benchmarks.

### Key Parameters

| Parameter | Typical Value |
|-----------|--------------|
| L1 cache | 32–48 KB |
| L2 cache | 256 KB – 1 MB |
| L3 cache | 8–64 MB (shared) |
| Cache line | 64 bytes |
| DRAM latency | 80–120 cycles |

## Use It

BLAS (Basic Linear Algebra Subprograms) libraries use blocking extensively:

- **OpenBLAS** — hand-tuned assembly kernels for each block size, dispatches by CPU microarchitecture at runtime
- **Intel MKL** — auto-tunes block sizes per cache hierarchy, uses AVX-512 intrinsics for inner kernels
- **cuBLAS** — tiles into GPU shared memory (analogous to cache blocking)

The `dgemm` routine (`C = alpha * A * B + beta * C`) is the workhorse. Calling it instead of writing your own triple loop can yield 10–50× speedup because the library has decades of cache-tuning built in.

## Read the Source

- [OpenBLAS kernel directory](https://github.com/xianyi/OpenBLAS/tree/develop/kernel) — assembly kernels tiled by cache level
- [glibc `memcmp` implementation](https://sourceware.org/git/?p=glibc.git;a=blob;f=sysdeps/x86_64/memcmp.S) — uses cache-line-aware byte comparison

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained cache-aware matrix multiply you can reuse in later phases.**

## Exercises

1. **Easy** — Use the cache line detector in `code/main.cpp` to measure your machine's cache line size. Verify it matches the value in `/sys/devices/system/cpu/cpu0/cache/index0/coherency_line_size` (Linux) or `sysctl -n hw.cachelinesize` (macOS).

2. **Medium** — Optimize a 3D stencil computation (Jacobi iteration) with 3D blocking. The naive version iterates `for i, j, k: new[i][j][k] = 0.25 * (old[i-1][j][k] + old[i+1][j][k] + old[i][j-1][k] + old[i][j+1][k])`. Apply tiling to improve spatial locality across all three dimensions.

3. **Hard** — Write a program that demonstrates false sharing: two threads increment separate counters in the same cache line vs. padded to separate lines. Measure and report the throughput difference. Then fix the false sharing and verify the speedup.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Temporal locality | "recently used data is reused" | If address *A* is accessed at time *t*, it is likely accessed again near time *t + small Δ* |
| Spatial locality | "nearby data is accessed together" | If address *A* is accessed, address *A + small offset* is likely accessed soon |
| Blocking / tiling | "process in cache-sized chunks" | Restructure loops so the working set of a nested loop nest fits in L1/L2 cache |
| Prefetch | "fetch data before it's needed" | Issue a load into cache ahead of the actual use to hide DRAM latency |
| False sharing | "coherence traffic on unrelated data" | Two cores write to different variables in the same cache line, causing unnecessary invalidations |
| Stride | "step size between accesses" | The byte difference between consecutive memory accesses (stride-1 = sequential = best) |

## Further Reading

- Agner Fog, [Optimizing subroutines in assembly](https://agner.org/optimize/) — Chapter 8: Cache
- Ulrich Drepper, *What Every Programmer Should Know About Memory* — comprehensive treatment of memory hierarchy
- Intel, [Intel 64 and IA-32 Architectures Optimization Reference Manual](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html) — Chapter 2: Cache
