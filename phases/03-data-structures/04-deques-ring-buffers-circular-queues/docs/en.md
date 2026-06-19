# Deques, Ring Buffers, Circular Queues

> The ring buffer is the data structure of low-latency systems: zero allocations per op, two atomic indices, single-cycle access. Master it and you've built the heart of every audio engine, network driver, and event loop.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** P03 L03 (ring buffer basics)
**Time:** ~60 minutes

## Learning Objectives

- Implement a **deque** (double-ended queue) with O(1) push/pop on both ends — typical interview question, real production primitive.
- Master the **ring buffer's full state machine**: full vs empty, the "phantom slot," using a separate count vs distinguishing by indices.
- Implement a **lock-free SPSC ring buffer** with `_Atomic` (C) / `AtomicUsize` (Rust) and prove it correct using release/acquire memory orderings.
- Recognize the canonical applications: audio I/O (JACK, PortAudio), network drivers (Linux skb queue), kernel-userspace channels (io_uring), tracing buffers (perf, ftrace).

## The Problem

A ring buffer is to a queue what a Vec is to an array — the production-grade primitive that backs almost every "queue of fixed items" you'll touch. The basic O(1) algorithm is from L03; this lesson lifts it to the variants real systems use:

- **Deque**: push/pop at front AND back. `front++` and `back--` both wrap.
- **Fixed-capacity ring with overwrite**: when full, overwrite the oldest (useful for tracing buffers; the most recent N samples).
- **Lock-free SPSC**: one producer thread, one consumer thread, no locks, no system calls. Backbone of low-latency systems.

The non-obvious bits — distinguishing "full" from "empty" cleanly, handling memory ordering correctly — are where most hand-rolled ring buffers go wrong.

## The Concept

### The full-vs-empty problem

With head and tail indices: head == tail can mean **empty** OR **full** (both go around the ring). Three standard solutions:

| Solution | Cost | Used by |
|----------|------|---------|
| **Separate `len` counter** | 1 extra field; needs atomic update for SPSC | Most C ring buffers |
| **Sacrifice one slot**: full when (tail + 1) % cap == head | cap-1 usable slots | Many CS textbooks |
| **Use uncapped indices**: head and tail are size_t that grow forever; index slot = (i & mask) | Wraps after 2^64 ops (~580 years at 1 GHz) | Linux kernel, real-time systems |

The uncapped-indices trick is elegant: `len = tail - head`, full = (tail - head == cap), empty = (tail == head). No special-casing.

### Deque ops

A deque adds two operations to a queue: `push_front` and `pop_back`. Both are still O(1) on a ring buffer:

```c
push_front(x):  head = (head - 1) & mask;  buf[head] = x;  len++
pop_back():     tail = (tail - 1) & mask;  return buf[tail];  len--
```

Note the subtraction. With `size_t` and wraparound semantics, `head - 1` underflows to `SIZE_MAX`, which then AND'd with the mask wraps correctly. Works for any unsigned integer.

### Overwriting ring (trace buffer)

For "keep the most recent N":

```c
push(x):  buf[tail] = x;  tail = (tail+1) & mask
          if (len == cap) head = (head+1) & mask  /* drop oldest */
          else len++
```

Used everywhere a fixed-size log helps: perf ring buffer, packet capture, postmortem debuggers. The "drop oldest, keep producer fast" tradeoff is correct for tracing; wrong for transmit queues (where you'd block instead).

### Lock-free SPSC ring buffer

Two threads, one producer, one consumer. Each owns one index — producer owns `tail`, consumer owns `head`. Each thread:

1. Reads its own index (cheap, no atomic).
2. Reads the *other* thread's index (atomic acquire load).
3. Tests for full/empty.
4. Writes the data.
5. Publishes its own index (atomic release store).

The publish-with-release-store + read-with-acquire-load is what makes the buffer *correct without locks*: it gives the consumer a happens-before relationship with the data write, so when the consumer sees the new tail, the data must be visible.

```c
/* Producer */
size_t t = atomic_load_explicit(&tail, memory_order_relaxed);
size_t h = atomic_load_explicit(&head, memory_order_acquire);
if (t - h == cap) return BUSY;
buf[t & mask] = x;
atomic_store_explicit(&tail, t + 1, memory_order_release);
```

If you get the memory orderings wrong, the data write can be reordered *after* the index publish, and the consumer sees the new index but stale data. Worse: it's correct on x86 (which has stronger defaults) and broken on ARM (which is weakly ordered). Hence the orderings are mandatory, not decorative.

### Why deques and ring buffers are *the same data structure* with different APIs

`VecDeque<T>` in Rust is literally a ring buffer with both front and back ops exposed. So is Python's `collections.deque` (mod blocks for very large sizes). Once you understand the ring buffer, deque is just "push at head/tail too."

## Build It

`code/main.c`:

1. Single-threaded deque with all four push/pop ops.
2. Overwriting trace buffer ("last 8 numbers" demo).
3. Lock-free SPSC ring buffer with two pthreads producing/consuming 1M items.

`code/main.rs` (Rust):

1. Same deque using `VecDeque<T>`.
2. Hand-rolled SPSC with `AtomicUsize` and `Ordering::{Acquire, Release, Relaxed}`.

### Run

```sh
clang -O2 -pthread main.c -o rb && ./rb
rustc -O main.rs -o rbr && ./rbr     # if rustc is installed
```

## Use It

- **JACK Audio Server**: every audio device is a SPSC ring buffer; producer is the driver IRQ, consumer is the userspace mixer.
- **Linux io_uring**: submission and completion rings are lock-free SPSC ring buffers shared kernel↔userspace.
- **Disruptor (LMAX)**: a multi-producer ring buffer used in low-latency trading systems; throughput ~6M msg/s on a laptop.
- **Linux skb queue**: every NIC has per-CPU RX/TX rings; the kernel and the device hardware are the producer/consumer pair.

## Read the Source

- [Rust `VecDeque`](https://doc.rust-lang.org/src/alloc/collections/vec_deque/mod.rs.html) — pure ring buffer; very readable.
- [LMAX Disruptor](https://github.com/LMAX-Exchange/disruptor) — Java multi-producer ring buffer with batching.
- [Linux `kernel/trace/ring_buffer.c`](https://github.com/torvalds/linux/blob/master/kernel/trace/ring_buffer.c) — per-CPU lock-free tracing ring with overwrite semantics.

## Ship It

This lesson ships **`outputs/spsc_ring.h`** — a header-only single-producer single-consumer ring buffer in C with all memory-orderings labeled.

## Exercises

1. **Easy.** Implement `iter()` on the deque: walk from head to tail without consuming. Test with a `VecDeque`-equivalent random workload.
2. **Medium.** Build a "sliding-window maximum" using a deque: given a stream and window size k, output the max of the last k. O(n) total time via the monotonic-deque trick.
3. **Hard.** Build a multi-producer single-consumer (MPSC) lock-free ring buffer. Producers use `compare_exchange_weak` on a shared tail index; consumer owns head. Verify with 4 producer threads and 1 consumer over 10M items.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Deque | "Double-ended queue" | Push/pop at both ends; usually a ring buffer |
| SPSC | "Single producer / single consumer" | One thread writes, one reads; admits lock-free O(1) impl |
| Memory ordering | "Acquire/release" | Compiler/CPU promise about what can be reordered around an atomic op |
| Mask | "cap - 1" | Used with power-of-2 cap to wrap indices via AND |
| Overwriting buffer | "Tracing ring" | Push drops oldest when full; consumer can be slow |

## Further Reading

- *The Art of Multiprocessor Programming* by Herlihy & Shavit, Ch. 10 — SPSC queue correctness proofs.
- [Preshing on Programming: memory barriers](https://preshing.com/20120710/memory-barriers-are-like-source-control-operations/) — best intro to acquire/release.
- [LMAX Disruptor paper](https://lmax-exchange.github.io/disruptor/disruptor.html) — the design philosophy of the bus-of-rings.
