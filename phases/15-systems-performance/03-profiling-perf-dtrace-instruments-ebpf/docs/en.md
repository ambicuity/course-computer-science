# Profiling — perf, dtrace, Instruments, eBPF

> You can't optimize what you can't measure. Profiling turns guesswork into data.

**Type:** Learn
**Languages:** Shell, C
**Prerequisites:** Phase 15 lessons 01–02
**Time:** ~75 minutes

## Learning Objectives

- Distinguish sampling from instrumentation and understand their overhead tradeoffs.
- Use `perf stat`, `perf record`, `perf report`, and `perf annotate` to find hotspots.
- Read hardware performance counters (cycles, instructions, cache misses, branch mispredictions).
- Write dtrace one-liners to trace syscalls and function entry/exit on macOS.
- Use Instruments (Time Profiler, Allocations, System Trace) via the GUI and CLI.
- Write bpftrace one-liners and use BCC tools for kernel-level observability on Linux.
- Choose the right profiler for the job based on what you need to observe and what overhead you can tolerate.

## The Problem

Your program is slow. Where do you look first?

Without profiling, you're guessing. You might add a print statement with a timer, recompile, run it, see 3.2 seconds, then add another print... and repeat. This is manual instrumentation — slow, invasive, and incomplete. You never think to measure the thing that's actually killing performance because you didn't know it was there.

Profiling gives you a map. It tells you *where* time is spent, *why* it's spent there (cache misses? syscalls? branch mispredictions?), and *how* the call graph connects the pieces. Without it, you're navigating a city blindfolded.

Concretely, *not* knowing this means you get stuck the moment you try to measure honestly, tune cache, branches, or IO. You can win 10x by knowing the machine — but only if you can see what the machine is doing.

## The Concept: Sampling vs Instrumentation

There are two fundamental approaches to profiling:

### Sampling (Statistical Profiling)

A sampler periodically interrupts the program and records where it is — the instruction pointer, the call stack. After running for a while, you get a statistical distribution: "30% of samples were in function X, 20% in function Y."

```
  Time ──────────────────────────────────────────────►

  Sample points:  ▼     ▼   ▼     ▼ ▼   ▼     ▼
  Thread state:  [foo] [foo] [bar] [foo][bar] [foo] [foo]

  Result: foo appears in 5/7 samples ≈ 71% of time
          bar appears in 2/7 samples ≈ 29% of time
```

**Overhead:** Low (~1–5%). The sampler only pokes the program at intervals (e.g., 1000 Hz or 4000 Hz). Between samples, the program runs at full speed.

**Accuracy:** Statistical. If a function takes 1% of total time and you sample at 1000 Hz for 10 seconds, you get about 100 samples total — and that function might show up 0 or 2 times, giving you 0% or 2% instead of 1%. Small hotspots can be missed or misestimated.

**Examples:** `perf record` (default), Instruments Time Profiler, `perf top`.

### Instrumentation (Exact Profiling)

An instrumented profiler inserts probes at function entry/exit (or other points) that record every call. You get exact counts and timings.

```
  Function foo() called 1,000,000 times
    ├─ Total time inside foo: 2.34s
    ├─ Average per call: 2.34μs
    └─ Callees: bar() called 500,000 times from foo

  Function bar() called 500,000 times
    ├─ Total time inside bar: 0.87s
    └─ Average per call: 1.74μs
```

**Overhead:** Higher (5–50%+). Every instrumented function call adds a probe hit. If your hot loop calls a function 10 million times, you get 10 million probe hits.

**Accuracy:** Exact. You know precisely how many times `foo` was called and how long it took.

**Examples:** `perf record -g --call-graph=dwarf` with instrumentation, dtrace probes, `bpftrace` with `uretprobe`, `gcc -pg` (gprof instrumentation).

### Choosing Between Them

```
  ┌──────────────────────────────────────────────────────────────┐
  │                    Which profiler?                           │
  │                                                              │
  │  "Where is CPU time going?"      ──►  Sampling (perf record)│
  │  "How many times was X called?"   ──►  Instrumentation       │
  │  "Which syscalls?"               ──►  dtrace / eBPF         │
  │  "Cache miss rate?"              ──►  perf stat (HW counters)│
  │  "Kernel event X?"               ──►  eBPF / bpftrace        │
  │  "macOS GUI profiling?"           ──►  Instruments           │
  └──────────────────────────────────────────────────────────────┘
```

## Linux perf: The Swiss Army Knife

`perf` is the primary profiling tool on Linux. It accesses CPU hardware performance counters via the kernel's `perf_events` subsystem.

### perf stat — Hardware Counter Collection

`perf stat` counts hardware events over a program's entire runtime:

```
  $ perf stat ./benchmark

   Performance counter stats for './benchmark':

        3,245,678,901      cycles
        5,812,345,678      instructions              # 1.79 IPC
          234,567,890      cache-references
           18,765,432      cache-misses               # 8.00% of all cache refs
            2,345,678      branch-misses

        1.234567890 seconds time elapsed
```

Key metrics:
- **IPC** (Instructions Per Cycle): < 1.0 means the CPU is stalled often. Good code hits 2–4 on modern CPUs.
- **cache-misses**: High miss rate means your data access patterns fight the cache hierarchy.
- **branch-misses**: Random branches that the predictor can't forecast waste pipeline slots.

### perf record + perf report — Hotspot Analysis

`perf record` samples the program's execution and writes to a `perf.data` file:

```
  $ perf record -g ./benchmark    # -g records call graphs
  [ perf record: Woken up 1 times to write data ]
  [ perf record: Captured and wrote 0.123 MB perf.data ]
```

`perf report` shows where time was spent:

```
  Overhead  Command    Object      Symbol
  ........  .......    ......      ......
    42.31%  benchmark  benchmark   [.] random_access
    28.15%  benchmark  benchmark   [.] matrix_multiply
    18.07%  benchmark  benchmark   [.] branchy
     5.20%  benchmark  benchmark   [.] sequential_access
     3.10%  benchmark  libc.so     [.] __random
     ...
```

The "Overhead" column tells you: 42% of samples were inside `random_access`. That's your hotspot.

### perf annotate — Inline Source Annotation

`perf annotate` shows assembly interleaved with source, marking which instructions were sampled most:

```
  $ perf annotate random_access

  Percent |      Source code & objdump
  --------+---------------------------------------------
          |  for (i = 0; i < n; i++) {
   12.03% |    sum += arr[indices[i]];
          |  }
```

The `12.03%` next to the line means 12% of all samples in this function hit this instruction. This is how you find the exact line causing slowdowns.

### Hardware Performance Counters

Modern CPUs expose a set of programmable counters. The most important:

```
  ┌──────────────────────┬────────────────────────────────────────────┐
  │ Counter              │ What it tells you                         │
  ├──────────────────────┼────────────────────────────────────────────┤
  │ cycles               │ Total CPU cycles elapsed                  │
  │ instructions         │ Total instructions retired                │
  │ cache-references     │ Number of cache lookups                  │
  │ cache-misses         │ Lookups that missed all cache levels     │
  │ branch-misses        │ Branches the predictor got wrong          │
  │ L1-dcache-loads      │ L1 data cache load operations             │
  │ L1-dcache-load-misses│ L1 data cache misses                     │
  │ LLC-loads            │ Last-level cache (L3) loads              │
  │ LLC-load-misses      │ Last-level cache misses                  │
  │ dTLB-loads           │ Data translation lookaside buffer lookups │
  │ dTLB-load-misses     │ TLB misses (page table walks needed)     │
  └──────────────────────┴────────────────────────────────────────────┘
```

IPC and cache miss rate are the two numbers to watch:

```
  IPC < 1.0  ──►  CPU is stalled (cache misses? branch mispredictions?)
  IPC > 2.0  ──►  Good instruction throughput (likely compute-bound)

  L1 miss rate > 5%    ──►  Poor spatial locality in inner loop
  LLC miss rate > 20%  ──►  Working set doesn't fit in L3 cache
```

## dtrace: Dynamic Tracing (macOS / Solaris / BSD)

dtrace was created at Sun Microsystems for Solaris, then ported to macOS, FreeBSD, and others. It lets you insert probes into running programs and the kernel — with zero modification to the target code.

### How dtrace Works

dtrace uses a kernel-level probe framework. Probes are organized by **provider:function:name**:

```
  Provider:  syscall    ──►  system call entry/return
  Provider:  vminfo     ──►  virtual memory statistics
  Provider:  io         ──►  disk I/O events
  Provider:  sched      ──►  CPU scheduling events
  Provider:  proc       ──►  process lifecycle (fork, exit, signal)
  Provider:  fbt        ──►  kernel function boundary tracing (Solaris)
```

On macOS, `dtrace` is available but some providers are restricted by SIP (System Integrity Protection). You may need to disable SIP or use `sudo`.

### dtrace One-Liners

```bash
# Count syscalls by process name
sudo dtrace -n 'syscall:::entry { @counts[execname] = count(); }'

# Time how long each syscall takes
sudo dtrace -n 'syscall:::entry { self->ts = timestamp; }
                 syscall:::return { @time[execname] = quantize(timestamp - self->ts); }'

# Trace all open() calls with filenames
sudo dtrace -n 'syscall::open:entry { printf("%s — %s", execname, copyinstr(arg0)); }'

# Count function calls in a specific process (by PID)
sudo dtrace -n 'pid$target:::entry { @calls[probefunc] = count(); }' -p 12345

# Profile on-CPU time at 997 Hz (like perf record)
sudo dtrace -n 'profile-997 { @stack[stack()] = count(); }'
```

The `quantize()` function is powerful — it produces a power-of-2 histogram:

```
  value  ------------- Distribution ------------- count
  1024   |                                         0
  2048   |@@@@@@@@@@@@@@@@@@@@@@@@@@@@@            84
  4096   |@@@@@@@@@@@@@@@@                         48
  8192   |@@                                       7
  16384  |                                         0
```

This tells you most syscalls complete in 2048–4096 nanoseconds, but a few take 8192+.

## Instruments: macOS Profiling GUI (and CLI)

Instruments is Apple's profiling suite. It provides several "instruments" (trace templates):

```
  ┌──────────────────────┬─────────────────────────────────────────────┐
  │ Instrument           │ What it traces                              │
  ├──────────────────────┼─────────────────────────────────────────────┤
  │ Time Profiler        │ CPU time per function (sampling-based)      │
  │ Allocations          │ Heap allocations (malloc/free tracking)     │
  │ System Trace         │ Syscalls, thread scheduling, I/O           │
  │ Leaks                │ Memory leaks (unreachable allocations)      │
  │ Counters             │ Hardware performance counters              │
  │ Custom               │ dtrace-based instruments                   │
  └──────────────────────┴─────────────────────────────────────────────┘
```

### Using Instruments from the CLI

You can drive Instruments from the command line:

```bash
# Record a Time Profile trace
xcrun xctrace record --template "Time Profiler" --launch ./benchmark

# Record and then open in the Instruments GUI
instruments -t "Time Profiler" -D trace.trace ./benchmark

# Convert .trace file to text (macOS 12+)
xcrun xctrace export --input recording.trace --xpath '/trace-toc/run/data/table[@schema="time-profile"]'
```

### Reading Instruments Output

In the Time Profiler:
- **Weight column**: percentage of CPU time in each function.
- **Heaviest stack trace**: the call stack that appeared most often in samples.
- **Self vs. Total**: "Self" is time in the function itself (excluding callees). "Total" includes callees. A function with high Self time is your hotspot.

## eBPF: Modern Linux Tracing

eBPF (extended Berkeley Packet Filter) is the modern evolution of kernel tracing on Linux. It lets you run sandboxed programs inside the kernel without modifying kernel source or loading modules.

### Why eBPF Over dtrace on Linux?

```
  ┌────────────────────────┬──────────────────┬───────────────────┐
  │ Feature                │ dtrace (Linux)   │ eBPF              │
  ├────────────────────────┼──────────────────┼───────────────────┤
  │ Kernel support         │ Out-of-tree module│ Built-in (4.x+)   │
  │ Safety                 │ Limited           │ Verifier checks    │
  │ Community traction     │ Small             │ Massive            │
  │ Tool ecosystem         │ dtrace            │ BCC, bpftrace,    │
  │                        │                   │ Cilium, Katran...  │
  │ macOS available        │ Yes (native)      │ No (Linux only)    │
  │ Java/Python USDT       │ Yes               │ Yes                │
  └────────────────────────┴──────────────────┴───────────────────┘
```

For new Linux kernel observability, use eBPF. For macOS, use dtrace or Instruments.

### bpftrace: One-Liners for eBPF

`bpftrace` is the eBPF equivalent of dtrace one-liners:

```bash
# Count syscalls by process
sudo bpftrace -e 'tracepoint:raw_syscalls:sys_enter { @[comm] = count(); }'

# Trace open() calls with filenames
sudo bpftrace -e 'tracepoint:syscalls:sys_enter_openat { printf("%s: %s\n", comm, str(args->filename)); }'

# Profile on-CPU stacks at 99 Hz
sudo bpftrace -e 'profile:hz:99 { @[ustack] = count(); }'

# Count function calls in a specific binary
sudo bpftrace -e 'uretprobe:/path/to/binary:random_access { @[probe] = count(); }'

# Measure latency of block I/O
sudo bpftrace -e 'kprobe:blk_start_request { @start[tid] = nsecs; }
                   kprobe:blk_finish_request { @ns = hist(nsecs - @start[tid]); delete(@start[tid]); }'
```

### BCC Tools: Pre-Built eBPF Programs

BCC (BPF Compiler Collection) ships dozens of ready-to-use tools:

```
  ┌──────────────────┬──────────────────────────────────────────┐
  │ Tool             │ What it does                              │
  ├──────────────────┼──────────────────────────────────────────┤
  │ execsnoop        │ Trace new process execution              │
  │ opensnoop        │ Trace open() syscall                     │
  │ biolatency       │ Block I/O latency histogram             │
  │ biosnoop         │ Trace block I/O with details             │
  │ cachestat        │ Cache hit/miss statistics                │
  │ tcpconnect       │ Trace TCP active connections             │
  │ tcpaccept        │ Trace TCP passive connections            │
  │ slabratetop      │ Kernel slab cache rate top               │
  │ offcputime       │ Off-CPU time by stack trace              │
  │ lock_contention  │ Kernel lock contention                  │
  └──────────────────┴──────────────────────────────────────────┘
```

### How eBPF Works Under the Hood

```
  User writes bpftrace one-liner
          │
          ▼
  bpftrace compiler ──► BPF bytecode (ELF)
          │
          ▼
  Kernel BPF verifier ──► "Safe to run" or REJECT
          │ (checks: no loops, bounded memory, no null derefs)
          ▼
  BPF program attached to hook point
   (kprobe, tracepoint, uprobe, etc.)
          │
          ▼
  On event ──► BPF program runs in kernel context
          │  (reads data, updates maps, sends to perf buffer)
          ▼
  User-space reads results via ring buffer
```

The verifier is what makes eBPF safe for production: it statically guarantees the program will terminate and won't crash the kernel.

## Overhead Comparison

Every profiler adds overhead. Understanding the overhead helps you choose the right tool and interpret results accurately:

```
  ┌──────────────────────────────────────────────────────────────────┐
  │ Profiler              │ Type          │ Typical Overhead        │
  ├────────────────────────┼───────────────┼─────────────────────────┤
  │ perf stat              │ Counter       │ < 1%                    │
  │ perf record (default)  │ Sampling      │ 1–5%                    │
  │ perf record (high-freq)│ Sampling      │ 5–15%                   │
  │ dtrace (syscall probe) │ Instrument.   │ 5–20% per probe        │
  │ bpftrace (kprobe)     │ Instrument.   │ 2–10% per probe         │
  │ Instruments Time Prof  │ Sampling      │ 1–5%                    │
  │ Instruments Allocations│ Instrument.  │ 10–30%                  │
  │ gcc -pg (gprof)       │ Instrument.   │ 10–50%                  │
  └──────────────────────────────────────────────────────────────────┘

  Rule of thumb: Sampling is cheaper but statistical.
                 Instrumentation is exact but expensive.
                 Lower probe frequency = lower overhead = more statistical.
```

## Reading Profiling Output: Finding the Hotspot

Regardless of the tool, the workflow is always:

1. **Run the profiler** on your program.
2. **Sort by overhead%** (or sample count) — the top entries are the hotspots.
3. **Look at "self" vs "inclusive" time**:
   - **Self time**: time spent in the function itself (excluding callees).
   - **Inclusive time**: time spent in the function and everything it calls.
   - A function with high inclusive time but low self time means a callee is the problem.
4. **Drill into the hotspot** — look at the call graph, see who calls it and why.
5. **Check hardware counters** — is the hotspot due to cache misses or branch mispredictions?
6. **Annotate the source** — find the specific line causing the issue.

```
  perf report example:

    42.31%  benchmark  benchmark  [.] random_access
               │
               ▼  ┌─ What code is in random_access?
                    random_access() {
                      for (i = 0; i < n; i++)     ←─ Is this cache-friendly?
                        sum += arr[indices[i]];    ←─ Random indices → cache misses
                    }

               ▼  ┌─ What hardware counters say:
                    perf stat says: 8% cache-miss rate
                    That's high! The array indices cause random access.

               ▼  ┌─ Fix:
                    Sort indices first → sequential access pattern → cache-friendly
```

## When to Use Which Profiler

```
  ┌─────────────────────────┬─────────────────────────────────────────┐
  │ Scenario                │ Best tool                               │
  ├─────────────────────────┼─────────────────────────────────────────┤
  │ "Which function is hot?"│ perf record / Instruments Time Profiler │
  │ "How many cache misses?"│ perf stat -e cache-misses,cycles        │
  │ "Which syscalls?"       │ dtrace / strace -c / bpftrace           │
  │ "Why is I/O slow?"      │ Instruments System Trace / biolatency    │
  │ "Kernel-level events?"  │ bpftrace / BCC tools                    │
  │ "Memory leaks?"         │ Instruments Allocations / valgrind       │
  │ "Quick CPU overview?"   │ perf top (live) / htop                  │
  │ "Off-CPU time?"         │ offcputime (BCC) / perf record --offcpu │
  │ "Continuous monitoring"  │ eBPF programs in production             │
  └─────────────────────────┴─────────────────────────────────────────┘
```

## Build It

See `code/main.c` for a C program with four benchmark functions designed to exhibit different profiling signatures. See `code/run.sh` for commands to profile it.

### The Four Benchmarks

1. **sequential_access** — Good cache behavior. Walks an array sequentially. Expect high IPC, low cache-miss rate.
2. **random_access** — Poor cache behavior. Walks an array via random indices. Expect high cache-miss rate, low IPC.
3. **branchy** — Branch prediction issues. Random if/else on unpredictable data. Expect high branch-miss rate.
4. **matrix_multiply** — Compute-bound. O(n^3) triple loop over arrays. Expect high cycles, moderate cache behavior.

### Running the Benchmarks

```bash
# Compile with optimization but keep symbols for profiling
gcc -O2 -g -o benchmark main.c -lm

# Run all benchmarks
./benchmark all

# Run individual benchmarks
./benchmark sequential
./benchmark random
./benchmark branchy
./benchmark matrix
```

### Profiling the Benchmarks

```bash
# Hardware counters for each benchmark
perf stat -e cycles,instructions,cache-references,cache-misses,branch-misses ./benchmark random

# Record and report
perf record -g ./benchmark all
perf report

# Annotate a specific function
perf annotate random_access
```

Expected profiling results:

```
  ┌────────────────────┬────────────┬──────────────┬──────────────────┐
  │ Benchmark          │ IPC        │ Cache Miss % │ Branch Miss %    │
  ├────────────────────┼────────────┼──────────────┼──────────────────┤
  │ sequential_access  │ ~3–4       │ < 1%         │ < 0.5%           │
  │ random_access      │ ~0.5–1.0   │ 8–15%        │ < 1%             │
  │ branchy            │ ~1.5–2.0   │ < 2%         │ 15–25%           │
  │ matrix_multiply    │ ~1.0–1.5   │ 5–10%        │ < 1%             │
  └────────────────────┴────────────┴──────────────┴──────────────────┘
```

These numbers will vary by CPU, array sizes, and compiler, but the *relative* ordering should hold: sequential_access is the fastest per element, random_access has the worst cache behavior, branchy has the worst branch prediction.

## Use It

### Production Equivalents

- **perf**: The Linux kernel's primary observability interface. See `tools/perf/` in the Linux source tree.
- **dtrace**: Originally developed at Sun for Solaris. The macOS implementation is in `usr/share/dtrace/` and `/usr/lib/dtrace/`.
- **eBPF / bpftrace**: bpftrace is open source at github.com/iovisor/bpftrace. BCC tools live at github.com/iovisor/bcc.
- **Instruments**: Apple's proprietary tool, part of Xcode. The `xctrace` CLI (Xcode 12+) replaces the older `instruments` CLI.

### Comparing Your Benchmark to Real Workloads

Your `main.c` benchmarks are microbenchmarks — they isolate one effect each. Real programs mix all these effects. When profiling real code:

1. Start with `perf record -g` or Instruments Time Profiler — find the top hotspot.
2. Run `perf stat` on the hotspot function — check for cache misses or branch mispredictions.
3. If the hotspot is a syscall, trace it with `strace -c`, dtrace, or bpftrace.
4. Iterate until IPC is above 2.0 and cache-miss rate is below 2%.

## Read the Source

- **Linux perf events subsystem**: `kernel/events/core.c` in the Linux source tree. This is where `perf_event_open()` is implemented.
- **eBPF verifier**: `kernel/bpf/verifier.c` — the static analysis that proves BPF programs are safe.
- **bpftrace**: github.com/iovisor/bpftrace — see `src/ast/codegen_llvm.cpp` for how one-liners become BPF bytecode.

## Ship It

The reusable artifact for this lesson is in `outputs/` — a profiling cheatsheet (`profiling_cheatsheet.md`) with commands for perf stat, perf record/report, dtrace one-liners, bpftrace examples, and Instruments equivalents, plus how to interpret their output.

## Exercises

1. **Easy** — Run `perf stat` on each of the four benchmarks. Write down the IPC and cache-miss rate. Do they match the expected values in the table above?
2. **Medium** — Modify `random_access` to sort the indices before accessing the array. Run `perf stat` again. How much does the cache-miss rate improve? How much faster does the function run?
3. **Hard** — Write a bpftrace one-liner that measures the latency distribution of `read()` syscalls for the benchmark binary. Compare the histogram to what `strace -c` reports for total time. Which gives you more actionable information?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Sampling | "Profile the code" | Periodically interrupt and record the instruction pointer / call stack. Statistical, not exact. |
| Instrumentation | "Instrument the code" | Insert probes that fire on every event. Exact counts but higher overhead. |
| IPC | "Instructions per clock" | Instructions Per Cycle — ratio of work done to time elapsed. Above 2.0 is good. |
| Hotspot | "The slow part" | The function or code region consuming the most CPU time in the profile. |
| Hardware counters | "CPU counters" | Performance Monitoring Unit (PMU) registers that count cycles, cache events, branches. |
| eBPF | "The new dtrace" | Extended BPF — a safe, sandboxed VM in the Linux kernel for observability and networking. |
| Call graph | "The stack trace" | A directed acyclic graph showing which functions called which, with time attribution. |
| Overhead | "Profiling cost" | The extra time/slowdown the profiler itself introduces. Sampling ≈ 1–5%, instrumentation ≈ 5–50%. |

## Further Reading

- Brendan Gregg, *Systems Performance* (2nd ed.) — the definitive reference for all profiling tools discussed here.
- Brendan Gregg's eBPF tracing tutorial: brendangregg.com/ebpf.html
- Linux `perf` wiki: perf.wiki.kernel.org
- bpftrace reference guide: github.com/iovisor/bpftrace/blob/master/docs/reference_guide.md
- Apple Instruments documentation: developer.apple.com/documentation/instruments
- USE Method (Utilization, Saturation, Errors): brendangregg.com/usemethod.html