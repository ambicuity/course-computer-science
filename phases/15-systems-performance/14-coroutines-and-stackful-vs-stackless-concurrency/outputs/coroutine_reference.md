# Coroutine Quick Reference

## Stackful vs Stackless

| Property | OS Thread | Stackful (Goroutine) | Stackless (C++/Rust) |
|----------|-----------|-----------------------|----------------------|
| Stack/frame | 1–8 MB | 4 KB initial, grows | 48–256 B frame |
| Switch cost | 1–10 μs | ~100 ns | ~10 ns |
| Suspend depth | Any call depth | Any call depth | Only at await/yield |
| Creation cost | ~50 μs | ~0.3 μs | ~0.05 μs |
| Max concurrent | ~10K | ~1M+ | ~10M+ |
| Debug stack | Full | Full per goroutine | Fragmented |

## C++20 Coroutine Keywords

- **`co_await expr`** — Suspend coroutine, resume when awaitable completes. `expr` must support `operator co_await` or be an awaitable with `await_ready`, `await_suspend`, `await_resume`.
- **`co_yield expr`** — Suspend and emit a value. Equivalent to `co_await promise.yield_value(expr)`.
- **`co_return expr`** — Complete the coroutine and return a value. Calls `promise.return_value(expr)`.

### promise_type Hooks

| Hook | Purpose |
|------|---------|
| `get_return_object()` | Creates the caller-facing handle |
| `initial_suspend()` | `suspend_always` = lazy start; `suspend_never` = eager start |
| `final_suspend()` | `suspend_always` = keep frame for result; `suspend_never` = destroy immediately |
| `yield_value(v)` | Handle `co_yield` — store value, return suspend point |
| `return_value(v)` | Handle `co_return` — store final value |
| `unhandled_exception()` | Handle exceptions thrown in coroutine body |

## Rust Async Primitives

### Future Trait
```rust
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

### Poll Enum
```rust
pub enum Poll<T> {
    Ready(T),    // Future completed with value
    Pending,     // Not ready — Waker will be called when progress possible
}
```

### Pin
- `Pin<P>` prevents the pointed-to value from being moved.
- Essential for self-referential async state machines.
- `Unpin` types can be freely moved even when pinned (most types are `Unpin` by default).
- `Pin` only matters for types that contain self-references, which async state machines may have.

### Waker
- Passed via `Context` to `Future::poll`.
- When a future returns `Pending`, it must arrange for `Waker::wake()` to be called later.
- The executor uses `Waker` to know when to re-poll — no wasted CPU cycles.

## Decision Flowchart

```
Is your workload I/O-bound (>90% waiting)?
├─ Yes ── Use stackless coroutines (async/await)
│         Minimal memory, fast switching, millions of concurrent tasks.
├─ No ── Is it CPU-bound (>90% computing)?
│  ├─ Yes ── Use OS threads or rayon-style parallelism.
│  │         Full core utilization, no scheduling overhead.
│  └─ No (mixed) ── Use async for I/O + thread pool for CPU.
│                    e.g., tokio::spawn_blocking for CPU work.
└─ Need to suspend in deep call stacks?
   └─ Yes ── Use stackful coroutines (goroutines, fibers).
             Stackless can only suspend at await points in the async chain.
```

## Stack Size Quick Math

```
100K concurrent tasks:
  OS threads:   100K × 8 MB  = 800 GB   ← impossible
  goroutines:   100K × 4 KB  = 400 MB   ← feasible
  C++/Rust:     100K × 64 B  = 6.4 MB   ← trivial
```

## Async Runtimes

| Runtime | Language | Model | Notes |
|---------|----------|-------|-------|
| Tokio | Rust | Multi-threaded work-stealing | Industry standard, `#[tokio::main]` |
| Glommio | Rust | Thread-per-core | Datadog, high cache affinity |
| asyncio | Python | Single-threaded event loop | Cooperative, GIL-limited |
| Go runtime | Go | M:N scheduler | Goroutines on N OS threads, preemptive since 1.14 |
| libunwind/fiber | C++ | Stackful fibers | Boost.Asio, Facebook folly |

## Key Terms at a Glance

| Term | One-line definition |
|------|-------------------|
| Coroutine | Function that can suspend and resume cooperatively |
| Stackful | Has own call stack, can suspend from any depth |
| Stackless | Compiler state machine, heap frame, suspend only at await |
| `co_await` | Suspend until awaitable completes |
| `co_yield` | Suspend and emit a value |
| `promise_type` | C++20 coroutine customization point |
| `Pin` | Rust wrapper preventing value moves |
| `Future::poll` | Check completion, register Waker if Pending |
| `Waker` | Callback to signal executor to re-poll |
| Goroutine | Go's stackful coroutine with growable stacks |
| Green thread | User-space thread managed by runtime, not OS |