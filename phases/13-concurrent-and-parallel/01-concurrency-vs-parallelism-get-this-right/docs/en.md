# Concurrency vs Parallelism — Get This Right

> Concurrency is about dealing with lots of things at once. Parallelism is about doing lots of things at once. They are not the same thing, and getting this wrong poisons every concurrent system you will ever design.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 12
**Time:** ~45 minutes

## Learning Objectives

- Explain the Rob Pike distinction between concurrency (structure) and parallelism (execution) in your own words.
- Given a system description, classify it as concurrent, parallel, both, or neither — and defend the classification.
- Compute the maximum theoretical speedup for a workload using Amdahl's Law and explain why serial bottlenecks dominate.
- Contrast Amdahl's Law (fixed-size problem) with Gustafson's Law (scaled problem) and identify which applies to a given scenario.
- Name three levels of concurrency (task, data, instruction) and give a concrete example of each.
- Compare six concurrency models (processes, threads, async/await, coroutines, actors, CSP) on memory, communication, and best-fit workloads.
- Identify when adding parallelism will make a system *worse* (diminishing returns, synchronization overhead, false sharing).

## The Problem

Most programmers conflate "concurrent" and "parallel." They say "make it faster by adding threads" without understanding the distinction. This leads to over-engineered code that is slower, harder to debug, and full of race conditions. Getting this wrong at the start of Phase 13 means every subsequent lesson — locks, atomics, lock-free data structures, async runtimes, GPUs — builds on a misunderstanding.

Consider a web server handling 10,000 concurrent connections. Is it doing 10,000 things at once? No. A single CPU core can only execute one instruction at a time. The server *structures* the work to overlap waiting (I/O) with computation — that's concurrency. Whether it runs on one core or eight is a separate decision: parallelism.

If you confuse the two, you will:
- Throw threads at an I/O-bound problem and wonder why it gets *slower* (context-switch thrash).
- Think a single-threaded event loop is "not concurrent" and dismiss Node.js, Redis, and Nginx as toys.
- Design parallel algorithms that spend more time synchronizing than computing.
- Misread Amdahl's Law and buy a 128-core machine expecting 128x speedup on code that's 95% serial.

## The Concept

### Rob Pike's Distinction

In his 2012 talk "Concurrency is not Parallelism," Rob Pike crystallized the difference:

| | Concurrency | Parallelism |
|---|---|---|
| **Focus** | Program *structure* | Execution *efficiency* |
| **Question** | How is the work decomposed? | How many operations run simultaneously? |
| **Minimum hardware** | Single core | Multiple cores |
| **Goal** | Responsiveness, clarity, composability | Throughput, speed |
| **Measure** | Tasks managed concurrently | Operations per second |

**Concurrency is about *dealing with* lots of things at once.** It's a design concern: how you decompose a program into independently executing pieces that can be composed, paused, and resumed.

**Parallelism is about *doing* lots of things at once.** It's an execution concern: how many operations the hardware actually performs simultaneously.

The critical insight: **concurrency enables parallelism, but they are not the same thing.** A concurrent program can run on a single core (time-slicing between tasks). A parallel program needs multiple cores.

```
Single-core time-slicing (concurrent, NOT parallel):

Thread A: ██░░██░░██░░██░░
Thread B: ░░██░░██░░██░░██
Time:     ────────────────→

Multi-core (parallel, may also be concurrent):

Core 0:   ████████░░░░░░░░
Core 1:   ░░░░░░████████░░
Time:     ────────────────→
```

### Examples

| System | Concurrent? | Parallel? | Why |
|--------|-------------|-----------|-----|
| Single-core CPU running 3 apps (time-sliced) | Yes | No | Structured as 3 tasks, but only 1 runs at a time |
| Multi-core CPU, 1 thread per core | No | Yes | No decomposition, just multiple independent executions |
| Go program with 100 goroutines on 4 cores | Yes | Yes | Goroutines structure the work; 4 cores execute simultaneously |
| Node.js event loop (single thread) | Yes | No | Async I/O interleaves tasks; one thread executes them |
| Web server handling 10k connections on 1 thread | Yes | No | Event loop multiplexes I/O; single-threaded execution |
| Python with `concurrent.futures` on 8 cores | Yes | Yes | Tasks are structured independently and run on multiple cores |
| Redis (single-threaded event loop) | Yes | No | Handles concurrent clients via multiplexing on one thread |

### Amdahl's Law — The Limit of Parallelism

Discovered by Gene Amdahl in 1967, this law gives the maximum speedup when parallelizing a workload:

```
S(N) = 1 / ((1 - P) + P/N)
```

Where:
- **P** = fraction of the workload that can be parallelized
- **N** = number of processors (cores)
- **S(N)** = speedup relative to single-core execution

**Worked example:** A program spends 95% of its time in a parallelizable loop and 5% in serial setup/teardown.

- With 4 cores: S = 1 / (0.05 + 0.95/4) = 1 / (0.05 + 0.2375) = 1 / 0.2875 ≈ **3.48x**
- With 16 cores: S = 1 / (0.05 + 0.95/16) = 1 / (0.05 + 0.0594) = 1 / 0.1094 ≈ **9.14x**
- With infinite cores: S = 1 / 0.05 = **20x** (the serial bottleneck dominates)

| Parallel fraction (P) | Max speedup (infinite cores) |
|-----------------------|------------------------------|
| 50% | 2x |
| 90% | 10x |
| 95% | 20x |
| 99% | 100x |
| 99.9% | 1000x |

**The takeaway:** Optimizing serial bottlenecks matters more than adding cores. If 5% of your code is serial, you can never get more than 20x speedup, no matter how many cores you buy.

### Gustafson's Law — The Optimist's View

John Gustafson argued in 1988 that Amdahl's Law assumes a fixed-size problem. In practice, as more cores become available, programmers scale the *problem size*, not keep it fixed.

```
Scaled speedup = N + (1 - P)(1 - N)
```

Or equivalently: **S(N) = N - (N - 1)(1 - P)**

Where P is now the parallel fraction of the *scaled* workload.

**Worked example:** Same 95% parallel fraction, but now the problem grows with N.

- With 4 cores: S = 4 - (3)(0.05) = 4 - 0.15 = **3.85x** (vs Amdahl's 3.48x)
- With 16 cores: S = 16 - (15)(0.05) = 16 - 0.75 = **15.25x** (vs Amdahl's 9.14x)

Which law applies depends on the situation:
- **Amdahl:** Fixed-size datasets, latency-critical workloads
- **Gustafson:** Large-scale simulations, ML training, where you use more cores to solve bigger problems

### Three Levels of Concurrency

1. **Task-level concurrency** — Multiple independent tasks run concurrently. The easiest and most common form. Example: a web server handling requests for different users simultaneously. Each request is a separate task with its own logic.

2. **Data-level concurrency (SIMD)** — The same operation applied to different pieces of data. Example: adding two vectors element-wise on a GPU or using AVX instructions on a CPU. Single instruction, multiple data.

3. **Instruction-level parallelism (ILP)** — A single CPU executes multiple instructions per cycle through pipelining, superscalar execution, and out-of-order execution. You get this for free from the hardware — no programming effort required.

```
Level          | Granularity  | Programmer control | Example
Task-level     | Coarse       | High               | Web request handlers
Data-level     | Fine         | Medium             | Vector addition, image filter
Instruction-lv | Very fine    | Low (compiler/CPU) | CPU pipeline
```

### Concurrency Models

| Model | Memory model | Communication | Example | Best for |
|-------|-------------|---------------|---------|----------|
| **Processes** | Isolated address spaces | IPC (pipes, sockets, shared memory, signals) | Unix `fork()` | Security isolation, fault containment |
| **Threads** | Shared address space | Shared memory + locks (mutex, rwlock) | pthreads, Java threads | CPU-bound computation, shared state |
| **Async/await** | Shared (single-threaded event loop) | Futures, promises, channels | JavaScript, Python asyncio, C# async | I/O-bound workloads, high connection counts |
| **Coroutines (stackful)** | Shared per process | Channels, async operations, mutex | Go goroutines, Lua coroutines | Mixed workloads, composable concurrency |
| **Actors** | Isolated per actor (no shared state) | Message passing (mailbox) | Erlang, Akka, Orleans | Fault-tolerant, distributed systems |
| **CSP** (Communicating Sequential Processes) | Shared per process | Synchronous channels | Go channels, Clojure core.async | Pipeline processing, dataflow |

### When NOT to Use Parallelism

Adding parallelism is not free. These costs can make parallel code *slower* than serial:

1. **Amdahl's diminishing returns** — The serial fraction dominates past ~16 cores for most workloads.

2. **Synchronization overhead** — Locks, barriers, and atomic operations cost 10–100 ns each. If each unit of work is too small (e.g., adding two integers), the synchronization cost exceeds the computation.

3. **Cache contention / false sharing** — Two cores writing to different variables on the same cache line force cache coherence traffic. A single cache line "bouncing" between cores can be 100x slower than local access.

4. **I/O-bound workloads** — Parallelizing across cores doesn't help if the bottleneck is disk or network. You need concurrency (overlap I/O with computation), not parallelism.

5. **Context-switch overhead** — More threads than cores means time-slicing. Each context switch costs 1–10 μs. At 10,000 threads, you spend more time switching than working.

6. **Memory bandwidth saturation** — Many parallel tasks reading/writing memory can saturate the memory bus. Adding cores doesn't help when all cores wait for the same DRAM channel.

**Rule of thumb:** The work per unit of synchronization must be at least 10,000 cycles for parallelism to pay off on modern hardware.

## Build It

This lesson uses Markdown as its language — the reference artifact is a comprehensive set of reference notes in `code/notes.md`. Open that file now.

The notes cover:
- Core definitions and the Pike distinction
- A comparison table of concurrency vs parallelism across 6 dimensions
- A detailed concurrency models comparison (6 models × 6 attributes)
- Amdahl's Law with worked examples at different parallel fractions
- Gustafson's Law with worked examples
- Three levels of concurrency with granularity and programmer control
- Real-world system classifications (Nginx, Node.js, Go, TensorFlow, Redis, PostgreSQL)
- When NOT to parallelize — six concrete failure modes
- Memory ordering and consistency model primer
- Performance considerations and rule-of-thumb numbers

Read `code/notes.md` alongside this lesson. Use it as a quick reference during the rest of Phase 13.

## Use It

Real-world systems embody the concurrency vs parallelism distinction differently. Understanding their design reveals the trade-offs.

**Nginx — concurrent, not parallel.**
Nginx uses an event loop (similar to Node.js) on a single process per CPU core. Each worker handles thousands of connections concurrently using non-blocking I/O. Connections are concurrent (many in-flight), but each worker is single-threaded — no parallelism within a worker. The "multiple workers" model gives parallelism across cores.

**Node.js — concurrent, not parallel (per process).**
The JavaScript event loop runs on a single thread. Async I/O callbacks are queued and processed sequentially. This is pure concurrency: structuring I/O-bound work to overlap waiting. CPU-bound work blocks the entire loop. Node.js uses a thread pool (`libuv`) for some operations, but the JS runtime is single-threaded.

**Go runtime — concurrent AND parallel.**
Goroutines are multiplexed onto OS threads by the Go scheduler. With `GOMAXPROCS=1`, goroutines run concurrently on one core (cooperative scheduling at I/O points). With `GOMAXPROCS=N`, goroutines run in parallel across N cores. The same code structure (goroutines + channels) works for both — concurrency is the design, parallelism is the deployment.

**Redis — concurrent (event loop), not parallel.**
Redis is single-threaded for all data operations. It handles thousands of concurrent clients through an event loop multiplexing I/O. This is why Redis can serve 100k+ ops/sec on a single core — the bottleneck is never context switching. This design only works because all operations are fast (in-memory).

**PostgreSQL — concurrent AND parallel (for SELECT).**
PostgreSQL uses processes (not threads) for concurrency. Each client connection gets a dedicated backend process. Since PostgreSQL 9.6, parallel query execution allows a single query to be split across multiple worker processes — true parallelism. But writes are serialized through the WAL (write-ahead log).

**TensorFlow — parallel (data parallelism across GPUs).**
TensorFlow's execution model is inherently parallel: the computation graph is partitioned across devices (GPUs/TPUs). Data parallelism replicates the model on each device and synchronizes gradients. This is parallelism (many operations simultaneously) built on top of a concurrent design (the graph executor schedules independent operations).

**Key lesson from each:**
- Nginx shows that concurrency without parallelism is fine for I/O.
- Node.js shows that concurrency on one thread is simpler and safer.
- Go shows that concurrency and parallelism are orthogonal — design for one, deploy for the other.
- Redis shows that single-threaded doesn't mean single-client.
- PostgreSQL shows that hybrid designs exist: concurrent process pool + parallel query.
- TensorFlow shows that parallelism requires careful synchronization to be correct.

## Read the Source

- [Rob Pike — "Concurrency is not Parallelism" (2012)](https://www.youtube.com/watch?v=oV9rvDllKEg) — The definitive 30-minute talk. Watch it. Pike walks through a concurrent image-processing pipeline on a single core, then shows how exactly the same design scales to multiple cores. This is the single best explanation of the distinction.
- [C.A.R. Hoare — "Communicating Sequential Processes" (1978)](https://www.cs.cmu.edu/~crary/819-f09/Hoare78.pdf) — The original paper that introduced CSP, which later inspired Go channels and Occam. Hoare argues that concurrent programs should communicate by passing data through channels, not by sharing memory.
- [Gene Amdahl — "Validity of the Single Processor Approach" (1967)](https://www.princeton.edu/~unix/Solaris/troubleshoot/amdahl.pdf) — The original conference paper where Amdahl presented his law. Four pages that every parallel programmer should read.
- [John Gustafson — "Reevaluating Amdahl's Law" (1988)](https://citeseerx.ist.psu.edu/document?repid=rep1&type=pdf&doi=4d4a5c1c5b5c5e5f5e5d5c5b5a5) — Gustafson's response arguing that problem size scales with resources.
- [Herlihy & Shavit — "The Art of Multiprocessor Programming"](https://shop.elsevier.com/books/the-art-of-multiprocessor-programming/herlihy/978-0-12-415950-1) — Chapter 1 covers the concurrency vs parallelism distinction in the context of multiprocessor algorithms. The "universal construction" in Chapter 6 is a mind-bending demonstration of what parallelism enables.

## Ship It

The reusable artifact from this lesson lives in `outputs/`. It is:

- **A comprehensive concurrency vs parallelism reference** — the `code/notes.md` file. Use it as a quick-reference during the remaining lessons in Phase 13:
  - Lesson 02 (Race Conditions): review the memory model notes
  - Lesson 04 (Locks): reference the concurrency models table
  - Lesson 07 (Atomics): revisit the consistency model notes
  - Lesson 14 (CSP and Go channels): the CSP row in the concurrency models table gives the context for why channels exist
  - Lesson 19 (GPU / CUDA): the data-level parallelism section

Every time you ask "should I add threads here?" or "is this concurrent or parallel?", pull up `code/notes.md` and check the comparison table.

## Exercises

1. **Easy** — Classify each system as concurrent-only, parallel-only, both, or neither: (a) A web browser loading images in separate threads on a dual-core CPU. (b) A batch script that processes files one at a time on a single core. (c) An MPI program running on a 256-node cluster. (d) A spreadsheet recalculating cells in dependency order on one thread. For each, write one sentence defending your classification using the Pike definition.

2. **Medium** — You have a program that takes 100 seconds to run. 80 seconds of that is in a function you can parallelize perfectly (no overhead). The remaining 20 seconds is serial. (a) What is the maximum speedup using Amdahl's Law? (b) How many cores are needed to reach 3x speedup? (c) If you use 64 cores, what is the actual speedup — and what does that tell you about the value of the 63rd and 64th cores? (d) Now apply Gustafson's Law: if you could scale the problem by 2x for every doubling of cores, what speedup would you get at 64 cores?

3. **Hard** — Design a concurrent image-processing pipeline (like Pike's example) that composites thumbnail images from 4 source URLs, applies a filter, and uploads the result. First, design it as purely concurrent — no assumption about multiple cores. Then, explain which parts could benefit from parallelism and what limits would apply (Amdahl's serial fraction, memory bandwidth, synchronization). Finally, identify one scenario where the parallel version would be *slower* than the concurrent version (see "When NOT to use parallelism"). Do not write code — draw the pipeline as a diagram using ASCII or a markdown description.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Concurrency | "Doing many things at once" | Structuring a program as independently executing tasks that can be composed, paused, and resumed — may run on a single core via time-slicing |
| Parallelism | "Doing many things at once" | Executing multiple operations simultaneously on different cores — requires hardware support |
| Amdahl's Law | "Speedup is limited by serial code" | S(N) = 1 / ((1-P) + P/N) — maximum speedup is bounded by the serial fraction, no matter how many cores you add |
| Gustafson's Law | "Bigger problems get more speedup" | S(N) = N - (N-1)(1-P) — if problem size scales with resources, speedup grows nearly linearly |
| Time-slicing | "The OS shares the CPU" | A single core rapidly switches between tasks, giving the illusion of simultaneity |
| SIMD | "One instruction, multiple data" | A single CPU instruction operates on multiple data elements simultaneously — e.g., adding 8 pairs of floats in one cycle |
| Process | "A running program" | An OS unit with isolated address space, file descriptors, and credentials — communication requires IPC |
| Thread | "A lightweight process" | An execution flow within a process, sharing address space with sibling threads — communicates via shared memory |
| Coroutine | "A function you can pause" | A subroutine that can suspend execution and later resume from the suspension point — stackful (goroutine) or stackless (async) |
| CSP | "Go channels" | Communicating Sequential Processes — a model where independent processes communicate through synchronous channels |
| Actor | "Send messages, no shared state" | A model where each actor has private state and communicates only via asynchronous messages |

## Further Reading

- Rob Pike, "Concurrency is not Parallelism" (2012 talk) — The 30-minute video that should be required viewing for every CS student. Pike refines the definition through a running example of a concurrent image-processing pipeline.
- C.A.R. Hoare, "Communicating Sequential Processes" (1978) — The seminal paper. CSP formalizes concurrent systems as processes that communicate through named channels. Every Go programmer benefits from understanding where `chan` comes from.
- Gene Amdahl, "Validity of the Single Processor Approach to Achieving Large Scale Computing Capabilities" (1967) — The original Amdahl's Law paper. Read it to see the argument in its pure form, before it became a textbook formula.
- John L. Gustafson, "Reevaluating Amdahl's Law" (1988) — Gustafson's response. Argues that Amdahl's fixed-size assumption doesn't match real parallel workloads where problem size grows with available resources.
- Maurice Herlihy and Nir Shavit, "The Art of Multiprocessor Programming" (2nd ed.) — Chapters 1–2 cover the concurrency vs parallelism distinction and the shared-memory model. The rest of the book builds on this foundation.
