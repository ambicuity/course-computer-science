# Flamegraph Reference Card

Quick reference for generating, reading, and troubleshooting flamegraphs.

## Generation Pipeline

### CPU Flamegraph
```bash
# Record sampling data
perf record -F 99 -g -- ./your_program
# -F 99: sample at 99 Hz (avoids lock-step with timer interrupts)
# -g: record call graphs (stack traces)

# Convert binary perf.data to folded stacks
perf script | stackcollapse-perf.pl > cpu.stacks

# Generate SVG
flamegraph.pl cpu.stacks > cpu.svg
```

### Off-CPU Flamegraph
```bash
# Method 1: perf with sched events
perf record -e sched:sched_stat_sleep -g -- ./your_program
perf script | stackcollapse-perf.pl | flamegraph.pl --color=io > offcpu.svg

# Method 2: bcc offcputime (better resolution)
offcputime -p <pid> > offcpu.stacks
stackcollapse-bpftrace.pl offcpu.stacks | flamegraph.pl --color=io > offcpu.svg
```

### Differential Flamegraph
```bash
# Before optimization
perf record -F 99 -g -- ./your_program
perf script | stackcollapse-perf.pl > before.stacks

# ... apply optimization ...

# After optimization
perf record -F 99 -g -- ./your_program
perf script | stackcollapse-perf.pl > after.stacks

# Generate differential
difffolded.pl before.stacks after.stacks | flamegraph.pl --negate > diff.svg
# Red = regression (more samples), Blue = improvement (fewer samples)
```

### Memory Flamegraph
```bash
perf record -e malloc -g -- ./your_program
perf script | stackcollapse-perf.pl | flamegraph.pl --color=mem > mem.svg
```

## Reading Flamegraphs

### Visual Guide
```
        ┌──────┐ ┌──────────┐
        │leaf_a│ │  leaf_b  │   ← y-axis = stack depth (top = leaf)
        └──────┘ └──────────┘
   ┌──────────────┐ ┌──────────────┐
   │  parent_a    │ │  parent_b    │   ← width = sample count (hotspot = wide)
   └──────────────┘ └──────────────┘
   ┌───────────────────────────────────┐
   │             main()                │
   └───────────────────────────────────┘
  ◄────── x-axis = population ────────►
  (frames sorted alphabetically, NOT time-ordered)
```

### Key Reading Rules

| Visual Feature | Means | Action |
|---|---|---|
| Wide bar | Hotspot — many samples | Optimize this function |
| Tall narrow spike | Deep call chain, few samples | Low priority — few samples |
| Color (random warm) | Nothing — visual differentiation only | Ignore color, focus on width |
| Wide bar at bottom | Popular root function | Look up to find which callee is hot |
| Wide bar at top | Popular leaf function | The function itself is the bottleneck |

### Flamegraph Types Cheat Sheet

| Type | Command | Shows | Color |
|---|---|---|---|
| CPU | `flamegraph.pl` | On-CPU time | Warm (default) |
| Off-CPU | `flamegraph.pl --color=io` | Blocked/waiting time | Blue |
| Memory | `flamegraph.pl --color=mem` | Allocation sites | Green |
| Differential | `flamegraph.pl --negate` | Before/after delta | Red=regression, Blue=improvement |
| Icicle | `flamegraph.pl --reverse` | Top-down view | Same as source |

## Troubleshooting Missing Frames

### Problem: Stacks are truncated (only 1-2 frames)

**Cause**: Compiler omitted frame pointers (`-fomit-frame-pointer` is default in GCC/Clang -O2+).

**Solutions**:
1. Recompile with `-fno-omit-frame-pointer` (simplest, slight performance cost)
2. Use DWARF unwinding: `perf record --call-graph dwarf -F 99 -- ./your_program`
3. Use LBR on Intel: `perf record --call-graph lbr -- ./your_program` (limited depth)
4. Use eBPF with DWARF: `bpf_get_stackid()` with BTF info

### Problem: JIT-compiled functions show as `jitted_1234`

**Cause**: JIT compilers (JVM, V8, SpiderMonkey) generate code at runtime; perf can't resolve symbols.

**Solutions**:
- **JVM**: Add `-XX:+PreserveFramePointer` and use `perf-map-agent`
- **Node.js**: Run with `--perf-prof` and `--interpreted-frames-native`
- **Python**: Use `perf` support in Python 3.12+ (`python -Xperf`)

### Problem: Kernel frames appear as `[unknown]`

**Cause**: Missing kernel symbols.

**Solution**: Install kernel debug symbols:
```bash
# Ubuntu/Debian
sudo apt install linux-image-$(uname -r)-dbgsym

# RHEL/CentOS
sudo debuginfo-install kernel-$(uname -r)

# Verify
perf script | grep '\\[k\\]' | head
```

### Problem: Flamegraph appears mostly blank

**Cause**: Sampling was too short or frequency too low.

**Solution**: Record longer or at higher frequency:
```bash
perf record -F 999 -g -- sleep 30  # 999 Hz for 30 seconds
# or attach to running process:
perf record -F 99 -g -p <pid> -- sleep 60
```

## Where to Get the Tools

- **Brendan Gregg's FlameGraph**: https://github.com/brendangregg/FlameGraph
  ```bash
  git clone https://github.com/brendangregg/FlameGraph.git
  export PATH="$PATH:$(pwd)/FlameGraph"
  ```
- **perf**: Pre-installed on most Linux distros (`linux-tools-common`, `linux-tools-$(uname -r)`)
- **bcc tools**: https://github.com/iovisor/bcc (for `offcputime`, `profile.py`, etc.)

## Common Commands Quick Reference

```bash
# Record CPU profile (attach to PID)
perf record -F 99 -g -p <pid> -- sleep 60

# Record specific events
perf record -e cache-misses -g -- ./program

# List available events
perf list

# View raw samples
perf script

# Generate folded stacks directly
perf script | stackcollapse-perf.pl > out.stacks

# Merge multiple perf.data files
perf inject -s -i perf.data -o merged.data

# View perf report (text-based, not flamegraph)
perf report

# Quick flamegraph in one line
perf record -F 99 -g -- ./program && perf script | stackcollapse-perf.pl | flamegraph.pl > out.svg
```