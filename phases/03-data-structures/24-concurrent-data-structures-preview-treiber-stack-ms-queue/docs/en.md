# Concurrent Data Structures Preview — Treiber Stack, MS Queue

> A preview of lock-free programming: two structures that beat locks via atomic CAS. Phase 13 covers the full theory; this teaches the shapes.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** P03 L02 (lists), P02 L17 (defensive programming)
**Time:** ~75 minutes

## Learning Objectives

- Implement the **Treiber lock-free stack**: push/pop via `compare_exchange` on the head pointer.
- Sketch the **Michael-Scott (MS) queue**: classic lock-free FIFO.
- Recognize the **ABA problem** and standard mitigations (hazard pointers, epoch GC, double-word CAS).
- Understand memory ordering (relaxed, acquire, release, seq_cst) at the level needed for correctness.

## The Problem

In single-threaded code, push/pop on a linked list is trivial. With concurrent threads, two threads can simultaneously try to update head — one's update gets lost.

The naïve fix: a mutex. Locks WORK, but:

- Contention causes serialization → throughput plateau.
- Priority inversion is possible.
- Deadlock if you compose two locked operations.
- Bookkeeping overhead.

Lock-free structures use atomic primitives (compare-and-swap) instead. They guarantee **system-wide progress**: at any time, at least one thread makes forward progress, even if others stall. No deadlock, no priority inversion, no contention serialization (mostly).

This lesson introduces two canonical lock-free structures. Phase 13 (Concurrent & Parallel) covers theory and more sophisticated ones (lock-free hash maps, software transactional memory).

## The Concept

### Compare-and-swap (CAS)

`atomic_compare_exchange(ptr, &expected, new_value)`:

1. Atomically reads `*ptr`.
2. If it equals `expected`, writes `new_value`. Returns true.
3. Else, writes the actual value to `*expected`. Returns false.

The hardware primitive that makes lock-free possible. Available on every modern CPU: x86 `LOCK CMPXCHG`, ARM `LDREX/STREX`.

### Treiber stack (1986)

```c
typedef struct Node { int value; struct Node *next; } Node;
_Atomic(Node *) head;

void push(int x) {
    Node *n = malloc(sizeof(Node));
    n->value = x;
    Node *old_head;
    do {
        old_head = atomic_load(&head);
        n->next = old_head;
    } while (!atomic_compare_exchange_weak(&head, &old_head, n));
}

int *pop(void) {
    Node *old_head;
    do {
        old_head = atomic_load(&head);
        if (!old_head) return NULL;
    } while (!atomic_compare_exchange_weak(&head, &old_head, old_head->next));
    return &old_head->value;       /* (free issue — see ABA below) */
}
```

If two pushes race, only one's CAS succeeds; the other retries with the new head. No locks. Progress guaranteed.

### Memory orderings

C11 provides several:

- `memory_order_relaxed`: no ordering, just atomicity.
- `memory_order_acquire`: paired with release; reads see all writes from before the release.
- `memory_order_release`: paired with acquire; writes are visible to readers after the acquire.
- `memory_order_seq_cst`: total order across all atomics; default but expensive.

Treiber stack push: the CAS uses release (publishes the new node). Pop uses acquire (reads see the published writes). Together they ensure threads see consistent linked-list state on a weakly-ordered CPU (ARM, POWER).

### The ABA problem

Thread A is preparing to pop A. It reads head = A; head.next = B.

Now Thread B pops A, pops B (head = C), then pushes A back (head = A, with A.next = C).

Thread A resumes: CAS(head, A, B) — succeeds! But head should now be C, not B. The structure corrupts.

The cause: head's pointer value is the SAME (it's A), but the world has changed in between.

Mitigations:

1. **Hazard pointers** (Michael 2004): each thread publishes which pointers it's holding; reclamation waits for all hazards to clear.
2. **Epoch-based GC**: free memory only when no thread is in an old "epoch."
3. **Tagged pointers / double-word CAS**: head is `(pointer, version_counter)`; bump version on every change.

Modern lock-free libraries (Boost.Lockfree, crossbeam-rs, Folly) handle this. Don't write lock-free code without one.

### Michael-Scott queue (1996)

Linked queue with separate head and tail pointers; both updated via CAS. Two operations per enqueue (update tail.next, then advance tail) — necessitates a clever invariant that admits intermediate states.

Used by Java's `ConcurrentLinkedQueue`, .NET's `ConcurrentQueue`, Linux's `lib/llist.c` (single-linked variant).

### When NOT to use lock-free

- **Hot contended**: still serializes; locks may be faster due to less retry waste.
- **Need fairness**: lock-free admits starvation under perpetual contention.
- **Memory reclamation hard**: hazard pointers or epoch GC adds complexity.

For most software, a `Mutex<HashMap>` or `RwLock<Vec>` is correct and fast enough. Reach for lock-free only when profiles show lock contention is the bottleneck.

## Build It

`code/main.c`:

1. Treiber stack with `_Atomic` pointer.
2. Multi-threaded test: 4 threads pushing + popping 100K items each; verify count.
3. Compare with a mutex-protected stack; measure ns/op.

`code/main.rs` uses `crossbeam::epoch` for safe reclamation.

### Run

```sh
clang -O2 -pthread main.c -o cds && ./cds
```

## Use It

- **crossbeam-rs / Boost.Lockfree**: Treiber stack and MS queue in production form.
- **Java ConcurrentLinkedQueue, .NET ConcurrentQueue**: MS queue.
- **Linux `lib/llist.c`**: single-linked lock-free list for kernel state.
- **Disruptor (LMAX)**: not Treiber/MS, but a bounded multi-producer multi-consumer ring buffer using CAS.

## Read the Source

- *Treiber 1986*: A Correct and Simple ... Stack Implementation.
- *Michael & Scott 1996*: Simple, Fast, and Practical Non-Blocking and Blocking Concurrent Queue Algorithms.
- [Java ConcurrentLinkedQueue source](https://github.com/openjdk/jdk/blob/master/src/java.base/share/classes/java/util/concurrent/ConcurrentLinkedQueue.java).
- [crossbeam-rs](https://github.com/crossbeam-rs/crossbeam) — Rust's production lock-free toolkit.

## Ship It

This lesson ships **`outputs/treiber_stack.h`** — header-only Treiber stack with proper orderings.

## Exercises

1. **Easy.** Write a multi-thread stress test pushing 1M items and popping all; verify count matches.
2. **Medium.** Implement the ABA-mitigation tagged-pointer trick: head is `(pointer, version)`. Double-word CAS updates both atomically.
3. **Hard.** Implement Michael-Scott queue. The trickiest part is "help" tail forward when a competing thread left tail behind.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Lock-free | "Non-blocking" | At least one thread always makes progress; no deadlock |
| CAS | "Compare-and-swap" | Atomic conditional write; the lock-free primitive |
| ABA | "Same value, different world" | Two reads see same pointer but world changed in between |
| Memory ordering | "Acquire/release" | Compiler/CPU guarantees about reordering around atomics |
| Hazard pointer | "Reclamation safety" | Each thread publishes pointers it's using; free only after all hazards clear |

## Further Reading

- *The Art of Multiprocessor Programming* (Herlihy & Shavit) — comprehensive.
- *Memory Models for C/C++ Programmers* (Russ Cox): https://research.swtch.com/hwmm.
- [Linux per-CPU atomics tour](https://lwn.net/Articles/707849/).
