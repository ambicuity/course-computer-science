# CSP and Go Channels

> CSP and Go Channels — the part of CS you can't skip.

**Type:** Build
**Languages:** Go
**Prerequisites:** Phase 13 lessons 01–13
**Time:** ~60 minutes
**Version:** Go 1.21+

## Learning Objectives

- Explain the Communicating Sequential Processes model (Hoare, 1978) and how it differs from shared-memory concurrency.
- Identify the three core primitives Go inherits from CSP: goroutines, channels, and select.
- Distinguish unbuffered (synchronous rendezvous) channels from buffered (asynchronous) channels and choose the correct one for a scenario.
- Implement a multi-stage pipeline that chains goroutines via channels.
- Implement a fan-out / fan-in pattern that distributes work across multiple goroutines and merges results.
- Use select to multiplex channel operations with timeouts and cancellation.
- Apply the quit channel pattern to cleanly shut down long-lived goroutines and prevent goroutine leaks.
- Explain Go's M:N scheduler (goroutines multiplexed onto OS threads) and why goroutines are cheaper than OS threads.

## The Problem

You are building a concurrent system — maybe a web crawler, a log processor, or a real-time analytics pipeline. The obvious approach is shared memory: a dozen threads, a mutex protecting each data structure, and condition variables to signal state changes.

This works, but it is *brittle*:

- Every mutex is an invitation to deadlock. Every condition variable is a chance to miss a wakeup.
- You cannot compose lock-based code. Combining two thread-safe operations almost always produces a result that is *not* thread-safe.
- Reasoning about interleavings is exponentially hard. With `n` threads and `k` shared variables, the number of possible schedules is astronomical.
- OS threads are expensive: each reserves ~1–8 MB of stack space, context switching requires a kernel trap, and thousands of threads overwhelm the scheduler.

Go approaches concurrency differently. Instead of sharing memory and synchronising with locks, you share *channels* and let each goroutine own its data. This is the heart of Communicating Sequential Processes: **don't communicate by sharing memory; share memory by communicating.**

This lesson builds the muscle memory for thinking and coding in CSP. You will implement the five canonical patterns that form the backbone of every concurrent Go program.

## The Concept

### Communicating Sequential Processes (C. A. R. Hoare, 1978)

In 1978 Tony Hoare published "Communicating Sequential Processes" (CSP), a formal language for describing patterns of interaction in concurrent systems. The core idea is radical:

> A concurrent program is a collection of independent *processes* that can communicate only through *channels*. There is no shared state.

Each process is a sequential computation. It can send a value on a channel, receive a value from a channel, or compute locally. That is it. No locks, no shared variables, no memory barriers.

**Why this matters for correctness:**

CSP eliminates the two hardest problems in shared-memory concurrency:

1. **Data races** — if only one process owns a piece of data and sends it over a channel, there is never a moment where two processes access the same memory concurrently.
2. **Deadlock from lock ordering** — channel operations can block, but deadlock is always a circular wait on channel sends/receives, which is easier to detect (Go's runtime can even report it).

**The channel rendezvous:**

An unbuffered channel acts as a *synchronisation point*. When process A sends on channel `c` and process B receives from `c`, both block until the other is ready. The handoff is simultaneous — no interleaving is possible between the send and the receive. This is strictly stronger than any lock-based scheme: the data moves directly from A's stack to B's stack with no intermediate shared buffer.

### How Go Implements CSP

Go adapts CSP with three primitives:

| CSP concept | Go primitive | Description |
|---|---|---|
| Process | `goroutine` | Lightweight thread managed by the Go runtime |
| Channel | `chan T` | Typed conduit for sending/receiving values of type `T` |
| Guarded command | `select` | Waits on multiple channel operations; the first ready one wins |

**Goroutines vs OS threads:**

- Goroutines start with ~4 KB of stack (growable, not fixed). An OS thread reserves ~1–8 MB.
- Goroutines are multiplexed onto OS threads by Go's *M:N scheduler*: `M` goroutines run on `N` OS threads.
- Context switching between goroutines is a user-space operation — no kernel trap, no TLB flush.
- You can launch hundreds of thousands of goroutines on a laptop. A few hundred OS threads would be prohibitive.

**Channel types:**

- **Unbuffered** (`make(chan T)`) — synchronous; send blocks until receive is ready.
- **Buffered** (`make(chan T, N)`) — asynchronous; send blocks only when the buffer is full.
- **Directional** (`chan<- T` write-only, `<-chan T` read-only) — static safety checked at compile time.

**Select:**

`select` is like a `switch` for channel operations. It evaluates all cases, picks one at random if multiple are ready, and executes it. The `default` case (if present) fires when no channel is ready, enabling non-blocking operations.

## Build It

All code for this lesson is in `code/main.go`. Build the five steps sequentially inside `main()`.

### Step 1 — Goroutines + Unbuffered Channels (Sync Handoff)

The simplest possible CSP program: one goroutine sends a string, and `main` receives it.

```go
ch := make(chan string)

go func() {
    ch <- "ping"   // blocks until main receives
}()

msg := <-ch        // blocks until the goroutine sends
fmt.Println(msg)   // "ping"
```

**What to observe:**

- The sender blocks at `ch <- "ping"` until the receiver is ready. A `time.Sleep` before the receive proves the sender is genuinely waiting.
- The value moves directly from the goroutine's stack to `main`'s stack — no shared buffer, no mutex.
- If you swap the sleep to after the receive, the program still works (the receive blocks instead).

### Step 2 — Pipeline (generate → square → print)

A pipeline chains stages via channels. Each stage is a goroutine that reads from an input channel and writes to an output channel. Stages are connected by simply assigning channels.

```
generate(5) → square → print
```

- `generate` sends `1, 2, 3, 4, 5` on its output channel and closes it when done.
- `square` reads from its input, squares each value, and sends the result on its output channel.
- `print` reads and prints until the channel is closed.

Closing a channel is the signal that no more values will arrive. The receiver detects closure with the `for v := range ch` loop.

**Error to avoid:** Always ensure the final stage closes its output channel (or the sender closes it). An unclosed channel in a pipeline causes the consumer to block forever — a goroutine leak.

### Step 3 — Fan-out / Fan-in

**Fan-out:** One channel is read by multiple goroutines, distributing work.

**Fan-in:** Multiple channels are merged into one.

```
                   ┌→ worker 1 ─┐
jobs ─────────────┼→ worker 2 ─┼─→ merge ─→ results
                   └→ worker 3 ─┘
```

Implementation notes:

- The `jobs` channel may be buffered to decouple job production from consumption.
- Each worker has its own output channel so merge can collect from each independently.
- A `sync.WaitGroup` tracks when all workers are done. The merge goroutine waits on the WaitGroup, then closes the merged output.

**Key insight:** Fan-out is safe because channel operations are inherently concurrent-safe. Multiple goroutines can send to or receive from the same channel without a mutex.

### Step 4 — select with Timeout

`select` lets you race a channel operation against `time.After`:

```go
select {
case res := <-slowOp:
    fmt.Println(res)
case <-time.After(100 * time.Millisecond):
    fmt.Println("timeout")
}
```

If `slowOp` does not produce a value within 100 ms, the timeout case fires and the function continues. The `slowOp` goroutine is left blocked on its send — in production you would combine this with a context or a quit channel.

**Empty select:** `select{}` blocks forever. The Go runtime detects this and reports a fatal error: "fatal error: all goroutines are asleep - deadlock!"

### Step 5 — Quit Channel Pattern

A quit channel signals a goroutine to stop. The worker selects on both its work channel and a `quit` channel:

```go
quit := make(chan struct{})

go func() {
    for {
        select {
        case work <- i:
            i++
        case <-quit:
            return   // cleanup and exit
        }
    }
}()
```

To stop the worker, close or send on `quit`:

```go
close(quit)  // all receivers get the zero value immediately
```

Closing a channel is the preferred signal: it unblocks *all* goroutines waiting on that channel (broadcast) and does not consume a value.

**Why this matters:** Without the quit channel, a goroutine that blocks on a send or receive that nobody will ever service is a *goroutine leak*. Leaked goroutines accumulate stack memory and eventually crash the process. The quit pattern gives you deterministic cleanup.

## Use It

The patterns you just built are not toy examples — they are the exact patterns used in Go's standard library and the wider ecosystem.

- **`net/http`** — each accepted connection is handled in a new goroutine. The HTTP server uses goroutines per-connection and channels internally for shutdown signalling.
- **`io.Pipe`** — creates an in-memory pipe using an unbuffered channel under the hood, exactly like Step 1.
- **`context.Context`** — cancellation is propagated through channels. `ctx.Done()` returns a `<-chan struct{}` that is closed when the context is cancelled, which is the quit-channel pattern at scale.
- **`text/template` and `html/template`** — template execution pipelines mirror the channel-pipeline pattern from Step 2.
- **`database/sql`** — connection pooling uses a buffered channel of `*sql.Conn` values as the pool itself.

**What production does differently:**

Your hand-built version uses `time.Sleep` for demonstration. Production code replaces sleeps with `context.Context` deadlines, `sync.WaitGroup` for coordination, and `errgroup.Group` for error propagation across goroutines. The structure — channels connecting goroutines — is identical.

## Read the Source

- **Go runtime scheduler:** `runtime/proc.go` — the `schedule()` function is the heart of Go's M:N scheduler. Look for how it finds a runnable goroutine (around line 3300+ in Go 1.21).
- **Channel implementation:** `runtime/chan.go` — `chansend()` and `chanrecv()` implement the blocking send/receive with the full sudog queueing mechanism. The `select` runtime support is in `runtime/select.go`.
- **`net/http` server:** `net/http/server.go` — `Server.Serve` accepts connections in a loop, spawning a goroutine per connection via `c.serve()`.
- **`context` cancellation:** `context/context.go` — `WithCancel` returns a `chan struct{}` (the quit channel pattern). `select` on `Done()` is the idiomatic way to wait for cancellation.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained Go program (`code/main.go`) demonstrating all five CSP patterns**, which you can run, modify, and reuse as a reference for any concurrent Go project.

## Exercises

1. **Easy** — Reproduce all five steps from memory. Run the program and confirm the output matches the expected order. Experiment by changing buffer sizes and timeout durations.

2. **Medium** — Extend the pipeline (Step 2) with a fourth stage that filters out squares divisible by 3. Use a directional `chan<-` type for the filter output. Then convert the pipeline to use buffered channels of size 2 — measure whether the output order changes.

3. **Hard** — Implement a "cancellable pipeline" using a `context.Context`. Modify each pipeline stage to accept a `ctx context.Context` parameter and select on `ctx.Done()` in the main loop. When the context is cancelled after 10 ms, all stages should shut down cleanly within one iteration. Use `errgroup.Group` to propagate errors: if one stage fails, the entire pipeline cancels. Verify with a stage that deliberately fails after processing 3 items.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| CSP | Communicating Sequential Processes | A formal model by Hoare where independent processes communicate only through channels; no shared state. |
| Goroutine | A lightweight thread | A stackful coroutine managed by Go's runtime, starting at ~4 KB with growable stacks, multiplexed onto OS threads via M:N scheduling. |
| Channel | A typed conduit for values | A concurrent-safe FIFO queue that coordinates goroutines; unbuffered channels synchronise (rendezvous), buffered channels decouple. |
| Select | A switch for channels | A control structure that waits on multiple channel operations and executes the first ready one; random if multiple are ready. |
| Pipeline | A chain of stages connected by channels | Each stage is a goroutine that reads from an input channel, transforms the value, and writes to an output channel; stages run concurrently. |
| Fan-out | Distribute work to multiple goroutines | One channel is read by multiple goroutines, each processing a subset of values; balanced by the runtime scheduler. |
| Fan-in | Merge multiple channels into one | Multiple output channels are read by a single goroutine (or merge function) that writes to a unified result channel. |
| Multiplexing | Handle multiple channel ops with select | Select waits on several channels simultaneously; enables timeouts, cancellation, and non-blocking sends. |
| Goroutine leak | A goroutine that never exits | A goroutine blocked forever on a channel send/receive (or an infinite loop) whose stack is never freed; cumulative leaks crash the process. |
| Go scheduler | The M:N scheduler | Schedules `M` goroutines onto `N` OS threads using work-stealing; avoids kernel context switches for goroutine scheduling. |
| M:N scheduling | Many user-level threads on few kernel threads | The runtime multiplexes M goroutines onto N OS threads; handoff between goroutines is user-space, not a kernel call. |

## Further Reading

1. Hoare, C. A. R. "Communicating Sequential Processes." *Communications of the ACM* 21.8 (1978): 666–677. — The original paper that introduced CSP.
2. Hoare, C. A. R. *Communicating Sequential Processes*. Prentice Hall, 1985. — The book-length treatment (available free online).
3. Pike, Rob. "Concurrency Is Not Parallelism." Heroku Waza, 2012. — The talk that crystallises Go's approach to concurrency.
4. Donovan, Alan A. A., and Brian W. Kernighan. *The Go Programming Language*. Addison-Wesley, 2015. — Chapters 8 and 9 cover goroutines, channels, and select.
5. Cox, Russ. "Go's work-stealing scheduler." — Analysis of how the Go scheduler distributes goroutines across OS threads.
6. Go source: `runtime/chan.go`, `runtime/proc.go`, `runtime/select.go` — The real channel and scheduler implementations.
