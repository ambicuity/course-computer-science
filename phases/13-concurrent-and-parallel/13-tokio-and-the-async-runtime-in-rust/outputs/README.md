# Output: Tokio and the Async Runtime in Rust

## Artifact

A self-contained async demonstration covering Tokio's core primitives: task spawning, TCP I/O, shared-state synchronization, concurrent execution combinators, and custom runtime configuration.

## Files

| File | Description |
|------|-------------|
| `../code/src/main.rs` | Rust: tokio::spawn, TCP echo server (TcpListener), shared state (Mutex, RwLock, mpsc, broadcast), join!/select!, custom runtime builder |
| `../code/Cargo.toml` | Package manifest with `tokio = { version = "1", features = ["full"] }` |

## Usage

```bash
cd code
cargo run
```

Requires Rust 1.70+ and a working networking stack (loopback interface at 127.0.0.1).

## Key Design Decisions

1. **Port 0 binding.** The TCP echo server binds to port 0, letting the OS pick a free port. This avoids conflicts with existing services. The address is printed at startup.

2. **Task-per-connection model.** Each accepted TCP connection is dispatched to its own lightweight task. This is the standard Tokio pattern for network services — tasks cost ~µs to spawn, unlike OS threads.

3. **tokio::sync over std::sync.** The lesson uses `tokio::sync::Mutex` (not `std::sync::Mutex`) because its async `lock()` is safe to hold across `.await` points. `std::sync::MutexGuard` is not `Send` and would cause a compile error if held across `.await`.

4. **Custom runtime builder.** A multi-threaded runtime is built with explicit `worker_threads(4)` and `event_interval(61)`, demonstrating that the default `#[tokio::main]` is just one possible configuration.

## Integration

- The **echo server pattern** (`TcpListener` + accept loop + per-connection spawn) is the standard way to write async TCP services in Rust.
- The **shared-state patterns** (Mutex for counters, mpsc for producer-consumer, broadcast for fan-out) apply directly to any Tokio-based application.
- The **join!/select! combinators** are the primary tools for orchestrating concurrent async work.
- The **runtime builder** is useful when you need fine-grained control over thread count, thread naming, or the I/O event polling interval.
