# Power, Frequency Scaling & Thermal Throttling — Quick Reference

## P-States (Active Performance Levels)

| State | Description | Latency | Exit To |
|-------|-------------|---------|---------|
| P0 | Max performance (highest f, V) | — | — |
| P1 | Next lower level | ~10 µs | P0 |
| P2 | ... | ~10 µs | P1 |
| Pn | Min performance (lowest f, V) | ~10 µs | Pn-1 |

- Modern Intel (HWP): hardware selects P-state autonomously within OS-set range
- Legacy: OS selects via governor policy
- Key sysfs: `/sys/devices/system/cpu/cpu*/cpufreq/scaling_*`

## C-States (Idle/Sleep Levels)

| C-State | Name | What's Off | Exit Latency | Power Saving |
|---------|------|-----------|--------------|-------------|
| C0 | Active | Nothing | 0 | 0% |
| C1 | Halt | Clock gated | ~1 µs | ~50% |
| C1E | Enhanced Halt | Clock + V lowered | ~1 µs | ~60% |
| C3 | Deep Sleep | Clock + PLL off | ~50 µs | ~80% |
| C6 | Deep Power Down | L2 flush + V off | ~100 µs | ~95% |
| C7+ | Deeper | Progressive shutdown | ~200+ µs | ~97%+ |

Rule: Only enter C-state if predicted idle > 2× exit latency.

Control: `/sys/devices/system/cpu/cpu*/cpuidle/state*/disable`

## Turbo Boost Quick Facts

| Term | Value/Description |
|------|-------------------|
| PL1 | Sustained power limit = TDP, indefinite |
| PL2 | Short-term turbo power limit, for Tau seconds only |
| Tau | PL2 time window, typically 28–56 seconds |
| Tjmax | Max junction temp, typically 95–105°C |
| PROCHOT | Emergency throttle → minimum frequency (~800 MHz) |

Turbo frequency decreases with: more active cores ↑, higher temperature ↑, less power headroom ↑.

Disable turbo on Linux: `echo 1 > /sys/devices/system/cpu/intel_pstate/no_turbo`

## Temperature vs. Turbo (TVB)

| Temp Range | Behavior |
|-----------|----------|
| < 50°C | Full turbo available |
| 50–80°C | Turbo reduces in ~100 MHz steps |
| 80–95°C | Turbo significantly reduced |
| > Tjmax | PROCHOT → forced minimum frequency |

## RAPL Commands

### Read Power/Energy

```bash
# Package energy (microjoules)
cat /sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj

# Core energy (PP0)
cat /sys/class/powercap/intel-rapl/intel-rapl:0/intel-rapl:0:0/energy_uj

# Uncore energy (PP1)
cat /sys/class/powercap/intel-rapl/intel-rapl:0/intel-rapl:0:1/energy_uj

# DRAM energy (server only)
cat /sys/class/powercap/intel-rapl/intel-rapl:0/intel-rapl:0:2/energy_uj
```

### Measure Workload Energy

```bash
perf stat -e power/energy-pkg/,power/energy-cores/,power/energy-ram/ ./workload
```

### Set Power Cap

```bash
# Long-term (PL1) to 45W
echo 45000000 | sudo tee /sys/class/powercap/intel-rapl/intel-rapl:0/constraint_0_power_limit_uw

# Short-term (PL2) to 65W
echo 65000000 | sudo tee /sys/class/powercap/intel-rapl/intel-rapl:0/constraint_1_power_limit_uw
```

### Real-time Monitoring

```bash
# Per-core frequency, C-state residency, temperature, power
sudo turbostat -i 1

# Key columns: Bzy_MHz (actual freq), Pkg_Watt, RAM_Watt, PKG_%, Core_%
```

### Handle RAPL Counter Wraparound

```python
def read_energy():
    with open('/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj') as f:
        return int(f.read().strip())

def measure_energy_ms():
    start = read_energy()
    # ... run workload ...
    end = read_energy()
    MAX = 2**32
    return (end - start) % MAX  # handles wraparound
```

## Frequency Governors

| Governor | Strategy | Transition Speed | Best For |
|-----------|----------|-------------------|----------|
| `performance` | Always max frequency | None (fixed) | Benchmarking |
| `powersave` | Always min frequency | None (fixed) | Max efficiency / idle |
| `schedutil` | Scheduler-driven DVFS | <1 ms | Modern default |
| `ondemand` | Sample-based scaling | ~10 ms | Legacy general use |
| `conservative` | Slow ramp up | ~100 ms | Battery life |

### Set Governor

```bash
# Check current
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor

# Set to performance (for benchmarking)
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Set to powersave (for efficiency)
echo powersave | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Check available frequencies
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_available_frequencies

# Pin to specific frequency
sudo cpufreq-set -c 0 -f 3.0GHz
```

## EPP (Energy Performance Preference)

| Value | Meaning | Use Case |
|-------|---------|----------|
| 0 | Max performance | Latency-critical |
| 64 | Balance performance | General server |
| 128 | Balance power | Default |
| 192 | Balance efficiency | Throughput workloads |
| 255 | Max efficiency | Background tasks |

Set: `echo 0 | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference`

## Benchmarking Power Checklist

```bash
# 1. Pin governor to performance
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# 2. Disable turbo for consistent results
echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo

# 3. Pre-heat CPU (60+ seconds)
stress-ng --cpu 0 --timeout 60

# 4. Pin to specific core
taskset -c 0 ./workload

# 5. Monitor during run
sudo turbostat -i 1 &

# 6. Measure energy
perf stat -e power/energy-pkg/,power/energy-cores/,instructions,cycles ./workload

# 7. Re-enable turbo after benchmarking
echo 0 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo
```

## DVFS Power Math

```
P = α · C · V² · f

Example (relative units):
  f=1GHz, V=0.8V → P = 1×1×0.64×1 = 0.64  (baseline)
  f=2GHz, V=0.9V → P = 1×1×0.81×2 = 1.62  (2.5x power for 2x freq)
  f=4GHz, V=1.1V → P = 1×1×1.21×4 = 4.84  (7.6x power for 4x freq)
  f=5GHz, V=1.3V → P = 1×1×1.69×5 = 8.45  (13.2x power for 5x freq)

Rule of thumb: top 25% of frequency costs 2-3x the power of the bottom 75%.
```

## Cloud VM Power Behavior

| Factor | Bare Metal | Cloud VM |
|--------|-----------|----------|
| Governor control | Full | None |
| Turbo toggle | Available | Unavailable |
| Turbo sustainability | Predictable | Varies with neighbor load |
| RAPL access | Yes | No |
| Frequency measurement | `turbostat` | Unavailable |
| Performance variance | < 5% | 20–30% |
| C-state control | Full | None |

Mitigation: Use dedicated hosts, long test durations (>60s), report p50/p95/p99.