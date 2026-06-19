# Tail Latency & Reliability Engineering — Reference Card

_Quick reference for reliability patterns in distributed systems._

---

## Latency Percentiles

| Percentile | Meaning | Use Case |
|-------------|---------|----------|
| **p50** (median) | Half of requests are faster | Baseline monitoring, not SLAs |
| **p90** | 90% faster than this | Internal health checks |
| **p95** | 95% faster than this | Hedge delay selection |
| **p99** | 99% faster than this | Standard SLA target |
| **p999** | 99.9% faster than this | Large-scale systems (billions of requests) |

**Key insight**: In a fan-out of N parallel calls, P(at least one exceeds pX) = 1 - (1-X/100)^N.

| Fan-out | P(any call hits p99) | P(any call hits p999) |
|---------|----------------------|------------------------|
| 1 | 1% | 0.1% |
| 10 | 9.6% | 1.0% |
| 100 | **63.4%** | 9.5% |
| 1,000 | ~100% | 63.2% |

---

## Jeff Dean's Latency Numbers (2009)

| Operation | Latency |
|-----------|---------|
| L1 cache reference | 0.5 ns |
| Branch mispredict | 5 ns |
| L2 cache reference | 7 ns |
| Mutex lock/unlock | 25 ns |
| Main memory reference | 100 ns |
| Compress 1 KB (Snappy) | 10 µs |
| Send 1 KB over 1 Gbps | 10 µs |
| Read 4 KB randomly from SSD | 100 µs |
| Read 1 MB sequentially from SSD | 500 µs |
| Data center round trip | 500 µs |
| Read 1 MB sequentially from disk | 10 ms |
| Disk seek | 10 ms |
| Packet CA→Netherlands | 150 ms |

**Key ratios**: L1:L2:Memory:SSD:Disk:Network = 1:14:200:200K:20M:300K

---

## Hedged Request Pattern

```
        ┌──────────┐
 ──→ ───│ Backend A │ (primary)
  │      └──────────┘
  │          │ (slow)
  │      wait hedgeDelay (e.g., p95)
  │          ▼
Client──→ ┌──────────┐
  │      │ Backend B │ (hedge)
  │      └──────────┘
  │          │ (fast) ✓
  │          │
  └── take first, cancel other
```

**Parameters**:
| Parameter | Recommended | Why |
|-----------|-------------|-----|
| Hedge delay | p95 latency | Catches slow primaries without doubling load |
| Max hedged requests | 1-2 | More hedging = more load, diminishing returns |
| Cancellation | Immediate on first response | Free the backend resource |

**Load overhead**: Typically 5-15% at p95 hedge delay vs 100% for immediate dual-send.

**When to use**: High fan-out, independent backends, bimodal latency.
**When to avoid**: Low fan-out, shared resource pools, correlated failures.

---

## Circuit Breaker States

```
                    failure threshold
                    crossed
          ┌──────────────────────────┐
          │                          ▼
    ┌─────────┐              ┌──────────┐
    │ Closed  │              │   Open   │
    │ (normal)│              │ (fail    │
    └─────────┘              │  fast)   │
          ▲                  └──────────┘
          │                      │
          │  timeout expires     │
          │                      │
          │    ┌──────────┐      │
          └────│Half-Open │◄─────┘
   success │    │ (probe)  │
          └──────────┘
```

| State | Requests | On Success | On Failure |
|-------|----------|------------|------------|
| **Closed** | Pass through | Reset failure counter | Increment failure counter |
| **Open** | Fail fast (instant error) | N/A | N/A |
| **Half-Open** | Allow limited probes | → Closed | → Open |

**Tuning knobs**:
- **Failure threshold**: 5 failures in 10s (adjust to your error budget)
- **Open timeout**: 30s (too short = flapping, too long = slow recovery)
- **Half-open max requests**: 1 (conservative) to 5 (aggressive)

---

## Timeout Guidelines

| Strategy | How | Pros | Cons |
|----------|-----|------|------|
| **Fixed** | Set timeout = p99 + buffer | Simple, predictable | Doesn't adapt to load |
| **Adaptive** | Track recent p99, set timeout = 2× p99 | Adapts to conditions | Can be too aggressive during spikes |
| **Deadline propagation** | Pass absolute deadline through RPC chain | Cascading cancellation | Requires framework support |

**Rule of thumb**:
- Fixed timeout: 5-10× p99 (safety net)
- Hedge delay: p95 (minimize unnecessary sends)
- Adaptive timeout window: 30-60s of recent data
- Monitor: If >1% of requests hit timeout, it's too tight

---

## Bulkheading Patterns

| Pattern | Isolation | Overhead | Best For |
|---------|-----------|----------|----------|
| **Thread pool per backend** | Medium | Medium | Blocking I/O |
| **Connection pool per downstream** | Medium | Low | Network calls |
| **Fixed capacity partitions** | High | Medium | Mixed workloads |
| **Semaphore per tenant** | Medium | Low | Multi-tenant SaaS |

---

## Retry Budgets

| Parameter | Recommended | Notes |
|-----------|-------------|-------|
| Max retries | 1 (user-facing), 2 (background) | More retries = more tail latency |
| Retry budget | 10% of total requests | Prevent retry storms |
| Backoff | Exponential: 1s, 2s, 4s | Add jitter: random 0-1s |
| Idempotency | Only retry GET/PUT/DELETE | Never retry POST without idempotency key |

**Retry budget formula**: `retries_allowed = total_requests × budget_fraction × (1 + backoff_factor)`

---

## Key Equations

**Fan-out tail probability**: P(at least one ≥ pX) = 1 - (1 - X/100)^N

**Hedged tail probability**: P(both ≥ pX) = (X/100)^2 (with 2 backends)

**M/M/1 p99 wait time**: W_p99 ≈ (service_time) × ρ/(1-ρ) × percentile_of_exponential(ρ/(1-ρ))
where ρ = utilization. At 90% utilization, p99 ≈ 9× service time.

**Adaptive timeout**: timeout(t) = 2 × EWMA(p99, t)

---

## Production References

- **Google**: "The Tail at Scale" (Dean & Barroso, CACM 2013) — hedged requests at Google scale
- **Envoy**: Outlier detection + circuit breaking (`envoy.yaml` circuit_breakers, outlier_detection)
- **resilience4j**: CircuitBreaker, RateLimiter, Retry — Java microservice standard
- **gRPC**: Retry throttle (`retry_throttle` in channel args) — token bucket for retry budget
- **Go**: `context.WithTimeout` / `context.WithDeadline` — deadline propagation
- **Rust**: `tokio::time::timeout` — deadline enforcement