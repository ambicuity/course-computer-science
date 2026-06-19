# Lesson 19: Garbage Collection — Mark-Sweep, Copying, Generational

## Overview

Programming languages must manage memory. Some languages leave this to the programmer, while others automate it through **garbage collection (GC)** — a runtime system that reclaims memory no longer reachable by the program. Understanding GC internals is essential for writing high-performance applications and tuning runtime behavior.

## Manual vs Automatic Memory Management

**Manual (C, C++, Rust):** The programmer allocates and frees memory explicitly.

```c
// C — manual management
int *arr = malloc(100 * sizeof(int));
// ... use arr ...
free(arr);        // programmer must not forget this
```

**Automatic (Java, Go, Python, JavaScript):** The runtime tracks live objects and reclaims dead ones.

```java
// Java — GC handles memory
List<String> list = new ArrayList<>();
list.add("hello");
// No free — GC reclaims when unreachable
```

**Rust** takes a third approach: ownership and borrowing rules at compile time eliminate the need for a runtime GC, while guaranteeing memory safety. No garbage collector, no dangling pointers, no double-frees.

## Reference Counting

The simplest GC strategy. Each object stores a count of how many references point to it.

- When a reference is assigned: increment count.
- When a reference goes out of scope: decrement count.
- When count reaches zero: free the object immediately.

**Advantage:** Deterministic deallocation — objects are freed the moment they become unreachable (Python's primary mechanism).

**Problem:** Cycles. If A references B and B references A, both have count ≥ 1 even if nothing else references them.

```
A → B → A    (both counts = 1, but no root references them)
```

Python's cycle detector catches these by periodically tracing objects. Languages like Swift use weak references to break cycles. Rust's `Rc<T>` and `Arc<T>` provide reference counting with the programmer's help to avoid cycles via `Weak` references.

## Mark-Sweep GC

The classic tracing garbage collector, used by early Lisp systems and the basis for many modern GCs.

### Algorithm

1. **Mark phase:** Starting from **roots** (global variables, stack variables, registers), traverse all reachable objects via DFS or BFS. Mark each visited object as "alive."
2. **Sweep phase:** Iterate through all objects in the heap. Free any object not marked.

```
Roots: [stack, globals]
  ↓
  [A marked] → [B marked] → [C marked]
                ↕              ↕
  [D marked]   [E marked]   [F unmarked] → freed
```

### Fragmentation

Mark-sweep does **not** compact memory. After many allocations and frees, the heap becomes fragmented — many small free blocks scattered throughout, even if total free space is sufficient for a large allocation. This forces the allocator to search for fitting blocks and can cause allocation failures despite available memory.

```
Before sweep: [A][B][C][D][E][F][G][H]
After sweep:  [A][   ][C][   ][E][   ][   ][H]
                  ↑       ↑       ↑
               free    free    free (fragmented)
```

## Mark-Compact GC

An improvement over mark-sweep that eliminates fragmentation by **compacting** live objects after marking.

### Algorithm

1. Mark phase (same as mark-sweep).
2. Compute new addresses for live objects by sliding them to one end of the heap.
3. Update all pointers to reflect new addresses.
4. Relocate objects.

```
Before: [A][   ][C][   ][E][   ][   ][H]
After:  [A][C][E][H][                   ]
```

**Trade-off:** No fragmentation, but compacting is expensive — requires updating every pointer. Used by the JVM's Serial and Parallel collectors for old generation.

## Copying GC (Semi-Space)

Divides the heap into two equal halves: **from-space** and **to-space**.

### Algorithm (Cheney's)

1. Allocate only in from-space.
2. When from-space fills up:
   - Copy all live objects from from-space to to-space.
   - Flip: from-space becomes to-space and vice versa.

```
From: [A][  ][C][  ][E][  ]
To:   [                    ]

After copy:
From: [                    ]  ← now to-space
To:   [A][C][E][           ]  ← now from-space
```

**Advantages:**
- Automatic compaction — all live objects are contiguous after copy.
- Allocation is fast: bump a pointer, no free list needed.

**Disadvantages:**
- Halves available memory (only half the heap is usable at any time).
- Copies all live objects on every collection — expensive if most objects survive.

Used by the JVM's young generation (survivor spaces) and Go's GC for certain phases.

## Generational GC

Based on the **weak generational hypothesis**: most objects die young. Only a small fraction survive long-term.

### Design

Split the heap into generations:
- **Young generation (nursery):** New objects allocated here. Collected frequently with copying GC. Most objects die and are never copied.
- **Old generation (tenured):** Objects that survive several young-gen collections are **promoted** here. Collected infrequently with mark-sweep or mark-compact.

```
Allocation → Young Gen (frequent GC, copying)
                  ↓ survives N collections
               Old Gen (infrequent GC, mark-sweep)
```

**Write barrier:** When an old-gen object references a young-gen object, the GC must track this to avoid missing live young objects. The runtime inserts a **write barrier** — code that runs on every pointer write to record cross-generational references in a **card table**.

### Performance

The young gen is small and collected quickly (sub-millisecond pauses). The old gen is large but collected rarely. This separates pause times — most GC pauses are short and only major collections are long.

## GC Algorithms in Practice

| Language/VM | GC Algorithm | Notes |
|------------|-------------|-------|
| Java (G1) | Generational, region-based | Predictable pause times |
| Java (ZGC) | Concurrent, colored pointers | <1ms pauses, TB heaps |
| Go | Concurrent tri-color mark-sweep | Low latency, no compaction |
| Python | Reference counting + cycle GC | Deterministic, GIL limits concurrency |
| JavaScript (V8) | Generational, concurrent | Scavenger (young) + Mark-Sweep-Compact (old) |
| .NET (CLR) | Generational, background | Server GC for multi-core |

## Build It: GC Simulation

In the companion code, we simulate mark-sweep, copying, and generational garbage collection in Rust. Each GC tracks statistics: objects collected, pause time, memory overhead, and throughput.

## Use It

**Java:** `java -XX:+UseG1GC -Xmx4g MyApp` — tune GC for your workload.

**Go:** `GOGC=100` controls how much garbage triggers collection. `GOMEMLIMIT` sets a soft memory limit (Go 1.19+).

**Python:** `gc.collect()` forces a cycle collection. `gc.disable()` for manual control in performance-critical sections.

## Ship It: GC Library

Our simulation demonstrates that no single GC strategy wins everywhere. The best choice depends on allocation rate, object lifetimes, pause time requirements, and heap size.

## Exercises

**Level 1:** Add a `mark()` function that returns the set of reachable objects from a given set of roots. Test it with a simple object graph.

**Level 2:** Implement a two-space copying collector. Measure the ratio of objects copied vs total objects across 100 random allocation/collection cycles.

**Level 3:** Add a write barrier to the generational collector. Track cross-generational references and verify that no young-gen object reachable from an old-gen object is incorrectly collected.
