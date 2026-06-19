# The Actor Model — Erlang and Akka principles

> The Actor Model — Erlang and Akka principles — the part of CS you can't skip.

**Type:** Build
**Languages:** Erlang, Rust
**Prerequisites:** Phase 13 lessons 01–14
**Time:** ~60 minutes

## Learning Objectives

- Understand the actor model: encapsulated state, asynchronous message passing, no shared memory.
- Implement ping-pong and stateful counter actors in Erlang using `spawn`, `!`, and `receive`.
- Implement equivalent actors in Rust using `std::sync::mpsc` channels.
- Explain the "let it crash" philosophy and supervisor tree pattern in Erlang/OTP.
- Compare Erlang OTP, Akka (Java/Scala), and Actix (Rust) as production actor frameworks.

## The Problem

Consider a chat server handling 100,000 concurrent connections. Each connection has mutable state (username, room, unread count). A naive approach uses shared mutable state protected by locks:

```c
pthread_mutex_lock(&global_lock);
user->unread_count++;
pthread_mutex_unlock(&global_lock);
```

This works until two threads deadlock on overlapping lock acquisitions, or a third bug corrupts the user table under contention. The scheduler interrupts threads at arbitrary points, making reasoning about interleavings a nightmare. With locks, the developer must manually prove that every possible interleaving is safe — a task that fails at scale.

The actor model eliminates shared state entirely. Each connection owns its state inside a lightweight process (actor). Actors communicate only through asynchronous messages. No locks, no shared memory, no data races. Erlang was designed for exactly this: telephone交换机software handling millions of concurrent calls with five-nines reliability (99.999% uptime).

Without the actor model, building fault-tolerant, massively concurrent systems requires heroics: meticulous lock ordering, thread-safe data structures, and testing that can never prove the absence of race conditions. The actor model makes concurrency a *local* property — each actor is sequential inside, concurrent outside.

## The Concept

### What Is an Actor?

An **actor** is the universal primitive of concurrent computation, just as an object is the universal primitive of OOP. Every actor has:

1. **Encapsulated state** — data that only the actor itself can read or write.
2. A **mailbox** (message queue) — incoming messages arrive here.
3. **Behavior** — a function that processes one message at a time, sequentially.

When processing a message, an actor can:
- Send messages to other actors (by their address/pid).
- Create new actors (`spawn`).
- Update its own state for the next message.

There is **no shared memory**. Actors communicate exclusively through **asynchronous message passing**. The sender does not block waiting for the receiver.

```
┌─────────────┐     message      ┌─────────────┐
│   Actor A   │ ───────────────> │   Actor B   │
│             │                  │             │
│  state: x   │                  │  state: y   │
│  mailbox[ ] │                  │  mailbox[ ] │
└─────────────┘                  └─────────────┘
       │                              │
       │  spawn                       │  spawn
       ▼                              ▼
┌─────────────┐              ┌─────────────┐
│   Actor C   │              │   Actor D   │
└─────────────┘              └─────────────┘
```

### Erlang/OTP — The Original Actor System

Erlang was designed at Ericsson in the 1980s for telecom applications. Its runtime provides:

- **Processes** — extremely lightweight (≈300 words each). Millions of processes are feasible on one machine.
- **`spawn`** — creates a new process running a given function.
- **`!` (send)** — sends a message to a process identifier (pid).
- **`receive`** — pattern-matches against messages in the mailbox, blocking until a match arrives.
- **`link` / `monitor`** — processes can link; if one dies, the other receives an exit signal.
- **`process_flag(trap_exit, true)`** — traps exit signals as regular messages instead of crashing.

**The "Let It Crash" Philosophy:** Instead of writing defensive code that catches every possible error and tries to recover, let the process crash and let a **supervisor** restart it to a known good state. This produces simpler, more reliable code.

```
    ┌──────────────────┐
    │   Supervisor     │
    │  restart=strategy│
    └──┬─────────────┬─┘
       │             │
    ┌──▼──┐     ┌───▼──┐
    │worker│     │worker│  (if one crashes,
    └──────┘     └──────┘   supervisor restarts it)
```

### Akka — Actors for the JVM

Akka (Swedish for "grandmother") brings the actor model to the JVM (Java/Scala):

- **Typed Actors** (`ActorRef<T>`) — messages are type-safe, avoiding Erlang's dynamic typing.
- **`tell`** — fire-and-forget message send (equivalent to `!`).
- **`ask`** — request-reply pattern with a `Future`.
- **Actor lifecycle** — `preStart`, `postStop`, `preRestart` hooks.
- **Supervisor strategy** — `OneForOneStrategy` (restart only the failed child) or `AllForOneStrategy` (restart all children).

Key difference from Erlang: Akka actors are compiled classes, not interpreted function calls, giving JVM type safety at the cost of some flexibility.

### Actix — Actors in Rust

Actix is a Rust actor framework built on top of `tokio`:

- **`Actor` trait** — define an actor with a `Context`.
- **`Handler<M>` trait** — handle messages of type `M`.
- **`Addr`** — an address handle to send messages (analogous to Erlang's pid).
- **`Arbiter`** — an executor that runs actors on a thread pool.

Actix leverages Rust's ownership system: each actor owns its state, and message types are statically checked at compile time. The `actix` crate provides the actor runtime; the `actix-web` crate builds an HTTP framework on top.

## Build It

### Step 1: Erlang — Ping-Pong Actors

Create `code/main.erl`. The ping actor sends a message to pong and waits for a reply. Pong responds until ping says "finished".

```erlang
ping(0, PongPid) ->
    PongPid ! finished,
    io:format("ping: done~n");
ping(N, PongPid) ->
    PongPid ! {ping, self()},
    receive
        pong -> ping(N - 1, PongPid)
    end.

pong() ->
    receive
        {ping, PingPid} ->
            PingPid ! pong,
            pong();
        finished ->
            io:format("pong: done~n")
    end.
```

Compile and run:

```bash
$ erlc main.erl
$ erl -noshell -s main start
```

Each call to `spawn(fun .../0)` creates a new Erlang process — an actor. Messages are sent with `Pid ! Msg`. The `receive` block pattern-matches messages from the mailbox. The recursive tail call (`pong()`) keeps the actor alive without growing the stack.

### Step 2: Erlang — Stateful Counter Actor

Now add an actor that maintains internal state across messages:

```erlang
counter(Count) ->
    receive
        {increment, Amount} -> counter(Count + Amount);
        {decrement, Amount} -> counter(Count - Amount);
        {get, Caller}       -> Caller ! {count, Count}, counter(Count);
        stop                -> io:format("counter: final=~p~n", [Count])
    end.
```

The actor's state is the parameter `Count` passed recursively. Each message mutates the state by calling `counter/1` with a new value. The `{get, Caller}` message replies to the caller without changing state.

### Step 3: Rust — Manual Actor with Channels

The Rust version demonstrates the same principles using `std::sync::mpsc` channels:

```rust
// A counter actor: runs in its own thread, owns its state,
// processes messages from a channel receiver.
fn counter_actor(rx: Receiver<CounterMsg>, id: &str) {
    let mut count = 0i64;
    for msg in rx {
        match msg {
            CounterMsg::Increment(n) => count += n,
            CounterMsg::Decrement(n) => count -= n,
            CounterMsg::Get(reply) => {
                let _ = reply.send(count);
            }
            CounterMsg::Stop => break,
        }
    }
}
```

The `Sender<CounterMsg>` handle is the actor's address. Sending a message is non-blocking (asynchronous). State is owned by the thread — no mutex needed.

### Step 4: Rust — Supervisor Pattern

A supervisor monitors the counter actor and restarts it on crash:

```rust
fn supervisor(rx: Receiver<()>) {
    loop {
        let child = spawn_counter();
        match rx.recv() {
            Err(_) => {
                eprintln!("supervisor: child crashed, restarting...");
                continue;
            }
            Ok(()) => break,
        }
    }
}
```

This mirrors Erlang's supervisor tree: if the child panics, the channel drops, the supervisor detects it, and spawns a replacement.

## Use It

**Erlang/OTP** provides the production supervisor via `supervisor.erl`. In OTP, you define a `init/1` callback returning child specifications:

```erlang
init([]) ->
    {ok, {{one_for_one, 5, 10},
          [{counter, {counter, start_link, []},
            permanent, 5000, worker, [counter]}]}}.
```

This gives you automatic restart with backoff, logging, and integration with the OTP release system.

**Akka** provides `akka.actor.SupervisorStrategy` with configurable directives (Resume, Restart, Stop, Escalate):

```scala
override val supervisorStrategy = OneForOneStrategy(maxNrOfRetries = 10) {
  case _: ArithmeticException => Resume
  case _: NullPointerException => Restart
}
```

**Actix** uses `Supervisor::start()` to wrap an actor with automatic restart on failure.

Compare your hand-built version: the production frameworks add:
- Backoff / restart limits (to avoid crash loops).
- Lifecycle hooks (`preStart`, `postStop`).
- Distributed actor addresses (Erlang distribution, Akka Cluster, Actix `Recipient`).
- Logging, metrics, and integration with the runtime.

## Read the Source

- **Erlang/OTP supervisor**: `<otp_src>/lib/stdlib/src/supervisor.erl` — the 30-year-old reference implementation of the supervisor behaviour.
- **Akka actor**: `akka-actor/src/main/scala/akka/actor/ActorCell.scala` — core message dispatch loop. Look at `invoke()` and `receiveMessage()`.
- **Actix**: `actix/src/actor.rs` — the `Actor` trait and `Context` lifecycle.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. It is:

- **A dual-language actor model reference** — Erlang and Rust implementations of ping-pong, stateful counter, and supervisor patterns. Use these snippets as templates in later concurrent systems work.

## Exercises

1. **Easy** — Reproduce the Erlang ping-pong from memory. Add a third actor (`ping2`) that chains: ping → pong → ping2.
2. **Medium** — Extend the Rust counter to support `Reset` and `Multiply` messages. Add a second actor that sends periodic tick messages.
3. **Hard** — Implement a minimal actor registry in Erlang (name → pid lookup). Build a `whereis` equivalent using an ETS table or a registry actor, with support for concurrent `register` and `send` operations.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Actor | A concurrent computation primitive | An independent process with encapsulated state, a mailbox, and sequential message processing. No shared memory. |
| Message passing | Actors talk to each other | Sending asynchronous data from one actor to another via their pid/address. The sender never blocks on the receiver. |
| Process | A lightweight thread in Erlang VM | Not an OS thread. Erlang processes weigh ≈300 words, scheduled by the BEAM VM preemptively. Millions are feasible. |
| Mailbox | Where messages wait | A per-actor queue of unreceived messages, pattern-matched by `receive`. |
| Supervisor | A process that watches children | A tree node that monitors worker processes and restarts them on failure. Core of OTP fault tolerance. |
| Let-it-crash | Don't write defensive error handlers | Let processes fail; supervisors restore known-good state. Produces simpler, more reliable code. |
| OTP | Open Telecom Platform | Erlang's suite of libraries and design principles: `gen_server`, `supervisor`, `gen_statem`, etc. |
| Actix | Rust actor framework | An actor runtime on top of `tokio` with type-safe messages via the `Handler` trait. |
| Akka | JVM actor framework | Typed actor system for Java/Scala with cluster support, stream processing (Akka Streams), and HTTP (Akka HTTP). |
| Location transparency | Actors don't know where the other actor lives | An actor's address works the same whether the target is local or on another node. Erlang's `Pid ! Msg` works across nodes transparently. |

## Further Reading

- Carl Hewitt, Peter Bishop, Richard Steiger. *A Universal Modular ACTOR Formalism for Artificial Intelligence* (1973) — the original paper.
- Joe Armstrong. *Programming Erlang: Software for a Concurrent World* (2nd ed., 2013).
- Jonas Bonér et al. *Akka in Action* (2016).
- Erlang documentation: [https://www.erlang.org/docs](https://www.erlang.org/docs)
- Actix documentation: [https://actix.rs/docs](https://actix.rs/docs)
