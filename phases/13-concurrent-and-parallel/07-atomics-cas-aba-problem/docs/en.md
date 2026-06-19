# Atomics, CAS, ABA Problem

> Atomics, CAS, ABA Problem — lock-free programming demands understanding hardware atomics,
> the CAS primitive, and the subtle ABA bug that silently corrupts data.

**Type:** Build
**Languages:** Rust, C++
**Prerequisites:** Phase 13 lessons 01–06 (threads, synchronization, memory model)
**Time:** ~75 minutes

## Learning Objectives

- Explain why atomic operations outperform locks for simple shared state.
- Implement compare-and-swap (CAS), fetch-and-add (FAA) in Rust and C++.
- Build a lock-free Treiber stack using CAS on an atomic head pointer.
- Demonstrate the ABA problem and why address recycling defeats naive CAS.
- Fix ABA with a tagged pointer (version counter embedded in pointer bits).
- Compare lock-free vs. mutex-based approaches on throughput.
- Map the lesson's concepts to production use (Linux kernel atomics, crossbeam).

## The Problem

**Locks are heavyweight for simple operations.** Every mutex acquisition involves:

- A system call (or at least a futex / syscall on contention).
- Context switching when threads block.
- Cache-line bouncing (the lock cache line moves between cores).
- Risk of priority inversion, deadlock, and convoying.

For a **counter increment**, a **flag toggle**, or a **pointer swap**, the cost of a mutex
dominates the actual work by orders of magnitude. Hardware provides atomic instructions
that perform these operations in a single CPU cycle with no context switch.

**The gap this lesson fills:** you must understand three things that are rarely taught together:

1. How hardware atomics actually work (CAS, FAA, LL/SC).
2. Why naive use of CAS produces the ABA problem.
3. How real systems solve ABA (tagged pointers, RCU, hazard pointers).

Without this knowledge, any lock-free structure you write will either be incorrect (ABA bug)
or slower than a mutex (wrong memory ordering, false sharing).

## The Concept

### Hardware Atomic Instructions

Modern CPUs guarantee that certain read-modify-write operations are **indivisible**:

| Operation | Mnemonic | Effect |
|-----------|----------|--------|
| Compare-and-Swap | CAS | `if *p == expected { *p = new; true } else { expected = *p; false }` |
| Fetch-and-Add | FAA | `old = *p; *p += delta; return old` |
| Load-Linked / Store-Conditional | LL/SC | `LL: tmp = *p; ...; SC: *p = new if p not modified` |
| Test-and-Set | TAS | `old = *p; *p = 1; return old` |
| Swap / Exchange | XCHG | `old = *p; *p = new; return old` |

**CAS is the universal primitive.** On x86 it maps to `CMPXCHG` (or `CMPXCHG16B` for double-word).
ARM uses LL/SC (`LDREX`/`STREX`) which provides the same semantics with weaker guarantees.
RISC-V has `AMO*` (atomic memory operation) instructions plus `LR`/`SC`.

CAS returns whether the swap happened. The standard pattern is a **CAS loop** (retry loop):

```
while not CAS(&ptr, expected, desired):
    expected = ptr   // reload
    // optionally recompute desired from new expected
```

### Memory Ordering

Atomics alone are not enough — you also need **memory ordering** to prevent the CPU and
compiler from reordering memory accesses around the atomic operation. The C++ / Rust
memory model defines orderings:

| Ordering | Guarantees |
|----------|-----------|
| `Relaxed` | Atomicity only. No ordering against other accesses. |
| `Acquire` | Reads after the atomic cannot be reordered before it (load-acquire). |
| `Release` | Writes before the atomic cannot be reordered after it (store-release). |
| `AcqRel` | Acquire + Release (used with read-modify-write). |
| `SeqCst` | AcqRel + a single total order across all SeqCst operations (most expensive). |

### The ABA Problem

CAS checks that a memory location has a specific *value*. If that value is a **pointer**,
CAS checks for *address equality*. When memory is recycled:

1. Thread T1 reads `head = NodeA` (address `0x1000`). Plans to CAS `&head` from `0x1000` to `NodeA->next`.
2. Thread T2 pops `NodeA`, pops `NodeB`. Frees `NodeA` and `NodeB`.
3. Thread T2 allocates a new node `NodeC`. The allocator reuses address `0x1000`.
4. Thread T2 pushes `NodeC` at `head`. `head` is now `0x1000` again.
5. Thread T1 resumes. CAS checks `&head == 0x1000`? Yes. CAS succeeds: `head = NodeA->next`.
6. But `NodeA->next` pointed to the *freed* `NodeB`. **Data corruption.**

The value went from A → B → A. CAS saw the "same" value and assumed nothing changed.

### Solutions

| Solution | How it works | Used by |
|----------|-------------|---------|
| **Tagged pointer** | Embed version counter in unused pointer bits. Every CAS increments the tag. Even if addr cycles, tag won't match. | Linux kernel (`atomic_t`), Java `AtomicStampedReference` |
| **Hazard pointers** | Each thread announces which pointers it is about to dereference. A reclaimer must wait until no thread hazards a pointer before freeing. | `crossbeam-epoch` (Rust) |
| **RCU** (Read-Copy-Update) | Writers make a new copy, publish via atomic store. Readers see a consistent snapshot. Grace period before reclamation. | Linux kernel, `arcu` (Rust) |
| **Double-word CAS** (DCAS) | CAS on 16 bytes (pointer + ABA counter atomically). CMPXCHG16B on x86. | Intel 64, IBM z/Architecture |

## Build It

We build four artifacts in sequence, each building on the previous:

1. **Atomic Counter** — the simplest possible use of atomics.
2. **Lock-Free Stack** — a CAS-based Treiber stack.
3. **ABA Problem** — demonstrating the bug with memory recycling (C++).
4. **Tagged Pointer** — fixing ABA with an embedded version counter.

### Step 1: Atomic Counter

The problem: N threads increment a shared counter. Compare `Mutex<u64>` vs `AtomicU64::fetch_add`.

```
Threads: 8, Increments per thread: 1,000,000
  Atomic counter: 8,000,000 in 42ms
  Mutex counter:  8,000,000 in 285ms
  Speedup: 6.8x
```

The atomic approach uses `fetch_add` with `Ordering::Relaxed` because there is no
cross-thread dependency on the counter's value — we only need atomicity, not ordering.
(The final `get()` uses `Relaxed` too since the join guarantees all increments are visible.)

### Step 2: Lock-Free Stack (Treiber Stack)

A LIFO structure where every operation uses CAS on the `head` pointer:

```
push(v):
  n = new Node(v)
  loop:
    n.next = head
    if CAS(&head, n.next, n): break

pop() -> v:
  loop:
    n = head
    if n == null: return empty
    if CAS(&head, n, n.next): return n.value
```

Key memory ordering choices:
- **`Acquire`** on the `load` of `head`: guarantees subsequent reads see the node's data.
- **`Release`** on the successful `store` in CAS: guarantees prior writes to the node are visible.
- **`Relaxed`** on the failed CAS: no ordering needed for a failed attempt.

Rust's ownership model means we can safely implement this with `AtomicPtr` and
`Box::into_raw` / `Box::from_raw`. Each node is owned by exactly one thread at a time
(either in the stack or in the caller's `Box`).

The `Drop` implementation drains the stack (single-threaded at drop time) to avoid
leaking memory.

### Step 3: The ABA Problem

In C++ (or any language without a borrow checker that prevents use-after-free), the ABA
problem manifests naturally when nodes are freed and the allocator reuses their addresses.

**Demonstration setup:**

1. Create a stack with two nodes: `A → B → null`.
2. Thread T1 reads `head = A`, computes `new_head = B`.
3. Thread T2 pops `A` and `B`, frees both.
4. Thread T2 allocates `C` at `A`'s address (recycling), pushes `C`.
5. T1's CAS sees `head == A` (same address). CAS succeeds.
6. `head` is now `B` — which was freed. Corruption.

The C++ demo (`main.cpp`) implements this deterministically: a single-slot recycling
allocator guarantees address reuse so the bug is always visible.

In Rust, the borrow checker prevents the simple ABA scenario — you cannot free a `Box`
while a raw pointer to it still exists without `unsafe` code that fundamentally
recreates the C++ pattern. This is a genuine safety win. The Rust code explains the
concept and references the C++ demo for the concrete demonstration.

### Step 4: Tagged Pointer (ABA Solution)

Embed a version counter in the unused bits of the pointer:

```
On x86-64: 48 bits for address, 16 bits tag
Or use alignment: lowest N bits are always 0 → use as tag

head (AtomicUsize) = ptr_bits | (tag << shift)

push:
  loop:
    old = head.load(Acquire)
    (ptr, tag) = unpack(old)
    node.next = ptr
    new = pack(node, (tag + 1) & TAG_MASK)
    if CAS(&head, old, new): break

pop:
  loop:
    old = head.load(Acquire)
    (ptr, tag) = unpack(old)
    if ptr == null: return None
    new = pack(ptr.next, (tag + 1) & TAG_MASK)
    if CAS(&head, old, new): return ptr.value
```

Each successful CAS increments the tag. Even if the pointer value cycles back to an
old address, the tag will differ and CAS will fail — forcing a retry that reads
the current (correct) head.

With 3 tag bits, the probability of both address and tag matching after an ABA cycle
is 1/8. Production systems use 14–16 bits for near-zero probability.

## Use It

### Rust `std::sync::atomic`

The standard library provides:
- `AtomicBool`, `AtomicI8`, `AtomicI16`, `AtomicI32`, `AtomicI64`, `AtomicIsize`
- `AtomicU8`, `AtomicU16`, `AtomicU32`, `AtomicU64`, `AtomicUsize`
- `AtomicPtr<T>` — atomic pointer

All support `load`, `store`, `swap`, `compare_exchange`, `compare_exchange_weak`,
`fetch_add`, `fetch_sub`, `fetch_and`, `fetch_or`, `fetch_xor`, `fetch_update`.

### C++ `std::atomic`

The standard library provides `std::atomic<T>` for trivially copyable types:
- `load()`, `store()`, `exchange()`, `compare_exchange_weak()`, `compare_exchange_strong()`
- `fetch_add()`, `fetch_sub()`, `fetch_and()`, `fetch_or()`, `fetch_xor()`
- Memory ordering parameter: `std::memory_order_relaxed`, `_acquire`, `_release`, `_acq_rel`, `_seq_cst`

### Linux kernel `atomic_t` and `cmpxchg`

The kernel provides architecture-optimized atomics:
- `atomic_read()`, `atomic_set()`, `atomic_add()`, `atomic_sub()`
- `atomic_cmpxchg()` — directly maps to CAS
- `atomic_try_cmpxchg()` — like C++ `compare_exchange` (updates old on failure)

The Linux kernel's RCU implementation is the gold standard for safe lock-free
reading with deferred reclamation.

## Read the Source

- **Linux kernel `include/linux/atomic.h`** — architecture-independent atomic API.
  See `atomic_long_cmpxchg()` and friends.
- **Linux kernel `include/linux/rcupdate.h`** — RCU implementation.
- **Rust standard library `library/core/src/sync/atomic.rs`** — atomic types.
- **crossbeam-epoch (Rust crate)** — `src/epoch.rs` for hazard-pointer-like epoch-based
  reclamation. This is what production Rust uses instead of tagged pointers.
- **Java `java.util.concurrent.atomic.AtomicStampedReference`** — reference + stamp
  (tagged pointer in user space).

## Ship It

The reusable artifact lives in `outputs/`. It contains:

- **A lock-free Treiber stack in Rust** — ready to adapt for work-stealing deques,
  lock-free queues, and concurrent graph structures.
- **A tagged-pointer version** — the pattern for ABA-safe CAS.
- **C++ ABA demonstration** — reference for understanding the bug.

To reuse in later phases: extract the `TaggedStack<T>` or `LockFreeStack<T>` struct
and its operations. The pattern extends to any CAS-based concurrent data structure.

## Exercises

### Easy

Reproduce the atomic counter benchmark from scratch: write a program that spawns
8 threads, each calling `fetch_add` 1,000,000 times. Compare with a `Mutex<u64>`.
Now change the thread count to 1, 2, 4, 8, 16 and graph the results.

### Medium

Extend the lock-free stack to a lock-free queue (Michael-Scott queue) using CAS on
both head and tail pointers. Test with concurrent enqueue/dequeue from 8 threads.

**Hint:** The Michael-Scott queue uses a dummy sentinel node and CAS on both `head`
and `tail`. The tail CAS may need to help — if you observe `tail->next != null`,
try to advance tail first.

### Hard

Implement hazard pointers for the lock-free stack in C++:
1. Each thread has a small array of hazard pointers (announced pointers).
2. Before dereferencing a pointer from CAS, store it in a hazard pointer.
3. Before freeing a node, check all threads' hazard pointers. If any thread hazards
   the node, defer the free to a retire list.
4. Periodically purge the retire list (when a threshold is met).

Compare throughput of the hazard-pointer stack vs. the tagged-pointer stack vs.
a mutex-based stack.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| CAS | Compare-and-swap | CPU instruction: if `*addr == expected`, store `new` and return success; else reload `expected` and return failure. |
| FAA | Fetch-and-add | Atomic: `old = *addr; *addr += delta; return old`. Counter example: `fetch_add(1)`. |
| LL/SC | Load-linked / Store-conditional | ARM/RISC-V primitive: LL reads a location, SC stores only if the location hasn't been modified since the LL. Provides CAS semantics without the ABA problem (at the hardware level). |
| ABA problem | Pointer value changes A→B→A, CAS doesn't detect it | A CAS that only checks address equality cannot distinguish "nothing changed" from "something changed and then changed back." |
| Tagged pointer | Version counter in pointer bits | Embed a monotonic counter in unused pointer bits (upper or lower). Every CAS increments the counter. Address cycles no longer defeat CAS. |
| Lock-free | At least one thread makes progress | System-wide progress: if any thread is stuck, at least one other thread is making progress (no deadlock). |
| Wait-free | Every thread makes progress | Per-thread progress: every operation completes in a bounded number of steps regardless of other threads. |
| Hazard pointer | Announce-in-use pointers | Before dereferencing, a thread announces the pointer. A reclaimer must wait until no thread hazards a pointer before freeing it. |
| RCU | Read-copy-update | Writers copy, modify, and atomically publish. Readers see consistent old state. Grace period separates publication from reclamation. |
| Double-word CAS | DCAS | CAS on 16 bytes (e.g., pointer + ABA counter pair atomically). `CMPXCHG16B` on x86-64. |

## Further Reading

- Herlihy, Shavit — *The Art of Multiprocessor Programming* (chapters on lock-free data structures).
- McKenney — *Is Parallel Programming Hard, And, If So, What Can You Do About It?* (RCU deep dive).
- C++ memory model: cppreference.com `std::memory_order`.
- Rust `std::sync::atomic` documentation.
- `crossbeam-epoch` crate documentation (epoch-based reclamation in Rust).
- Linux kernel documentation: `Documentation/RCU/whatisRCU.rst`.
