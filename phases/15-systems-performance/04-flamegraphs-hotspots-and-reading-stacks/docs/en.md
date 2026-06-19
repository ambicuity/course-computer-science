# Flamegraphs, Hotspots, and Reading Stacks

> If you can't see where time goes, you can't make it go faster.

**Type:** Learn
**Languages:** Shell, Perl
**Prerequisites:** Phase 15 lessons 01–03
**Time:** ~60 minutes

## Learning Objectives

- Read a flamegraph fluently: decode what x-axis, y-axis, width, and color actually mean.
- Generate CPU, off-CPU, and differential flamegraphs from raw perf data.
- Distinguish hotspots (wide bars) from deep call chains (tall stacks) and know when each matters.
- Understand why frame pointers are essential for accurate stack traces and what breaks without them.
- Produce a self-contained SVG flamegraph without requiring Brendan Gregg's toolchain.

## The Problem

You just profiled your web server. It handles 10,000 requests per second but p99 latency is 800 ms.
`top` says CPU usage is 35%. `strace` shows a blur of syscalls. Where is the time going?

Tools like `top` and `strace` tell you *that* something is slow, but not *where* in your code. A
flamegraph compresses millions of samples into a single picture you can read at a glance. Without
flamegraphs you are guessing. With them, you are measuring.

## The Concept

### What Is a Flamegraph?

A flamegraph is a visualization of stack trace samples. It was invented by Brendan Gregg in 2011 and
has become the standard way to see where a program spends its resources.

The axes:

```
          ┌─────────────────────────────────────────────────┐
          │               top of stack                      │
          │                                                 │
   y      │  ┌──────────┐                                  │
   axis    │  │ func_c() │                                  │
   =      │  └──────────┘                                  │
   stack  │  ┌────────────────────┐                        │
   depth  │  │     func_b()       │                        │
          │  └────────────────────┘                        │
          │  ┌──────────────────────────────────┐          │
          │  │          func_a()                │          │
          │  └──────────────────────────────────┘          │
          │  ┌────────────────────────────────────────────┐│
          │  │              main()                        ││
          │  └────────────────────────────────────────────┘│
          │               bottom of stack                  │
          └─────────────────────────────────────────────────┘
               ◄────────── x-axis = population ──────────►
               (frames sorted alphabetically at each level)
```

- **x-axis**: Not time. It is *population* — stack frames sorted alphabetically. The width of a
  bar represents the **number of samples** where that function was on the stack, not wall-clock time.
- **y-axis**: Stack depth. The bottom is the root (e.g., `main()`). The top is the leaf function
  currently executing.
- **Width**: Proportional to how often that frame appeared in samples. A wide bar = a hotspot.
- **Color**: Random warm colors (red/orange/yellow). Color does *not* encode heat or frequency. It
  exists only to visually distinguish adjacent frames.

### How to Read a Flamegraph

**Step 1: Find wide bars.** A bar that spans a large fraction of the x-axis is a hotspot — that
function (and everything below it) accounts for many samples. Start your optimization there.

**Step 2: Look up from a wide bar.** The frames above it are its callees — what it calls. The
frames below it are its callers — who calls it. The full stack from bottom to top is the call
path that led to the sample.

**Step 3: Don't confuse tall with hot.** A tall, narrow sliver means a deep call chain that was
sampled rarely. It looks dramatic but probably isn't your bottleneck. Focus on *wide* bars.

**Step 4: Hover/tlick for details.** In SVG flamegraphs, hovering shows the exact sample count and
percentage. The visual width is approximate — the tooltip gives precision.

### Worked Example

Suppose we sample a program 1,000 times and see these folded stacks:

```
main;parse;tokenize  300
main;parse;validate  200
main;compress;lz4    400
main;compress;zstd   100
```

The flamegraph looks like:

```
┌──────┐ ┌───────┐ ┌──────────┐ ┌────┐
│tokenize│ │validate│ │   lz4    │ │zstd│
└──────┘ └───────┘ └──────────┘ └────┘
┌────────────────┐ ┌────────────────────┐
│     parse      │ │     compress        │
└────────────────┘ └────────────────────┘
┌─────────────────────────────────────────┐
│                main                      │
└─────────────────────────────────────────┘
```

- `lz4` under `compress` is the widest leaf (400 samples = 40%). Optimize that first.
- `compress` as a whole is 50% of samples (500/1000). It's your parent hotspot.
- `tokenize` is 30%. Not as hot as `lz4`, but worth examining.
- `zstd` is 10%. Probably not your first target.

### Flamegraph Types

| Type         | What it shows                                    | When to use                          |
|--------------|--------------------------------------------------|--------------------------------------|
| CPU          | Functions on-CPU (running code)                  | CPU-bound bottlenecks                |
| Off-CPU      | Functions blocked (sleeping, waiting for I/O)    | Latency problems, high wait times    |
| Memory       | Allocation call paths (malloc/new sites)          | Memory bloat, leaks                  |
| I/O          | Disk or network I/O call paths                   | I/O-bound workloads                  |
| Differential | Difference between two flamegraphs (before/after)| Validating optimization impact       |

### Off-CPU Flamegraphs

A CPU flamegraph shows what your program is *doing*. An off-CPU flamegraph shows what it is
*waiting for*. If p99 latency is bad but CPU usage is low, you need off-CPU:

```
What on-CPU sees:         What off-CPU sees:
┌──────────────────┐      ┌──────────────────┐
│   compute()      │ 15%  │   futex_wait()    │ 70%
└──────────────────┘      └──────────────────┘
┌──────────────────┐      ┌──────────────────┐
│   hash()         │ 10%  │   read()          │ 15%
└──────────────────┘      └──────────────────┘
┌──────────────────┐      ┌──────────────────┐
│   main()         │      │   main()          │
└──────────────────┘      └──────────────────┘
```

Off-CPU is measured with `perf record -e sched:sched_stat_sleep` or via bcc/BPF tools like
`offcputime`.

### Differential Flamegraphs

After you optimize, generate a differential flamegraph that colors bars:
- **Red**: function got *more* samples (regression)
- **Blue**: function got *fewer* samples (improvement)
- **Gray**: no significant change

This lets you verify that your optimization actually helped and didn't just shift the bottleneck.

### Icicle Graphs

An icicle graph is a top-down variant: the root is at the top, and callees descend downward. Some
people find this more intuitive (the call hierarchy reads top-to-bottom like source code). The data
is identical; only the orientation differs.

```
          ┌─────────────────────┐
          │       main()        │   ← root at top
          └──────┬──────────────┘
                 │
       ┌─────────┴─────────┐
       │                   │
  ┌────┴─────┐      ┌──────┴─────┐
  │ parse()  │      │ compress() │
  └──┬───┬───┘      └──────┬─────┘
     │   │                  │
 ┌───┘   └───┐        ┌─────┼─────┐
 │           │        │           │
│tokenize││validate│  │   lz4  ││zstd│
```

### Reading Stack Traces

Before you can make a flamegraph, you need to understand the raw stacks that feed into it:

```
nginx 12345 [000] 1234.567:  100000 cpu-clock:
    7f8a9b123456 ngx_http_process_request [nginx]
    7f8a9b234567 ngx_http_core_run_phases [nginx]
    7f8a9b345678 ngx_http_proxy_handler [nginx]
```

Each line is one frame. The bottom line is the deepest caller. Key things to know:

- **Inlined frames**: The compiler may inline a function, so it disappears from the stack. Use
  `-fno-omit-frame-pointer` and `-fno-inline` for profiling builds, or use DWARF unwinding
  (`perf record --call-graph dwarf`).
- **Kernel frames**: Marked with `[k]` suffix. These are kernel functions running on behalf of
  your process.
- **JIT frames**: JVM and V8 emit code at runtime. Without JIT symbol translation (`perf map`),
  you see `jitted_1234` instead of function names. Use `-XX:+PreserveFramePointer` (JVM) or
  `--perf-prof` (Node.js).

### Why Frame Pointers Matter

The `perf` tool walks the stack by following frame pointers (the `rbp` register on x86-64). Modern
compilers omit frame pointers by default (`-fomit-frame-pointer`) to free up a register. This
breaks stack walking:

```
With frame pointers:              Without frame pointers:
main()                            main()
  └─ parse()                        └─ ???  (can't walk)
       └─ tokenize()                     └─ ???  (can't walk)
```

Solutions:
1. Compile with `-fno-omit-frame-pointer` for profiling builds.
2. Use DWARF unwinding: `perf record --call-graph dwarf`.
3. Use `perf record --call-graph lbr` on Intel (uses Last Branch Records).
4. Use eBPF: `bpf_get_stackid()` can use DWARF info.

## Build It

Now we'll write shell scripts that generate every type of flamegraph. The scripts in `code/run.sh`
walk through the full pipeline: `perf record` → `perf script` → `stackcollapse` → `flamegraph.pl`,
plus a self-contained SVG generator so you can visualize stacks without installing any extras.

## Use It

### Production Tooling: Brendan Gregg's FlameGraph

Brendan Gregg's original Perl scripts live at https://github.com/brendangregg/FlameGraph. The core
pipeline:

```bash
perf record -F 99 -g -- ./your_program
perf script | stackcollapse-perf.pl | flamegraph.pl > cpu.svg
```

Key scripts:
- `flamegraph.pl` — generate CPU flamegraph
- `difffolded.pl` + `flamegraph.pl --negate` — differential flamegraph
- `stackcollapse-perf.pl` — fold `perf script` output
- `stackcollapse-bpftrace.pl` — fold bpftrace output
- `stackcollapse-jstack.pl` — fold Java stack traces

### What the Production Version Does That Ours Doesn't

Brendan Gregg's tools handle:
- **Search and zoom**: click to zoom into a subtree, type to search/highlight
- **Multiple input formats**: Java, Node.js, DTrace, bpftrace, not just perf
- **Reverse stack order**: icicle graph variant (`--reverse`)
- **Color palettes**: `--color=io`, `--color=mem`, etc. for semantic coloring
- **Thread merging**: combine samples from multiple threads into one graph

Our self-contained generator produces the essential visualization without these niceties.

## Read the Source

- **FlameGraph/flamegraph.pl** — The original flamegraph generator. Read the SVG generation
  logic (lines ~200–400) to see how x-coordinate math works. Every bar's x position is computed
  from cumulative sample counts.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. It is:

- **A reference card** (`flamegraph_guide.md`) with the complete generation pipeline, common
  commands, troubleshooting for missing frames, and links to Brendan Gregg's tools.

## Exercises

1. **Easy** — Profile a `find /` command, generate a CPU flamegraph using the full pipeline, and
   identify the top 3 hotspots.
2. **Medium** — Generate both a CPU and an off-CPU flamegraph for the same workload. Write a
   paragraph explaining what each reveals that the other doesn't.
3. **Hard** — Implement a differential flamegraph comparison: given two folded-stack files, produce
   an SVG where bars are colored red (regression), blue (improvement), or gray (no change). You
   may extend the self-contained SVG generator from `run.sh`.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Flamegraph | "That fire chart" | A visualization of folded stack traces where width = sample count, not time |
| Folded stack | "The collapsed format" | Single-line format: `caller;callee;leaf <count>` — the input to flamegraph generators |
| Hotspot | "The hot part" | A wide bar representing many samples — your optimization target |
| Off-CPU | "When it's sleeping" | Time spent blocked (I/O wait, lock contention) rather than running on a CPU |
| Frame pointer | "That register thing" | The `rbp` register that chains stack frames; compiler often omits it, breaking `perf` |
| Differential | "The diff one" | A flamegraph colored by whether each function got more or fewer samples after a change |
| Stackcollapse | "The folding step" | The process of converting multi-line stacks into the semicolon-delimited folded format |

## Further Reading

- Brendan Gregg, *The Flame Graph* (CACM, 2014) — the original paper
- Brendan Gregg's FlameGraph repo: https://github.com/brendangregg/FlameGraph
- Brendan Gregg, *Systems Performance* (2nd ed.), Chapter 18 — flamegraphs in context
- `perf` documentation: `man perf-record`, `man perf-script`
- Red Hat blog: "How to generate flamegraphs on RHEL"