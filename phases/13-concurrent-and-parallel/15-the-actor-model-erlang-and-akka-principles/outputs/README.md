# Actor Model Reference — Erlang & Rust

**Phase 13, Lesson 15** — The Actor Model: Erlang and Akka principles.

## Artifact

A dual-language reference implementing the core actor model patterns:

| Pattern | Erlang (`main.erl`) | Rust (`main.rs`) |
|---------|--------------------|------------------|
| Ping-Pong | Spawned `ping` / `pong` actors using `!` / `receive` | Thread-per-actor with `mpsc::channel`, request-reply pattern |
| Stateful Counter | Tail-recursive `counter(Count)` loop | `counter_actor` thread owning `count: i64` |
| Supervisor | `trap_exit=true` + `spawn_link` + restart on `'EXIT'` | Channel-drop detection + `spawn_supervised_counter` |
| Name Registry | `registry(Map)` actor using `maps` | `HashMap`-backed `RegistryMsg` handler |
| Batch Ops | `{batch, Ops}` applied atomically via `apply_batch` | Iterated channel sends |
| Benchmark | 10,000 sends via message-passing loop | 100,000 sends with timing |

## How to Use

```bash
# Erlang
cd code
erlc main.erl
erl -noshell -s main start

# Rust
cd code
cargo run --release
# or: rustc -O main.rs && ./main
```

## Key Principles Demonstrated

1. **Encapsulated state** — each actor's state is private (function parameter or thread-local variable).
2. **Async message passing** — senders never block; messages queue in the actor's mailbox.
3. **No shared memory** — all communication is via message copies (Erlang) or channel sends (Rust).
4. **Let it crash** — supervisors detect failures and restart children to known-good state.
5. **Location transparency** — actor addresses abstract over local/remote (Erlang pid works across nodes).

## Comparison with Production Frameworks

| Feature | Hand-built | Erlang/OTP | Akka | Actix |
|---------|-----------|------------|------|-------|
| Restart backoff | Simple restart | Max restart intensity + backoff | `OneForOneStrategy` with limits | `Supervisor::start()` |
| Lifecycle hooks | None | `init/1`, `terminate/2` | `preStart`, `preRestart` | `started`, `stopping` |
| Distribution | Not supported | Built-in via `net_adm` | Akka Cluster | `actix-rt` remote |
| Message typing | Dynamic (Erlang) / enum (Rust) | Dynamic pattern matching | Typed `ActorRef<T>` | Typed `Handler<M>` |

The production frameworks add distribution, backpressure, backoff, and lifecycle management. The core pattern — encapsulated state + async message passing — is the same.
