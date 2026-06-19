# Outputs — How to Think About Performance

## Artifact: `performance_checklist.md`

This lesson's reusable artifact is a one-page performance investigation checklist. Print it, pin it to your monitor, and follow it every time you need to optimize something.

### What's in it

- The 8-step profiling-driven optimization workflow (measure → profile → identify → optimize → verify)
- Quick-reference decision trees: compute-bound vs memory-bound, latency vs throughput
- Amdahl's Law and Gustafson's Law formulas for estimating maximum speedup
- Memory hierarchy cost table (L1 through network)
- Profiling tool cheat sheet (`perf`, `flamegraph`, `cachegrind`, `strace`, `iotop`)
- Anti-pattern checklist (7 common mistakes that waste optimization time)

### How to use it

```
cat outputs/performance_checklist.md     # read it in terminal
grep "roofline" outputs/performance_checklist.md  # find the decision tree
```

Or print it and keep it next to your monitor during performance work.

### Relationship to other lessons

| Lesson in Phase 15 | Relevant checklist section |
|---|---|
| 02 — Profiling & Measuring | Profiling tool cheat sheet, 8-step workflow |
| 03 — Cache Optimization | Memory hierarchy cost table, compute vs memory-bound decision tree |
| 04 — Branch Prediction | CPI decomposition, profiling tools |
| 05 — I/O & Storage Performance | Latency vs throughput, batching amortization |
| 06 — Network Performance | Memory hierarchy cost table (network entries), latency percentiles |
| Capstone — Profile-Guided Optimization | Full workflow |