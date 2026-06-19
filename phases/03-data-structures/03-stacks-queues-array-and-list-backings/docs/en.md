# Stacks & Queues — Array and List Backings

> Two ADTs (LIFO and FIFO), two backings (array, linked list). The choice matters more than the abstraction.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L01-02
**Time:** ~45 minutes

## Learning Objectives

- Implement a stack and a queue with both array and linked-list backings.
- Understand why a naïve "array-backed queue" is O(n) per dequeue, and the two standard fixes (shift-on-dequeue vs ring buffer).
- Pick the right ADT for common problems: function-call stack, BFS frontier, undo history, work queue.

## The Problem

Stack (LIFO) and queue (FIFO) are the two most fundamental abstract data types. They appear in every program — explicit (your code) or implicit (the CPU's call stack, the OS's run queue, the scheduler's work queue).

Both can be implemented in two ways:

1. **Array-backed**: contiguous memory, indices for ends.
2. **Linked-list-backed**: nodes with `next` (and `prev` for deque).

Same interface; very different performance characteristics. The lesson is that **picking the backing is more important than picking the ADT**.

## The Concept

### Stack (LIFO)

```c
push(x), pop() -> x, peek() -> x, len()
```

Array-backed: `push` = write to end + len++; `pop` = len-- + read. Both O(1). Grows like a Vec.

List-backed: `push_front`/`pop_front` on an SLL. Both O(1).

**Tradeoff**: array is faster (cache locality) and has no per-element pointer overhead. List allows iterator stability across pushes (no growth-induced invalidation).

**Real callers**: function call stacks (array, grows down), undo stacks (array), evaluator stacks (array), CPS continuations (list, sometimes).

### Queue (FIFO)

```c
enqueue(x), dequeue() -> x, peek() -> x, len()
```

#### Naïve array queue (broken)

```c
push: arr[len++] = x;
pop:  return arr[head++];   /* head walks forward */
```

`head` grows monotonically — memory waste accumulates. Eventually `head + len > cap`, and you must shift everything down: O(n).

#### Fix 1: ring buffer

```c
head, tail indices wrap modulo cap
push: arr[tail] = x; tail = (tail+1) % cap;
pop:  x = arr[head]; head = (head+1) % cap;
```

O(1) per op, no shifting. Growing requires re-rolling indices (un-wrap before realloc, re-wrap after). This is `VecDeque<T>` in Rust, `collections.deque` in Python, `ArrayDeque` in Java.

The trick: capacity is usually a power of 2, so `(i+1) % cap` becomes `(i+1) & (cap-1)` — a single AND.

#### Fix 2: linked-list queue

`head` pointer to read from, `tail` pointer to write to. enqueue updates `tail->next` and `tail`; dequeue advances `head`. O(1) per op. Per-element pointer cost.

**When linked queue wins**: lock-free multi-producer/multi-consumer queues (Michael-Scott).

**When ring buffer wins**: anything else. Cache locality, simpler invariants, no per-enqueue allocation.

### Capacity policy

Stack: same growth as Vec (doubling). O(1) amortized.

Ring-buffer queue: doubling + un-wrap (copy in two halves). One-time O(n) per growth, O(1) per op amortized.

## Build It

`code/main.c` implements:

1. Stack-on-array (with growth)
2. Stack-on-list (SLL push/pop front)
3. Queue-on-naïve-array (with shifting — shows O(n) per op for repeated enqueue/dequeue)
4. Queue-on-ring-buffer
5. Queue-on-linked-list

It runs each on a 50K push/pop workload and prints ns/op. You'll see the naïve queue is 100× slower.

`code/main.py` benchmarks Python's `collections.deque` against a `list.pop(0)` queue.

`code/main.rs` uses `VecDeque<T>` (production ring buffer) and writes a hand-rolled one for comparison.

### Run

```sh
clang -O2 main.c -o sq && ./sq
python3 main.py
```

## Use It

- **CPython's `collections.deque`**: a doubly-linked list of fixed-size blocks (a hybrid). Optimized for both ends.
- **Rust `VecDeque<T>`**: ring buffer with power-of-2 capacity.
- **Java `ArrayDeque<T>`**: same — ring buffer, prefer this over the legacy `Stack`/`Queue` classes.
- **Linux scheduler run queue**: per-CPU intrusive lists.

## Read the Source

- [Rust `VecDeque`](https://doc.rust-lang.org/std/collections/struct.VecDeque.html) — pure ring buffer. Source: `library/alloc/src/collections/vec_deque/mod.rs`.
- [Python `Objects/listobject.c`](https://github.com/python/cpython/blob/main/Objects/listobject.c) — `list.pop(0)` is O(n) for exactly the shifting reason above.
- [Michael-Scott queue paper (1996)](https://www.cs.rochester.edu/~scott/papers/1996_PODC_queues.pdf) — the canonical lock-free linked queue.

## Ship It

This lesson ships **`outputs/ringbuf.h`** — a single-header power-of-2 ring buffer.

## Exercises

1. **Easy.** Stack-based bracket-matching: given a string of `()[]{}`, return whether it's balanced. O(n) time, O(n) space.
2. **Medium.** Implement a queue using two stacks. Amortized O(1) per op.
3. **Hard.** Implement a fixed-size lock-free SPSC ring buffer using atomics. Verify with two threads producing/consuming millions of items.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Stack | "LIFO" | Last-in, first-out; push/pop at one end |
| Queue | "FIFO" | First-in, first-out; enqueue at tail, dequeue at head |
| Ring buffer | "Circular array" | Array with wraparound indices; O(1) at both ends |
| Deque | "Double-ended queue" | Push/pop at both ends; usually a ring buffer |
| SPSC | "Single-producer single-consumer" | A queue with exactly one writer and one reader; admits lock-free implementations |

## Further Reading

- *Algorithms* (Sedgewick) Ch. 1.3 — clean walkthrough of both backings.
- [Doug Lea's Concurrent Programming in Java](https://gee.cs.oswego.edu/dl/cpj/) — concurrent queue chapters.
- [Memory Models 1: cache coherence](https://research.swtch.com/hwmm) by Russ Cox — for writing correct lock-free queues.
