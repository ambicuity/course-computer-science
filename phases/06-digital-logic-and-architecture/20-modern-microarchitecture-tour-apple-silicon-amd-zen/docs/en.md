# Modern Microarchitecture Tour (Apple Silicon, AMD Zen)

> The fastest general-purpose processors you can buy today are not faster because of a single trick. They are fast because of hundreds of microarchitectural decisions — and understanding them tells you where your code's cycles actually go.

**Type:** Reference
**Languages:** None (reading-only survey)
**Prerequisites:** Phase 06 lessons 01–19
**Time:** ~60 minutes

## Learning Objectives

- Describe the pipeline stages of a modern out-of-order core (fetch, decode, rename, dispatch, execute, retire).
- Compare Apple M-series, AMD Zen 4/5, and Intel Alder Lake/Raptor Lake microarchitectures at the block-diagram level.
- Explain hybrid big.LITTLE / P-core + E-core designs and why they exist.
- Identify the cache hierarchy and memory subsystem characteristics of each architecture.
- Use this knowledge to reason about why code performs differently on different CPUs.

## The Problem

Lessons 11–13 covered pipelining, hazards, branch prediction, and out-of-order execution in the abstract — a generic 5-stage RISC pipeline. Real processors are far more complex: Apple's M4 has a 10-wide decode, a 630+ entry reorder buffer, and a custom AMX coprocessor. AMD's Zen 5 has separate integer and floating-point schedulers, 3D V-Cache stacking, and a micro-op cache that bypasses the decoder entirely. Intel ships P-cores and E-cores on the same die, with the OS scheduler deciding which threads get the fast cores.

If you do not understand these real architectures, you cannot explain why the same C code runs at different speeds on different machines, why branchy code penalizes some CPUs more than others, or why a matrix multiply kernel benefits from AMX but not from AVX-512 on the same chip.

## The Concept

### The Generic Out-of-Order Pipeline

Every modern high-performance core follows this general flow, though the details vary enormously:

```
Branch Predictor → Fetch → Decode → Rename → Dispatch → Execute → Retire
       ↑             ↓        ↓        ↓         ↓          ↓
       └─────── Branch mispredict recovery ←───────────────┘
```

1. **Branch predictor** guesses the next PC. Modern predictors (TAGE, perceptron-based) achieve 95–99% accuracy.
2. **Fetch** reads 16–32 bytes per cycle from the L1I cache.
3. **Decode** converts x86 (variable-length) or ARM (fixed-length) instructions into internal **micro-ops** (μops).
4. **Rename** maps architectural registers to a larger physical register file, eliminating false data dependencies (WAR, WAW).
5. **Dispatch** sends ready μops to execution units. μops wait in reservation stations (or a unified scheduler) until their operands are available.
6. **Execute** runs the μop on the appropriate unit (ALU, FPU, AGU, branch unit).
7. **Retire** commits results in program order from the reorder buffer (ROB).

The **reorder buffer** is the key structure: it holds all in-flight μops and ensures they retire in order even though they executed out of order. ROB size directly limits the processor's ability to find instruction-level parallelism (ILP).

### Apple M-Series (M1 → M4)

Apple's custom ARM cores are the widest consumer CPUs ever shipped.

**P-core (Avalanche / Everest / the M4 P-core):**
- **Decode**: 8-wide (M1–M3), reportedly 10-wide on M4. This is the number of ARM instructions decoded per cycle.
- **ROB**: ~630 entries (M1), growing each generation. For comparison, Intel Golden Cove has 512.
- **Execution units**: 6 integer ALUs, 4 FP/SIMD units, 3 load + 2 store AGUs.
- **Micro-op cache**: 1920 entries (M1). Previously decoded μops bypass the decoder entirely, saving power and decode bandwidth.
- **Branch predictor**: Perceptron-based with very large history tables. Apple's branch mispredict penalty is among the lowest in the industry (~11 cycles on M1).
- **AMX coprocessor**: A separate execution engine attached to each P-core (and E-core cluster). It accelerates matrix operations (GEMM, convolution) with its own register file and instruction set. AMX is not exposed via a public ISA — Apple uses it internally in the Accelerate framework and Core ML.

**E-core (Icestorm / Sawtooth):**
- 4-wide decode, ~130 entry ROB.
- Much smaller than P-cores but extremely power-efficient.
- Designed for background tasks, keeping the P-cores idle when not needed.

**Unified Memory Architecture (UMA):**
- CPU, GPU, and AMX share the same physical DRAM.
- No PCIe transfer needed between CPU and GPU — a massive win for workloads that alternate between serial and parallel phases.
- Memory bandwidth: 100 GB/s (M1) to 400+ GB/s (M4 Max).

| Feature | M1 P-core | M2 P-core | M3 P-core | M4 P-core |
|---------|-----------|-----------|-----------|-----------|
| Decode width | 8 | 8 | 8 | 10 (est.) |
| ROB entries | ~630 | ~700+ | ~700+ | ~700+ |
| L1D | 128 KB | 128 KB | 128 KB | 128 KB |
| L1I | 192 KB | 192 KB | 192 KB | 192 KB |
| L2 (shared) | 12 MB | 16 MB | 16 MB | 16 MB |

### AMD Zen 4 and Zen 5

AMD's chiplet architecture separates **core complex dies (CCD)** from the I/O die.

**CCD structure:**
- Each CCD contains one **CCX** (Core Complex) with 8 cores.
- Each core has private L1I (32 KB) and L1D (32 KB) caches.
- L2 cache: 1 MB per core (Zen 4/5, up from 512 KB in Zen 3).
- L3 cache: 32 MB per CCD, shared among all 8 cores.

**Zen 4 core pipeline:**
- **Frontend**: TAGE branch predictor → fetch 32 bytes/cycle from L1I → decode 4 x86 instructions/cycle → μop cache (6.75K entries) can bypass decoder.
- **Backend**: 6 integer ALUs + 4 FP units. Separate integer and FP schedulers. 320 entry ROB.
- **Load/store**: 3 loads + 2 stores per cycle. 72 entry load queue, 44 entry store queue.
- **Execution width**: Can retire 6 μops/cycle (integer) and 4 μops/cycle (FP).

**Zen 5 (2024):**
- Wider frontend: 2 × 32-byte fetch paths (dual fetch).
- Wider decode: 4→8 instructions/cycle (effectively doubled with dual decode).
- Larger ROB: 448 entries.
- 512-bit data paths for AVX-512 (full width, not double-pumped like Zen 4).

**3D V-Cache:**
- AMD stacks an additional 64 MB of SRAM on top of the CCD using hybrid bonding.
- Total L3 becomes 96 MB per CCD. This dramatically benefits workloads with large working sets (games, databases, scientific simulation).
- The stacked cache runs at the same speed as the base L3 but at lower voltage.

```
┌──────────────────────────────────────┐
│              I/O Die                 │
│   Memory Controller, PCIe, Infinity  │
│              Fabric                  │
└──────────┬──────────┬────────────────┘
           │          │
    ┌──────┴───┐ ┌────┴────┐
    │  CCD 0   │ │  CCD 1  │
    │ 8 cores  │ │ 8 cores │
    │ 32 MB L3 │ │ 32 MB L3│
    │(96 w/V-  │ │         │
    │ Cache)   │ │         │
    └──────────┘ └─────────┘
```

### Intel Alder Lake / Raptor Lake (Hybrid Architecture)

Intel adopted a hybrid design starting with 12th-gen Alder Lake (2021), combining two types of cores on the same die.

**P-cores (Golden Cove / Raptor Cove):**
- 6-wide decode, 512 entry ROB.
- Hyper-threading (SMT): each P-core runs 2 threads.
- L1D: 48 KB, L1I: 32 KB, L2: 1.25–2 MB per core.
- Very high single-thread performance.

**E-cores (Gracemont):**
- 4-wide decode, ~256 entry ROB.
- No SMT — one thread per core.
- L1D: 32 KB, L1I: 64 KB, L2: 2 MB shared per 4-core cluster.
- Optimized for throughput per watt. Four E-cores fit in the area of one P-core.

**Thread Director:**
- A hardware scheduler that monitors each thread's behavior (memory-bound vs. compute-bound, branch-heavy vs. straight-line).
- Classifies threads as foreground (P-core) or background (E-core).
- The OS scheduler (Windows 11, Linux 5.18+) uses this information to place threads on the right core type.

**Why hybrid?**
- Not all threads need peak single-thread speed. Background compilation, indexing, and streaming benefit from many efficient cores.
- Peak single-thread speed still matters for the "hot" path. A P-core delivers that.
- The combination gives better overall throughput within a fixed power budget.

| Feature | Alder Lake P-core | Alder Lake E-core | Raptor Lake P-core | Raptor Lake E-core |
|---------|------------------|------------------|-------------------|-------------------|
| Architecture | Golden Cove | Gracemont | Raptor Cove | Gracemont |
| Decode width | 6 | 4 | 6 | 4 |
| ROB entries | 512 | ~256 | 512 | ~256 |
| SMT | Yes (2 threads) | No | Yes (2 threads) | No |
| L1D | 48 KB | 32 KB | 48 KB | 32 KB |
| L2 per core | 1.25 MB | 2 MB / 4 cores | 2 MB | 2 MB / 4 cores |

### Key Microarchitectural Concepts (Cross-Architecture)

#### Superscalar Execution

All three architectures are **superscalar**: they issue multiple instructions per cycle to multiple execution units. The decode width and the number of execution ports determine the peak IPC (instructions per cycle).

#### Out-of-Order Depth

The ROB size is the primary measure of out-of-order depth. A larger ROB lets the processor look further ahead in the instruction stream to find independent work while waiting for a cache miss or branch resolution.

| Processor | ROB entries | OoO window |
|-----------|-------------|------------|
| Apple M1 P-core | ~630 | Very deep |
| AMD Zen 4 | 320 | Moderate |
| AMD Zen 5 | 448 | Deep |
| Intel Golden Cove | 512 | Deep |
| Intel Gracemont | ~256 | Moderate |

#### Cache Hierarchy

Every design uses the same general hierarchy, but sizes and latencies differ:

```
Core
├── L1I (instruction) — 32–192 KB, ~3–4 cycles
├── L1D (data) — 32–128 KB, ~4–5 cycles
├── L2 (per-core or per-cluster) — 256 KB–2 MB, ~12–14 cycles
└── L3 (shared, on-die or stacked) — 16–96 MB, ~40–50 cycles
    └── DRAM (off-die) — 8–192 GB, ~60–80 ns
```

Apple's L1 caches are unusually large (128 KB L1D, 192 KB L1I) compared to AMD and Intel. This reduces L1 miss rates and compensates for the lack of an L3 cache on some M-series SKUs.

#### Memory Subsystem

- **Apple UMA**: CPU, GPU, and media engines share one pool. Bandwidth up to 400+ GB/s on M4 Max. No copy overhead between CPU and GPU.
- **AMD**: DDR5 memory controllers on the I/O die. Dual-channel consumer (up to ~80 GB/s), quad-channel Threadripper/EPYC.
- **Intel**: DDR5 or DDR4 (platform-dependent). On-chip memory controller. Thunderbolt for external I/O bandwidth.

#### Power Management

All three architectures use aggressive power management:

- **Clock gating**: Disables the clock to idle execution units, saving dynamic power.
- **Power gating**: Completely shuts off power to unused cores (C-states). Zero leakage power when gated.
- **DVFS** (Dynamic Voltage and Frequency Scaling): Adjusts V and f together. Power scales as ~V²f, so dropping voltage by 10% saves ~19% power.
- **Turbo Boost / Precision Boost / Apple Turbo**: When thermal and power budgets allow, individual cores boost above base frequency for short bursts.

## Use It

Understanding these architectures helps you write faster code:

1. **Branch prediction matters more on narrow machines.** AMD Zen 4 has a 4-wide decode; a mispredicted branch wastes 4 cycles of fetch/decode bandwidth per cycle of pipeline depth. Apple's wider decode amortizes this cost across more instructions.

2. **Cache-sensitive code benefits enormously from 3D V-Cache.** Games and databases see 10–15% FPS/throughput gains from the extra 64 MB L3 because their working sets no longer fit in 32 MB.

3. **AMX accelerates matrix operations transparently.** If you call `cblas_sgemm` from Apple's Accelerate framework, it dispatches to AMX without you writing a single GPU kernel.

4. **Hybrid core awareness matters for latency-critical threads.** On Intel, pin your critical thread to a P-core with `sched_setaffinity` or let Thread Director classify it correctly by avoiding excessive I/O waits.

5. **Unified memory eliminates CPU-GPU transfer overhead.** On Apple Silicon, a neural network can run its serial control logic on the CPU and its parallel compute on the GPU without copying data.

## Read the Source

- **Apple**: [Apple Silicon at WWDC sessions](https://developer.apple.com/videos/play/wwdc2020/10007/) — "Bring your Metal app to Apple Silicon" covers the UMA and GPU architecture.
- **AMD**: [AMD Zen 4 Core — wikichip.org](https://fuse.wikichip.org/news/7363/amd-zen-4-core-a-look-into-the-next-gen-architecture/) — detailed block diagrams of the Zen 4 frontend and backend.
- **Intel**: [Intel Architecture Day 2021](https://www.intel.com/content/www/us/en/newsroom/resources/architecture-day-2021.html) — Alder Lake hybrid architecture walkthrough with Thread Director details.
- **Agner Fog's microarchitecture guide**: [agner.org/optimize](https://www.agner.org/optimize/microarchitecture.pdf) — the most detailed public resource on x86 microarchitectures from Pentium to Zen 5.

## Ship It

This is a reading-only lesson. The reusable artifact is the architectural mental model you build — the ability to look at a CPU benchmark result and reason about *which* microarchitectural feature caused it. No code artifact is produced.

## Exercises

1. **Easy** — Draw a block diagram of an AMD Zen 4 core showing the frontend (branch predictor, fetch, decode, μop cache) and backend (integer scheduler, FP scheduler, ROB, load/store units). Label the widths and sizes.
2. **Medium** — A matrix multiply kernel runs 2× faster on Apple M2 than on Intel i7-12700K despite similar single-thread benchmarks. List three microarchitectural reasons this could happen (hint: consider cache sizes, AMX, and UMA).
3. **Hard** — You have a multi-threaded server workload with 80% memory-bound threads and 20% compute-bound threads. Explain how you would assign threads to P-cores vs. E-cores on an Intel i9-13900K (8P + 16E). Would your strategy differ on AMD Zen 5 (16 homogeneous cores)? Justify with microarchitectural reasoning.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Superscalar | "Multiple instructions per cycle" | The processor has parallel execution units and issues independent μops to different units in the same cycle |
| Reorder buffer (ROB) | "How far ahead the CPU looks" | Circular buffer holding all in-flight μops; ensures in-order retirement despite out-of-order execution |
| μop cache | "Decoded instruction cache" | Caches previously decoded micro-ops so subsequent executions skip the decode stage entirely |
| Hybrid architecture | "big.LITTLE for x86" | Combining high-performance cores (P-cores) with efficient cores (E-cores) on the same die to balance peak speed and power efficiency |
| Thread Director | "Hardware scheduler" | Microarchitectural unit that classifies thread behavior and hints the OS scheduler on core placement |
| Unified Memory (UMA) | "Shared CPU/GPU memory" | CPU, GPU, and accelerators access the same physical DRAM without copies; Apple's key architectural advantage |
| 3D V-Cache | "Stacked cache" | AMD's technology of bonding an extra SRAM die on top of the CCD to triple L3 capacity (32→96 MB) |
| AMX | "Apple matrix coprocessor" | Dedicated matrix execution engine on Apple Silicon, used internally by Accelerate/Core ML for GEMM and convolution |
| DVFS | "Dynamic frequency scaling" | Adjusting voltage and frequency together at runtime; power ∝ V²f, so lowering V saves disproportionately more power |

## Further Reading

- *Computer Architecture: A Quantitative Approach* by Hennessy & Patterson, 6th ed. — Chapters 1–3 cover superscalar design, branch prediction, and cache hierarchies in depth.
- Agner Fog, "Microarchitecture of Intel, AMD, and VIA CPUs" — [agner.org/optimize](https://www.agner.org/optimize/) — cycle-by-cycle analysis of every major x86 microarchitecture.
- Anandtech's Apple M1 deep dive — detailed performance analysis with real benchmarks showing the M1's wide decode and large ROB in action.
