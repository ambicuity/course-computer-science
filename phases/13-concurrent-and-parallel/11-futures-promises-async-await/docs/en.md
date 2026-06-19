# Futures, Promises, async/await

> A future is a placeholder for a value that hasn't arrived yet. A promise is the mechanism that delivers it. Async/await is the syntax that makes it readable. Together they turn callback spaghetti into sequential-looking code — and they're the foundation of every modern I/O runtime.

**Type:** Build
**Languages:** Rust, TypeScript
**Prerequisites:** Phase 9 (closures, traits), Phase 12 (OS, I/O)
**Time:** ~75 minutes

## Learning Objectives

- Explain what a future/promise is and why it solves callback hell.
- Distinguish lazy (Rust) vs eager (JavaScript) evaluation of futures.
- Implement a minimal manual future using callbacks in Rust and TypeScript.
- Rewrite the manual future using async/await syntax.
- Build a toy executor in Rust that polls futures to completion.
- Compose multiple futures using `join!`, `select!` (Rust) and `Promise.all` (TypeScript).
- Handle errors in async code with `?` (Rust) and `.catch`/try-await (TypeScript).
- Describe the roles of executor, reactor, poll, waker, Pin, and the event loop.

## The Problem

Consider reading from two network sockets and combining the results. In a synchronous world you block until each finishes — wasting CPU while waiting. In a threaded world you spawn one thread per socket — wasting memory on stack per connection. The sweet spot is **non-blocking I/O** with **concurrent tasks**: start both reads, do other work while they're in flight, and collect the results when both arrive.

Before futures, this meant callbacks:

```javascript
readSocket(socket1, (err, data1) => {
  if (err) return handleError(err);
  readSocket(socket2, (err, data2) => {
    if (err) return handleError(err);
    combine(data1, data2);
  });
});
```

This is two nested calls. Real systems chain a dozen. Error handling tangles every level. This is **callback hell** — and futures are the escape.

A **future** (or **promise**) is a value that will be available *later*. You get back a placeholder immediately, attach a continuation, and the runtime calls your continuation when the value arrives. Instead of nesting callbacks, you *chain* or *compose* futures.

### Lazy vs Eager

| Language | Evaluation | First polled/awaited | Standard library |
|----------|------------|---------------------|------------------|
| Rust | Lazy | Nothing happens until `poll` or `.await` | `std::future::Future` |
| JavaScript | Eager | A `Promise` executor runs immediately | `Promise` built-in |

Rust's futures do nothing until polled — they are inert state machines. JavaScript's promises run the executor function as soon as they are created. This is the single most important difference to keep in mind.

## The Core Abstractions

### Rust: `Future` trait

```rust
trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

enum Poll<T> {
    Ready(T),
    Pending,
}
```

- `poll` returns `Ready(value)` when complete, `Pending` otherwise.
- `Pin` ensures the future is not moved after first poll (self-referential structs in async state machines).
- `Context` contains a `Waker` that the future calls when it is ready to make progress.

### TypeScript: `Promise`

```typescript
class Promise<T> {
  constructor(executor: (resolve: (value: T) => void, reject: (reason?: any) => void) => void);
  then<TResult>(onFulfilled?: (value: T) => TResult): Promise<TResult>;
  catch(onRejected?: (reason: any) => void): Promise<T>;
  finally(onFinally?: () => void): Promise<T>;
}
```

- The executor runs *immediately* (eager).
- `.then` registers a callback for when the promise settles.
- `.catch` registers a callback for rejection.
- Promises are scheduled as **microtasks** on the event loop — they run before the next macrotask (setTimeout, I/O).

### The Event Loop (JavaScript)

```
┌───────────────────────────┐
│         macrotasks        │  ← setTimeout, setInterval, I/O callbacks
│   ┌─────────────────┐     │
│   │   microtasks     │     │  ← Promise.then, queueMicrotask, async/await continuations
│   └─────────────────┘     │
└───────────────────────────┘
       ↑ event loop tick
```

1. Execute one macrotask (oldest callback in the macrotask queue).
2. Drain the entire microtask queue.
3. Render (if needed).
4. Repeat.

This means `Promise.then` callbacks always run **before** the next `setTimeout` callback, even if the promise resolved synchronously.

### Executor and Reactor (Rust)

- **Executor**: polls futures that are ready to make progress. Tokio's executor runs on a thread pool.
- **Reactor**: registers interest in I/O events (epoll, kqueue, io_uring) and wakes futures when events arrive.
- **Waker**: a clonable handle that the future stores. When the future can make progress (e.g., socket data arrived), it calls `waker.wake()` to tell the executor to re-poll it.

## Build It

Open `code/main.rs` and `code/main.ts` alongside this lesson.

### Step 1: Manual Future (Callback-Based)

**Rust** (`code/main.rs:1`–`80`):

We define a `ManualFuture` that takes a callback and calls it when the value is ready. This is not zero-cost — it heap-allocates and uses dynamic dispatch — but it mirrors what async/await does at a higher level. We run it with a simple `block_on` loop that repeatedly checks if the value is ready.

Key details:
- The future wraps a `Cell<Option<T>>` to store the result.
- `block_on` calls `poll` in a busy loop (no waker — we add that in Step 3).
- The callback is stored as `Box<dyn FnOnce(T)>` — a heap-allocated closure.

**TypeScript** (`code/main.ts:1`–`70`):

We implement a minimal `SimplePromise` class. The executor receives `resolve` and `reject` callbacks. `.then` registers a handler that fires when the promise settles. This is essentially how `Promise` worked in early polyfills.

Key details:
- We use a `status` enum (`Pending | Resolved | Rejected`).
- `.then` either queues the handler (if pending) or calls it immediately (if already settled).
- The executor runs synchronously in the constructor (eager — like real JavaScript promises).

### Step 2: Async/Await Rewrite

**Rust** (`code/main.rs:81`–`120`):

Rewrite the manual future to use `async fn`. Rust desugars `async fn` into a state machine struct that implements `Future`. Each `.await` point becomes a state transition. The resulting struct is lazy — nothing runs until polled.

```rust
async fn fetch_data(id: u32) -> String {
    simulate_work(id).await;
    format!("data-{}", id)
}
```

**TypeScript** (`code/main.ts:71`–`120`):

Rewrite `SimplePromise` usage with `async/await`. The `async` keyword wraps the return value in a `Promise`. `await` suspends the function until the promise settles, without blocking the thread.

```typescript
async function fetchData(id: number): Promise<string> {
    await simulateWork(id);
    return `data-${id}`;
}
```

### Step 3: Custom Executor in Rust

**Rust** (`code/main.rs:121`–`240`):

We build a toy executor:

1. **`Spawner`** — queues futures onto a shared channel.
2. **`Executor`** — runs a loop: dequeue a future, poll it, if `Pending` re-queue it.
3. **Waker** — a simple counter-based waker: `wake()` increments a shared atomic counter. The executor checks the counter and re-polls all pending futures.

This is *not* efficient (busy-polling without epoll) — but it reveals exactly how executors work:

```rust
struct ToyExecutor {
    ready_queue: Arc<Mutex<VecDeque<Box<dyn Future<Output = ()> + Send>>>>,
}
impl ToyExecutor {
    fn run(&self) {
        loop {
            let mut queue = self.ready_queue.lock().unwrap();
            if let Some(mut fut) = queue.pop_front() {
                let waker = noop_waker();
                let mut cx = Context::from_waker(&waker);
                let _ = fut.as_mut().poll(&mut cx);
            } else {
                break; // nothing to do
            }
        }
    }
}
```

We incrementally improve it:
- Add `Arc<AtomicU64>` waker that counts wake calls.
- The executor drain loop re-polls futures whose waker fired.
- Add `spawn` / `block_on` helpers.

### Step 4: Error Handling and Composition

**Rust** (`code/main.rs:241`–`320`):

- `?` operator in async fn: propagates errors through `Result`.
- `join!` runs two futures concurrently on the same executor.
- `select!` runs two futures and picks whichever finishes first (with `tokio::select!`-like logic in our toy executor).

```rust
async fn fetch_both() -> (String, String) {
    let f1 = fetch_data(1);
    let f2 = fetch_data(2);
    futures::join!(f1, f2)  // in our toy: join_all
}
```

**TypeScript** (`code/main.ts:121`–`250`):

- `try/catch` with `await` for error handling.
- `Promise.all([p1, p2])` for concurrent composition.
- `Promise.race([p1, p2])` for select-style composition.
- `Promise.allSettled` for collecting results regardless of rejection.

```typescript
async function fetchBoth(): Promise<[string, string]> {
    const [a, b] = await Promise.all([fetchData(1), fetchData(2)]);
    return [a, b];
}
```

## Use It

### Rust: `std::future` and Tokio

- `tokio::spawn` queues a future on Tokio's multi-threaded executor.
- `tokio::time::sleep` returns a future that completes after a duration.
- `tokio::net::TcpStream` methods (`read`, `write`) return futures that complete when I/O is ready.
- The Tokio reactor uses `epoll` (Linux) or `kqueue` (macOS) to wait for I/O events efficiently.

### TypeScript: Event Loop Patterns

- Use `Promise.all` when tasks are independent — all run concurrently.
- Use `Promise.race` when you need the first result (e.g., timeout guard).
- Use `Promise.allSettled` when you need all results including failures.
- Avoid mixing `.then` and `await` in the same function — pick one style.
- Never pass `async` functions as event listeners without error handling — unhandled rejections crash Node.js.

### Common Pitfalls

| Pitfall | Rust | TypeScript |
|---------|------|------------|
| Blocking the executor | Calling `thread::sleep` inside `.await` stalls the thread | A synchronous `while(true)` blocks the event loop |
| Forgetting to `.await` | `let x = fut;` → `x` is a Future, not the value | `let x = promise;` → `x` is a Promise, not the value |
| Deadlocked executor | Spawning a future that waits for another future on the same single-threaded executor | `await` in a `Promise` constructor executor |
| Unpin errors | "`Future` is not `Unpin`" — use `Box::pin` or `pin_mut!` | N/A (JS heap-allocates everything) |
| Unhandled rejection | Cancelled via `Drop` (no leak) | Missing `.catch` → Node warns, process may exit |

## Read the Source

- [Rust Async Book](https://rust-lang.github.io/async-book/) — The official guide to async/await in Rust. Chapters 1-4 cover futures, wakers, and executors. Chapter 4 walks through building a toy executor exactly like the one in this lesson.
- [JavaScript Promise Reference (MDN)](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Promise) — The canonical reference for `Promise`, `async`, `await`, `Promise.all`, `Promise.race`, `Promise.allSettled`.
- [JavaScript Event Loop (MDN)](https://developer.mozilla.org/en-US/docs/Web/JavaScript/EventLoop) — How the event loop works: macrotasks, microtasks, and rendering.
- [C# Async/await (Microsoft)](https://docs.microsoft.com/en-us/dotnet/csharp/programming-guide/concepts/async/) — The original async/await design. C# introduced the pattern in 2012, and Rust's version is heavily inspired by it.
- ["Zero-Cost Futures in Rust" (withoutboats)](https://without.boats/blog/zero-cost-futures/) — Explains why Rust's futures are lazy and what "zero-cost" means: no heap allocation, no dynamic dispatch for the state machine.
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) — The most popular Rust async runtime. The "spawning" and "select" chapters are directly relevant to this lesson.

## Ship It

The reusable artifact from this lesson lives in `outputs/`. It is:

- **A minimal async executor + composed async tasks in Rust** — `outputs/README.md` documents the build and run steps, the output of `main.rs` (the toy executor running composed futures), and the output of `main.ts` (the promise composition). Use this as a reference when:
  - Lesson 12 (Reactor/Proactor): contrast this toy executor with epoll/kqueue-based reactors.
  - Lesson 13 (Tokio): compare Tokio's executor to the toy one here.
  - Building any async system: remember that an executor is just a loop that polls futures.

## Exercises

1. **Easy** — In `main.rs`, add a third `fetch_data` call and compose all three with `join_all`. Verify all three complete concurrently (the total time is the max of the three, not the sum).

2. **Medium** — In `main.ts`, write a `timeout` function that takes a `Promise<T>` and a `ms` number, returning `Promise<T | "timeout">` using `Promise.race`. Then write a test that verifies a slow promise triggers the timeout.

3. **Hard** — In `main.rs`, modify the `ToyExecutor` to *not* busy-loop. Instead, use a `Condvar` (or `std::sync::mpsc`) to put the executor thread to sleep when there are no ready futures, and have the waker wake it via `Condvar::notify_one`. Then add a `spawn` that sends a future plus its waker to the executor thread. This is closer to how Tokio's single-threaded executor works.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Future | "A value that hasn't arrived yet" | A trait (Rust) or object (JS) representing an asynchronous computation that will produce a value later. In Rust it is lazy (nothing happens until polled). |
| Promise | "A future's counterpart" | The *producer* side of a future. In JS, `Promise` is both future and promise. In Rust, you typically use a `oneshot` channel. |
| Async | "Makes a function asynchronous" | A keyword that transforms a function into one returning a future. The body is compiled into a state machine. |
| Await | "Wait for a future without blocking" | A suspension point: yields control back to the executor, which can run other futures until the awaited future is ready. |
| Executor | "The runtime that runs futures" | A loop that polls futures. When a future returns `Pending`, the executor moves on to another future. |
| Reactor | "The I/O event source" | A component that registers interest in I/O events (epoll, kqueue, iocp) and wakes futures when events occur. |
| Poll | "Check if a future is done" | The method on `Future` that advances the state machine. Returns `Ready(value)` or `Pending`. |
| Waker | "Notify the executor" | A handle that a future saves. Calling `waker.wake()` tells the executor to re-poll this future. |
| Pin | "Don't move me" | A Rust type that guarantees a value won't be moved in memory, required for self-referential async state machines. |
| Event loop | "The JS main loop" | A single-threaded loop that processes macrotasks (I/O, timers) and between each macrotask drains the microtask queue (Promise callbacks). |
| Microtask | "A Promise callback" | A callback queued by `Promise.then` or `queueMicrotask`. Microtasks run before the next macrotask. |
| Callback hell | "Nested async callbacks" | The pattern of deeply nested callback functions that makes error handling and control flow unreadable. Futures solve this. |

## Further Reading

- Steve Klabnik and Carol Nichols, *The Rust Programming Language* (2nd ed.) — Chapter 16 covers concurrency (threads, message passing, shared state). The async/await chapter in the online edition covers futures, `Pin`, and executors at a gentler pace than the Async Book.
- Douglas Crockford, *JavaScript: The Good Parts* — While it predates promises, Crockford's discussion of callbacks and the event loop in Chapter 8 is still the clearest explanation of why JavaScript needs concurrency patterns.
- Martin Kleppmann, *Designing Data-Intensive Applications* — Chapter 12 (Future of Data Systems) discusses how async/await and futures compose in distributed systems. The treatment of "executor" and "reactor" in the context of distributed transactions is illuminating.
- Robert Nystrom, *Game Programming Patterns* — The "Event Loop" and "Game Loop" chapters provide a practical explanation of why you would structure code around non-blocking patterns. The "Update Method" pattern is essentially a manual future.
- "Futures and Promises" on Wikipedia — The history: the term was coined in the 1970s in the context of multi-threaded programming (Baker and Hewitt). The modern async/await form was popularized by C# in 2012.
