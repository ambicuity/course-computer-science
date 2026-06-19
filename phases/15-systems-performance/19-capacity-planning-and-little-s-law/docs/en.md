# Capacity Planning and Little's Law

> The math that explains why your system falls apart at 80% CPU — and how to size everything correctly.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 15 lessons 01–18
**Time:** ~60 minutes

## Learning Objectives

- State Little's Law (L = λW) and explain what each variable means.
- Apply Little's Law to real systems: request queues, thread pools, database connection pools.
- Use the utilization law and understand why latency explodes near 100% utilization.
- Size thread pools and connection pools using proven formulas.
- Distinguish Amdahl's Law from Little's Law and know when to use each.
- Plan capacity with headroom targets and autoscaling triggers.

## The Problem

You deploy a service that handles 1,000 requests/sec. Response time is 50 ms. Traffic doubles. You double the servers. But response time *triples*. What happened?

The answer lives in queueing theory — specifically Little's Law and the utilization law. Without these tools you're guessing at capacity, oversizing by 3× "just in case" or undersizing and watching p99 latency spike during peak hours.

This lesson gives you the mathematical framework to answer "how many servers/threads/connections do I need?" with confidence instead of gut feelings.

## The Concept

### Little's Law: L = λW

Little's Law is one of the few results in queueing theory that holds for **any** system with a steady state — no assumptions about arrival distributions, service time distributions, or scheduling discipline needed.

| Variable | Meaning | Units |
|----------|---------|-------|
| **L** | Average number of items **in the system** (being processed + waiting) | items |
| **λ** | Average **arrival rate** | items / second |
| **W** | Average **time in system** (wait + service) | seconds |

The law says: **the average number of items in the system equals the arrival rate times the average time each item spends in the system.**

#### Intuition

If 10 people per hour enter a restaurant (λ = 10/hr) and each person spends 30 minutes inside (W = 0.5 hr), then on average there are 10 × 0.5 = 5 people inside. That's it. No distributional assumptions.

#### Worked Example: Web Server

- Arrival rate: λ = 200 requests/sec
- Average response time: W = 0.05 sec (50 ms)
- Average requests in system: L = 200 × 0.05 = **10 requests**

You need at least 10 concurrent processing slots. If your server has 8 worker threads, 2 requests are always queued.

### The Utilization Law: ρ = λ/μ

For a single-server queue (M/M/1):

| Variable | Meaning |
|----------|---------|
| **ρ** | Server utilization (0 to 1) |
| **λ** | Arrival rate |
| **μ** | Service rate (1/average service time) |

Utilization ρ = λ/μ. If λ = 100 req/sec and μ = 150 req/sec, then ρ = 0.667 (66.7% busy).

#### Why Latency Explodes Near 100%

For an M/M/1 queue, average time in system:

**W = 1 / (μ - λ) = S / (1 - ρ)**

Where S = 1/μ is the average service time.

| Utilization ρ | Latency Multiplier (vs idle service time) |
|---------------|--------------------------------------------|
| 50% | 2.0× |
| 70% | 3.3× |
| 80% | 5.0× |
| 90% | 10.0× |
| 95% | 20.0× |
| 99% | 100.0× |

At 90% utilization, a request that takes 10 ms at idle now takes 100 ms on average. The last 10% of capacity costs you 10× in latency.

**This is why the industry targets 70% utilization for steady-state workloads.** Below 70%, you're wasting money. Above 80%, you're one traffic spike away from cascading failure.

### M/M/1 Queue Formulas

The simplest meaningful queueing model:

- **Arrivals**: Poisson process (rate λ)
- **Service**: Exponential distribution (rate μ)
- **Servers**: 1

Key results:
- Utilization: ρ = λ/μ
- Average number in system: L = ρ/(1 - ρ)
- Average number in queue: L_q = ρ²/(1 - ρ)
- Average time in system: W = 1/(μ - λ)
- Average wait in queue: W_q = ρ/(μ - λ)
- Probability of zero jobs: P₀ = 1 - ρ

### M/M/c Queue (Multiple Servers)

When you have c identical servers:

- Utilization per server: ρ = λ/(cμ)
- System utilization: U = cρ = λ/μ
- The Erlang C formula gives P(queueing), the probability a request must wait:

**C(c, ρ) = (cρ)ᶜ / (c! × (1 - ρ)) / [Σₖ₌₀ᶜ⁻¹ (cρ)ᵏ/k! + (cρ)ᶜ/(c!(1 - ρ))]**

The average number in queue: L_q = C(c, ρ) × ρ/(1 - ρ)

### Connection Pool Sizing with Little's Law

Database connection pool sizing is a direct application:

- λ = peak request rate needing DB access
- W = average DB query time (connection held)
- L = λ × W = required pool size

Example:
- λ = 500 queries/sec
- W = 0.02 sec (20 ms avg query time)
- Pool size ≥ L = 500 × 0.02 = **10 connections**

Add 20-30% headroom → 12-13 connections.

**Key insight**: pool size depends on *throughput × latency*, not on the number of app instances.

### Thread Pool Sizing

Two regimes:

**CPU-bound tasks** (compression, hashing, image processing):
```
threads = number_of_CPU_cores
```
More threads than cores just adds context-switching overhead.

**I/O-bound tasks** (HTTP calls, DB queries, file reads):
```
threads = N_cores × (1 + W_io / C_cpu)
```
Where:
- W_io = average I/O wait time
- C_cpu = average CPU time per task

From Little's Law: if each thread handles 1 request, and R = requests/sec, then:

L = R × W → threads_needed = R × W_per_request

Example:
- 8 cores, task uses 5 ms CPU + 45 ms I/O wall-clock
- threads = 8 × (1 + 45/5) = 8 × 10 = 80 threads

### Autoscaling Triggers

When should you add capacity?

- **Scale-up trigger**: sustained utilization > 70-75% over 2-5 minute window
- **Scale-down trigger**: sustained utilization < 30% over 10-15 minute window
- **Emergency trigger**: utilization > 90% or p99 latency > 2× baseline

The asymmetry (fast scale-up, slow scale-down) prevents thrashing.

### Headroom Planning

- **Steady-state target**: 50-70% utilization (leaves room for traffic variance)
- **Burst capacity**: 30-50% headroom above peak for 5-15 minute traffic spikes
- **The "N+1" rule**: for every N servers needed at target utilization, run N+1 minimum for fault tolerance

Rule of thumb: if you need 8 servers at 70% utilization, run 9-10 to handle 1 failure and moderate traffic spikes.

### Queueing Delay vs Processing Time

Total response time = Queue wait time + Service time

At low utilization, service time dominates. At high utilization, queue wait dominates. The crossover typically happens around 70-75% utilization where queue wait ≈ service time.

This means: if you're at 70% utilization and see response times double, the problem isn't processing — it's queuing. Add capacity or shed load.

### Measuring Arrival Rate and Service Time in Production

**Arrival rate (λ)**:
- Count incoming requests per second over 1-minute sliding windows
- Use a histogram: p50/p90/p99 of per-second arrival rates
- Watch for burst patterns (10× average for 30 seconds is common)

**Service time (1/μ)**:
- Measure actual processing time, not including queue wait
- Use histograms, not averages (tail latency matters)
- Subtract queue wait from total response time if you can't measure directly

**Utilization (ρ)**:
- CPU utilization is a proxy but not the whole story
- Thread pool utilization: fraction of threads busy
- Connection pool utilization: fraction of connections checked out

### Amdahl's Law vs Little's Law

| | Amdahl's Law | Little's Law |
|---|---|---|
| **Question** | How much speedup from parallelization? | How much capacity do I need? |
| **Inputs** | Serial fraction, parallel fraction | Arrival rate, service time |
| **Output** | Maximum speedup | Required resources |
| **Scope** | Single task optimization | System-level capacity |
| **Assumption** | Fixed problem size | Steady-state flow |

Use Amdahl's Law when asking "how much faster can I make *this computation*?" Use Little's Law when asking "how many *servers/threads/pools* do I need to handle *this load*?"

They answer fundamentally different questions. Confusing them leads to either over-provisioning (treating a throughput problem as a parallelism problem) or under-provisioning (treating a capacity problem as a speedup problem).

## Build It

### Step 1: Minimal Little's Law Calculator

The smallest correct version — plug in λ and W, get L:

```python
def littles_law(arrival_rate, avg_time_in_system):
    return arrival_rate * avg_time_in_system
```

### Step 2: Full Implementation

The realistic version handles queueing models, pool sizing, and latency-vs-utilization curves. See `code/main.py`.

## Use It

### Production Tools

- **PostgreSQL** uses a connection pooler (pgbouncer) exactly sized by Little's Law: `max_connections = max_queries_per_sec × avg_query_time`.
- **Tomcat** default thread pool: 200 threads (tuned via `maxThreads`), sized for I/O-bound web serving using the N × (1 + W/C) formula.
- **Envoy** circuit breakers use utilization thresholds (5xx rate > threshold → open circuit), directly implementing the "shed load before queue explodes" principle from the utilization law.

### Read the Source

- **Linux CFS scheduler** (`kernel/sched/fair.c`): uses load averages that are derived from Little's Law — the "load average" numbers (1/5/15 min) are literally L in L = λW, measured over different windows.
- **Java ThreadPoolExecutor** (`java/util/concurrent/ThreadPoolExecutor.java`): the core/max pool parameters implement the thread sizing formula. The work queue is the queue buffer whose growth is predicted by L_q = ρ²/(1-ρ).

## Ship It

The reusable reference card lives in `outputs/capacity_reference.md`:

- Little's Law formulas
- M/M/1 and M/M/c key equations
- Pool sizing cheat sheet
- Utilization thresholds and their latency implications

## Exercises

1. **Easy** — Given λ = 500 req/sec and W = 20 ms, calculate L. How many worker threads do you need?
2. **Medium** — Your M/M/1 system has λ = 80 and μ = 100. Calculate the average number in queue, average wait time, and probability the system is empty. At what λ does average latency exceed 5× the service time?
3. **Hard** — Model a system with two service stages (web server → database) where each stage is an M/M/c queue. Derive the end-to-end latency formula and write a simulator that validates it.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Little's Law | "The throughput formula" | L = λW — average items in system equals arrival rate times average time in system, holds for *any* stable system |
| Utilization | "CPU usage" | Fraction of time the server is busy: ρ = λ/μ, the single most dangerous metric when it approaches 1 |
| M/M/1 | "The simple queue model" | Single-server queue with Poisson arrivals and exponential service times — simplest model that still shows latency explosion |
| Erlang C | "The call center formula" | Probability a request must wait in an M/M/c queue with c servers — foundational for sizing server pools |
| Headroom | "Extra capacity" | The utilization gap between your target (70%) and 100%; without it, any burst causes a latency spike |
| Arrival rate | "Requests per second" | λ — the rate at which work arrives; spikes in λ (not increases in service time) cause most capacity incidents |

## Further Reading

- John D. C. Little, "A Proof for the Queuing Formula: L = λW" — *Operations Research* 9(3), 1961. The original proof.
- Leonard Kleinrock, *Queueing Systems, Volume 1: Theory* — the standard textbook on queueing theory.
- Baron Schwartz, *Efficient MySQL Performance* — Chapter on connection pool sizing with Little's Law.
- Martin Thompson, "Smart Thread Pool Sizing" — practical guide to thread pool sizing using the N × (1 + W/C) formula.
- USPS Queueing Theory Applied to Web Systems — practical applications of M/M/c to capacity planning.