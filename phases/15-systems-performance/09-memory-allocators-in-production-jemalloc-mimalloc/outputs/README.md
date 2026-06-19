# Allocator Reference Card

## Allocator Comparison Table

| Allocator       | Throughput | Latency (p99) | Fragmentation | Thread Scaling | Memory Overhead | Complexity |
|----------------|------------|---------------|---------------|----------------|-----------------|------------|
| glibc ptmalloc2 | Medium   | Moderate      | High          | Poor (1 lock)  | Medium          | Low        |
| jemalloc        | High      | Low           | Low           | Excellent      | ~4 MB base     | Medium     |
| mimalloc        | High      | Very Low     | Very Low      | Excellent      | ~1 KB/segment  | Medium     |
| tcmalloc        | High      | Low           | Medium        | Good           | ~256 KB/thread  | Medium     |
| bump/arena      | Very High | Minimal      | None*         | N/A            | Region size     | Minimal   |
| slab/pool       | Very High | Minimal      | None*         | Needs locking  | Slot count      | Low        |
| tlsf            | Medium    | Low (deterministic) | Low   | Needs locking  | Low             | Low        |

*\*No fragmentation within arena or slab — external fragmentation still possible across arenas/slabs.*

## Size Classes

| Allocator   | Size Classes | Min Alignment | Max Small Object |
|-------------|-------------|---------------|------------------|
| jemalloc    | ~40         | 16 bytes      | ~4 KB            |
| mimalloc    | 28          | 16 bytes      | ~8 KB (page)     |
| tcmalloc    | ~88         | 8 bytes       | ~256 KB          |
| glibc       | ~128 bins   | 16 bytes      | ~512 KB          |

Size class granularity matters: more classes = less internal fragmentation but more metadata. jemalloc uses sparse size classes (8, 16, 32, 48, 64, 80, 96, ...) to balance the two.

## When to Use Which Allocator

| Scenario                                | Recommended       | Why                                          |
|-----------------------------------------|-------------------|----------------------------------------------|
| General-purpose Linux server            | jemalloc           | Battle-tested, great throughput, good fragmentation |
| Low-latency trading / gaming            | mimalloc           | Predictable p99, eager memory return         |
| Memory-constrained embedded             | tlsf               | Deterministic O(1), low overhead              |
| Parse tree / AST (free all at once)     | Bump / arena       | Fastest possible, zero per-free overhead      |
| Object pool (fixed size, high churn)    | Slab / pool        | O(1) alloc+free, cache-friendly              |
| NUMA-aware server                       | jemalloc           | NUMA-aware arena placement                   |
| Short-lived temp buffers in a function  | Stack allocator    | Zero malloc calls, cache-local               |
| Debugging memory errors                 | jemalloc + prof    | Built-in profiling, leak detection           |

## Common Patterns

### Arena / Region Pattern
```c
bump_allocator arena;
bump_init(&arena, MiB(64));
// ... allocate freely ...
bump_destroy(&arena);  // free everything at once
```
Best for: parse trees, game frames, request processing scopes.

### Object Pool Pattern
```c
// Pre-allocate N objects of fixed size
typedef struct { void *next; /* fields */ } pool_node;
pool_node *free_list = NULL;
for (int i = 0; i < N; i++) {
    pool_node *n = (pool_node *)malloc(sizeof(pool_node));
    n->next = free_list;
    free_list = n;
}
// Alloc: pop from free_list
// Free:  push to free_list
```
Best for: connection objects, AST nodes, entity components.

### Realloc Strategy
```c
size_t capacity = 16;
size_t used = 0;
T *arr = malloc(capacity * sizeof(T));

void append(T val) {
    if (used == capacity) {
        capacity = capacity * 2;       // doubling strategy
        arr = realloc(arr, capacity * sizeof(T));
    }
    arr[used++] = val;
}
```
Best for: dynamic arrays, hash tables, string builders.

## Benchmarking Cheat Sheet

```bash
# Run with different allocators
LD_PRELOAD=/usr/lib/libjemalloc.so ./your_program
LD_PRELOAD=/usr/lib/libmimalloc.so ./your_program

# jemalloc stats at exit
MALLOC_CONF="prof:true,prof_prefix:heap" ./your_program
jemalloc_stats_print()

# mimalloc stats at exit
mi_stats_print_out(NULL, NULL)

# Measure with perf
perf stat -e cache-misses,instructions ./your_program

# Measure RSS
/usr/bin/time -v ./your_program   # look at "Maximum resident set size"
```

## Key Metrics to Watch

| Metric          | What it tells you              | How to measure                    |
|----------------|-------------------------------|-----------------------------------|
| Throughput     | Allocs/sec by pattern          | `clock_gettime` around loop       |
| p99 Latency    | Lock contention                | Per-alloc timing, histogram       |
| RSS / Peak     | Fragmentation + overhead       | `/usr/bin/time -v` or `getrusage` |
| Cache misses   | Data locality                  | `perf stat -e cache-misses`      |
| Malloc count   | Allocation rate                | Wrap `malloc` with interposer      |