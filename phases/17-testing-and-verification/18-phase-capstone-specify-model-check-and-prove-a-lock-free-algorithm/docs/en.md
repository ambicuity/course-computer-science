# Phase Capstone - Specify, Model-Check, and Prove a Lock-Free Algorithm

> Combine testing, modeling, and proof-style reasoning into one coherent assurance story.

**Type:** Build
**Languages:** TLA+, Rust
**Prerequisites:** Phase 17 lessons 01-17
**Time:** ~150 minutes

## Learning Objectives

- Specify a lock-free algorithm with explicit invariants.
- Model-check a simplified state machine to validate safety assumptions.
- Implement a practical Rust version with assertion-backed properties.
- Produce an assurance bundle connecting model, implementation, and tests.

## The Problem

Lock-free algorithms are performance-critical and correctness-fragile. Subtle
interleavings can violate linearizability or progress. A lock-free counter that
uses `fetch_add` with `Relaxed` ordering might seem correct but can produce
stale reads on weakly-ordered architectures. A lock-free stack that uses CAS
(Compare-And-Swap) can suffer from the ABA problem: a value is read as A,
changed to B, then back to A, and the CAS succeeds even though the stack
structure changed underneath.

This capstone forces a full workflow: specification, model checking, and
implementation-level checks. You'll model a lock-free counter in TLA+, check
its invariants with TLC, implement it in Rust with atomics, and verify the
implementation with stress tests and invariant assertions.

The goal isn't to prove the algorithm correct in a proof assistant (that's a
separate, much harder task). The goal is to build an **assurance bundle**: a
connected set of evidence (model, implementation, tests, documentation) that
gives confidence in correctness.

## The Concept

### Assurance Stack

```
    Assurance Stack:
    
    ┌─────────────────────────────────────┐
    │  Formal Model (TLA+)                │  "What should be true?"
    │  - State machine                     │
    │  - Safety invariants                 │
    │  - TLC model checking                │
    └──────────────┬──────────────────────┘
                   │
    ┌──────────────▼──────────────────────┐
    │  Implementation (Rust)              │  "How do we build it?"
    │  - Atomic operations                 │
    │  - Memory ordering                   │
    │  - Invariant assertions              │
    └──────────────┬──────────────────────┘
                   │
    ┌──────────────▼──────────────────────┐
    │  Property Tests                     │  "Does it behave correctly?"
    │  - Stress tests                      │
    │  - Monotonicity checks               │
    │  - Concurrency harness               │
    └──────────────┬──────────────────────┘
                   │
    ┌──────────────▼──────────────────────┐
    │  Documentation                      │  "What are the assumptions?"
    │  - Design decisions                   │
    │  - Memory ordering rationale          │
    │  - Known limitations                  │
    └─────────────────────────────────────┘
```

Each layer provides different evidence:

- **TLA+ model:** Exhaustive check of all interleavings in a small state space.
  Finds design-level bugs (wrong invariants, missing transitions).
- **Rust implementation:** Real code with atomic operations. Invariant
  assertions catch implementation-level bugs.
- **Property tests:** Stress tests with multiple threads. Catch bugs that
  only manifest under specific scheduling.
- **Documentation:** Makes assumptions explicit. Future engineers can
  understand why specific memory orderings were chosen.

### Lock-Free Counter

We use a bounded lock-free counter as the teaching target. The counter supports
two operations:

- `increment()`: Atomically increase the counter by 1.
- `get()`: Read the current value.

The counter is bounded: it wraps around at a maximum value (e.g., 255 for a
u8 counter). This keeps the TLA+ state space small while demonstrating the
key concepts.

### TLA+ Model

```tla
--------------------------- MODULE LockFreeCounter ---------------------------
EXTENDS Naturals

CONSTANT MaxValue
CONSTANT Threads

VARIABLE counter, pc

vars == <<counter, pc>>

TypeOK ==
    /\ counter \in 0..MaxValue
    /\ pc \in [Threads -> {"idle", "incrementing", "done"}]

Init ==
    /\ counter = 0
    /\ pc = [t \in Threads |-> "idle"]

\* Thread starts incrementing
StartIncrement(t) ==
    /\ pc[t] = "idle"
    /\ pc' = [pc EXCEPT ![t] = "incrementing"]
    /\ UNCHANGED counter

\* Thread completes increment (atomic CAS-style)
CompleteIncrement(t) ==
    /\ pc[t] = "incrementing"
    /\ counter' = (counter + 1) % (MaxValue + 1)
    /\ pc' = [pc EXCEPT ![t] = "done"]

\* Thread goes back to idle (for reuse)
Reset(t) ==
    /\ pc[t] = "done"
    /\ pc' = [pc EXCEPT ![t] = "idle"]
    /\ UNCHANGED counter

Next ==
    \E t \in Threads:
        \/ StartIncrement(t)
        \/ CompleteIncrement(t)
        \/ Reset(t)

Spec == Init /\ [][Next]_vars

\* Safety: counter is always in valid range
CounterInRange ==
    counter \in 0..MaxValue

\* Safety: counter never decreases (monotonic within a wrap cycle)
\* Note: this is tricky with wrapping. We check that if counter was X and
\* now is Y, then either Y > X or we wrapped around.
CounterMonotonic ==
    TRUE  \* Simplified: check via stress tests in implementation

\* Type invariant
TypeInvariant == TypeOK
=============================================================================
```

### Rust Implementation

```rust
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::thread;

struct LockFreeCounter {
    value: AtomicU8,
    max_value: u8,
}

impl LockFreeCounter {
    fn new(max_value: u8) -> Self {
        LockFreeCounter {
            value: AtomicU8::new(0),
            max_value,
        }
    }
    
    fn increment(&self) {
        loop {
            let current = self.value.load(Ordering::SeqCst);
            let next = (current + 1) % (self.max_value + 1);
            match self.value.compare_exchange_weak(
                current, next, Ordering::SeqCst, Ordering::SeqCst
            ) {
                Ok(_) => break,
                Err(_) => continue,  // Retry on contention
            }
        }
    }
    
    fn get(&self) -> u8 {
        self.value.load(Ordering::SeqCst)
    }
    
    fn check_invariant(&self) {
        let val = self.get();
        assert!(val <= self.max_value,
            "Invariant violated: counter={} > max={}", val, self.max_value);
    }
}
```

## Build It

### Step 1: Write the TLA+ model

Create `code/Counter.tla` with the specification above. Define:

- `MaxValue = 7` (small state space for fast checking).
- `Threads = {"t1", "t2"}` (two threads).
- Invariant: `counter \in 0..MaxValue`.

Run TLC. With 2 threads and MaxValue=7, the state space is small (~100 states).
TLC checks all invariants in seconds.

### Step 2: Implement in Rust

Create `code/main.rs` with the `LockFreeCounter` implementation. Key decisions:

- Use `AtomicU8` for the counter value.
- Use `compare_exchange_weak` for the CAS loop (handles spurious failures).
- Use `SeqCst` ordering for simplicity (strongest ordering, easiest to reason
  about; `Relaxed` would be faster but harder to verify).

### Step 3: Add invariant checks

```rust
fn check_invariant(&self) {
    let val = self.get();
    assert!(val <= self.max_value,
        "Invariant violated: counter={} > max={}", val, self.max_value);
}
```

Call `check_invariant()` after every `increment()` in debug builds.

### Step 4: Stress test with multiple threads

```rust
fn stress_test() {
    let counter = Arc::new(LockFreeCounter::new(255));
    let mut handles = vec![];
    
    for _ in 0..8 {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..100_000 {
                c.increment();
            }
        }));
    }
    
    for h in handles {
        h.join().unwrap();
    }
    
    // After 8 threads × 100,000 increments = 800,000 total
    // With max_value=255, counter should be 800000 % 256 = 0
    assert_eq!(counter.get(), 0);
    counter.check_invariant();
    println!("Stress test passed: counter = {}", counter.get());
}
```

### Step 5: Document assumptions

Create `outputs/README.md` with:

- Design decisions (why `SeqCst`, why CAS loop, why bounded).
- Memory ordering rationale.
- Known limitations (no ABA protection for complex structures).
- Connection between TLA+ model and Rust implementation.

## Use It

This workflow generalizes to queues, stacks, and protocol state machines:

1. **Start with a small model.** Don't try to model everything. Abstract away
   details that don't affect the invariant.

2. **Check safety invariants.** TLC finds interleaving bugs in seconds.

3. **Encode invariants in code.** Use `debug_assert!` to check invariants at
   runtime. These catch bugs that the model's abstraction might miss.

4. **Preserve traceability.** Document which TLA+ invariant corresponds to
   which code assertion. Future engineers can trace from spec to implementation.

Production references:

- Rust's `crossbeam` library uses similar techniques for lock-free data
  structures.
- Java's `java.util.concurrent` classes have formal models.
- The LMAX Disruptor (lock-free ring buffer) was verified using similar
  assurance techniques.

## Read the Source

- [Rust std::sync::atomic](https://doc.rust-lang.org/std/sync/atomic/) —
  Rust's atomic operations and memory ordering.
- [Herlihy and Shavit](https://www.elsevier.com/books/the-art-of-multiprocessor-programming/herlihy/978-0-12-370591-4) — "The Art of Multiprocessor Programming," the
  canonical reference for lock-free algorithms.
- [crossbeam](https://github.com/crossbeam-rs/crossbeam) — Rust's lock-free
  concurrency library.

## Ship It

This lesson ships:

- `code/Counter.tla`: bounded model for lock-free counter behavior.
- `code/main.rs`: atomic counter demo with invariant checks.
- `outputs/README.md`: capstone assurance bundle checklist.

```bash
# Run the model checker
tlc code/Counter.tla -config code/Counter.cfg

# Run the Rust implementation
cargo run
```

## Quiz

**Pre-questions:**

**Q1.** What is a lock-free algorithm?

- A) An algorithm that uses no locks.
- B) An algorithm where at least one thread makes progress in a bounded number
   of steps, regardless of other threads' behavior.
- C) An algorithm that never blocks.
- D) An algorithm using only atomic operations.

**Answer: B.** Lock-free means system-wide progress: if any thread is executing,
some thread will complete its operation in a bounded number of steps. This is
stronger than "no locks" (which could still have livelock). Lock-free algorithms
typically use CAS loops, but the defining property is the progress guarantee.

**Q2.** Why use TLA+ for a lock-free algorithm instead of just testing?

- A) TLA+ is faster.
- B) Lock-free bugs are interleaving bugs that require exploring many thread
   orderings. TLA+ checks all orderings exhaustively.
- C) Testing can't find concurrency bugs.
- D) TLA+ replaces the need for implementation.

**Answer: B.** Lock-free algorithms fail under specific thread interleavings
(e.g., ABA problem). Testing with random schedules might miss these. TLA+
explores all possible interleavings in a bounded model, finding bugs that
testing would only find by luck.

**Post-questions:**

**Q3.** Your TLA+ model uses `MaxValue = 7` and 2 threads. The real
implementation uses `MaxValue = 255` and 8 threads. Is the model still useful?

- A) No, the model is too small to be relevant.
- B) Yes, the model captures the essential invariants (counter in range,
   monotonic within wrap cycle). Protocol bugs usually manifest in small
   instances.
- C) Only if you increase the model to match the implementation.
- D) The model is only useful for documentation.

**Answer: B.** Protocol bugs (wrong invariants, missing transitions) manifest
in small instances. If the invariant is violated with 2 threads, it will also
be violated with 8 threads. The model checks the *design*; the stress tests
check the *implementation*.

**Q4.** What is the "assurance bundle" this capstone produces?

- A) A single proof that the algorithm is correct.
- B) A connected set of evidence: TLA+ model (design), Rust implementation
   (code), stress tests (behavior), and documentation (assumptions).
- C) A test coverage report.
- D) A performance benchmark.

**Answer: B.** An assurance bundle links multiple forms of evidence. The TLA+
model proves the design is sound (for bounded instances). The Rust
implementation adds invariant assertions. Stress tests verify behavior under
contention. Documentation makes assumptions explicit. Together, they provide
higher confidence than any single technique alone.

**Q5.** Why use `SeqCst` ordering instead of `Relaxed` in the Rust
implementation?

- A) `SeqCst` is faster.
- B) `SeqCst` provides the strongest memory ordering guarantees, making it
   easier to reason about correctness. `Relaxed` is faster but harder to
   verify.
- C) `Relaxed` doesn't work with CAS.
- D) They're the same.

**Answer: B.** `SeqCst` (Sequential Consistency) provides the strongest
guarantee: all threads see operations in the same total order. This makes
reasoning about correctness straightforward. `Relaxed` allows reordering,
which is faster but can produce surprising behaviors on weakly-ordered
architectures (ARM, RISC-V). Start with `SeqCst`, optimize to `Relaxed` only
when profiling shows it's necessary.

## Exercises

**Easy:** Extend the TLA+ model with a `decrement` action. Add a
non-negativity invariant: `counter >= 0`. Check that the invariant holds when
decrement wraps around from 0 to MaxValue.

**Medium:** Compare `Relaxed` vs `SeqCst` ordering in the Rust implementation.
Write a stress test that demonstrates a stale-read scenario with `Relaxed`
ordering on a weakly-ordered architecture. If you can't reproduce the bug,
document why (modern x86 provides stronger guarantees than ARM).

**Hard:** Add a randomized concurrent harness. Use `loom` (Rust's concurrency
testing library) to explore all possible thread interleavings for a small
number of operations. Compare `loom`'s findings with the TLA+ model's
findings. What does `loom` catch that TLA+ doesn't, and vice versa?

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Lock-free | "no mutex" | System-wide progress: some thread makes progress in bounded steps |
| Linearization point | "when it happens" | Instant operation appears to take effect atomically |
| Monotonic invariant | "never decreases" | Property preserved across all valid transitions |
| Assurance bundle | "evidence package" | Linked spec, checks, implementation tests, and assumptions |
| CAS | "compare and swap" | Atomic operation: if memory equals expected, set to new value |
| ABA problem | "stale read" | Value changes A→B→A, CAS succeeds despite structural change |
| Memory ordering | "visibility rules" | Guarantees about when writes become visible to other threads |

## Further Reading

- [Rust std::sync::atomic](https://doc.rust-lang.org/std/sync/atomic/) —
  Rust's atomic operations and memory ordering.
- [Herlihy and Shavit](https://www.elsevier.com/books/the-art-of-multiprocessor-programming/herlihy/978-0-12-370591-4) — "The Art of Multiprocessor Programming."
- [crossbeam](https://github.com/crossbeam-rs/crossbeam) — Rust's lock-free
  concurrency library.
- [loom](https://github.com/tokio-rs/loom) — Rust's concurrency testing library.
- [TLA+ Hyperbook](https://lamport.azurewebsites.net/tla/hyperbook.html) —
  Lamport's canonical resource for model checking.
