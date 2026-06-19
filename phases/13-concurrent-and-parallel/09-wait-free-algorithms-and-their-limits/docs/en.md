# Wait-Free Algorithms and Their Limits

> Every thread makes progress, always. No retries. No spinning. No starvation.

**Type:** Learn + Build
**Languages:** Rust
**Prerequisites:** Phase 13 lessons 01–08 (especially 07 on atomics/CAS and 08 on lock-free structures)
**Time:** ~60 minutes

## Learning Objectives

- Understand why lock-freedom does not guarantee per-thread progress.
- Distinguish the four levels of the progress hierarchy: blocking, obstruction-free, lock-free, wait-free.
- Implement a wait-free counter using hardware-fetch-add.
- Implement a wait-free snapshot using the double-collect technique.
- Compare lock-free and wait-free behaviours under contention.
- Understand the Herlihy consensus hierarchy and why wait-free consensus is impossible for arbitrary objects.

## The Problem

Lock-free data structures guarantee that *some* thread makes progress on every operation. That is enough for
system-wide throughput, but it does *nothing* for latency predictability. Under high contention, a thread can
retry its CAS indefinitely while faster threads keep modifying the shared state.

Consider a Treiber stack with three threads pushing concurrently:

```
Thread A: read top = NodeX
Thread B: read top = NodeX → CAS(top, NodeX → NodeB) succeeds
Thread C: read top = NodeB → CAS(top, NodeB → NodeC) succeeds
Thread A: CAS(top, NodeX, NodeA) → FAILS (top is now NodeC)
Thread A: retry: read top = NodeC → CAS(top, NodeC, NodeA) → FAILS (B wrote NodeD)
Thread A: retry, retry, retry...  // starves
```

Thread A does everything right — it follows the lock-free protocol — yet it makes no progress.
Lock-freedom promises *system-wide* progress, not *per-thread* progress. For soft-real-time systems
(audio pipelines, game engines, trading systems), starvation is unacceptable.

Wait-freedom closes this gap.

## The Concept

### Progress Hierarchy

```
Blocking        — one thread can block all others (mutex contention)
  ↓
Obstruction-free — a thread makes progress if it runs in isolation (no contention)
  ↓
Lock-free       — system-wide progress: at least one thread completes in finite steps
  ↓
Wait-free       — every thread completes in a bounded number of steps
```

Each level adds a stronger guarantee. The gap between lock-free and wait-free is the widest:
it costs the most in terms of algorithmic complexity and hardware support.

### Wait-Free Defined

An operation is **wait-free** if every invocation completes in a **bounded number of steps**,
regardless of the interleaving with other threads' operations.

Key properties:
- **Starvation-freedom:** no thread can be permanently delayed by other threads.
- **Bounded steps:** the number of loop iterations, CAS attempts, or memory operations per
  call has a fixed upper bound that depends only on the number of threads, not on contention.
- **No retry loops:** a wait-free algorithm cannot spin until a CAS succeeds, because that
  spin could be infinite under adversarial scheduling.

### Common Wait-Free Structures

| Structure | Technique | Bounded? |
|-----------|-----------|----------|
| Atomic counter | `fetch_add` (hardware) | 1 step |
| Atomic register | `store` / `load` (hardware) | 1 step |
| Snapshot of N registers | Double-collect with sequence numbers | O(N) steps |
| Multi-word CAS | LL/SC pair or CAS2 on aligned pairs | 1–2 steps |
| Read–write queue | Single-reader, single-writer FIFO array | 1 step each |

### The Herlihy Consensus Hierarchy

Maurice Herlihy (1991) proved that not all synchronization primitives are equally powerful.
The **consensus number** of a primitive is the maximum number of threads for which that primitive
can solve the consensus problem (all threads agree on one thread's proposed value) with a
wait-free algorithm.

| Level | Consensus Number | Primitives | What you can build |
|-------|-----------------|------------|-------------------|
| 1 | 1 | Atomic read / write registers | Registers, but not wait-free consensus for 2+ threads |
| 2 | 2 | test&set, swap, fetch&add, LL/SC | Wait-free consensus for 2 threads only |
| 3 | ∞ | compare&swap (CAS), load-linked/store-conditional | Wait-free consensus for any number of threads |

**Key result:** An object with consensus number k cannot implement a wait-free object with
consensus number > k using only primitives of consensus number ≤ k.

**Impossibility for arbitrary objects:** To implement a wait-free consensus protocol for an
arbitrary number of threads, you need infinite memory (the protocol must be prepared for any
number of proposals). This means **there is no universal wait-free construction for arbitrary
objects** using only registers and CAS — you need either:
1. A universal primitive with consensus number ∞ (CAS is sufficient for *shared-memory* consensus
   but not for all object types), OR
2. Object-specific knowledge to bound the number of possible states.

### Why This Matters

The hierarchy tells you what is *expressible* at each level:
- With only read/write registers: you can build wait-free counters and snapshot objects,
  but you *cannot* build a wait-free queue for 3+ threads.
- With CAS: you can build wait-free versions of many objects (counter, stack, queue) but
  not *arbitrary* objects — and the constructions are complex.
- The lock-free Treiber stack uses CAS but is not wait-free because the CAS retry loop
  is unbounded.

## Build It

We implement three artifacts in Rust to build intuition for wait-freedom:
1. A **Wait-Free Counter** — trivially wait-free via hardware.
2. A **Wait-Free Snapshot** — wait-free via double-collect.
3. A **Lock-Free Treiber Stack** — for comparison, to show starvation.

All code lives in `code/main.rs`. Run it with `cargo run` (or `rustc code/main.rs && ./main`).

### Step 1: Wait-Free Counter

The simplest wait-free object. On x86-64, `fetch_add` is a single `LOCK XADD` instruction.
It completes in exactly one step regardless of contention.

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

struct WaitFreeCounter {
    value: AtomicUsize,
}

impl WaitFreeCounter {
    fn new(init: usize) -> Self {
        WaitFreeCounter { value: AtomicUsize::new(init) }
    }

    /// Increment and return the previous value.
    /// This is wait-free: exactly 1 atomic step, bounded.
    fn fetch_add(&self, delta: usize) -> usize {
        self.value.fetch_add(delta, Ordering::SeqCst)
    }

    fn load(&self) -> usize {
        self.value.load(Ordering::SeqCst)
    }
}
```

Every call to `fetch_add` takes exactly one hardware instruction. No loops. No retries.
Every thread completes in 1 step. This is the platonic ideal of wait-freedom.

### Step 2: Wait-Free Snapshot (Double-Collect)

A snapshot object lets threads read N atomic registers atomically — they see a consistent
state across all registers that existed at a single point in time.

The **double-collect technique** works as follows:

1. Each register has a sequence number that increments on every write.
2. To snapshot: read all (value, seq) pairs twice, in the same order:
   - First collect: `[(v1, s1), (v2, s2), ..., (vN, sN)]`
   - Second collect: `[(v1', s1'), (v2', s2'), ..., (vN', sN')]`
3. If both collects see the same sequence numbers (`si == si'` for all i), then the first
   collect's values form a consistent snapshot.
4. If not, retry.

The retry count is bounded by the number of concurrent writes. Worst case: N concurrent
writers force O(N) retries. This makes it **wait-free** — bounded by N, not by contention.

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

struct SnapshotRegister {
    value: AtomicUsize,
    seq: AtomicUsize,
}

struct AtomicSnapshot {
    registers: Vec<SnapshotRegister>,
}

impl AtomicSnapshot {
    fn new(values: &[usize]) -> Self {
        let registers = values.iter().map(|&v| SnapshotRegister {
            value: AtomicUsize::new(v),
            seq: AtomicUsize::new(0),
        }).collect();
        AtomicSnapshot { registers }
    }

    fn update(&self, index: usize, new_value: usize) {
        let reg = &self.registers[index];
        reg.value.store(new_value, Ordering::SeqCst);
        reg.seq.fetch_add(1, Ordering::SeqCst);
    }

    fn scan(&self) -> Vec<usize> {
        loop {
            // First collect
            let first: Vec<(usize, usize)> = self.registers.iter().map(|r| {
                (r.value.load(Ordering::SeqCst), r.seq.load(Ordering::SeqCst))
            }).collect();

            // Second collect
            let second: Vec<(usize, usize)> = self.registers.iter().map(|r| {
                (r.value.load(Ordering::SeqCst), r.seq.load(Ordering::SeqCst))
            }).collect();

            // If all sequence numbers match, first collect is consistent
            if first.iter().zip(second.iter()).all(|(a, b)| a.1 == b.1) {
                return first.into_iter().map(|(v, _)| v).collect();
            }
            // Otherwise retry — bounded by number of distinct writes during scan
        }
    }
}
```

**Why it is wait-free:** The loop can iterate at most `N + 1` times where N is the number of
registers, because each iteration requires at least one writer to increment a sequence number
between the two collects, and a writer can overtake the scanner at most once per register.
After N + 1 iterations, the scanner must succeed.

### Step 3: Lock-Free Treiber Stack (For Comparison)

This is the same as Lesson 08's implementation. We include it here to measure starvation.

```rust
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;
use std::mem;

struct TreiberStack<T> {
    head: AtomicPtr<Node<T>>,
}

struct Node<T> {
    value: T,
    next: *mut Node<T>,
}

impl<T> TreiberStack<T> {
    fn new() -> Self {
        TreiberStack { head: AtomicPtr::new(ptr::null_mut()) }
    }

    fn push(&self, value: T) {
        let node = Box::into_raw(Box::new(Node {
            value,
            next: ptr::null_mut(),
        }));
        loop {
            let head = self.head.load(Ordering::Acquire);
            unsafe { (*node).next = head; }
            if self.head.compare_exchange(head, node, Ordering::Release, Ordering::Relaxed).is_ok() {
                return;
            }
            // CAS failed — retry. This is the unbounded loop.
        }
    }

    fn pop(&self) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            if head.is_null() { return None; }
            let next = unsafe { (*head).next };
            if self.head.compare_exchange(head, next, Ordering::Release, Ordering::Relaxed).is_ok() {
                let node = unsafe { Box::from_raw(head) };
                return Some(node.value);
            }
        }
    }
}
```

The loop in `push` and `pop` is **unbounded**. Under contention, a thread may spin forever.
Run the comparison test in `main.rs` with 8 threads hammering the stack — you will see
some threads complete far fewer operations than others.

### Putting It Together

The `main.rs` binary:
1. Spawns 8 threads that each `fetch_add` on a wait-free counter 100,000 times.
   Every thread completes all 100,000 operations — no stragglers.
2. Spawns 8 threads on a Treiber stack doing the same. The distribution of completed
   operations is uneven — some threads starve.

**Expected output pattern:**
```
--- Wait-Free Counter (8 threads, 100k ops each) ---
Thread 0 completed 100000 ops
Thread 1 completed 100000 ops
...
Thread 7 completed 100000 ops
All threads equal. Wait-free guarantee holds.

--- Lock-Free Stack (8 threads, 100k ops each) ---
Thread 0 completed 100000 ops
Thread 1 completed 7321 ops     ← starved!
Thread 2 completed 100000 ops
Thread 3 completed 56002 ops
...
Thread 7 completed 99812 ops
Threads have uneven counts. Lock-free does not prevent starvation.
```

## Use It

### Go's sync.Map

Go's `sync.Map` is **lock-free for reads** (uses atomic loads) but **not wait-free**.
Writes can acquire an internal mutex, and a reader that encounters a concurrent writer
may fall through to a slow path. Under heavy contention, individual goroutines can stall.

### Java's java.util.concurrent

Java's `j.u.c` provides a mix:
- `AtomicInteger`, `AtomicLong` — wait-free increment via `getAndAdd` (uses CAS in a loop
  on most JVMs, so technically lock-free but practically wait-free under light contention).
- `ConcurrentLinkedQueue` — lock-free (Michael-Scott queue variant).
- `ConcurrentHashMap` — lock-free for most reads, but resizes block all threads briefly.
- No structure in `j.u.c` guarantees per-operation bounded steps for all operations.

### Rust's Crossbeam

The `crossbeam-epoch` crate provides lock-free memory reclamation for Treiber stacks
and Michael-Scott queues. Crossbeam's `SegQueue` is lock-free. No production Rust queue
guarantees wait-freedom for all operations — it is too costly for general-purpose use.

### When Wait-Freedom Matters

- **Audio processing:** a buffer underrun from a starved thread causes audible glitches.
- **Real-time trading:** a delayed order can lose money.
- **Game engines:** a physics tick that misses its deadline drops frames.
- **Embedded systems:** missed deadlines can cause physical damage.

In all these cases, you pay the extra complexity cost for the bounded-step guarantee.

## Read the Source

- **Linux kernel `atomic_long.h`:** The `atomic_long_add_return` macro maps to `LOCK XADD`
  on x86 — a hardware wait-free operation for a single register. `include/asm-generic/atomic-long.h`
- **Java `AtomicInteger.java`:** The `getAndAdd` method (Java 9+) uses `VarHandle.getAndAdd`,
  which lowers to `atomic::xadd` in the HotSpot JVM — wait-free on x86.
- **Panagiota Fatourou's wait-free snapshot lecture:** Implements the double-collect algorithm
  with correctness proofs. Search "Wait-Free Snapshots — Fatourou" for the canonical treatment.
- **Herlihy & Shavit, *The Art of Multiprocessor Programming*, Ch. 6–7:** The definitive
  textbook treatment of the consensus hierarchy and universal constructions.
- **Herlihy (1991), "Wait-Free Synchronization":** The seminal paper that introduced the
  consensus hierarchy and proved the impossibility of universal wait-free construction.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained reference snippet** (`main.rs`) implementing a wait-free counter,
  a wait-free snapshot via double-collect, and a lock-free Treiber stack for comparison.
  Reuse the `WaitFreeCounter` and `AtomicSnapshot` types in later phases where you need
  starvation-free concurrent state.

## Exercises

1. **Easy** — Run `main.rs` with 4, 8, and 16 threads. Record the min/mean/max operations
   completed for the wait-free counter vs the lock-free stack. Confirm the counter has zero
   variance.

2. **Medium** — Extend `AtomicSnapshot` to support a `multi_update` that changes k registers
   atomically (all at once). Prove that the double-collect still works by reasoning about
   sequence numbers.

3. **Hard** — Implement a wait-free MPMC queue for 2 producers and 2 consumers using only
   fetch_add and atomic stores (no CAS). Hint: use bounded arrays and ticket-based slots.
   Measure throughput vs a lock-free Michael-Scott queue.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Wait-free | "No waiting" | Every operation completes in a bounded number of steps regardless of other threads' scheduling. The strongest progress guarantee. |
| Bounded steps | "Finite loops" | The number of iterations in a concurrent operation has a fixed upper bound that depends only on N (threads/registers), not on contention. |
| Starvation-freedom | "No thread gets skipped forever" | Every thread that calls an operation eventually completes it. Lock-freedom does NOT imply starvation-freedom. |
| Herlihy consensus hierarchy | "A ranking of primitives" | A classification of synchronization primitives by their consensus number — the maximum number of threads for which they can solve wait-free consensus. |
| Consensus number | "How many can agree" | The largest n such that a primitive can solve wait-free n-thread consensus. CAS has consensus number ∞; read/write registers have consensus number 1. |
| Snapshot | "A point-in-time view" | Reading N atomic registers in a way that yields a consistent state — all values existed simultaneously at some point during the scan. |
| Double-collect | "Read twice, compare" | A wait-free technique: collect all (value, seq) pairs twice; if sequence numbers match, the first collect is a consistent snapshot. |
| Register | "A memory location" | A single word that supports atomic read and write. The weakest synchronization primitive (consensus number 1). |

## Further Reading

- **Herlihy & Shavit, *The Art of Multiprocessor Programming*, Revised First Edition (2012).**
  Chapters 5 (consensus hierarchy) and 6 (universal construction) are the canonical treatment.
- **Herlihy (1991), "Wait-Free Synchronization," *ACM Trans. Program. Lang. Syst.* 13(1): 124–149.**
  The paper that introduced the consensus hierarchy and proved that CAS is universal for
  shared-memory consensus.
- **Afek et al. (1993), "Wait-Free Made Simple."** A simpler presentation of wait-free
  snapshot algorithms with the double-collect technique.
- **Michael & Scott (1996), "Simple, Fast, and Practical Non-Blocking and Blocking
  Concurrent Queue Algorithms."** The original lock-free queue paper — useful for contrast
  with wait-free approaches.
- **Wikipedia: "Wait-freedom"** — a concise overview with links to primary sources.
- **Crossbeam documentation:** The `crossbeam` crate's epoch-based reclamation is the
  closest production-ready Rust gets to wait-free memory management.
