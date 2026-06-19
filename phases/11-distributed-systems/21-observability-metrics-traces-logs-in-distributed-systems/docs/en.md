# Observability — Metrics, Traces, Logs in Distributed Systems

> Metrics tell you *it's broken*, traces tell you *where*, logs tell you *why*.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 11 lessons 01–20
**Time:** ~60 minutes

## Learning Objectives

- Explain the three pillars of observability and the question each answers: metrics (is it broken?), traces (where is it broken?), logs (why is it broken?).
- Implement metrics primitives: counters, gauges, and histograms, and compute rate, latency percentiles (p50, p95, p99), and saturation.
- Apply the USE method (Utilization, Saturation, Errors) to any resource in a system.
- Construct distributed traces: spans with trace context propagation across service boundaries (W3C Trace Context, B3).
- Describe OpenTelemetry's architecture: API → SDK → Exporter → Collector → Backend, and why vendor neutrality matters.
- Produce structured logs with correlation IDs (trace_id, span_id in every log line) and choose appropriate log levels.
- Walk through the debugging journey: alert → slow trace → slow span → error log → root cause.
- Build a mini observability stack in Rust: structured logging, trace context propagation, span timing, trace visualization, and critical-path analysis.

## The Problem

Your microservice architecture has five services: a gateway, auth, orders, database, and cache. At 3 AM, PagerDuty fires: p99 latency for the gateway just crossed 2 seconds. You SSH into the gateway machine and see a flood of log lines. Which ones matter? The logs show requests "succeeded" — but the latency metric says otherwise. The request traversed four services, each writing logs to its own file, with no shared identifiers. You can't tell which log line in `auth.log` corresponds to which entry in `gateway.log`. You can't see which service in the chain is slow. You can't find the error because the logs are unstructured text and grep takes 20 minutes across five machines.

This is the observability problem. Without metrics you can't detect degradation. Without traces you can't locate the slow service. Without logs you can't diagnose the root cause. And without correlation IDs tying them together, you can't jump between them.

## The Concept

### The Three Pillars

Each pillar answers a different question:

| Pillar | Question | Granularity | Cost | Example |
|--------|----------|-------------|------|---------|
| Metrics | Is it broken? | Aggregated numbers | Low | request_latency_ms{route="/orders"} p99 = 2340 |
| Traces | Where is it broken? | Per-request across services | Medium | trace abc123: gateway 12ms → auth 5ms → db 2320ms |
| Logs | Why is it broken? | Per-event detail | High | {"trace_id":"abc123","span_id":"def456","level":"error","msg":"connection pool exhausted"} |

Metrics are cheap to store and fast to alert on. Traces show you the path a single request takes through your system. Logs give you the words you need to understand the specific failure, but are expensive at scale. You need all three, connected by correlation IDs.

### Metrics: Counters, Gauges, Histograms

**Counters** go up. Reset on restart. Use for: total requests, total errors. What you compute: rate (requests/sec = Δcounter / Δtime).

**Gauges** go up and down. Use for: current connections, CPU usage, queue depth. What you compute: current value, average over window.

**Histograms** observe distributions. You give them a value; they bucket it. Use for: request latency, response size. What you compute: percentiles (p50, p95, p99).

```
Counter:  http_requests_total{method="GET", path="/orders"}  48231
Gauge:    active_connections{pool="primary"}  47
Histogram: http_request_duration_seconds{method="GET"}
           bucket ≤0.05: 120
           bucket ≤0.1:  340
           bucket ≤0.5:  410
           bucket ≤1.0:  445
           bucket ≤5.0:  480
           bucket +Inf:  482  ← total count
           sum: 187.3        ← total latency in seconds
```

The Prometheus data model: a metric name + a set of labels (key-value pairs) → a time series of (timestamp, value) samples. Labels let you slice: `http_requests_total{method="GET"}` vs `http_requests_total{method="POST"}`.

### Rate, Latency, Saturation — and the USE Method

**Rate**: how many operations per unit time. Derived from counters: `rate(http_requests_total[5m])`.

**Latency percentiles**: p50 means "50% of requests are faster than this." p99 means "1% of requests are slower." P99 is what your worst-case users experience. Always report percentiles, never averages — a single 30-second outlier pulls the average but leaves p50 untouched.

**The USE Method** (Brendan Gregg): for every resource, check:

| Dimension | Question | Metric |
|-----------|----------|--------|
| Utilization | How busy is it? | % time the resource is in use |
| Saturation | How much work is queued? | Queue depth, wait time |
| Errors | How many operations failed? | Error count, error rate |

Use method applied to a connection pool:

```
Utilization: pool.active / pool.max = 47/50 = 94%
Saturation:  pool.waiters = 12 (12 requests waiting for a connection)
Errors:       pool.timeouts_total = 3 in last 5 minutes
```

### Traces: Following a Request Across Service Boundaries

A **trace** is the complete journey of one request through your system. A **span** is one operation within that trace: it has an operation name, a start time, a duration, and tags.

```
Trace abc123 (total: 2341ms)
├── Span 1: gateway.handle_request     (0ms → 2341ms, 12ms own work)
│   ├── Span 2: auth.validate_token     (1ms → 16ms, 15ms)
│   ├── Span 3: orders.get_order        (16ms → 2337ms, 15ms own work)
│   │   ├── Span 4: db.query            (17ms → 2335ms, 2318ms)  ← SLOW SPAN
│   │   └── Span 5: cache.lookup        (2335ms → 2337ms, 2ms)
│   └── Span 6: gateway.serialize_resp  (2337ms → 2341ms, 4ms)
```

The slow span is Span 4 (db.query at 2318ms). Without traces, you'd only see "gateway took 2.3s" and have no idea which downstream service caused it.

**Trace context propagation**: the trace ID and span ID travel with the request. The gateway generates a `trace_id` and a `span_id`. When it calls auth, it injects these into HTTP headers:

```
# W3C Trace Context (standard)
traceparent: 00-abc123-def456-01
tracestate:  roletype=user

# B3 (Zipkin, legacy)
X-B3-TraceId: abc123
X-B3-SpanId: def456
X-B3-ParentSpanId: -
X-B3-Sampled: 1
```

The downstream service reads these headers, creates a child span (with `parent_span_id = def456`), and propagates the same `trace_id`. This is how one logical request becomes one trace across N services.

### OpenTelemetry

OpenTelemetry (OTel) is a vendor-neutral API + SDK for emitting traces, metrics, and logs. Before OTel, you'd instrument your code for Jaeger, then switch to Zipkin and rewrite all instrumentation. OTel fixes this: you instrument once, then configure an exporter.

```
Your Code
    │
    ▼
OTel API (interfaces: Tracer, Meter, Logger)
    │
    ▼
OTel SDK (implementation: sampling, batching, processing)
    │
    ▼
OTel Exporter (format: OTLP/HTTP, OTLP/gRPC)
    │
    ▼
OTel Collector (receives, processes, exports — the pipeline hub)
    │
    ├──→ Jaeger (traces)
    ├──→ Prometheus (metrics)
    └──→ Elasticsearch (logs)
```

The Collector is the key architectural piece: it sits between your services and your backends. It can sample (reduce trace volume), batch (reduce network calls), and route (send traces to Jaeger, metrics to Prometheus) without changing application code.

### Logs: Structured Logging and Correlation IDs

**Unstructured log**: `2024-01-15 03:12:44 ERROR Failed to connect to database`

**Structured log** (JSON):
```json
{
  "timestamp": "2024-01-15T03:12:44.123Z",
  "level": "error",
  "trace_id": "abc123",
  "span_id": "def456",
  "service": "orders",
  "message": "database connection failed",
  "error_code": "CONN_POOL_EXHAUSTED",
  "pool_active": 50,
  "pool_max": 50,
  "pool_waiters": 12
}
```

Two things make this useful: (1) it's JSON, so you can index and query fields; (2) it includes `trace_id` and `span_id`, so you can jump from a metric alert → a trace → these exact log lines.

**Log levels**: Use them intentionally.

| Level | When to use |
|-------|-------------|
| DEBUG | Development-time detail; off in production |
| INFO | Normal operations: request started, request completed |
| WARN | Degradation: retry succeeded, slow query, approaching limit |
| ERROR | Something failed but the system can continue |
| FATAL | The process is about to terminate |

Don't log everything at INFO. Don't log at DEBUG in production. A flood of unimportant logs masks the signal.

### The Debugging Journey

```
1. Alert fires: p99 latency for gateway > 2s (metric breach)
       ↓
2. Find the slow trace: query traces where gateway latency > 2s
       ↓
3. Find the slow span: db.query took 2318ms within that trace
       ↓
4. Find the error log: search logs with matching trace_id
       → "connection pool exhausted, pool_active=50, pool_max=50"
       ↓
5. Root cause: pool_max too small for current traffic, or slow downstream
   queries holding connections too long.
```

Without metrics, you'd never know there's a problem. Without traces, you'd know something is slow but not which service. Without logs, you'd know which service but not why. Without correlation IDs, you'd have three disconnected piles of data.

### Metrics vs. Traces vs. Logs: When to Use Each

| Use case | Pillar | Why |
|----------|--------|-----|
| Alert on degradation | Metrics | Low cost, fast query, aggregation built in |
| Find which service is slow | Traces | Per-request, cross-service, shows the critical path |
| Diagnose a specific failure | Logs | Detailed context, error messages, stack traces |
| Capacity planning | Metrics | Historical trends, rate graphs |
| Understand request flow | Traces | Visualize the call graph with timing |
| Debug a rare edge case | Logs | Full application state at point of failure |

**Don't over-log**: logging every request body at INFO level in production is how you get a $50K AWS bill and an Elasticsearch cluster that can't keep up. **Don't under-trace**: sampling at 0.1% means you'll miss the 1-in-1000 slow request that's causing your p99 to spike. Sample at 100% for errors, 1-10% for success.

## Build It

We'll build a mini observability stack in Rust that produces structured logs, propagates trace context, times spans, computes metrics, and identifies the critical path in a distributed trace.

### Step 1: Span and Trace — The Core Data Structures

A `Span` records one operation: its trace ID, span ID, parent reference, operation name, timing, and tags. A `Trace` groups all spans sharing the same trace ID.

```
Span {
    trace_id: "abc123",
    span_id: "def456",
    parent_span_id: None,
    operation_name: "gateway.handle_request",
    start_time: 0ms,
    duration: 2341ms,
    tags: {"http.method": "GET", "http.path": "/orders/42"}
}
```

### Step 2: Metrics — Counters, Gauges, Histograms

A `Metrics` struct tracks counters (monotonic increments), gauges (current values), and histograms (observed distributions). We'll compute percentiles from histogram data.

### Step 3: Structured Logs with Correlation IDs

A `StructuredLog` emits JSON lines that include `trace_id` and `span_id`, tying every log entry to the request that generated it.

### Step 4: Trace Context Propagation

When service A calls service B, it passes the trace context (trace ID + parent span ID). Service B creates a child span. We'll simulate this with a `TraceContext` that flows across "service" boundaries.

### Step 5: Critical Path and TraceCollector

The `TraceCollector` groups spans by trace ID and computes the **critical path** — the longest chain of spans from root to leaf. This is what determines total request latency.

### Step 6: End-to-End Simulation

Simulate a request: gateway → auth → orders → db + cache. Build the trace with proper parent-child relationships. Record metrics (latency histogram). Emit structured logs. Identify the slow span on the critical path.

Run it with `cd code && cargo run`.

## Use It

**Prometheus** stores metrics as time-series identified by metric name + labels. It supports PromQL for queries like `histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))` to compute p99 latency. Our `Metrics` struct implements the same data model (counters, gauges, histograms) in-memory.

**Jaeger** is a distributed tracing backend that receives spans via OpenTelemetry and renders the waterfall diagram we built in `TraceCollector.visualize()`. Jaeger adds service name coloring, log correlation, and span comparison across traces. Our `Trace` with span tree rendering mirrors Jaeger's trace detail view.

**OpenTelemetry's Rust SDK** (`opentelemetry` crate) provides the `Tracer`, `Meter`, and `Logger` interfaces our code mimics. The key API call is `tracer.start("operation_name")` which returns a `Span` — exactly our `Span::new()` with context propagation. The SDK handles sampling, export, and the Collector pipeline.

Compare: our implementation is a single-process, in-memory version. Production systems add:
- **Sampling**: not every trace is recorded (head-based at the gateway, tail-based on error)
- **Batching and export**: spans are batched and sent to a Collector over gRPC/HTTP
- **Persistent storage**: Prometheus TSDB, Jaeger's Elasticsearch/Cassandra backend
- **Cardinality limits**: histograms with infinite label combinations cause Prometheus to OOM; production systems cap label cardinality

## Read the Source

- [OpenTelemetry Rust SDK — `opentelemetry` crate](https://github.com/open-telemetry/opentelemetry-rust) — `sdk::trace::span` for the `Span` data model and `sdk::trace::provider` for tracer creation. Compare our `Span` struct to theirs.
- [Prometheus `promql` engine](https://github.com/prometheus/prometheus/blob/main/promql/engine.go) — the `eval` function that processes PromQL queries like `histogram_quantile`. Our percentile calculation mirrors its bucket interpolation logic.
- [Jaeger `model` package](https://github.com/jaegertracing/jaeger/blob/main/model/model.pb.go) — the `Trace` and `Span` protobuf definitions that define how spans are stored and rendered.

## Ship It

The reusable artifact lives in `code/`. It's a self-contained Rust program that you can run with `cargo run`:

- Structured logging with trace correlation
- Trace context propagation across service boundaries
- Span timing and critical-path analysis
- Metrics (counters, gauges, histograms) with percentile computation

Import the data structures in a later phase's capstone or your own projects.

## Exercises

1. **Easy** — Add a new metric: `error_rate` counter that increments whenever a span's tags contain `"error": true`. Compute error rate as errors / total requests over a time window.
2. **Medium** — Implement tail-based sampling: after a complete trace is collected, only export it if its total duration exceeds a threshold (e.g., > 1 second) or if any span has an error tag. This mirrors how production systems sample 100% of error traces but only 1-10% of successful ones.
3. **Hard** — Add log rate limiting: if the same log message (same level + message template) fires more than N times per minute, collapse subsequent identical messages into a single summary line `"message repeated 47 times"`. This is what production log pipelines do to prevent log floods.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Observability | "Monitoring" | The ability to answer arbitrary questions about system state without deploying new code — metrics give you aggregates, traces give you request paths, logs give you detail |
| Metrics | "Numbers about the system" | Time-series data (counters, gauges, histograms) keyed by name + labels; optimized for storage, aggregation, and alerting |
| Trace | "A distributed call stack" | The complete path of one request across service boundaries, composed of spans linked by parent-child relationships |
| Span | "A trace segment" | One operation within a trace: name, start time, duration, parent reference, and tags — the unit of work that traces are built from |
| Critical path | "The slowest part" | The longest chain of sequential spans from root to leaf; reducing any span on the critical path reduces total latency |
| USE method | "Check CPU and memory" | For every resource, check Utilization (% busy), Saturation (queue depth), and Errors (failure count) — systematic, not ad-hoc |
| Structured logging | "JSON logs" | Log entries as key-value pairs (JSON) instead of unstructured text, enabling indexing, filtering, and correlation by trace_id |
| OpenTelemetry | "A tracing library" | A vendor-neutral API + SDK + Collector pipeline for emitting traces, metrics, and logs — instrument once, export anywhere |

## Further Reading

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/) — the official docs for the API, SDK, and Collector. Start here for any implementation.
- [The USE Method](https://www.brendangregg.com/usemethod.html) — Brendan Gregg's original post. The systematic "for every resource, check utilization, saturation, errors" approach.
- [Distributed Tracing with OpenTelemetry (O'Reilly)](https://www.oreilly.com/library/view/distributed-tracing-with/9781492063235/) — covers the theory and practice of tracing, from Dapper to OpenTelemetry.
- [Prometheus: Up & Running](https://www.oreilly.com/library/view/prometheus-up/9781492034111/) — chapters 4-6 cover the data model, histograms, and PromQL. Our percentile computation mirrors the `histogram_quantile` function.