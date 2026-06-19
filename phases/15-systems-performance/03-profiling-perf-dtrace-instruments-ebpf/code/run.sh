#!/usr/bin/env bash
# ============================================================
# Profiling Demo Script — perf, dtrace, Instruments, eBPF
# Phase 15.03 — Systems Performance
#
# This script demonstrates real profiling commands.
# Some commands require Linux (perf, bpftrace), others macOS (dtrace, instruments).
# Run the sections that match your platform.
# ============================================================
set -euo pipefail

BENCHMARK="${1:-./benchmark}"
if [[ ! -x "$BENCHMARK" ]]; then
    echo "Building benchmark..."
    gcc -O2 -g -o benchmark main.c -lm
    BENCHMARK="./benchmark"
fi

echo "============================================================"
echo "  PHASE 1: Linux perf — Hardware Counters (perf stat)"
echo "============================================================"
echo ""
echo "# perf stat counts hardware events over the entire run."
echo "# This is the lowest-overhead profiling method (<1%)."
echo ""

# --- perf stat: collect hardware counters for each benchmark ---
echo "## sequential_access — expect high IPC, low cache-miss rate"
perf stat -e cycles,instructions,cache-references,cache-misses,branch-misses \
    "$BENCHMARK" sequential 2>&1 || echo "(perf not available on this platform)"

echo ""
echo "## random_access — expect low IPC, high cache-miss rate"
perf stat -e cycles,instructions,cache-references,cache-misses,branch-misses \
    "$BENCHMARK" random 2>&1 || echo "(perf not available on this platform)"

echo ""
echo "## branchy — expect high branch-miss rate"
perf stat -e cycles,instructions,cache-references,cache-misses,branch-misses \
    "$BENCHMARK" branchy 2>&1 || echo "(perf not available on this platform)"

echo ""
echo "## matrix_multiply — expect high cycle count, compute-bound"
perf stat -e cycles,instructions,cache-references,cache-misses,branch-misses \
    "$BENCHMARK" matrix 2>&1 || echo "(perf not available on this platform)"

echo ""
echo "============================================================"
echo "  PHASE 2: Linux perf — Hotspot Analysis (perf record/report)"
echo "============================================================"
echo ""
echo "# perf record samples the program at a frequency (default 1000 Hz)"
echo "# and writes to perf.data. perf report shows where time was spent."
echo ""

# --- perf record: sample the program running all benchmarks ---
perf record -g -o perf.data -- "$BENCHMARK" all 2>&1 || echo "(perf record not available)"

# --- perf report: show the hotspot breakdown ---
echo ""
echo "## Showing top 10 hotspots:"
perf report -i perf.data --stdio --max-stack 5 -n 2>&1 | head -40 || echo "(perf report not available)"

echo ""
echo "============================================================"
echo "  PHASE 3: Linux perf — Source Annotation (perf annotate)"
echo "============================================================"
echo ""
echo "# perf annotate shows which instructions in each function"
echo "# were sampled most frequently, interleaved with source."
echo ""

# --- perf annotate: show source-level annotation for random_access ---
perf annotate -i perf.data --stdio random_access 2>&1 | head -30 || echo "(perf annotate not available)"

echo ""
echo "============================================================"
echo "  PHASE 4: dtrace — Syscall and Function Tracing (macOS/Solaris/BSD)"
echo "============================================================"
echo ""
echo "# dtrace probes are organized as provider:module:function:name"
echo "# On macOS, some probes require SIP disabled or sudo."
echo ""

# --- dtrace: count syscalls by process ---
echo "## Count syscalls by process name (5 second snapshot):"
sudo dtrace -n 'syscall:::entry { @counts[execname] = count(); }' \
    -c "$BENCHMARK all" 2>&1 || echo "(dtrace not available on this platform)"

# --- dtrace: time how long each syscall takes ---
echo ""
echo "## Measure syscall latency distribution:"
sudo dtrace -n '
    syscall:::entry { self->ts = timestamp; }
    syscall:::return /self->ts/ {
        @time[execname] = quantize(timestamp - self->ts);
        self->ts = 0;
    }
' -c "$BENCHMARK all" 2>&1 || echo "(dtrace not available)"

# --- dtrace: trace open() calls ---
echo ""
echo "## Trace open() syscalls with filenames:"
sudo dtrace -n 'syscall::open:entry { printf("%s — %s", execname, copyinstr(arg0)); }' \
    -c "$BENCHMARK all" 2>&1 || echo "(dtrace not available)"

# --- dtrace: profile on-CPU time ---
echo ""
echo "## Profile on-CPU stacks at 997 Hz:"
sudo dtrace -n 'profile-997 { @stacks[ustack()] = count(); }' \
    -c "$BENCHMARK all" 2>&1 | head -30 || echo "(dtrace not available)"

echo ""
echo "============================================================"
echo "  PHASE 5: Instruments — macOS Profiling (CLI)"
echo "============================================================"
echo ""
echo "# Instruments is Apple's profiling GUI. It also has a CLI."
echo "# The 'xctrace' command (Xcode 12+) replaces older 'instruments' CLI."
echo ""

# --- Instruments: record a Time Profile trace ---
echo "## Record a Time Profiler trace:"
xcrun xctrace record --template "Time Profiler" --launch -- "$BENCHMARK all" 2>&1 \
    || echo "(xctrace not available — are you on macOS with Xcode?)"

# --- Instruments: record and export ---
echo ""
echo "## Export recorded trace data:"
echo "# xcrun xctrace export --input Recording.trace --xpath '/trace-toc/run/data/table[@schema=\"time-profile\"]'"
echo ""

echo "============================================================"
echo "  PHASE 6: eBPF / bpftrace — Kernel-Level Tracing (Linux 4.x+)"
echo "============================================================"
echo ""
echo "# bpftrace is the eBPF equivalent of dtrace one-liners."
echo "# It compiles one-liners to safe BPF bytecode and attaches"
echo "# them to kernel tracepoints, kprobes, or uprobes."
echo ""

# --- bpftrace: count syscalls by process ---
echo "## Count syscalls by process name:"
sudo bpftrace -e 'tracepoint:raw_syscalls:sys_enter { @[comm] = count(); }' \
    -c "$BENCHMARK all" 2>&1 || echo "(bpftrace not available on this platform)"

# --- bpftrace: trace openat() calls ---
echo ""
echo "## Trace openat() syscalls with filenames:"
sudo bpftrace -e 'tracepoint:syscalls:sys_enter_openat {
    printf("%s: %s\n", comm, str(args->filename));
}' -c "$BENCHMARK all" 2>&1 || echo "(bpftrace not available)"

# --- bpftrace: profile on-CPU stacks ---
echo ""
echo "## Profile on-CPU user stacks at 99 Hz:"
sudo bpftrace -e 'profile:hz:99 { @[ustack] = count(); }' \
    -c "$BENCHMARK all" 2>&1 | head -30 || echo "(bpftrace not available)"

# --- bpftrace: measure user-level function latency ---
echo ""
echo "## Measure latency of random_access function:"
sudo bpftrace -e "uretprobe:$BENCHMARK:random_access {
    printf(\"random_access took %d ns\n\", nsecs - @start[tid]);
}
uprobe:$BENCHMARK:random_access {
    @start[tid] = nsecs;
}" -c "$BENCHMARK random" 2>&1 || echo "(bpftrace not available)"

echo ""
echo "============================================================"
echo "  PHASE 7: BCC Tools — Pre-Built eBPF Programs"
echo "============================================================"
echo ""
echo "# BCC ships dozens of ready-to-use tools. Key ones for profiling:"
echo ""
echo "#   cachestat   — show cache hit/miss statistics every second"
echo "#   biolatency  — block I/O latency histogram"
echo "#   execsnoop   — trace every new process execution"
echo "#   opensnoop   — trace every open() syscall"
echo "#   offcputime  — show threads blocked off-CPU (waiting for I/O, locks)"
echo ""

# --- BCC: cachestat ---
echo "## Cache statistics while running benchmark:"
sudo cachestat 1 3 &>/dev/null &
CACHSTAT_PID=$!
sleep 0.5
"$BENCHMARK" all
kill "$CACHSTAT_PID" 2>/dev/null || true
wait "$CACHSTAT_PID" 2>/dev/null || true

echo ""
echo "============================================================"
echo "  PHASE 8: Comparing Approaches"
echo "============================================================"
echo ""
echo "# Summary of profiling approaches:"
echo ""
echo "#   ┌──────────────────────┬──────────────┬────────────┬──────────────┐"
echo "#   │ Tool                 │ Type         │ Overhead   │ Best For     │"
echo "#   ├──────────────────────┼──────────────┼────────────┼──────────────┤"
echo "#   │ perf stat            │ Counter      │ < 1%       │ HW counters  │"
echo "#   │ perf record          │ Sampling     │ 1–5%       │ Hotspots     │"
echo "#   │ dtrace               │ Instrument.  │ 5–20%      │ Syscalls     │"
echo "#   │ bpftrace             │ Instrument.  │ 2–10%      │ Kernel events│"
echo "#   │ Instruments TimeProf │ Sampling     │ 1–5%       │ macOS CPU    │"
echo "#   │ Instruments Allocs   │ Instrument.  │ 10–30%     │ Memory leaks │"
echo "#   └──────────────────────┴──────────────┴────────────┴──────────────┘"
echo ""
echo "Done."