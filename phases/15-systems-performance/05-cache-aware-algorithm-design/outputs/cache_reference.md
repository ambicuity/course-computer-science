# Cache Reference Card

Quick reference for cache-aware algorithm design. Keep this next to your desk.

## Cache Hierarchy

| Level | Size | Latency | Notes |
|-------|------|---------|-------|
| L1 Data | 32 KB | 4 cycles | Per-core, fastest |
| L2 | 256 KB | 12 cycles | Per-core |
| L3 | 6–32 MB | 40 cycles | Shared across cores |
| DRAM | GBs | 200+ cycles | Main memory |

**Cache line size: 64 bytes** (512 bits). This is the minimum unit of transfer.

## Key Ratios

- DRAM access ≈ **50× L1** latency
- One cache line = **8 doubles** or **16 ints**
- L1 holds ≈ **512 doubles** (32 KB / 8 bytes)
- A 64×64 double tile = 32 KB — fits in L1

## Loop Tiling Algorithm

```
// Naive: O(n³) but cache-hostile
for i in 0..N:
    for j in 0..N:
        for k in 0..N:
            C[i][j] += A[i][k] * B[k][j]

// Tiled: O(n³) but cache-friendly
TILE = 32  // So 3 tiles fit in L1: 3 × 32² × 8 = 24 KB
for i in 0..N step TILE:
    for j in 0..N step TILE:
        for k in 0..N step TILE:
            for ii in i..min(i+TILE,N):
                for jj in j..min(j+TILE,N):
                    for kk in k..min(k+TILE,N):
                        C[ii][jj] += A[ii][kk] * B[kk][jj]
```

**Expected speedup: 5–30× for N ≥ 512.**

## AoS vs SoA Decision Guide

### Array of Structures (AoS)
```cpp
struct Particle {
    double x, y, z;      // hot fields
    double vx, vy, vz;   // warm fields
    double mass, charge;  // cold fields
};
Particle particles[N];
```
- ✅ Best when: loop touches most/all fields
- ✅ Natural for OOP, easier to read
- ❌ Wastes cache when loop touches few fields (85% of cache line unused)

### Structure of Arrays (SoA)
```cpp
struct Particles {
    double x[N], y[N], z[N];
    double vx[N], vy[N], vz[N];
    double mass[N], charge[N];
};
```
- ✅ Best when: loop touches 1–3 fields per element
- ✅ Maximum spatial locality for field-specific loops
- ✅ SIMD-friendly (contiguous data)
- ❌ Harder to read, more bookkeeping for insert/delete

**Rule of thumb**: If hot loop touches < 50% of fields → SoA. Otherwise → AoS.

### Hot/Cold Splitting
```cpp
struct ParticleHot {  // 24 bytes — fits 2 per cache line
    double x, y, z;
};
struct ParticleCold {  // rarely accessed, separate allocation
    double vx, vy, vz, mass, charge;
};
```

## Cache-Friendly Patterns

### ✅ Do This
- **Sequential access** — Arrays, contiguous iteration
- **Tile your loops** — Keep working set under L1/L2 size
- **Fit working set in cache** — Profile, resize, repeat
- **Use power-of-two+1 sizes** — Avoid cache conflicts from power-of-two strides (or pad)
- **Group hot fields** — Most-accessed fields first, pack into 64 bytes

### ❌ Avoid This
- **Pointer chasing** — Linked lists, tree nodes on heap
- **Column-major strides on row-major data** — The classic matrix mistake
- **Power-of-two strides** — Causes 4-way set-associative conflict misses
- **Random access patterns** — Defeats hardware prefetchers
- **False sharing** — Threads writing different vars on same cache line (see L06)

## Quick Benchmark Numbers

For 512×512 double matrix multiply (typical x86):

| Method | Time (ms) | Relative | L1 misses |
|--------|-----------|----------|-----------|
| Naive  | ~800      | 1.0×     | Very high |
| Tiled (32) | ~60   | ~13×     | Low       |
| Cache-oblivious | ~80 | ~10× | Good |

Actual numbers vary by CPU, compiler, and optimization level.

## Why B-Trees Beat BSTs

| | BST | B-Tree |
|---|-----|--------|
| Node size | ~24 bytes | ~4 KB (page) |
| Keys per node | 1 | ~300 |
| Tree height (10M keys) | ~23 levels | ~3 levels |
| Lookups | ~23 cache misses | ~3 cache misses |
| Latency | ~4,600 cycles | ~120 cycles |

## Comparison: Arrays vs Linked Lists

For N = 1,000,000 elements, sequential sum:

| | Array | Linked List |
|---|-------|-------------|
| Access pattern | Sequential | Random (pointer chase) |
| Cache behavior | Prefetcher-friendly | Miss per access |
| Typical time | ~1 ms | ~200 ms |
| Ratio | 1× | ~200× slower |

## Memory Latency Cheat Sheet

```
L1 hit:      ~1 ns    (4 cycles @ 3 GHz)
L2 hit:      ~4 ns    (12 cycles)
L3 hit:      ~13 ns   (40 cycles)
DRAM:        ~67 ns   (200+ cycles)
SSD:         ~100 μs  (100,000× L1)
Network:     ~1 ms    (1,000,000× L1)
```

Remember: **A cache miss costs about the same as 50 L1 hits. Design accordingly.**