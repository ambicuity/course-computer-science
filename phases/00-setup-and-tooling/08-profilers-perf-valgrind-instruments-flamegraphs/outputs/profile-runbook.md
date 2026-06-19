# Profile Runbook — One-Page Workflow

## Step 0: Ask the question precisely

| Symptom | Likely tool |
|---------|-------------|
| "It's slow CPU-bound" | Sampling profiler → flamegraph |
| "It's slow but CPU is idle" | Off-CPU profiler / strace / async tracer |
| "It crashes intermittently / corrupt data" | Sanitizers + valgrind memcheck |
| "It works but allocates too much" | Heap profiler (`heaptrack`, `bytehound`) |
| "I need to know exact call counts" | callgrind / instrumented build |
| "I want to find lock contention" | `perf lock` / `mutrace` / TSAN |

## Step 1: Build correctly

```sh
# CPU sampling:  -O2 or -O3 (release-realistic) + -g (debug symbols, no perf cost)
gcc -O2 -g main.c -o bin

# Memory checking:  -O0 -g (clearest reports; perf doesn't matter under valgrind)
gcc -O0 -g main.c -o bin-dbg

# Frame-pointer-aware sampling (improves DWARF unwinding speed)
gcc -O2 -g -fno-omit-frame-pointer main.c -o bin
```

Never profile a `-O0` build for CPU; the answer will be misleading (inlining absent).

## Step 2: Sample (Linux)

```sh
# 30-second sample at 999 Hz with DWARF call-graph
perf record -F 999 -g --call-graph dwarf -o perf.data -- ./bin <args>

# Read interactively
perf report

# Or flamegraph (one-time setup):
git clone https://github.com/brendangregg/FlameGraph
perf script | ./FlameGraph/stackcollapse-perf.pl | ./FlameGraph/flamegraph.pl > flame.svg
```

## Step 2: Sample (macOS)

```sh
# Time-profile launch
xctrace record --template "Time Profiler" --launch ./bin --output bin.trace
open bin.trace                              # Instruments.app

# Or attach to a running PID
xctrace record --template "Time Profiler" --attach <pid>
```

## Step 3: Read the result

- **Top of flamegraph, widest bar** = the function holding the most CPU. Click in.
- **A flat plateau across many roots** = no single hotspot; look at startup, allocation pressure, GC.
- **`malloc` / `free` / `memcpy` at the top** = allocation pressure, not slow `malloc`. Reduce allocations.
- **`__GI___...` / `lib*.so` at the top** = your hot code is calling into a library; figure out which call and why.

## Step 4: Hypothesis → change → re-measure

Never commit a performance change without before/after profiles. The before/after delta in the wide bar is the only valid evidence.

## Step 5: Memory hygiene

```sh
valgrind --tool=memcheck --leak-check=full --show-leak-kinds=all ./bin-dbg
# read top-of-stack for each error; fix from top
```

Or, with newer toolchains (faster, more lifelike than valgrind):

```sh
gcc -O1 -g -fsanitize=address,undefined main.c -o bin-asan
./bin-asan
```

## Anti-patterns

- Profiling under `gdb` (huge overhead, distorted timing).
- Profiling with the wrong binary (release shipped, dev profiled — not the same code).
- Profiling at the wrong N (run for ≥ 5 s of representative work).
- Trusting one run — sampling is noisy. Re-run 3×; numbers within ~10% are agreement.
