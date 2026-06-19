# Reliability Engineering — Tail Latency, Hedging

> An individual request that falls in the 99th percentile can drag your whole system's perceived latency down — not because most requests are slow, but because enough of them are to matter.

**Type:** Learn
**Languages:** Go, Rust
**Prerequisites:** Phase 15 lessons 01–17
**Time:** ~60 minutes

## Learning Objectives

- Explain why tail latency (p99, p999) matters more than average latency for distributed systems.
- Describe why latency distributions are heavy-tailed, not normal, and the implications.
- Identify sources of tail latency: GC pauses, cache misses, network jitter, lock contention.
- Implement hedged requests: send  duplicates, take the first response.
- Apply deadline-based cancellation to bound resource usage per request.
- Use circuit breakers and bulkheading to prevent tail-amplification cascades.
- Choose timeout strategies (adaptive vs fixed) and reason about retry budgets.

## The Problem

You run a service that responds in a median of 10 ms. Your p99 is 1 second. A user makes a single request — 99 times out of 100 it finishes fast. But that 1-in-100 slow response? They notice. Now consider a fan-out: the user's single action triggers 100 parallel backend calls. The probability at least one of those 100 calls hits the p99 is `1 - 0.99^100 ≈ 63%`. The user almost certainly sees a slow response. This is the **latency lottery problem**: in distributed systems, tails compound.

This lesson sits in **Phase 15 — Systems Programming & Performance**. Without understanding tail latency, you cannot honestly measure, tune, or reliability-engineer any system that fans out across multiple backends. The phase capstone — profile-guided optimization — requires you to know what "slow" actually means, not just what the average says.

## The Concept

### Why Tail Latency Matters

Most monitoring dashboards show *average* latency. Averages lie. If 99 requests finish in 1 ms and 1 request takes 10 seconds, the average is ~100 ms — which tells you nothing about the user who waited 10 seconds. That user is real, and in a fan-out architecture they're *typical*, not exceptional.

**Percentiles tell the truth:**

| Metric | What it captures | Weakness |
|--------|----------------|----------|
| p50 (median) | Typical experience | Hides the slow tail entirely |
| p90 | 90% of users' experience | Misses the worst 10% |
| p99 | 99% of users' experience | The standard for SLAs |
| p999 | 99.9% of users' experience | Critical for large-scale systems |
| Max | Worst case ever | One outlier dominates, noisy |

Google's Jeff Dean famously showed that at Google scale, a p99 latency of 100 ms means millions of requests per day are slow. When your system serves billions of requests, even p999 matters.

### Latency Distributions: Not Normal

Latency is **not** normally distributed. It's right-skewed and heavy-tailed:

- **Normal distribution**: symmetric, thin tails, predictable. Mean ≈ median.
- **Latency distribution**: asymmetric, heavy right tail. Mean >> median. The "tail" contains significant probability mass.

Why heavy-tailed? Because latency has a hard floor (zero) but no ceiling. A GC pause can add seconds. A cache miss that kicks off a disk read is 1000× slower than a cache hit. A network retransmission can add hundreds of milliseconds. These aren't rare outliers — they're structural features of the system.

**Latency numbers every engineer should know** (Jeff Dean, 2009, updated):

| Operation | Approx. Latency |
|-----------|-----------------|
| L1 cache reference | 0.5 ns |
| Branch mispredict | 5 ns |
| L2 cache reference | 7 ns |
| Mutex lock/unlock | 25 ns |
| Main memory reference | 100 ns |
| Compress 1 KB with Snappy | 10 μs |
| Send 1 KB over 1 Gbps network | 10 μs |
| Read 4 KB randomly from SSD | 100 μs |
| Read 1 MB sequentially from SSD | 500 μs |
| Round trip within data center | 500 μs |
| Read 1 MB sequentially from disk | 10 ms |
| Disk seek | 10 ms |
| Send packet CA→Netherlands | 150 ms |

Notice the orders of magnitude: L1 (0.5 ns) vs disk seek (10 ms) is a 20,000,000× difference. A single cache miss or disk read swamps everything else in the distribution.

### Sources of Tail Latency

**Garbage Collection Pauses**: Stop-the-world GC can pause a process for 100 ms to seconds. Go's concurrent GC helps but doesn't eliminate pauses. JVM systems are notorious for GC-induced latency spikes.

**Cache Misses**: L1 vs main memory is ~200× latency difference. A hot loop that misses L2 once can blow out a request's latency by an order of magnitude. At the application level, a memcache miss that falls through to database is 1000× slower.

**Network Jitter**: TCP retransmissions, congested switch buffers, noisy neighbors on shared links. A packet that's dropped and retransmitted adds ~200 ms (retransmit timeout). In data centers, micro-bursts cause transient queuing delays.

**Lock Contention**: When many goroutines/threads contend on a mutex, the unlucky ones queue. Contention latency is unpredictable: it depends on how many others arrived first. Under load, a long critical section amplifies tail latency for everyone waiting.

**Background Load**: A compaction in LSM-tree storage, a log rotation, a metrics flush — background work steals CPU, cache, and I/O bandwidth from foreground requests.

**Queueing Effects**: In M/M/1 queues, the 99th percentile wait time grows as `ρ/(1-ρ)` where ρ is utilization. At 90% utilization, p99 wait is ~9× the service time. At 99% utilization, it's ~99×.

### Hedged Requests

The core idea: **don't wait for a slow response — send a second request and take whichever finishes first.**

```
                    ┌──────────┐
         Request ──→│ Backend A │── slow
        ╱           └──────────┘
Client ──
        ╲           ┌──────────┐
         Hedged ───→│ Backend B │── fast ✓
                    └──────────┘
```

**Algorithm**:
1. Send primary request to backend A.
2. After a *hedging delay* (e.g., p95 latency), if A hasn't responded, send a second request to backend B.
3. Take the first response (from either A or B).
4. Cancel the outstanding request.

**Why hedging delay matters**: If you send both requests simultaneously, you double your load. With a hedging delay, you only send the second request when the first is unusually slow. If p95 is 20 ms and your primary responds in 15 ms (faster than p95), no hedge is sent. Load increase is small: typically 5–15% rather than 100%.

**When hedging helps**:
- High fan-out (aggregating many backend calls) — tails compound, hedging clips them.
- Bimodal latency (fast path + slow path) — hedging nearly always catches the fast one.
- Independent backends — if backends share failure domains, hedging amplifies correlated failures.

**When hedging hurts**:
- Low fan-out (1 backend call) — the tail probability is low, not worth the complexity.
- Shared resource pools — hedged requests compete with primaries for the same resources.
- Correlated slow-downs — if all backends slow down together, hedging just doubles load for no gain.

**Google's approach**: Jeff Dean and Barroso's "The Tail at Scale" (CACM 2013) describes Google's use of hedged and "tied" requests. In their system:
- Requests are sent to multiple replicas.
- If any replica is slow, the client sends a "tied request" to another replica.
- The original slow replica is told to cancel (its result becomes a cache warm-up).
- At Google scale, this reduced p99 latency by 30–50% while increasing total load by only 5–10%.

### Deadline-Based Cancellation

Every request should carry a **deadline**: the absolute time by which a response is needed.

```
deadline = now() + timeout
```

Benefits:
- Prevents zombie requests from consuming resources after the caller has given up.
- Enables cascading cancellation: if the caller's deadline expires, it cancels downstream calls.
- Makes timeout propagation explicit rather than relying on each hop's independent timeout.

**Go context**: `context.WithTimeout` and `context.WithDeadline` encode this. Every function that does I/O should accept a `ctx context.Context` and respect its deadline.

**Rust tokio**: Use `tokio::time::timeout` or pass deadlines through and check `deadline < Instant::now()`.

### Bounding Resources Per Request

Without per-request resource bounds, a single slow request can consume unbounded memory (buffering responses), CPU (retrying), or connections (holding connections open).

**Strategies**:
- **Max response size**: Cap how much data you'll read. If the response exceeds N bytes, truncate or error.
- **Max in-flight per client**: A client should have at most K concurrent requests to a given backend. Queueing excess requests prevents overload.
- **Connection pools with limits**: Each client holds a bounded pool of connections. When all are in use, new requests wait or fail fast rather than opening unlimited connections.
- **Memory budgets**: Allocate a fixed memory budget per request. Streaming parsers enforce this naturally.

### Circuit Breakers

A circuit breaker prevents cascading failures by stopping requests to a failing backend:

```
State machine:
  Closed ──(failure threshold crossed)──→ Open
  Open ──(timeout expires)──→ Half-Open
  Half-Open ──(success)──→ Closed
  Half-Open ──(failure)──→ Open
```

| State | Behavior |
|-------|----------|
| **Closed** | Normal operation. Track failures. |
| **Open** | Fail fast. Don't send requests. Start a timer. |
| **Half-Open** | Allow one probe request. If it succeeds → closed. If it fails → open. |

**Why circuit breakers help tail latency**: When a backend is degraded, its latency spikes. Without a breaker, every request still tries that backend, adding slow responses. With a breaker, the system fails fast (typically in <1 ms) and routes around the problem.

**Tuning**: Set the failure threshold (e.g., 5 failures in 10 seconds) and the open timeout (e.g., 30 seconds). Too aggressive = false positives. Too lenient = slow to protect.

### Bulkheading

Bulkheading isolates failure domains so one slow component can't take down the whole system:

- **Thread pool per backend**: If backend A is slow, its thread pool fills up, but backend B's pool is unaffected.
- **Connection pool per downstream**: Same principle for connections.
- **Resource partitioning**: Allocate fixed capacity slices. A burst to service A can only consume A's slice, not B's.

Without bulkheading, a slow backend consumes shared threads/connections, starving healthy backends. This converts a single slow backend into a system-wide outage — tail latency amplifier.

### Retry Budgets

Retries are essential for transient failures but dangerous for tail latency:

- **Retry storm**: A retry that fails triggers more retries. Under load, this amplifies traffic by 2×, 3×, 4×.
- **Retry budget**: Allow at most N retries per second across all clients. When the budget is exhausted, fail rather than retry.
- **Exponential backoff + jitter**: Space retries: 1 s, 2 s, 4 s. Add random jitter (0–1 s) to prevent thundering herds.
- **Non-idempotent operations**: Don't retry POSTs or mutations unless you can handle duplicates.

**Rule of thumb**: Maximum 1 retry for user-facing requests. At most 2 retries for background jobs. Always with backoff.

### The Latency Lottery

Consider a web page that makes 100 parallel API calls to render:

```
P(any call hits p99) = 1 - (0.99)^100 ≈ 63%
P(any call hits p999) = 1 - (0.999)^100 ≈ 10%
```

At the p99 level, **63% of page loads** experience a slow call. The user's experience is determined by the *worst* of 100 calls, not the average. This is why median latency is insufficient for distributed systems — what matters is the tail, because tails compound across fan-outs.

With hedging:
```
P(both primary AND hedge hit p99) = (0.01)^2 = 0.01%
```
The combined p99 drops from ~1% to ~0.01% — a 100× improvement in tail probability.

### Timeout Selection: Adaptive vs Fixed

**Fixed timeouts**: Simple, predictable. Set p99 + 2× standard deviation as the timeout. Downside: doesn't adapt to changing conditions. A timeout that's right at p95 load is wrong at peak.

**Adaptive timeouts**: Track recent latency (e.g., exponential moving average of p99) and set timeout to `2× current_p99`. Adapts to load changes. Downside: can be too aggressive during transient spikes (cutting off requests that would have succeeded).

**Practical advice**:
- Use fixed timeouts as safety nets (upper bounds that should never be hit in normal operation).
- Use adaptive timeouts for hedging delay decisions.
- Set the fixed timeout at 5–10× p99 to handle rare but legitimate slow responses.
- Monitor timeout rates. If >1% of requests hit the timeout, it's too tight.

## Build It

### Step 1: Minimal Version — Hedged Request

```go
func hedgedRequest(ctx context.Context, backends []Backend, req Request, hedgeDelay time.Duration) (Response, error) {
    type result struct {
        resp Response
        err  error
    }
    ch := make(chan result, len(backends))

    // Send primary request
    go func() {
        resp, err := backends[0].Call(ctx, req)
        ch <- result{resp, err}
    }()

    // Wait for hedgeDelay, then send second request
    timer := time.NewTimer(hedgeDelay)
    hedgeSent := false
    select {
    case r := <-ch:
        return r.resp, r.err
    case <-timer.C:
        if len(backends) > 1 {
            go func() {
                resp, err := backends[1].Call(ctx, req)
                ch <- result{resp, err}
            }()
            hedgeSent = true
        }
    }

    // Wait for first response
    r := <-ch
    if hedgeSent {
        // Drain the other response
        go func() { <-ch }()
    }
    return r.resp, r.err
}
```

### Step 2: Realistic Version

Add circuit breaker, deadline propagation, retry budget, and metrics collection. See the full Go implementation in `code/main.go`.

## Use It

### Production Hedging: Google

Google's production RPC system (gRPC descendant of Stubby) implements:
- **Hedged RPCs**: Client-side hedging with configurable delay and max hedged requests.
- **Per-RPC deadlines**: Every RPC carries a deadline propagated to downstream servers.
- **Adaptive throttling**: If the percentage of rejected requests exceeds a threshold, the client reduces its send rate.

### Production Circuit Breaking: resilience4j / envoy

**resilience4j** (Java): Circuit breaker with configurable sliding window (count-based or time-based), failure rate threshold, slow call duration threshold, and slow call rate threshold.

**Envoy Proxy**: Outlier detection (ejecting unhealthy hosts from load balancing) and circuit breaking (max connections, max pending requests, max retries per host).

### Production Retry Budgets: gRPC

gRPC's retry mechanism includes:
- **Max retry attempts**: Configurable per-method.
- **Retry throttle**: A token bucket that limits retries to a fraction of total requests (default 10%).
- **Transparent retries**: Automatically retry idempotent requests on certain errors.

## Read the Source

- **Envoy outlier detection**: `source/common/http/outlier_detection_impl.cc` in the Envoy repo. Look at how it tracks consecutive failures and ejects hosts.
- **gRPC retry throttle**: `src/core/lib/channel/retry_throttle.cc` in the gRPC repo. The token bucket that limits retry percentage.
- **Go context deadline propagation**: `src/context/context.go` in the Go repo. See how `WithDeadline` chains parent and child contexts.

## Ship It

The reusable artifact for this lesson is in `outputs/`:

- **`tail_latency_reference.md`** — A reference card with latency percentiles, hedging patterns, circuit breaker states, and timeout guidelines.

## Exercises

1. **Easy** — Run both the Go and Rust programs with different seed values. Observe how p99 changes across runs. What happens when you increase the number of backends?

2. **Medium** — Modify the hedging implementation to use an adaptive hedge delay: instead of a fixed delay, track the p95 latency of recent requests and set the hedge delay to p95. Measure the effect on both p99 latency and total request volume.

3. **Hard** — Implement a full "tail at scale" simulator: a system with N fan-out levels, each with M backends. At each level, apply hedging and circuit breaking. Vary the per-backend latency distribution (some fast, some slow, some intermittent). Find the configuration that minimizes end-to-end p99 while keeping total load under 1.5×.

## Key Terms

| Term | What people say | What it actually means |
|------|-----------------|------------------------|
| Tail latency | "latency" | The slowest percentile (p99, p999) of the latency distribution — the worst experiences real users have |
| Hedged request | "sending two requests" | Sending a backup request after a delay; taking the first response and cancelling the rest |
| Hedge delay | "the timeout before hedging" | Time to wait before sending the second request — typically set at p95 to minimize unnecessary duplication |
| Circuit breaker | "fail fast" | A state machine (closed → open → half-open) that stops sending requests to a failing backend |
| Bulkheading | "isolation" | Partitioning resources (thread pools, connections) so one failure can't consume another partition's capacity |
| Retry budget | "rate limit on retries" | A cap on the percentage of total requests that can be retries, preventing retry storms |
| Deadline | "timeout" | An absolute time by which a response must arrive; propagated across RPCs for cascading cancellation |
| p99 | "99th percentile" | The latency below which 99% of requests complete. 1% of requests are slower than this. |

## Further Reading

- Jeff Dean and Luiz André Barroso, "The Tail at Scale," *Communications of the ACM*, 2013.
- Jeff Dean, "Designs, Lessons and Advice from Building Large Distributed Systems," LADIS 2009 — the latency numbers talk.
- Google SRE Book, Chapter 22: "Addressing Cascading Failures" — circuit breakers and retry budgets in production.
- Envoy Proxy documentation: "Circuit Break" and "Outlier Detection."
- resilience4j documentation: circuit breaker, rate limiter, retry configurations.
- Micah Hausler, "SRE at Google: Incident Response and the Mathematics of Cascading Failure."