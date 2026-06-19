# Outputs — Condition Variables and Monitors

This directory contains the reusable artifacts from this lesson.

## Artifacts

### Bounded Channel Snippet (C)

File: `../code/main.c` — the `bounded_buffer_t` type and `bb_put`/`bb_get` functions.

A drop-in bounded buffer using `pthread_cond_t` with two condition variables (`can_consume`, `can_produce`). Handles spurious wakeups via `while` loops. Ready to be extracted and used in:

- Phase 14's work-stealing scheduler (worker task queue).
- Any multi-threaded pipeline where producers and consumers run at different rates.

**Usage sketch:**

```c
bounded_buffer_t bb;
bb_init(&bb);

// Producer
bb_put(&bb, my_item);

// Consumer (blocks until data available)
int item = bb_get(&bb);

bb_destroy(&bb);
```

### Bounded Channel Snippet (Rust)

File: `../code/main.rs` — the `BoundedQueue<T>` type.

A type-safe, generic bounded queue using `std::sync::Condvar` with two condition variables. The `push()` and `pop()` methods handle all locking and signaling internally — callers cannot forget to signal.

**Usage sketch:**

```rust
let queue = Arc::new(BoundedQueue::new(10));
queue.push(42);
let item = queue.pop();
```

## Comparison

| Aspect | C (`bounded_buffer_t`) | Rust (`BoundedQueue<T>`) |
|--------|------------------------|-------------------------|
| Lock type | `pthread_mutex_t` | `std::sync::Mutex` |
| CV type | `pthread_cond_t` | `std::sync::Condvar` |
| Spurious wakeup handling | Manual `while` loop | Manual `while` loop |
| API safety | No protection (caller must lock) | Type-safe (lock inside methods) |
| Memory management | Manual `init`/`destroy` | RAII (drop when done) |
| Generic | No (`int` buffer) | Yes (`T`) |
