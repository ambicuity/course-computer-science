# Capacity Planning & Little's Law — Quick Reference

## Core Formulas

### Little's Law
```
L = λ × W
```
| Symbol | Meaning | Units |
|--------|---------|-------|
| L | Avg # items in system | count |
| λ | Arrival rate | items/sec |
| W | Avg time in system | seconds |

**Rearrangements:**
- λ = L / W  (given items and time, find rate)
- W = L / λ  (given items and rate, find time)

---

## M/M/1 Queue

```
ρ = λ / μ           utilization
L = ρ / (1 - ρ)     avg in system
L_q = ρ² / (1 - ρ)  avg in queue
W = 1 / (μ - λ)     avg time in system
W_q = ρ / (μ - λ)   avg wait in queue
P₀ = 1 - ρ          prob system empty
```

**⚠ System unstable when λ ≥ μ**

---

## M/M/c Queue (c servers)

```
ρ = λ / (c × μ)              per-server utilization
C(c, ρ) = Erlang C formula    prob(must wait)
L_q = C(c,ρ) × ρ / (1 - ρ)   avg in queue
```

---

## Latency vs Utilization (M/M/1)

```
W = S / (1 - ρ)     where S = 1/μ = avg service time
```

| Utilization | Latency Multiplier |
|-------------|-------------------|
| 50% | 2.0× |
| 70% | 3.3× |
| 80% | 5.0× |
| 90% | 10.0× |
| 95% | 20.0× |
| 99% | 100.0× |

**Target: 50–70% steady-state, 80% max before scaling**

---

## Pool Sizing

### Connection Pool
```
pool_size = peak_qps × avg_query_time
add 20–30% headroom
```

### Thread Pool — CPU-Bound
```
threads = N_cores
```

### Thread Pool — I/O-Bound
```
threads = N_cores × (1 + W_io / C_cpu)
```
- W_io = avg I/O wait time
- C_cpu = avg CPU time per task

---

## Utilization Thresholds

| Range | Status | Action |
|-------|--------|--------|
| 0–30% | Over-provisioned | Scale down |
| 30–50% | Healthy, high headroom | Monitor |
| 50–70% | Optimal steady-state | Normal ops |
| 70–80% | Approaching limit | Scale-up trigger |
| 80–90% | Danger zone | Emergency scale |
| >90% | Latency explosion imminent | Shed load NOW |

---

## Autoscaling Rules

- **Scale up**: sustained util > 70–75% over 2–5 min
- **Scale down**: sustained util < 30% over 10–15 min
- **Emergency**: util > 90% or p99 latency > 2× baseline

---

## Amdahl's Law vs Little's Law

| | Amdahl's | Little's |
|---|---|---|
| **Question** | Max speedup? | Capacity needed? |
| **Inputs** | Serial/parallel fraction | Arrival rate, service time |
| **Output** | Speedup limit | Resource count |
| **Use when** | Optimizing one computation | Sizing a running system |

---

## Quick Sizing Checklist

1. Measure λ (arrival rate) and S (service time) in production
2. Compute ρ = λ × S (utilization per server)
3. If ρ > 0.7, add servers: c ≥ ⌈λ × S / 0.7⌉
4. Size connection pools: pool ≥ λ × S × 1.25
5. Size thread pools per formula above
6. Verify with latency curve: check W = S/(1−ρ) at target