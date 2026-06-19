# Outputs — Futures, Promises, async/await

## Build & Run

### Rust

```bash
cd code
rustc main.rs -o futures_lesson
./futures_lesson
```

*Requires:* Rust compiler (rustc). No external dependencies.

### TypeScript

```bash
cd code
node main.ts
```

*Requires:* Node.js ≥ 14 (for `queueMicrotask` and `Promise.allSettled`).

## Expected Output

### Rust (`main.rs`)

```
=== Phase 13.11: Futures, Promises, async/await (Rust) ===

--- Step 1: Manual Callback-Based Future ---
  Manual future completed in 102.345ms
  (spawns real OS thread — wasteful but pedagogically clear)

--- Step 2: Async/Await Rewrite ---
    fetch_data(42): starting
    fetch_data(42): complete -> data-42
  Result: data-42 in 51.234ms
  (zero-cost: no heap alloc for state machine)

--- Step 3: Custom Toy Executor with Waker ---
    toy_task(1): spawned
    toy_task(3): spawned
    toy_task(2): spawned
    toy_task(3): done after 10ms
    toy_task(2): done after 20ms
    toy_task(1): done after 30ms
  All toy tasks completed in 31.456ms
  (total time ≈ max duration, not sum — concurrent polling!)

--- Step 4a: Join (run concurrently, collect all) ---
  Results: [2, 4, 6] in 92.345ms
  (sequential would take ~180ms, concurrent took ~90ms)

--- Step 4b: Select (race futures, pick first) ---
  Winner: fast in 21.234ms
  (the slower future is abandoned)

--- Step 4c: Error Handling ---
  ok=computation succeeded, fail=fallback value
  (? operator works in async fns returning Result)

=== All steps completed. ===
Key insight: Rust futures are *lazy* — nothing runs without poll().
The toy executor reveals that async/await is just syntax for
state machines driven by a poll loop with wake notifications.
```

### TypeScript (`main.ts`)

```
=== Phase 13.11: Futures, Promises, async/await (TypeScript) ===

--- Step 1: Manual Callback-Based Promise ---
  SimplePromise resolved after 50ms
  (SimplePromise executor runs synchronously in constructor)

--- Step 2: Async/Await Rewrite ---
    fetchData(42): starting
    fetchData(42): complete -> data-42
  Result: data-42 in 52ms
  (async/await desugars to .then() chains)

--- Step 3: Microtask vs Macrotask ---
  1: synchronous start
  3: microtask (Promise.then)
  4: microtask (queueMicrotask)
  2: after await (microtask)
  5: macrotask (setTimeout)

--- Step 4a: Promise.all (run concurrently, collect all) ---
  Results: [2,4,6] in 91ms
  (sequential would take ~180ms, concurrent took ~90ms)
  Promise.all fail-fast: caught boom

--- Step 4b: Promise.race (first settles wins) ---
  Winner: fast in 21ms
  (the slower promise continues but its result is ignored)
  Timeout pattern: timeout after 30ms

--- Step 4c: Promise.allSettled (wait for all, collect all) ---
  Fulfilled: ok
  Rejected: Error: fail
  Fulfilled: delayed ok

--- Step 4d: Error Handling ---
  Success: computation succeeded
  After catch: fallback: computation failed
  Isolated errors: a=default-a, b=default-b

--- Step 4e: Async Iterator (bonus) ---
  Async iterated: [1,2,3] (each yielded after 10ms)

=== All steps completed. ===
Key insight: JavaScript Promises are *eager* — the executor
runs immediately in the constructor. Async/await is syntactic
sugar over .then() chains, scheduled as microtasks on the event loop.
```

## Reference

| File | Language | Lines | What it demonstrates |
|------|----------|-------|---------------------|
| `code/main.rs` | Rust | ~310 | Manual future, async fn, toy executor with waker + Condvar, join_all, select_first, error handling |
| `code/main.ts` | TypeScript | ~250 | SimplePromise polyfill, async/await, microtask scheduling, Promise.all/race/allSettled, error handling, async iterator |

### Key API Surface

- **Rust:** `Future` trait, `poll()`, `Pin`, `Context`, `Waker`, `RawWaker`, `async fn`, `await`, `Box::pin`, `thread::spawn` + channels for toy join/select
- **TypeScript:** `Promise` constructor, `.then()`, `.catch()`, `async/await`, `Promise.all()`, `Promise.race()`, `Promise.allSettled()`, `queueMicrotask`, `AsyncGenerator`, `for await...of`

### Use This When: 

- **Lesson 12** (Reactor/Proactor): contrast the toy executor's busy-spin loop with epoll/kqueue-based reactors
- **Lesson 13** (Tokio): compare Tokio's `spawn` + `block_on` to the toy executor's `Spawner` + `run`
- Building any async system: the executor pattern is universal — poll, waker, reactor, repeat
