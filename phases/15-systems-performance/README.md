# Phase 15 — Systems Programming & Performance

> Measure honestly. Tune cache, branches, IO. Win 10x by knowing the machine.

**Lessons:** 20 &nbsp;·&nbsp; **Estimated time:** ~24 h 15 min
**Phase capstone artifact:** A profile-guided optimization walk-through.

## Lessons

| # | Lesson | Status | Time |
|---|--------|--------|------|
| 01 | [How to Think About Performance](./01-how-to-think-about-performance/) | ✅ | ~45 min |
| 02 | [Measurement Discipline — Benchmarks That Don't Lie](./02-measurement-discipline-benchmarks-that-don-t-lie/) | ✅ | ~75 min |
| 03 | [Profiling — perf, dtrace, Instruments, eBPF](./03-profiling-perf-dtrace-instruments-ebpf/) | ✅ | ~75 min |
| 04 | [Flamegraphs, Hotspots, and Reading Stacks](./04-flamegraphs-hotspots-and-reading-stacks/) | ✅ | ~60 min |
| 05 | [Cache-Aware Algorithm Design](./05-cache-aware-algorithm-design/) | ✅ | ~75 min |
| 06 | [False Sharing and NUMA](./06-false-sharing-and-numa/) | ✅ | ~75 min |
| 07 | [Branch Prediction and Layout Tricks](./07-branch-prediction-and-layout-tricks/) | ✅ | ~60 min |
| 08 | [Vectorization in Practice (auto and intrinsics)](./08-vectorization-in-practice-auto-and-intrinsics/) | ✅ | ~75 min |
| 09 | [Memory Allocators in Production — jemalloc, mimalloc](./09-memory-allocators-in-production-jemalloc-mimalloc/) | ✅ | ~75 min |
| 10 | [Zero-Copy and mmap](./10-zero-copy-and-mmap/) | ✅ | ~60 min |
| 11 | [Asynchronous I/O — io_uring Deep Dive](./11-asynchronous-i-o-io-uring-deep-dive/) | ✅ | ~90 min |
| 12 | [Kernel Bypass — DPDK, SPDK, AF_XDP](./12-kernel-bypass-dpdk-spdk-af-xdp/) | ✅ | ~75 min |
| 13 | [Lock Contention Patterns and Cures](./13-lock-contention-patterns-and-cures/) | ✅ | ~75 min |
| 14 | [Coroutines and Stackful vs Stackless Concurrency](./14-coroutines-and-stackful-vs-stackless-concurrency/) | ✅ | ~75 min |
| 15 | [C++ Low-Latency Idioms](./15-c-low-latency-idioms/) | ✅ | ~75 min |
| 16 | [Rust for High Performance — UnsafeCell, MaybeUninit, alignment](./16-rust-for-high-performance-unsafecell-maybeuninit-alignment/) | ✅ | ~75 min |
| 17 | [Power, Frequency Scaling, Thermal Throttling](./17-power-frequency-scaling-thermal-throttling/) | ✅ | ~45 min |
| 18 | [Reliability Engineering — Tail Latency, Hedging](./18-reliability-engineering-tail-latency-hedging/) | ✅ | ~60 min |
| 19 | [Capacity Planning and Little's Law](./19-capacity-planning-and-little-s-law/) | ✅ | ~60 min |
| 20 | [Phase Capstone — A Profile-Guided Optimization Walk-Through](./20-phase-capstone-a-profile-guided-optimization-walk-through/) | ✅ | ~150 min |

**Legend:** ✅ Complete &nbsp;·&nbsp; 🚧 In Progress &nbsp;·&nbsp; ⬚ Planned

## How this phase fits

See [`../../ROADMAP.md`](../../ROADMAP.md) for the full curriculum and prerequisites.
See [`../../LESSON_TEMPLATE.md`](../../LESSON_TEMPLATE.md) for the shape every lesson follows.
