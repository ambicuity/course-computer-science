# Profiling Cheatsheet — perf, dtrace, Instruments, eBPF

Quick reference card for Linux and macOS profiling commands and how to interpret their output.

---

## perf (Linux)

### perf stat — Hardware Counter Collection

```bash
# Basic: cycles, instructions, cache refs/misses, branch misses
perf stat ./benchmark

# Specific events
perf stat -e cycles,instructions,cache-references,cache-misses,branch-misses ./benchmark

# L1 and LLC cache details
perf stat -e L1-dcache-loads,L1-dcache-load-misses,LLC-loads,LLC-load-misses ./benchmark

# Per-thread breakdown (for multi-threaded programs)
perf stat -p <pid> --per-thread
```

**How to read it:**
- **IPC** (Instructions Per Cycle) = instructions / cycles. Above 2.0 = good. Below 1.0 = CPU stalling.
- **Cache-miss rate** = cache-misses / cache-references. Above 5% = poor locality.
- **Branch-miss rate** = branch-misses / branches. Above 5% = unpredictable branching.

### perf record + perf report — Hotspot Analysis

```bash
# Record with call graphs (DWARF unwinding)
perf record -g -- ./benchmark

# Record at higher frequency (more samples = more accuracy, more overhead)
perf record -F 9999 -g -- ./benchmark

# Record a specific process
perf record -g -p <pid> -- sleep 10

# Report: interactive TUI
perf report

# Report: text output
perf report --stdio

# Report: top N entries
perf report --stdio | head -30
```

**How to read it:**
- **Overhead %** = fraction of samples in this function. The top entry is your hotspot.
- **Self** vs **Children**: Self = time in the function itself. Children = time in callees.
- **Call graph**: Shows which callers led to the hotspot. Follow the arrows to find *why* the hotspot is reached.

### perf annotate — Inline Source Annotation

```bash
# Annotate a specific function
perf annotate random_access

# Annotate all functions
perf annotate
```

**How to read it:**
- Percentages next to instructions show what fraction of samples hit that line.
- A high percentage on a load instruction usually means a cache miss.
- A high percentage on a branch instruction usually means branch misprediction.

### perf top — Live Profiling

```bash
# Live view of hotspots (like top, but per-function)
sudo perf top

# Filter to a specific PID
sudo perf top -p <pid>
```

---

## dtrace (macOS / Solaris / BSD)

### One-Liners

```bash
# Count syscalls by process name
sudo dtrace -n 'syscall:::entry { @counts[execname] = count(); }'

# Measure syscall latency distribution (histogram)
sudo dtrace -n '
    syscall:::entry { self->ts = timestamp; }
    syscall:::return /self->ts/ {
        @time[execname] = quantize(timestamp - self->ts);
        self->ts = 0;
    }'

# Trace open() calls with filenames
sudo dtrace -n 'syscall::open:entry { printf("%s — %s", execname, copyinstr(arg0)); }'

# Profile on-CPU stacks at 997 Hz
sudo dtrace -n 'profile-997 { @stacks[ustack()] = count(); }'

# Count function calls in a process by PID
sudo dtrace -n 'pid$target:::entry { @calls[probefunc] = count(); }' -p <pid>

# Time a specific function
sudo dtrace -n 'pid$target:myapp:myfunc:entry { self->ts = timestamp; }
                 pid$target:myapp:myfunc:return { printf("%d ns", timestamp - self->ts); }' -p <pid>
```

### How to Read dtrace Output

- **count()** produces total counts. Sort by highest count.
- **quantize()** produces power-of-2 histograms. Look at which bucket has the most entries.
- **ustack()** shows user-level call stacks. The top frame is the innermost function.
- Timestamps are in nanoseconds. Divide by 1e6 for milliseconds.

---

## Instruments (macOS)

### CLI Commands

```bash
# Record a Time Profiler trace
xcrun xctrace record --template "Time Profiler" --launch -- ./benchmark

# Record with a specific template
xcrun xctrace record --template "Allocations" --launch -- ./benchmark

# Record System Trace (syscalls + scheduling)
xcrun xctrace record --template "System Trace" --launch -- ./benchmark

# Export trace data to text
xcrun xctrace export --input Recording.trace \
    --xpath '/trace-toc/run/data/table[@schema="time-profile"]'

# List available templates
xcrun xctrace list templates
```

### GUI Workflow

1. Open Instruments → Choose "Time Profiler"
2. Select target process or executable
3. Click Record
4. Run your workload
5. Stop recording
6. Sort by "Weight" (percentage of CPU time)
7. Click a function → see its call graph and source annotation

### Key Instruments

| Instrument | What it shows | When to use |
|---|---|---|
| Time Profiler | CPU time per function (sampling) | Finding hotspots |
| Allocations | Heap allocation tracking | Memory bloat, leak detection |
| System Trace | Syscalls, thread scheduling, I/O | Understanding I/O or scheduling overhead |
| Counters | Hardware PMU events | Cache miss rates, branch prediction |
| Leaks | Unreachable allocations | Finding memory leaks |

---

## eBPF / bpftrace (Linux 4.x+)

### bpftrace One-Liners

```bash
# Count syscalls by process
sudo bpftrace -e 'tracepoint:raw_syscalls:sys_enter { @[comm] = count(); }'

# Trace openat() calls with filenames
sudo bpftrace -e 'tracepoint:syscalls:sys_enter_openat {
    printf("%s: %s\n", comm, str(args->filename)); }'

# Profile on-CPU user stacks at 99 Hz
sudo bpftrace -e 'profile:hz:99 { @[ustack] = count(); }'

# Measure function latency (uprobe)
sudo bpftrace -e 'uprobe:/path/to/binary:function_name { @start[tid] = nsecs; }
                  uretprobe:/path/to/binary:function_name {
                      @ns = hist(nsecs - @start[tid]); delete(@start[tid]); }'

# Trace block I/O latency
sudo bpftrace -e 'kprobe:blk_start_request { @start[tid] = nsecs; }
                   kprobe:blk_finish_request { @ns = hist(nsecs - @start[tid]); }'

# Run with a target command
sudo bpftrace -e 'tracepoint:raw_syscalls:sys_enter { @[comm] = count(); }' -c ./benchmark
```

### BCC Tools (Pre-Built)

```bash
cachestat          # Cache hit/miss rates per second
biolatency         # Block I/O latency histogram
biosnoop           # Per-I/O block trace with latency
execsnoop          # Trace every new process execution
opensnoop          # Trace every open() syscall
tcpconnect         # Trace TCP active connections
tcpaccept          # Trace TCP passive connections
offcputime         # Off-CPU time by stack trace
funclatency        # Function latency histogram
slabratetop        # Kernel slab cache allocation rate
```

---

## Choosing the Right Tool

| Question | Tool | Platform |
|---|---|---|
| Which function is the hotspot? | perf record / Instruments Time Profiler | Linux / macOS |
| How many cache misses? | perf stat -e cache-misses | Linux |
| Cache hit rate over time? | cachestat (BCC) | Linux |
| Which syscalls? | strace -c / dtrace / bpftrace | Linux / macOS / Linux |
| Syscall latency distribution? | dtrace quantize / bpftrace hist | macOS / Linux |
| Kernel events? | bpftrace / BCC tools | Linux |
| Memory leaks? | Instruments Allocations / valgrind | macOS / Linux |
| Off-CPU bottlenecks? | offcputime (BCC) / perf record --off-cpu | Linux |
| Live overview? | perf top / htop / Instruments | Linux / macOS |

---

## Interpreting Profiling Output

### Finding the Hotspot

1. Sort by **Overhead %** or **Weight %** — the top entries are the hotspots.
2. Check **Self** vs **Inclusive** time:
   - High Self = the function itself is slow → optimize the function body
   - High Inclusive, low Self = callees are slow → optimize the callees
3. For the hotspot, check hardware counters: is it cache misses? branch mispredictions?

### IPC Guide

| IPC Range | Interpretation |
|---|---|
| < 0.5 | Severely stalled — likely cache misses or lock contention |
| 0.5 – 1.0 | Stalled — data dependency or memory-bound |
| 1.0 – 2.0 | Moderate — some stalls, could improve |
| 2.0 – 4.0 | Good — CPU is doing real work |
| > 4.0 | Excellent — likely compute-bound with good vectorization |

### Cache Miss Rate Guide

| L1 Miss Rate | LLC Miss Rate | Likely Issue |
|---|---|---|
| < 1% | < 5% | Good locality |
| 1–5% | 5–15% | Moderate — consider prefetching |
| > 5% | > 15% | Poor locality — restructure data access |