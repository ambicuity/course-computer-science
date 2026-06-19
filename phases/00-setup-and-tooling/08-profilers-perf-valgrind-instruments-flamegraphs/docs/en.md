# Profilers — perf, valgrind, instruments, flamegraphs

> Optimizing without measuring is gambling. Profilers tell you *where the time actually goes* — and it's almost never where you guessed.

**Type:** Build
**Languages:** Shell, C
**Prerequisites:** Phase 00, Lessons 04, 07
**Time:** ~75 minutes

## Learning Objectives

- Distinguish *sampling* profilers (perf, Instruments) from *instrumentation* profilers (valgrind/callgrind) and pick the right one per question.
- Run a sampling profiler against a CPU-bound binary, generate a flamegraph, and read the call stack distribution.
- Use `valgrind --tool=memcheck` and `--tool=callgrind` to find memory bugs and per-call costs.
- Refute or confirm a performance hypothesis end-to-end: build → measure → identify hot path → change → re-measure.

## The Problem

"My program is slow." That sentence is everywhere, and it almost always comes with a guess: "I think the bottleneck is the disk." "I think the JSON parsing is killing us." "I think we're running out of cache." Those guesses are wrong more than half the time. A profiler cuts through them by *observing* — sample the program 1,000 times a second, count what function is on top of the stack each time, and you have a histogram of where time really goes.

But profilers come in flavors. Some sample (cheap, statistical, mostly accurate at scale). Some instrument every basic block (perfectly accurate, slow, distorts the program). Some measure the kernel (perf), some measure userspace, some measure cache misses, some measure I/O. Pick the wrong tool and you'll get a beautiful answer to a question you didn't ask.

This lesson is a tour of the four most useful profilers and the discipline of reading their output. The discipline is the lesson — the tools are interchangeable; the question you ask isn't.

## The Concept

### Sampling vs instrumentation

| | Sampling | Instrumentation |
|--|----------|-----------------|
| How it works | Interrupt the CPU N times/sec, record the program counter and call stack | Insert measurement code at every function entry/exit (or every basic block) |
| Overhead | Low (1–5%) | High (5×–50× slowdown) |
| Accuracy | Statistical (good for hot paths, noisy for rare events) | Exact counts |
| Tools | `perf record`, Instruments, Xcode, Visual Studio Profiler, Linux SystemTap | `valgrind --tool=callgrind`, gprof, instrumentation builds |
| When to use | "Where does my program spend time?" | "Exactly how many times is X called and how much does each call cost?" |

Modern profilers almost always start with sampling. Drop to instrumentation only when you need exact counts (e.g., cache-miss attribution at the instruction level).

### Flamegraphs

A flamegraph is the most useful visualization of sampled stacks:

```
   ┌─────────────────────────────────────────────────────────┐
   │ main                                                     │  ← bottom = root of stack
   ├─────────────────────────────────────────────────────────┤
   │ process_request          │ process_response              │  ← children
   ├──────────────┬──────────┼─────────────────┬─────────────┤
   │ parse_json   │ db_query │ render_template │ write_buf   │
   └──────────────┴──────────┴─────────────────┴─────────────┘
        width = % of total samples that frame appears in
```

Read width = "how much wall time is spent in this function's subtree?" Anything wide is hot; if you want to speed the program up, look there. The vertical axis just shows depth — it's not "time order."

### What sampling can't see

Sampling profilers see the **on-CPU** time. If your program is *off-CPU* — waiting on a disk read, a mutex, a network round-trip — sampling won't show you that. For off-CPU work, use:

- `perf record -e sched:sched_switch` (or BPF-based off-CPU profiling) to see what woke up after a wait.
- `strace`/`dtruss` for syscall traces.
- Async-aware tracers (`async-profiler` for JVM; OpenTelemetry traces for distributed work).

Knowing which kind of "slow" you have — CPU-bound vs IO-bound vs lock-contention — is half the problem.

### perf, Instruments, callgrind — pick your platform

| Platform | Sampling profiler | Memory checker | Per-call cost | Visualizer |
|----------|-------------------|----------------|---------------|------------|
| Linux  | `perf record / report` | `valgrind --tool=memcheck`, ASan/UBSan | `valgrind --tool=callgrind` | `perf script | FlameGraph/stackcollapse-perf.pl | FlameGraph/flamegraph.pl` |
| macOS  | Instruments.app (or `xctrace record`) | Address Sanitizer (`-fsanitize=address`) | Instruments → Time Profiler | Instruments has built-in flamegraph view |
| Cross  | `eBPF` (BCC, bpftrace) on Linux 4.x+ | sanitizers via clang/gcc | sampling profilers + custom instrumentation | Brendan Gregg's `FlameGraph` repo |

### Reading a profile honestly

Before you act on a profile, sanity check:

1. **Did you run a release build?** Profile data from `-O0` debug builds is misleading — inlining, dead-code elimination, vectorization all transform what runs.
2. **Was the workload representative?** A 100-ms run profiled is mostly start-up cost; profile something that runs ≥ 5 seconds.
3. **Did you include warm-up?** First-run effects (cold caches, JIT) skew the first few seconds.
4. **Is the bottleneck inside *your* code or a library?** A flamegraph dominated by `malloc` or `memcpy` is usually a sign of allocation pressure, not a slow `malloc`.

## Build It

The `code/main.c` is a tiny program with three functions, each doing different amounts of CPU work. You'll profile it.

### Step 1: Build for profiling

```sh
cd code/
gcc -O2 -g main.c -o profile-target           # -O2 to be representative; -g for symbol names
./profile-target                                # confirms it runs
```

Use `-O2` (or `-O3`) for profiling. Keep `-g` so the profiler can resolve addresses to symbols. Do NOT use `-pg` (the gprof flag) unless you specifically want instrumentation — it doesn't compose well with `perf`.

### Step 2: Sample with `perf` (Linux)

```sh
perf record --call-graph dwarf ./profile-target
perf report                                # interactive TUI
```

`--call-graph dwarf` tells perf to walk stacks using DWARF unwinding (works for builds without frame pointers).

### Step 3: Generate a flamegraph

```sh
# One-time install of the flamegraph tools
git clone https://github.com/brendangregg/FlameGraph
PATH="$PWD/FlameGraph:$PATH"

perf record --call-graph dwarf -F 999 -- ./profile-target
perf script | stackcollapse-perf.pl | flamegraph.pl > flame.svg
# open flame.svg in a browser
```

Look at the widest bar near the top. That's where the time is.

### Step 4: macOS — Instruments / `xctrace`

```sh
# CLI:
xctrace record --template 'Time Profiler' --launch ./profile-target
# Open the resulting .trace bundle in Instruments.app

# Or interactively:
open -a Instruments.app
# Choose "Time Profiler", click record, run your binary, stop, read.
```

Instruments' built-in flamegraph view is the same shape as Linux's; reading is identical.

### Step 5: valgrind — bug-finding mode

```sh
# Compile with -O0 -g for clearest reports
gcc -O0 -g main.c -o profile-target-dbg
valgrind --tool=memcheck --leak-check=full ./profile-target-dbg
```

`memcheck` finds:
- Uninitialized memory reads.
- Out-of-bounds heap accesses.
- Memory leaks (allocated and never freed).
- Use-after-free.

Each report is a stack trace at the moment of the misuse. Fix the top frame; rerun.

### Step 6: callgrind — exact call counts

```sh
valgrind --tool=callgrind ./profile-target-dbg
callgrind_annotate callgrind.out.<pid>             # text report
# or: kcachegrind callgrind.out.<pid>              # GUI (Linux)
```

callgrind ran your program inside an emulator, so it's slow (~20×). But the counts are exact: every call's instruction count, every cache miss, every branch mispredict.

### Step 7: Reduce a real hotspot

`main.c` has a deliberately slow function. Profile, identify which one, refactor it (e.g., precompute a value out of the loop), re-profile, confirm the bar is narrower. That before/after delta is the only valid evidence a "performance fix" actually helped.

## Use It

Real-world performance investigations use exactly this loop:

- **Linux kernel scheduler tuning** uses `perf sched` + flamegraphs to find which scheduler decisions cost the most.
- **PostgreSQL** ships with a built-in sampling profiler accessible via `pg_stat_statements`; for deeper work, devs run perf against the postgres process.
- **Chrome's tracing** (chrome://tracing) is the same model: sample, collect, flamegraph.
- **Datadog's continuous profiler** runs perf in production on every host and ships compressed stacks back to a central UI.

You'll meet sampling in Phase 13 (lock contention), Phase 15 (cache-aware design), Phase 17 (model-checking real systems against the profile).

## Read the Source

- [Brendan Gregg's "Linux Performance"](https://www.brendangregg.com/linuxperf.html) — the index of everything; start with "USE method" and "flame graphs."
- [`perf` wiki](https://perf.wiki.kernel.org/index.php/Main_Page) — semi-official Linux perf docs.
- [Valgrind manual](https://valgrind.org/docs/manual/manual.html) — clear, exhaustive.
- [Apple's "Improving Performance with Instruments"](https://developer.apple.com/documentation/xcode/improving-your-app-s-performance) — macOS equivalents.

## Ship It

This lesson ships **`outputs/profile-runbook.md`** — a 1-page workflow: pick a tool, run, read, decide. Glue it to your monitor.

## Exercises

1. **Easy.** Build the lesson's `main.c` with `-O0` and again with `-O2`. Profile each (sampled, 5 seconds). Compare the flamegraphs — how does optimization change what's on top?
2. **Medium.** Modify `main.c` to add a deliberate memory leak (`malloc` without `free`). Run under `valgrind --leak-check=full`. Read the report, fix the leak, re-run, confirm "no leaks possible."
3. **Hard.** Pick an open-source CLI you use (jq, rg, fzf). Profile it on a real-world workload (a big JSON, your home directory, etc.). Produce a flamegraph and a 1-paragraph hypothesis about the dominant hotspot. (You don't need to fix it — the *reading* is the exercise.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Sampling profiler | "It measures the program" | Interrupts the CPU at a fixed rate, records the call stack; the distribution approximates time spent |
| Instrumentation profiler | "Slower but more accurate" | Inserts measurement code at function/basic-block boundaries; exact but distorts timing |
| Flamegraph | "A bar chart" | A visualization of sampled call stacks where width = aggregate samples of that subtree |
| Off-CPU time | "Idle" | Time the thread is blocked on a syscall, lock, IO — *invisible* to a pure-CPU sampler |
| Frame pointer | "The base register" | The CPU register convention that lets stack walkers find each previous frame cheaply |

## Further Reading

- *Systems Performance: Enterprise and the Cloud* by Brendan Gregg — the textbook.
- [USE Method](https://www.brendangregg.com/usemethod.html) — a checklist for diagnosing a performance issue from first principles.
- [Top-Down Microarchitecture Analysis (Intel)](https://www.intel.com/content/www/us/en/docs/vtune-profiler/cookbook/2024-0/top-down-microarchitecture-analysis-method.html) — for when sampling isn't enough and you need to know *why* the CPU stalls.
