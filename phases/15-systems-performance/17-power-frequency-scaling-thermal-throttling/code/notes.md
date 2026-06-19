# Notes — Power, Frequency Scaling, Thermal Throttling

## Power Equation

```
P_dynamic = α · C · V² · f
```

- V² is the dominant term — halving voltage → 4x less power
- At high frequencies, V must increase, making the top end extremely expensive
- A 25% frequency increase can cost 2x power at the top of the V-f curve

## P-States (Active Performance Levels)

| P-State | Voltage | Frequency | Use Case |
|---------|----------|-----------|----------|
| P0 | 1.30V | 5.0 GHz | Max turbo, single-thread burst |
| P1 | 1.20V | 4.5 GHz | Multi-core turbo |
| P2 | 1.10V | 4.0 GHz | All-core turbo |
| P3 | 1.00V | 3.5 GHz | High performance |
| P4 | 0.90V | 3.0 GHz | Base frequency (TDP) |
| P5 | 0.80V | 2.0 GHz | Efficiency |
| P6 | 0.70V | 1.0 GHz | Minimum viable |

## C-States (Idle Levels)

| C-State | Exit Latency | Power Saving | When to Use |
|---------|-------------|---------------|-------------|
| C0 | 0 ns | 0% | Active execution |
| C1 (Halt) | ~1 µs | ~50% | Sub-microsecond idle |
| C1E | ~1 µs | ~60% | Short idle periods |
| C3 (Deep Sleep) | ~50 µs | ~80% | Medium idle (>100 µs) |
| C6 (Deep Power Down) | ~100 µs | ~95% | Long idle (>200 µs) |
| C7+ | ~200+ µs | ~97%+ | Very long idle (>500 µs) |

Rule: Enter C-state only if predicted idle duration > 2× exit latency

## DVFS Transition Sequence

```
1. OS/HWP selects target P-state
2. Voltage ramps to new level (V must lead f up)
3. Wait for voltage stabilization (~5-10 µs)
4. PLL re-locks to new frequency (~1 µs)
5. CPU resumes execution at new (V, f)
6. Total transition: 10-100 µs
```

During transition, CPU cannot execute instructions.

## Turbo Sustainability (PL1/PL2/Tau)

```
Power
  │
  │  ┌──── PL2 (turbo, e.g., 251W) ────┐
  │  │                                   │ Tau (28-56s)
  │  │                                   │
  │  └──────────────────────────────────┘
  │  PL1 (sustained, e.g., 125W) ←─────────────────
  │
  └─────────────────────────────────────── Time →
         │← Tau →│        After Tau → PL1 only
```

- Benchmarks < Tau: running entirely in turbo (unrealistic for sustained loads)
- PL1 = TDP = guaranteed frequency sustainable indefinitely
- If temperature hits Tjmax before Tau → early throttle regardless

## Thermal Throttling Hierarchy

```
1. TVB (proactive): Reduce turbo in 100 MHz increments as temp rises
   40°C → 5.8 GHz →  50°C → 5.7 GHz →  60°C → 5.5 GHz
   70°C → 5.3 GHz →  80°C → 5.1 GHz →  90°C → 4.9 GHz

2. Power limit (RAPL PL1): Cap at TDP, reduce frequency as needed
   Sustained load → CPU settles at base frequency

3. PROCHOT (reactive emergency): Force minimum frequency (~800 MHz)
   Die temp > Tjmax → brutal throttle to prevent damage
```

## Frequency Governors Comparison

| Governor | Latency | Power | Best For |
|----------|---------|-------|----------|
| performance | 0 (always max) | Worst | Benchmarking only |
| powersave | 0 (always min) | Best (for idle) | Low-power devices |
| ondemand | ~10 ms (sampling) | Moderate | Legacy general use |
| schedutil | <1 ms (scheduler) | Good balance | Modern default |
| conservative | ~100 ms (slow ramp) | Better | Battery life |

For benchmarking: ALWAYS use `performance` governor.

## RAPL Energy Counter Quick Reference

```
Domain    MSR          Sysfs Path
──────    ───          ──────────
PKG       0x611        intel-rapl:0/energy_uj
PP0       0x639        intel-rapl:0:0/energy_uj   (cores)
PP1       0x641        intel-rapl:0:1/energy_uj   (uncore)
DRAM      0x619        intel-rapl:0:2/energy_uj   (server only)
PSYS      0x64d        intel-rapl:0:3/energy_uj   (platform)
```

Counter wraps at 2^32 µJ ≈ 4.3 kJ. At 65W, wraps every ~66 seconds.

## DVFS Math: Energy vs Performance

```
Scenario: Process 10 billion instructions, IPC = 2

High-perf:  4 GHz, 1.2V, 80W
  Time  = 10×10⁹ / (4×10⁹ × 2) = 1.25 s
  Energy = 80 × 1.25 = 100 J
  EDP   = 100 × 1.25² = 156.25 J·s

Low-power: 1 GHz, 0.7V, ~10W
  Time  = 10×10⁹ / (1×10⁹ × 2) = 5.0 s
  Energy = 10 × 5.0 = 50 J
  EDP   = 50 × 5.0² = 1250 J·s

High-perf wins on latency (1.25s vs 5.0s)
Low-power wins on energy (50J vs 100J)
High-perf wins on EDP (156 vs 1250)

EDP² = Energy × Delay² penalizes slow more than energy-hungry.
```

## Cloud VM Power Reality

```
Same instance type, different times:
  Idle neighbor:  avg 3.8 GHz, 10.2s runtime, 55% turbo
  Busy neighbor:  avg 2.9 GHz, 13.4s runtime, 5% turbo

Performance delta: 31% — same code, same VM type, different neighbor load.

No access to:
  - RAPL counters
  - Frequency governors
  - C-state control
  - Turbo toggle

Mitigation: dedicated hosts, long benchmarks (>60s), report percentiles.
```

## ARM big.LITTLE Efficiency

```
Cortex-X3 (big):  3.3 GHz, 5.0W,  45 DMIPS → 9 DMIPS/W
Cortex-A510 (LITTLE): 2.0 GHz, 0.3W, 10 DMIPS → 33 DMIPS/W

LITTLE cores: 3.7x more efficient per watt
big cores: 4.5x more performance per thread

Scheduler (EAS) routes:
  - High-utilization threads → big cores
  - Low-utilization threads → LITTLE cores
  - Latency-sensitive → big cores
  - Background → LITTLE cores
```