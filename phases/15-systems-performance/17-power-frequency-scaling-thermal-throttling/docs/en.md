# Power, Frequency Scaling, Thermal Throttling

> Why your CPU isn't always running at its advertised speed — and what to do about it.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 15 lessons 01–16
**Time:** ~45 minutes

## Learning Objectives

- Explain why power is the fundamental constraint on CPU performance, not frequency or IPC alone.
- Describe how DVFS (Dynamic Voltage and Frequency Scaling) works and why voltage scales super-linearly with frequency.
- Distinguish P-states (operating performance levels) from C-states (idle/sleep levels) and when each is used.
- Explain turbo boost: how it works, why it's not always on, and how long it's sustainable.
- Identify thermal throttling triggers (PROCHOT, thermal velocity boost) and their impact on sustained performance.
- Use RAPL counters and `perf stat` to measure actual power consumption and energy efficiency.
- Compare Linux CPU frequency governors and their trade-offs for benchmarking and production workloads.
- Reason about power behavior in cloud VMs, ARM big.LITTLE systems, and real benchmarking scenarios.

## The Problem

You just optimized your code. Your microbenchmark runs 30% faster. You ship it. In production, it's only 5% faster. What happened?

The answer is almost always **power and thermal management**. Modern CPUs are not static devices — they dynamically adjust voltage, frequency, and power draw hundreds of times per second. Your microbenchmark ran with the CPU in turbo (high frequency, high power). Your production workload ran after the chip had already heated up and throttled back.

This lesson covers the power–frequency–thermal triangle that governs real-world CPU performance. Without understanding it, you cannot measure honestly, tune cache/branches/IO, or win 10x by knowing the machine.

## Why Power Matters for Performance

Power is the master constraint. Everything else — frequency, IPC, core count — is downstream of how much power you can deliver and how much heat you can remove.

The dynamic power equation for a CMOS transistor switching at frequency *f* is:

```
P_dynamic = α · C · V² · f
```

Where:
- **α** = activity factor (fraction of transistors switching, 0–1)
- **C** = capacitance (determined by transistor size and process node)
- **V** = supply voltage
- **f** = switching frequency

Key insight: **power scales with the square of voltage**. Halving voltage reduces dynamic power by 4x. This is why lowering voltage is far more effective than lowering frequency for saving power.

But there's a catch: a transistor needs a minimum voltage to switch reliably at a given speed. Higher frequency requires higher voltage. The relationship isn't linear — it follows a curve where voltage must increase faster than frequency at the top end.

```
Typical V-f curve (approximate):
f (GHz)   V (V)    P (relative)
1.0       0.8      1x
2.0       0.9      3.4x
3.0       1.0      8.1x
4.0       1.1      17.4x
5.0       1.3      38.0x
```

Going from 4 GHz to 5 GHz (25% more frequency) costs **2.2x more power**. This is why the last few hundred MHz are so expensive.

## DVFS: Dynamic Voltage and Frequency Scaling

DVFS is the OS/hardware mechanism that adjusts voltage and frequency together based on workload demand. This isn't just turning a dial — the hardware must:

1. **Request a new P-state** (target frequency/voltage pair)
2. **Ramp voltage** to the new level (voltage must lead frequency up and lag frequency down)
3. **Wait for voltage to stabilize** (microseconds)
4. **Switch frequency** to the new target
5. **Re-lock the PLL** (phase-locked loop that generates the clock)

A DVFS transition takes **10–100 µs** on modern hardware. During this time, the CPU may stall. The OS tries to minimize transitions by using hysteresis — it doesn't shift P-state on every idle, but waits until a threshold is crossed.

### The Efficacy Gap

DVFS saves less power than the ideal P ∝ V²f relationship suggests because:

- **Static (leakage) power** doesn't scale with frequency — it's always present
- At low frequencies, leakage becomes a larger fraction of total power
- Voltage can't drop below the minimum operating voltage (Vmin)
- Real DVFS steps are discrete, not continuous

The practical rule: DVFS is effective for **moderate** frequency ranges (say 50–80% of max). Below that, you're better off racing to sleep (C-states, covered next).

## P-States: Operating Performance Levels

P-states define discrete operating points (frequency + voltage pairs) that the CPU can switch between while actively executing instructions.

### Intel P-States (Modern)

On modern Intel CPUs (Skylake and later), the hardware manages P-states autonomously via **Intel Speed Shift Technology (HWP — Hardware P-States)**:

- The OS sets a range (min_perf, max_perf, and an energy_perf_preference)
- The hardware picks the actual frequency within that range every millisecond
- The hardware can ramp frequency in **< 1 ms** (vs. ~30 ms with OS-managed P-states)

HWP is controlled via MSR registers:
- `IA32_HWP_REQUEST` — min/max desired performance, energy preference
- `IA32_HWP_STATUS` — current performance level
- `IA32_HWP_CAPABILITIES` — highest/lowest/guaranteed performance levels

The **energy_perf_preference (EPP)** is critical:
- EPP = 0: maximum performance, minimum efficiency
- EPP = 128: balanced (default)
- EPP = 255: maximum efficiency, minimum performance

On Linux, this is exposed via `/sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference`.

### Legacy ACPI P-States

On older hardware (or when HWP is disabled), P-states follow the ACPI model:

| P-State | Meaning |
|---------|---------|
| P0 | Maximum performance (highest frequency, highest voltage) |
| P1 | Next lower performance level |
| ... | ... |
| Pn | Lowest performance level (lowest frequency, lowest voltage) |

The OS selects P-states based on the frequency governor policy (covered below). The number of P-states varies by processor — there may be anywhere from 2 to 16.

The key file on Linux: `/sys/devices/system/cpu/cpu*/cpufreq/scaling_available_frequencies` lists the available P-state frequencies for each CPU.

## C-States: Idle/Sleep Levels

C-states are the opposite of P-states. P-states say "I'm working, but at this speed." C-states say "I'm not working right now, power down what I can."

| C-State | Name | What's Off | Exit Latency | Power Savings |
|---------|------|------------|--------------|----------------|
| C0 | Active | Nothing | 0 ns | 0% |
| C1 | Halt | Clock gate | ~1 µs | ~50% |
| C1E | Enhanced Halt | Clock gate + voltage lower | ~1 µs | ~60% |
| C2 | Stop Grant | Bus disconnect | ~10 µs | ~70% |
| C3 | Deep Sleep | Clock + PLL off | ~50 µs | ~80% |
| C6 | Deep Power Down | L2 flush + voltage off | ~100 µs | ~95% |
| C7 | Deeper Power Down | L2 flush + all domains off | ~200 µs | ~97% |
| C8-C10 | Even Deeper | Progressively more cutoff | ~500+ µs | ~98%+ |

**Critical point:** deeper C-states save more power but take longer to wake from. If the CPU enters C6 but wakes up 50 µs later, you've wasted power (the transition cost more than you saved) and added latency.

The OS kernel scheduler and idle governor (typically `menu` or `teo` on Linux) predict how long the CPU will be idle and pick the shallowest C-state whose exit latency is less than the predicted idle duration.

C-states are controlled on Linux via:
- `/sys/devices/system/cpu/cpu*/cpuidle/state*/disable` — enable/disable individual C-states
- `processor.max_cstate=N` kernel parameter — limit deepest C-state
- `intel_idle.max_cstate=N` — override for Intel idle driver

### Racing to Sleep

The most power-efficient strategy for short bursts of work is: **run as fast as possible (high P-state) then drop into the deepest C-state possible (C6/C7)**. This "race to idle" can be more efficient than running slowly at a low P-state for longer.

Worked example:
- Task: process 1 billion cycles of work
- P0 (4 GHz): completes in 250 ms, uses 100W → 25 J of energy
- P4 (1 GHz): completes in 1000 ms, uses 25W → 25 J of energy
- Same energy! But P0 finishes faster, allowing the CPU to enter deep C-state for the remaining 750 ms
- P0 + C6 for 750 ms: 25 J + (0.05 W × 0.75 s) = 25.04 J total
- P4 for 1000 ms: 25 J total

In this example, they're close. But with realistic V² scaling:
- P0 (4 GHz @ 1.2V): 100W for 250 ms = 25 J
- P4 (1 GHz @ 0.8V): ~18W for 1000 ms = 18 J ← less energy

Race-to-sleep doesn't always win — it depends on the V-f curve and how deep the idle state is. This is why modern systems use a mix of DVFS + deep C-states rather than just one approach.

## Turbo Boost: How It Works and Why It's Not Always On

Turbo boost allows CPU cores to run **above their base (guaranteed) frequency** when thermal and power headroom exists.

### How Turbo Works

1. The CPU has a **thermal design power (TDP)** rating — e.g., 125W for an Intel Core i9-13900K
2. The base frequency is the frequency achievable while sustaining TDP indefinitely on all cores
3. When not all cores are active, the remaining thermal+power budget goes to the active cores
4. The CPU boosts active core frequency beyond base, up to the **max turbo frequency**

```
Example: Intel Core i9-13900K
- Base frequency:     3.0 GHz (all cores active, sustained TDP = 125W)
- Turbo 1 core:      5.8 GHz
- Turbo 2 cores:     5.7 GHz
- Turbo 4 cores:     5.5 GHz
- Turbo 8 cores:     5.2 GHz
- Turbo all 24 cores: ~4.7 GHz (not published, observed)
```

More active cores = less headroom = lower turbo frequency. This is why single-threaded benchmarks always look better than multi-threaded ones — the single core can turbo higher.

### Why Turbo Isn't Always On

Turbo depends on three headroom sources:

1. **Thermal headroom**: The CPU must be below Tjmax (junction temperature max, typically 100°C). If the die is hot, turbo is reduced or disabled.
2. **Power headroom**: The CPU must be within its power limit (PL1 = sustained, PL2 = short-term turbo). If the VRM can't deliver enough current, turbo is limited.
3. **Electrical headroom**: The voltage regulator must be able to supply the required voltage at the required current (V.Rail stability).

Even if the OS requests maximum performance, the hardware will reduce turbo or drop to base frequency if any of these constraints are hit.

### Turbo Sustainability

The key question for benchmarking: **how long does turbo last?**

Modern Intel CPUs use two power limits:
- **PL1 (Sustained)**: The long-term power limit, equal to TDP. The CPU can run at PL1 indefinitely.
- **PL2 (Turbo)**: A short-term power limit above PL1. The CPU can run at PL2 for **Tau** seconds (typically 28–56 seconds), then must drop to PL1.

```
Time →
     ┌────────┐
     │ PL2    │ ← Turbo power (e.g., 251W)
     │        │
PL1  │        └────────────────────── ← Sustained power (e.g., 125W)
     │                                 After Tau seconds, must drop to PL1
     └────────┘
         Tau
      (28-56s)
```

This means:
- Benchmarks shorter than Tau run entirely in turbo — **not representative of sustained performance**
- Real workloads that run for minutes see the PL1→PL2 transition
- The CPU may also throttle earlier if temperature exceeds Tjmax before Tau expires

On Linux, you can observe PL1/PL2 via RAPL (covered below) or Intel's `x86_energy_perf_policy` tool.

### Intel Speed Select Technology (SST)

On server CPUs (Xeon), Intel SST provides fine-grained control over turbo allocation:

- **SST-PP (Performance Profile)**: Select from multiple power/performance profiles (e.g., "performance" vs "efficiency" base frequencies)
- **SST-BF (Base Frequency)**: Prioritize specific cores for guaranteed minimum frequency
- **SST-TF (Turbo Frequency)**: Assign turbo budget to specific cores

This matters in cloud environments where you might pay for a "guaranteed base frequency" VM — SST-TF ensures certain cores always hit their rated speed.

## Thermal Throttling

When a CPU exceeds its maximum junction temperature (Tjmax, typically 95–105°C), it must reduce power to avoid damage. This is **thermal throttling**.

### PROCHOT

**PROCHOT** (Processor Hot) is a hardware signal that forces the CPU to its minimum frequency to prevent thermal damage. When PROCHOT asserts:

1. CPU frequency drops to the minimum P-state (often 800 MHz on Intel)
2. Voltage drops to Vmin
3. Power consumption drops dramatically
4. Performance falls off a cliff

PROCHOT is a last resort — it means thermal management has failed. You should never see PROCHOT in normal operation. If you do, the cooling system is inadequate.

You can check for PROCHOT events on Linux:
```bash
rdmsr -a 0x1B0  # IA32_THERM_STATUS, bit 0 = PROCHOT status
```

### Thermal Velocity Boost (TVB)

TVB is a more granular throttling mechanism. As temperature increases:
- The CPU reduces turbo frequency in 100 MHz increments
- This is proactive (prevents PROCHOT) rather than reactive
- TVB follows a temperature-vs-frequency curve

```
Temp (°C)   Max Turbo (1-core, example)
40          5.8 GHz
50          5.7 GHz
60          5.5 GHz
70          5.3 GHz
80          5.1 GHz
90          4.9 GHz
95          4.7 GHz  ← approaching PROCHOT territory
100         PROCHOT  ← forced minimum frequency
```

This means "5.8 GHz turbo" on the spec sheet is only achievable when the die is cool. Under sustained load, the actual turbo is much lower.

### Impact on Benchmarking

Thermal throttling is the #1 reason microbenchmarks mislead:

1. **Cold start**: First run is fast (CPU is cool, turbo is high). Subsequent runs are slower.
2. **Warm-up artifact**: The first N seconds of any benchmark include turbo that won't be sustained.
3. **Cooling-dependent**: Results vary with ambient temperature, fan speed, chassis design.
4. **Core count dependent**: Turbo on 1 core > turbo on 2 cores > turbo on all cores.

To get reproducible results:
- Pre-heat the CPU (run a burn-in for 60+ seconds before measuring)
- Pin the frequency with `cpufreq-set -g performance` and optionally `cpufreq-set -f <freq>`
- Monitor temperature with `sensors` and RAPL power with `perf stat`
- Report whether turbo was active during the measurement
- Run benchmarks for >60 seconds to get past the PL2/PL1 transition

## Power Capping: RAPL and Intel Speed Select

### RAPL (Running Average Power Limit)

RAPL is Intel's power management framework. It provides:

1. **Power measurement**: Per-domain energy counters (pkg, cores, uncore, DRAM)
2. **Power limiting**: Set power caps that the hardware enforces

RAPL domains:
| Domain | What It Covers | MSR |
|--------|---------------|-----|
| PP0 | Core power | MSR_PP0_ENERGY_STATUS |
| PP1 | Uncore (GPU on client) | MSR_PP1_ENERGY_STATUS |
| PKG | Entire package (cores + uncore + DRAM on server) | MSR_PKG_ENERGY_STATUS |
| DRAM | DRAM power (server only) | MSR_DRAM_ENERGY_STATUS |
| PSYS | Platform power (SoC, on some chips) | MSR_PLATFORM_ENERGY_STATUS |

Reading RAPL counters directly:
```bash
# Read package energy (in microjoules, wraps at ~2^32)
rdmsr -a 0x611  # MSR_PKG_ENERGY_STATUS

# Or via sysfs (easier):
cat /sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj
cat /sys/class/powercap/intel-rapl/intel-rapl:0/intel-rapl:0:0/energy_uj  # PP0 (cores)
cat /sys/class/powercap/intel-rapl/intel-rapl:0/intel-rapl:0:1/energy_uj  # PP1 (uncore)
```

Setting a power cap via RAPL:
```bash
# Set package power limit to 65W (short-term) and 45W (long-term)
# This writes to the RAPL control MSR
sudo rdmsr -a 0x610  # Read current limits first
sudo wrmsr -a 0x610 <value>  # Set new limits (bit-packed, see Intel SDM)
```

On Linux, the `powercap` sysfs interface provides a safer way:
```bash
# Set constraint 0 (long-term) to 45W
echo 45000000 | sudo tee /sys/class/powercap/intel-rapl/intel-rapl:0/constraint_0_power_limit_uw

# Set constraint 1 (short-term) to 65W
echo 65000000 | sudo tee /sys/class/powercap/intel-rapl/intel-rapl:0/constraint_1_power_limit_uw
```

When a power cap is active, the CPU will throttle to stay within the limit. This is deterministic — the frequency is adjusted smoothly, not by PROCHOT-style clamping.

### Intel Speed Select (SST)

On Xeon Scalable processors, Intel SST gives administrators per-core control over frequency and power:

```bash
# View current SST configuration
intel-speed-select -c 0 perf-profile info

# Set a specific core to guaranteed base frequency
intel-speed-select -c 0 perf-profile set --priority-core=3

# Enable SST-BF (base frequency boost for priority cores)
intel-speed-select -c 0 bf enable
```

In cloud environments, SST enables "guaranteed vCPU" offerings where specific cores promise a minimum frequency regardless of overall system load.

## Measuring Power: perf and RAPL

### Using perf stat

```bash
# Measure energy consumed during a workload
perf stat -e power/energy-pkg/,power/energy-cores/,power/energy-ram/ \
    ./my_workload

# Example output:
#   328.45 Joules power/energy-pkg/    (±0.03%)
#   189.12 Joules power/energy-cores/  (±0.05%)
#    52.30 Joules power/energy-ram/     (±0.08%)
```

To compute average power: `power (W) = energy (J) / time (s)`

```bash
# Time-limited measurement (5 seconds)
perf stat -e power/energy-pkg/ -I 1000 sleep 5
# Reports energy every second → compute power per interval
```

### RAPL Counter Details

RAPL energy counters are **accumulators** — they count up from boot and wrap around at a domain-specific maximum:

- PKG energy wraps at 2^32 µJ ≈ 4.3 kJ (at 65W, this wraps every ~66 seconds)
- DRAM energy wraps similarly

For accurate measurements, you must read the counter before and after your workload and handle wraparound:

```python
def read_rapl_energy():
    with open('/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj') as f:
        return int(f.read().strip())

def measure_energy(func, *args):
    start = read_rapl_energy()
    func(*args)
    end = read_rapl_energy()
    if end >= start:
        return end - start
    else:
        return (2**32 - start) + end  # wrapped around
```

### turbostat

`turbostat` is the best tool for real-time power and frequency monitoring:

```bash
# Show per-core frequency, C-state residency, temperature, and power every second
sudo turbostat -i 1

# Key columns:
#   PKG_%  — package C-state residency percentages
#   Core_% — per-core C-state residency
#   Bzy_MHz — actual busy frequency (averaged over the interval)
#   GFX_RC6 — GPU idle percentage
#   Pkg_Watt — package power in watts
#   RAM_Watt — DRAM power in watts
```

## Frequency Governors

Linux CPU frequency governors control P-state selection policy. Each governor has a different strategy for balancing performance vs. power.

| Governor | Strategy | DVFS Transitions | Use Case |
|-----------|----------|------------------|----------|
| `performance` | Always max frequency | Minimal (stays at max) | Benchmarking, latency-sensitive workloads |
| `powersave` | Always min frequency | Minimal (stays at min) | Maximum efficiency, low-power devices |
| `ondemand` | Scale up on load, down on idle | Frequent (sampling-based) | General purpose, older kernels |
| `conservative` | Like ondemand but slower to scale up | Less frequent | Battery life over responsiveness |
| `schedutil` | Scheduler-driven DVFS | Fast (scheduler hints) | Modern default, good balance |

### schedutil (Modern Default)

`schedutil` is the modern governor (introduced Linux 4.7) that uses scheduler utilization hints instead of periodic sampling:

```c
// Kernel pseudo-code for schedutil decision:
void schedutil_update_cpu(int cpu) {
    unsigned long util = cpu_util(cpu);      // 0-1023, from scheduler
    unsigned long max = cpu_max_freq(cpu);
    unsigned long next_f = (util * max) / 1023;
    
    if (util > 0)
        set_frequency(cpu, max(next_f, policy->min));
    else
        set_frequency(cpu, policy->min);
}
```

`schedutil` responds in <1ms to load changes (vs. ~10ms for `ondemand`'s sampling). With HWP, the hardware takes over and `schedutil` mainly sets the EPP preference.

### Setting the Governor

```bash
# Check current governor
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor

# Set governor to performance (for benchmarking)
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Set governor to powersave (for efficiency)
echo powersave | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
```

For benchmarking: **always use the `performance` governor**. If you don't, the governor is making frequency decisions that make your results non-deterministic.

## Energy vs. Performance: The Tradeoff

The fundamental tradeoff: **performance costs energy, and energy becomes heat**.

### Energy-Delay Product (EDP)

EDP = Energy × Delay² — a common metric for evaluating efficiency:

```
Architecture comparison:
Processor A: 10W, 100ms → EDP = 10 × 0.1² = 0.1 J·s
Processor B:  5W, 200ms → EDP =  5 × 0.2² = 0.2 J·s
Processor C: 20W,  50ms → EDP = 20 × 0.05² = 0.05 J·s  ← best EDP
```

Processor C uses more power but finishes faster, saving more total energy and having the best EDP. This is the race-to-sleep argument.

### Performance Per Watt

In data centers, the metric that matters is **performance per watt** (ops/J):

```bash
# Measure operations per joule:
perf stat -e power/energy-pkg/,instructions ./my_workload
# EDP = (energy_J) × (time_s)²

# Compute ops/J:
ops_per_joule = instructions_count / energy_joules
```

A server that does 10% more work per watt can mean millions of dollars in data center electricity savings.

## Real-World Impact: Power Management and Benchmarking

### The Benchmarking Checklist

Before running any performance benchmark:

1. **Pin frequency**: Set governor to `performance`, optionally fix frequency with `cpufreq-set -f`
2. **Disable turbo**: `echo 1 > /sys/devices/system/cpu/intel_pstate/no_turbo` (forces base frequency)
3. **Pre-heat**: Run a CPU-intensive warm-up for 60+ seconds to get past PL2
4. **Isolate cores**: Use `taskset` or `cgroups` to avoid scheduling interference
5. **Monitor**: Run `turbostat` or `perf stat` alongside to verify frequency stayed constant
6. **Report**: Always state governor, turbo status, and measured average frequency

### Cloud VM Power Behavior

In cloud environments (AWS, GCP, Azure), you don't control the hardware:

- **vCPU overcommit**: Your vCPU may share a physical core with other VMs
- **Shared power budget**: Other VMs' load reduces your turbo headroom
- **No RAPL access**: Cloud providers don't expose power counters
- **No frequency control**: You can't set governors or pin frequencies
- **Variable performance**: Same VM type can have 20-30% performance variation depending on neighbor load

Mitigation strategies:
- Use **dedicated hosts** or **CPU-optimized** instance types for consistent performance
- Run long enough (> 60s) to amortize turbo variability
- Use **perf stat** with hardware counters where available
- Accept variance and report confidence intervals (p50, p95, p99)

### ARM big.LITTLE Heterogeneous Computing

ARM's big.LITTLE (and DynamIQ) architecture introduces a different power model:

- **big cores**: High performance, high power (like Intel P-cores)
- **LITTLE cores**: Low performance, ultra-low power (like Intel E-cores, but more extreme)

```
ARM big.LITTLE power comparison (approximate):
Core Type    Freq (GHz)   Power (W)   DMIPS
Cortex-X3    3.3          5.0         45
Cortex-A715  2.5          0.8         18
Cortex-A510  2.0          0.3         10
```

The OS scheduler (or firmware) migrates tasks between big and LITTLE cores:
- **Threads with high utilization** → big cores
- **Threads with low utilization** → LITTLE cores
- **Background tasks** → LITTLE cores exclusively

This is similar to Intel's hybrid P-core/E-core architecture (Alder Lake, Raptor Lake) but ARM's performance gap between big and LITTLE is larger.

On Linux, this is managed by **Energy Aware Scheduling (EAS)**, which uses CPU utilization and energy models to place tasks on the most efficient core that still meets performance requirements.

```bash
# Check heterogeneous CPU topology:
ls /sys/devices/system/cpu/cpu*/topology/
# Look at "core_id" and "physical_package_id" — same package but different core_ids
# indicate big vs. LITTLE cores

# Check CPU capacity (higher = more powerful):
cat /sys/devices/system/cpu/cpu*/cpu_capacity
# Example output: big cores = 1024, LITTLE cores = 300
```

### Intel Hybrid: P-cores and E-cores

Intel's 12th+ Gen (Alder Lake, Raptor Lake, Meteor Lake) uses a similar model:

- **P-cores** (Performance): High frequency, high power, supports hyperthreading
- **E-cores** (Efficient): Lower frequency, lower power, no hyperthreading, more of them

```
Intel Core i9-13900K example:
8 P-cores @ 5.8 GHz (turbo) + 16 E-cores @ 4.3 GHz (turbo)
= 32 threads total

P-core power: ~25W each (at max turbo)
E-core power: ~5W each (at max turbo)

For throughput: E-cores are 2-3x more efficient (ops/watt)
For latency: P-cores are 2x faster per thread
```

Intel's Thread Director (hardware + OS driver) classifies threads and schedules them:
- Latency-sensitive → P-cores
- Throughput-oriented → E-cores
- Background → E-cores

On Linux, the scheduler uses **ASMP** (Asymmetric Multiprocessing) awareness to make similar decisions.

## The DVFS Math: Worked Examples

### Example 1: Power Budget Allocation

You have a 65W TDP processor with 4 cores. How much turbo can each core get?

```
At base frequency (3.0 GHz):
  4 cores × 16.25W/core = 65W ← exactly TDP

If only 1 core is active:
  TDP budget = 65W
  Idle cores ≈ 2W each (deep C-state) = 6W
  Available for active core = 65W - 6W = 59W
  But the core may be voltage-limited or frequency-limited
  Actual turbo = 5.0 GHz @ ~59W (voltage-limited before 65W)
```

### Example 2: Energy Comparison

Task: Process 10 billion instructions. Compare strategies:

```
Strategy A: High performance
  Frequency: 4.0 GHz, Voltage: 1.2V
  Power: 80W
  Time: 10×10⁹ / (4×10⁹ × 2 IPC) = 1.25 seconds
  Energy: 80W × 1.25s = 100 J

Strategy B: Race to sleep
  Frequency: 4.0 GHz, Voltage: 1.2V  
  Work: 1.25 seconds at 80W
  Sleep: 8.75 seconds at 0.5W (deep C-state)
  Total time: 10 seconds (for comparison)
  Energy: 100 J + 4.375 J = 104.4 J
  But finish time: 1.25 seconds

Strategy C: Low and slow
  Frequency: 1.0 GHz, Voltage: 0.7V
  Power: α×C×V²×f = ~10W (proportional)
  Time: 10×10⁹ / (1×10⁹ × 2 IPC) = 5 seconds
  Energy: 10W × 5s = 50 J ← half the energy!

Note: V² scaling means low frequency is much more efficient.
The "right" choice depends on whether you care about:
  - Latency (Strategy A)
  - Throughput per watt (Strategy C)
  - Energy per operation (Strategy C)
```

### Example 3: Cloud Variability

Two runs of the same benchmark on a cloud VM:

```
Run 1 (idle neighbor):
  Average frequency: 3.8 GHz
  Runtime: 10.2 seconds
  Turbo: active for 55% of runtime

Run 2 (busy neighbor):
  Average frequency: 2.9 GHz
  Runtime: 13.4 seconds
  Turbo: active for 5% of runtime

Performance difference: 31%
Same VM type, same code, different time of day.
```

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| DVFS | "CPU power saving" | Dynamic Voltage AND Frequency Scaling — adjusting V and f together to trade performance for power |
| P-state | "CPU speed setting" | A discrete (V, f) operating point the CPU can transition to while active |
| C-state | "Sleep mode" | An idle state that turns off portions of the CPU to save leakage power |
| Turbo boost | "Overclocking" | Running above base frequency when thermal/power headroom exists — temporary and conditional |
| TDP | "Max power" | Thermal Design Power — the sustained power the cooling system must dissipate, NOT the maximum power |
| PROCHOT | "Thermal throttle" | A hardware signal forcing the CPU to minimum frequency to prevent thermal damage |
| RAPL | "Power measurement" | Running Average Power Limit — Intel's framework for measuring energy and setting power caps |
| EPP | "Power preference" | Energy Performance Preference — 0-255 scale telling hardware how to trade performance for efficiency |
| Tau | "Turbo time limit" | The time window the CPU can operate at PL2 before dropping to PL1 |
| PL1/PL2 | "Power limits" | PL1 = sustained power limit (TDP); PL2 = short-term turbo power limit |

## Further Reading

- Intel 64 and IA-32 Architectures Software Developer's Manual, Volume 3, Chapter 14 — Power Management
- Intel RAPL Interface specification: https://www.intel.com/content/dam/www/public/us/en/documents/white-papers/rapl-power-interface.pdf
- Linux CPUFreq documentation: https://www.kernel.org/doc/html/latest/admin-guide/pm/cpufreq.html
- Linux CPU Idle documentation: https://www.kernel.org/doc/html/latest/admin-guide/pm/cpuidle.html
- "An Experimental Analysis of DVFS on Modern_processors" — Mair et al.
- Intel Speed Select Technology: https://www.intel.com/content/www/us/en/architecture-and-technology/speed-select-technology-article.html
- ARM big.LITTLE technology: https://developer.arm.com/Architectures/bigLITTLE