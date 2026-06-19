# Power, Heat, Reliability — Why Cores Stopped Scaling

> The physics wall that ended the "just make it faster" era of CPU design.

**Type:** Learn (Survey)
**Languages:** Markdown — no code files
**Prerequisites:** Phase 06 lessons 01–20
**Time:** ~45 minutes

## Learning Objectives

- Understand Dennard scaling, why it held for 30 years, and why it broke around 2005.
- Read the power equation P = α·C·V²·f and explain what each variable controls.
- Describe thermal throttling, dark silicon, and the reliability threats that limit modern chips.
- Connect these physical limits to the architectural shift toward multi-core, heterogeneous, and accelerator-based design.

## The Problem

From the 1970s through the early 2000s, CPU clock speeds rose roughly in step with transistor density.
A Pentium 4 at 3.8 GHz in 2004 felt like the natural continuation of a 16 MHz 80286 from 1982.
Then it stopped. Clock speeds plateaued. Intel canceled the 4 GHz Pentium 4 and pivoted to multi-core.

What happened? Physics happened. Three interlocking problems — power density, thermal limits, and
reliability decay — created a wall that no amount of clever engineering could tunnel through by
simply cranking frequency. Understanding this wall is essential context for every architecture
decision that followed.

## Dennard Scaling (1974 – ~2005)

In 1974, Robert Dennard and colleagues at IBM observed a remarkably convenient property of MOSFET
transistors:

**As you shrink a transistor by a factor *k*, its voltage, current, and capacitance all shrink
proportionally — so power density stays constant.**

```
 Shrink factor k = 0.7 (roughly one process node)
 ┌──────────────────────────────────────────────────┐
 │  Length, Width  →  ÷ k                           │
 │  Voltage (V)    →  ÷ k                           │
 │  Current (I)    →  ÷ k                           │
 │  Gate Cap (C)   →  ÷ k                           │
 │                                                   │
 │  Power per transistor = V × I   →  ÷ k²          │
 │  Transistors per area   →  × k²                  │
 │                                                   │
 │  Power density = constant  ✓                     │
 └──────────────────────────────────────────────────┘
```

This meant you could pack more transistors into the same area, run them at the same or higher
frequency, and the chip wouldn't melt. Every process shrink was a free lunch.

### What this bought you

| Decade | Process node | Example CPU | Clock speed |
|--------|-------------|-------------|-------------|
| 1980s  | 3 µm → 800 nm | 80286 → 80486 | 16 → 100 MHz |
| 1990s  | 800 nm → 180 nm | Pentium → Pentium III | 66 → 600 MHz |
| 2000–05 | 180 nm → 90 nm | Pentium 4 (Prescott) | 1.5 → 3.8 GHz |

The doubling cadence roughly matched Moore's Law, but the speed improvement came from Dennard
scaling, not just from having more transistors.

## The End of Dennard Scaling

Around the 90 nm node (2004–2006), a second-order effect broke the deal: **leakage current**.

```
 Dennard world (ideal)              Reality past 90 nm
 ┌─────────────────────┐           ┌─────────────────────┐
 │ Gate │                │           │ Gate │▓▓▓▓▓▓▓▓▓▓▓▓▓▓│
 │      │ Channel OFF    │           │      │ Leakage!     │
 │──────┴──────────────│           │──────┴──────────────│
 │  No current flows    │           │  Electrons tunnel   │
 │  when transistor off │           │  through thin oxide │
 └─────────────────────┘           └─────────────────────┘
```

- As gate oxide thickness shrank below ~1.2 nm, quantum tunneling allowed electrons to leak
  through even when the transistor was "off."
- To maintain noise margins, voltage could no longer shrink proportionally (V stopped at ~0.7–1.0 V
  instead of dropping further).
- Since V didn't drop, power density started climbing per generation instead of staying flat.

**The result:**

```
 Power density over time (schematic)
 ▲
 │                                          ┌─── Thermal limit
 │                                         ╱    (~100 W/cm², nuclear reactor core!)
 │                                       ╱
 │                                     ╱  ← power density
 │                                   ╱     rising
 │                               ╱╱
 │                          ╱╱╱
 │                    ╱╱╱╱
 │  ─ ─ ─ ─ ─ ╱╱╱╱ ─ ─ ─ ─ ─ ─ ─ Dennard scaling predicts constant
 │       ╱╱╱╱
 │  ╱╱╱╱
 └──────────────────────────────────────────────────────▶  Year
   1970       1985       2000  2005    2010     2020
```

Chips hit the thermal wall. You literally could not push more power through a few square
centimeters of silicon without the junction temperature exceeding reliability limits.

## The Power Equation

The dynamic power consumed by a CMOS circuit is:

```
 P_dynamic = α · C · V² · f
```

| Symbol | Name | Meaning |
|--------|------|---------|
| α | Switching activity | Fraction of gates that switch each clock cycle (0 to 1). A random logic block might have α ≈ 0.1–0.3; a clock tree has α = 1. |
| C | Capacitance | Total load capacitance being charged/discharged. Scales with wire length, gate area, and fanout. |
| V | Supply voltage | The voltage rail powering the logic (e.g., 1.0 V for modern cores). **Squared — the biggest lever.** |
| f | Clock frequency | How many times per second the gates switch. |

**Total power = dynamic + static:**

```
 P_total = α · C · V² · f  +  P_leakage
                    ↑              ↑
           switching power    wasted power when
                              transistor is "off"
```

### Why V² matters so much

Doubling the frequency from 3 GHz to 6 GHz requires roughly doubling V to maintain timing.
But power scales with V², so:

- 2× V → 4× dynamic power.
- And leakage current also rises exponentially with V.
- A 3.8 GHz Pentium 4 already consumed ~115 W in a desktop. Going to 7.6 GHz would have
  needed >400 W — in a space the size of a postage stamp.

This is why the industry abandoned "just increase f" and looked for parallelism instead.

## Thermal Throttling and DVFS

Modern CPUs don't just crash when they get hot. They **actively manage** their own temperature.

### How thermal throttling works

```
 ┌─────────────────────────────────────────────────┐
 │                  CPU Core                        │
 │                                                  │
 │   ┌──────────┐    ┌──────────────┐              │
 │   │ Thermal  │───▶│ Frequency    │──▶ Clock     │
 │   │ Sensor   │    │ Controller   │              │
 │   └──────────┘    └──────────────┘              │
 │        │                   ▲                     │
 │        │            Current temp                 │
 │        ▼            vs. target (TDP)             │
 │   Junction temp                                  │
 │   (on-die diode)                                 │
 └─────────────────────────────────────────────────┘
```

1. On-die thermal sensors measure junction temperature.
2. If temperature exceeds the TDP (Thermal Design Power) target, the clock is reduced.
3. If temperature keeps rising, cores are powered down entirely.

### DVFS — Dynamic Voltage and Frequency Scaling

Rather than binary throttling, modern chips use continuous adjustment:

| Scenario | Action | Effect on P = αCV²f |
|----------|--------|---------------------|
| Idle or light load | Drop f (and V) | Cubic power reduction (f drops, V² drops) |
| Burst workload | Turbo up f (and V) temporarily | Short spike, thermal headroom consumed |
| Sustained heavy load | Stabilize at base clock | Thermal equilibrium at TDP |
| Overheating emergency | Throttle below base | Sacrifice performance to survive |

This is why your laptop's fan spins up during a compile, and why a gaming laptop plugged in
for hours runs at a lower clock than its "boost" number.

## Dark Silicon

If you can't power all transistors simultaneously, the unpowered region is called **dark silicon**.

```
 Modern SoC die (schematic)
 ┌──────────────────────────────────────────────┐
 │ ██████  ░░░░░░  ██████  ░░░░░░  ██████      │
 │ Core 0  Dark    Core 1  Dark    Core 2       │
 │         region          region               │
 │                                              │
 │ ░░░░░░  ████████  ░░░░░░  ████████          │
 │ Dark    GPU block Dark    NPU block          │
 │                                              │
 │ ░░░░░░  ░░░░░░  ░░░░░░  ░░░░░░  ░░░░░░     │
 │         Dark silicon everywhere              │
 └──────────────────────────────────────────────┘

 ██ = Powered on, actively computing
 ░░ = Dark — transistors exist but are unpowered
```

At 7 nm, roughly **50–80%** of transistors on a chip may be dark at any given moment. The chip
has the transistor budget for incredible parallelism, but the power budget only lets you use a
fraction at a time.

This drives two design strategies:

1. **Heterogeneous blocks** — put a CPU, GPU, NPU, ISP, video encoder on the same die. Only power
   on what the workload needs.
2. **Race to idle** — run fast, finish quickly, then go dark. Spreading work over more time
   (lower clock) doesn't always save energy because leakage accumulates while the chip is on.

## Reliability Threats

Even if power and heat are managed, long-term reliability becomes a concern at advanced nodes.

### Electromigration

```
 Current flowing through metal wire
 ───────▶───────▶──────▶────────▶──────
      metal atoms migrate with electron flow
      
 At high current density + high temperature:
 ───────   ─────▶  ─────  ▶──── ───────
   voids form  ◄────────  hillocks form
   (opens)                  (shorts)
```

- High current density pushes metal atoms along with electron flow.
- Over time, voids (opens) and hillocks (shorts) form in interconnects.
- Mitigation: wider wires, copper interconnects, current density limits in design rules.

### Soft Errors (SEUs)

- A cosmic ray (or alpha particle from packaging) strikes a storage node.
- The charge deposited flips a bit: 0 → 1 or 1 → 0.
- Rate is measured in FIT (Failures in Time = failures per 10⁹ device-hours).
- A server chip at 7 nm might see ~1000 FIT — one soft error roughly every 50 days.
- Mitigation: ECC on SRAM/DRAM, redundant logic, parity checks.

### Aging Mechanisms

| Mechanism | Full name | What degrades |
|-----------|-----------|---------------|
| NBTI | Negative Bias Temperature Instability | PMOS threshold voltage shifts upward over time → slower switching |
| HCI | Hot Carrier Injection | High-energy electrons damage gate oxide → threshold drift |
| TDDB | Time-Dependent Dielectric Breakdown | Gate oxide eventually breaks down under electric field stress |

These effects worsen at smaller nodes and higher temperatures. Chips must be designed with
margins that accommodate years of aging — meaning they're slightly slower on day one so they
still work on day 3000.

## Implications: The Architectural Shift

The combined effect of all these limits reshaped processor design starting around 2005:

```
 The old way (frequency scaling)         The new way (parallelism + specialization)
 ┌──────────────────────┐                ┌──────────────────────────────────────┐
 │                      │                │ ┌──────┐ ┌──────┐ ┌──────┐         │
 │                      │                │ │Core 0│ │Core 1│ │Core 2│  ...    │
 │   ONE FAST CORE      │                │ └──────┘ └──────┘ └──────┘         │
 │   3.8 GHz            │                │ ┌──────┐ ┌──────┐ ┌────────────┐   │
 │                      │                │ │ LITTLE│ │ LITTLE│ │ GPU / NPU  │   │
 │                      │                │ │Core 0│ │Core 1│ │ Accelerator│   │
 └──────────────────────┘                │ └──────┘ └──────┘ └────────────┘   │
                                         └──────────────────────────────────────┘
  "Faster" (higher f)                    "Wider" (more parallel units)
```

### Multi-core

Instead of doubling clock speed every 2 years, designers doubled core count. Two 2 GHz cores
can do the work of one 4 GHz core — if the software is parallelizable. Amdahl's Law limits
the payoff for inherently sequential code.

### Heterogeneous Computing (big.LITTLE)

ARM's big.LITTLE (and Intel's hybrid architecture) pairs high-performance cores with
energy-efficient cores on the same die. Background tasks run on LITTLE cores (saving power);
demanding bursts run on big cores (maximizing performance within thermal limits).

### Accelerators

| Accelerator | Good at | Why it helps |
|-------------|---------|--------------|
| GPU | Massively parallel math (graphics, ML) | Thousands of simple cores >> few complex cores for throughput |
| TPU / NPU | Matrix multiply, inference | Fixed-function units are far more power-efficient than general-purpose ALUs |
| DSP | Signal processing, audio, modem | Domain-specific data paths beat general CPUs on perf/watt |
| VPU | Video encode/decode | Dedicated silicon for one task is always more efficient |

Each accelerator does one job extremely well at a fraction of the power a general-purpose CPU
would need for the same task. This is why a phone can do real-time face recognition at 5 W — it
uses dedicated silicon, not brute-force clock speed.

## Use It

Understanding these limits explains real-world phenomena:

- **Why did CPU frequency stop at ~4–5 GHz?** Dennard scaling ended; the power wall.
- **Why do phones have 8+ cores but the same battery life?** Most cores are LITTLE or dark.
- **Why does my laptop throttle during gaming?** TDP limit; thermal throttling kicks in.
- **Why does Apple's M-series get great perf/watt?** Heterogeneous design, tight thermal control,
  and accelerators (Neural Engine, media engine) offload work from general-purpose cores.
- **Why is chip design so expensive?** You need to design CPU + GPU + NPU + ISP + codec + ... on
  one die, manage power domains, and verify reliability — not just a faster ALU.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Dennard scaling | "Shrinking transistors gives free power savings" | Voltage, current, and capacitance all shrink proportionally → constant power density. Broke ~2005 due to leakage. |
| Leakage current | "Transistors leak even when off" | Electrons tunnel through ultra-thin gate oxide. Scales exponentially with voltage reduction failing. |
| TDP | "The chip's wattage rating" | Thermal Design Power — the sustained heat the cooling system must dissipate. Not peak power. |
| Dark silicon | "Transistors we can't afford to turn on" | Die area left unpowered because the thermal budget can't support activating everything simultaneously. |
| DVFS | "Dynamic clock scaling" | Dynamic Voltage and Frequency Scaling — continuously adjusts V and f to balance performance and power. |
| Electromigration | "Wires degrade over time" | Metal atoms migrate under high current density, causing opens or shorts over years of operation. |
| NBTI | "PMOS aging" | Negative Bias Temperature Instortion — PMOS transistors slow down over time as charges trap in the gate oxide. |
| Soft error | "Cosmic ray bit flip" | A high-energy particle strikes a storage node and flips a bit. Non-destructive but can corrupt data. |
| Amdahl's Law | "Parallelism has diminishing returns" | Speedup from adding processors is limited by the sequential fraction of the program. |

## Further Reading

- Dennard, R. H. et al., "Design of Ion-Implanted MOSFET's with Very Small Physical Dimensions," *IEEE JSSC*, 1974. The original scaling paper.
- Esmaeilzadeh, H. et al., "Dark Silicon and the End of Multicore Scaling," *IEEE Micro*, 2012. Quantitative analysis of the dark silicon problem.
- Hennessy, J. L. & Patterson, D. A., *Computer Architecture: A Quantitative Approach*, 6th ed., Ch. 1 (Power) and Ch. 7 (GPUs and accelerators).
- ARM, "big.LITTLE Technology: The Future of Mobile." Whitepaper on heterogeneous multiprocessing.
