# Outputs — Concurrency vs Parallelism Reference

## Artifact: `../code/notes.md`

This lesson's reusable artifact is the comprehensive reference notes in `code/notes.md`. It is not a binary or script — it is a structured Markdown document you can keep open alongside the other Phase 13 lessons.

### What's in it

- **Core definitions** — Rob Pike's distinction, the six-dimension comparison table
- **Concurrency models table** — Processes, threads, async/await, coroutines, actors, CSP compared across 5 attributes each
- **Amdahl's Law** — Formula, worked examples, speedup table for P=0.5 to 0.99
- **Gustafson's Law** — Formula, scaled speedup table, when to use which law
- **Three levels of concurrency** — Task, data, instruction with granularity, control, and examples
- **Real-world classification** — 12 systems classified as concurrent/parallel/both with rationale
- **When NOT to parallelize** — 6 failure modes with concrete numbers
- **Memory models** — SC, TSO, relaxed with reordering examples
- **Performance numbers** — ~20 latency numbers from L1 to SSD to network

### How to use it

```
less code/notes.md                           # quick reference in terminal
grep "Amdahl" code/notes.md                  # find specific formulas
grep -A5 "When NOT to use" code/notes.md     # anti-patterns
```

Or import relevant sections into your own notes.

### Relationship to other lessons

| Lesson in Phase 13 | Relevant reference section |
|---|---|
| 02 — Race Conditions | Memory models, Concurrency models |
| 04 — Locks | When NOT to use, Performance numbers |
| 07 — Atomics | CAS, ABA Problem, Memory models |
| 14 — CSP & Go channels | CSP row in models table |
| 19 — GPU / CUDA | Data-level parallelism (SIMD) |
