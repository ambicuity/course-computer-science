# Observability as a Design Concern

> Monitoring tells you what you expected. Observability lets you ask questions you didn't plan for.

**Type:** Learn
**Languages:** TypeScript, Go
**Prerequisites:** Phase 16 lessons 01–17
**Time:** ~60 minutes

## Learning Objectives

- Explain the three pillars of observability: metrics, logs, and traces.
- Distinguish monitoring from observability and articulate why the difference matters.
- Implement structured logging with JSON logs, correlation IDs, and contextual fields.
- Apply the RED and USE methodologies to instrument services.
- Use distributed tracing to follow a request across service boundaries.
- Set up OpenTelemetry as a unified observability framework.
- Design alerting on symptoms (not causes) and define SLIs/SLOs/SLAs with error budgets.
- Practice observability-driven development: designing for debuggability from the start.

## The Problem

You deploy a service. At 2 AM, it starts returning 500s. Your dashboard shows a red spike — but *why*? The logs are a flood of unstructured text. There are no traces. You can see *that* something broke, but not *where* or *why*. You're in the dark, paging people, guessing.

This is the difference between monitoring and observability. Monitoring answers questions you thought to ask in advance. Observability lets you ask questions you never planned for — by instrumenting your system so that debuggability is a design property, not an afterthought.

Without observability as a design concern, you cannot operate, debug, or improve distributed systems at scale. This lesson sits in **Phase 16 — Software Engineering & Architecture** because shipping code without observability is shipping code you cannot understand in production.

## The Concept

### The Three Pillars of Observability

Observability rests on three pillars. Each answers a different kind of question:

| Pillar | Answers | Grain | Cost |
|--------|---------|-------|------|
| **Logs** | *Why* did this happen? | Event-level | High (volume) |
| **Metrics** | *What* is happening right now? | Aggregated | Low (compact) |
| **Traces** | *Where* did this request go? | Request-scoped | Medium |

- **Logs** are immutable records of discrete events. Structured logs (JSON) let you filter, aggregate, and correlate.
- **Metrics** are numeric measurements aggregated over time windows — counters, gauges, histograms. They're cheap to store and fast to query.
- **Traces** follow a single request across process boundaries. A trace contains spans; each span is a unit of work within a service.

You need all three. Logs without traces lack context. Traces without metrics lack signal. Metrics without logs lack detail.

### Monitoring ≠ Observability

**Monitoring** is the act of collecting and displaying known signals. You define dashboards and alerts based on what you predict will matter. If you predicted wrong — or if the failure mode is novel — monitoring is silent.

**Observability** is a property of the system. A system is observable if you can understand its internal state by examining its external outputs — without deploying new code or adding new instrumentation. The key question: *Can I ask a question I didn't plan for?*

Example: Your monitoring says CPU is at 80%. Your observability lets you discover that the CPU spike comes from a specific regex backtracking in a newly deployed handler — because your traces show which endpoint, your structured logs show which pattern, and your metrics let you correlate the timing.

### Structured Logging

Plain text logs are human-readable but machine-hostile. Structured logs — JSON objects with consistent schemas — enable:

1. **Filtering**: `service=api AND status>=500 AND trace_id=abc123`
2. **Aggregation**: Count errors by route, latency bucket, or user tier
3. **Correlation**: Link log entries across services using trace IDs

Every structured log entry should include:

| Field | Purpose |
|-------|---------|
| `timestamp` | When the event occurred (ISO 8601, UTC) |
| `level` | Severity: TRACE, DEBUG, INFO, WARN, ERROR, FATAL |
| `message` | Human-readable description of the event |
| `trace_id` | Links to the distributed trace this event belongs to |
| `span_id` | The specific span within the trace |
| `service` | Which service emitted this log |
| Additional context | Any relevant business/domain fields |

**Correlation IDs** are unique identifiers assigned to a request at the edge (API gateway, load balancer) and propagated through every service. They let you reconstruct the full journey of a request by searching a single ID in your log aggregation system.

Example structured log:

```json
{
  "timestamp": "2025-01-15T02:31:00.123Z",
  "level": "ERROR",
  "message": "payment processing failed",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "service": "payment-service",
  "user_id": "usr_9x8f2k",
  "route": "POST /payments",
  "status_code": 502,
  "latency_ms": 3204,
  "error": "upstream timeout: stripe-api"
}
```

### Metrics: RED and USE

Two methodologies give you systematic coverage:

**RED** (for request-driven services — APIs, web handlers):

| Component | What It Measures | Instrument As |
|-----------|-----------------|---------------|
| **Rate** | Requests per second | Counter |
| **Errors** | Failed requests per second | Counter |
| **Duration** | Request latency distribution | Histogram |

RED answers: *Is this service working? How fast? How often does it fail?*

**USE** (for infrastructure — CPUs, disks, network links):

| Component | What It Measures | Instrument As |
|-----------|-----------------|---------------|
| **Utilization** | % of resource busy over time window | Gauge |
| **Saturation** | Amount of work queued/deferred | Gauge |
| **Errors** | Error count on the resource | Counter |

USE answers: *Is this resource overloaded? Running out of headroom? Silently failing?*

Apply RED to every service. Apply USE to every resource. These give you the starting point for every investigation.

### Distributed Tracing

A **trace** follows one request across all services it touches. A trace contains **spans** — each span represents a unit of work in one service.

```
Trace: 4bf92f3577b34da6a3ce929d0e0e4736
│
├── Span A: API Gateway (8ms)
│   ├── Span B: Auth Service (2ms)
│   └── Span C: Order Service (6ms)
│       ├── Span D: Inventory Service (3ms)
│       └── Span E: Payment Service (50ms) ← bottleneck
```

Key concepts:

- **Trace ID**: Unique identifier for the entire request flow.
- **Span ID**: Unique identifier for one unit of work.
- **Parent Span ID**: Links a span to its caller, forming a DAG.
- **Context Propagation**: The mechanism by which trace context (trace ID, span ID, flags) is passed between services — typically via HTTP headers (e.g., `traceparent`, `tracestate` per W3C standard).
- **Baggage**: User-defined key-value pairs propagated across all spans in a trace.

Without context propagation, traces break at service boundaries. Every inter-service call must inject and extract trace context — this is non-negotiable in a distributed system.

### OpenTelemetry: Unified Observability Framework

OpenTelemetry (OTel) is a CNCF incubating project that merges the former OpenTracing and OpenCensus projects. It provides:

1. **A standard API** — Language-specific SDKs for traces, metrics, and logs with a consistent data model.
2. **A wire protocol** — OTLP (OpenTelemetry Protocol) for exporting telemetry to any backend.
3. **Semantic conventions** — Standardized attribute names (e.g., `http.request.method`, `http.response.status_code`) so that all instrumentation speaks the same language.
4. **Auto-instrumentation** — Drop-in agents that instrument popular frameworks (HTTP servers, database drivers, RPC clients) without code changes.
5. **The Collector** — A vendor-agnostic proxy that receives telemetry, processes it, and exports to multiple backends.

The key insight: **instrument once, export everywhere**. Write your instrumentation against the OTel API. Switch backends (Datadog, Grafana, Jaeger, Honeycomb) by changing configuration, not code.

### Alerting: What to Alert On

**Alert on symptoms, not causes.** A symptom is what the user experiences. A cause is why it happened.

| Alert on (Symptom) | Don't alert on (Cause) |
|--------------------|------------------------|
| Error rate > 1% on checkout | Disk usage > 80% |
| P99 latency > 2s on login API | Connection pool nearing limit |
| SLO burn rate exceeded | CPU > 90% |

Why? Causes change. Symptoms are stable. You might fix the disk issue and add a connection pool tomorrow, but "users can't check out" is always worth alerting on.

**Alert design principles:**

1. **Every alert should be actionable.** If the on-call person can't fix it, it shouldn't be an alert.
2. **Minimize alert fatigue.** A page that gets snoozed is a broken alert.
3. **Use severity levels thoughtfully.** Page only for true emergencies; use tickets for slow-burn issues.
4. **Make alerts specific.** "High error rate on POST /payments" is better than "High error rate."
5. **Set thresholds based on SLOs**, not arbitrary percentages.

### SLIs, SLOs, and SLAs

| Term | Definition | Example |
|------|-------------|---------|
| **SLI** (Service Level Indicator) | A quantitative measure of service behavior | P99 latency, error rate, availability |
| **SLO** (Service Level Objective) | A target value for an SLI | P99 latency < 200ms, 99.9% availability |
| **SLA** (Service Level Agreement) | A contractual commitment about SLOs, with consequences for breach | 99.9% availability or refund 10% of monthly bill |

**Error budgets** are the inverse of an SLO. If your SLO is 99.9% availability, your error budget is 0.1% downtime per month — about 43 minutes. The error budget tells you how much failure you can tolerate before breaching the SLO. This is a powerful tool: if you've used 80% of your error budget, it's time to prioritize reliability over feature velocity. If you've only used 10%, you can take more risks (deploy riskier changes, run experiments).

```
Monthly error budget (99.9% SLO) = 30 days × 0.1% = 43.2 minutes
Used this month: 12 minutes → 72% remaining → deploy freely
Used this month: 40 minutes → 7% remaining → freeze deploys, focus on reliability
```

### The Golden Signals (Google SRE)

Google's Site Reliability Engineering book defines four golden signals for monitoring:

| Signal | What It Measures |
|--------|-----------------|
| **Latency** | Time to serve a request (distinguish success vs. error latency) |
| **Traffic** | Demand on the system — requests/sec, concurrent sessions |
| **Errors** | Rate of failed requests (explicit 5xx or implicit wrong content) |
| **Saturation** | How "full" the service is — resource utilization approaching limits |

If you can only measure four things, measure these four. They subsume RED (which is latency + traffic + errors) and add saturation.

### Observability-Driven Development

Observability-driven development (ODD) means designing for debuggability from the start — not bolting it on after the first 2 AM page.

**Practices:**

1. **Instrument before you code.** Define the metrics, log fields, and trace spans you'll need as part of the design doc.
2. **Correlate everything.** Every log line, metric label, and span attribute should carry trace context.
3. **Emit events, not summaries.** Raw events can be aggregated later. Aggregated data cannot be decomposed.
4. **Test your observability.** If a feature fails, can you find it in your telemetry? If not, add more instrumentation.
5. **Use feature flags with observability.** When you toggle a feature on, watch the golden signals. If latency spikes, toggle it off.

**Anti-patterns to avoid:**

- Writing unstructured logs to files and hoping `grep` saves you.
- Adding `fmt.Println` for debugging and leaving it in production.
- Instrumenting only the happy path.
- Creating dashboards for everything (dashboard sprawl) instead of focusing on SLO-based alerts.
- Ignoring trace propagation at service boundaries.

### Real-World Tools

| Tool | Category | Strengths |
|------|----------|-----------|
| **Datadog** | Unified platform | Correlates metrics, logs, traces in one UI; APM with auto-instrumentation |
| **Grafana** | Visualization / dashboarding | Open source; multi-backend dashboards; integrates with Prometheus, Loki, Tempo |
| **Prometheus** | Metrics collection | Pull-based scraping; powerful PromQL query language; battle-tested at scale |
| **Jaeger** | Distributed tracing | CNCF graduated; OpenTelemetry-native; good for Kubernetes environments |
| **Zipkin** | Distributed tracing | Lightweight; Java-native but language-agnostic |
| **Honeycomb** | Observability platform | High-cardinality event analysis; powerful query UI; designed for modern observability |
| **Loki** | Log aggregation | Grafana Labs; indexes labels only (not full text); cheap to operate |
| **Tempo** | Trace backend | Grafana Labs; cost-efficient trace storage; pairs with Loki and Prometheus |
| **PagerDuty** | Incident management | Alert routing, on-call scheduling, escalation policies |

### When to Add Observability

**During design, not after.** Every microservice design doc should specify:

- Which SLIs you'll track (and their SLO targets).
- Which RED/USE metrics you'll export.
- How trace context will be propagated.
- What structured log fields every request will carry.
- Which alerts will fire and who gets paged.

Adding observability after deployment means you're flying blind during the riskiest period (initial rollout). You need telemetry most when things are most likely to break.

### The Cost of Observability

Observability is not free. Consider:

| Cost Dimension | Mitigation |
|---------------|------------|
| **Storage** — High-volume services can emit millions of events/hour | Sampling: tail-based (keep interesting traces) or head-based (keep N%) |
| **Network** — Telemetry data must travel to backends | Batch exports; compress with Protocol Buffers (OTLP) |
| **CPU** — Serializing and exporting telemetry costs cycles | Async exporters; avoid blocking the request path |
| **Financial** — Vendor pricing often based on ingest volume | Pre-aggregate metrics; sample traces; use Grafana stack (open source) |
| **Complexity** — More pipeline = more failure modes | Use OTel Collector as a single control point |

**Sampling strategies:**

- **Head-based**: Decide at trace start (percentage chance) to keep or drop. Simple but may miss rare errors.
- **Tail-based**: Keep all spans in memory, decide after the trace completes whether to keep it (e.g., keep all error traces, sample 1% of success). More sophisticated, requires buffering.

**Aggregation trade-offs:**

- Metrics aggregate at collection time (cheap to store, lossy for investigations).
- Logs and traces preserve detail at ingestion time (expensive to store, rich for debugging).
- The right mix depends on your scale and failure modes.

## Build It

### Step 1: Minimal Version — Structured Logging

The simplest observability primitive: emit structured JSON logs with request context.

```go
// Minimal structured logger — Go
package main

import (
    "encoding/json"
    "log"
    "os"
    "time"
)

type LogEntry struct {
    Timestamp  string `json:"timestamp"`
    Level      string `json:"level"`
    Message    string `json:"message"`
    Service    string `json:"service"`
    TraceID    string `json:"trace_id,omitempty"`
    SpanID     string `json:"span_id,omitempty"`
    StatusCode int    `json:"status_code,omitempty"`
    LatencyMs  int64  `json:"latency_ms,omitempty"`
    Route       string `json:"route,omitempty"`
    Error      string `json:"error,omitempty"`
}

func Emit(entry LogEntry) {
    if entry.Timestamp == "" {
        entry.Timestamp = time.Now().UTC().Format(time.RFC3339Nano)
    }
    data, _ := json.Marshal(entry)
    log.Println(string(data))
}

func main() {
    Emit(LogEntry{
        Level:   "INFO",
        Message: "request completed",
        Service: "api-gateway",
        TraceID: "abc123",
        SpanID:  "def456",
        Route:   "GET /health",
        StatusCode: 200,
        LatencyMs: 12,
    })
}
```

### Step 2: Realistic Version — HTTP Server with Metrics + Tracing

See `code/main.go` and `code/main.ts` for full implementations that include:
- HTTP server with structured logging on every request
- Prometheus-compatible RED metrics (rate, errors, duration)
- OpenTelemetry-style distributed tracing with context propagation
- Error handling with full context in logs

## Use It

### Production: OpenTelemetry in Real Services

OpenTelemetry is the industry standard. Here's how it maps to what we built:

| Our Concept | OTel Equivalent |
|-------------|-----------------|
| `trace_id` | `trace.context.trace_id` from `otel.Tracer.Start()` |
| `span_id` | `span.SpanContext().SpanID()` |
| Context propagation | `otel.GetTextMapPropagator().Inject(ctx, carrier)` |
| RED metrics | `otel.Meter().Int64Counter()` + `Int64Histogram()` |
| Structured logs | `otel.Logger().Emit()` (logs signal coming soon) |

In production, you'd use the OTel SDK instead of hand-rolling. The patterns are the same, but the SDK handles:

- Trace context format (W3C `traceparent` header)
- Span creation, parenting, and lifecycle
- Metric aggregation and export
- Sampling decisions

**Real-world reference:** Look at how the [OpenTelemetry Go contrib](https://github.com/open-telemetry/opentelemetry-go-contrib) instruments `net/http` — it wraps each HTTP request in a span, records RED metrics, and propagates context automatically. Our manual implementation mirrors this pattern.

### Datadog, Grafana + Prometheus + Loki, Honeycomb

- **Datadog** auto-instruments popular frameworks and correlates metrics, traces, and logs via trace IDs injected into log entries.
- **Grafana + Prometheus + Loki + Tempo** is the open-source stack: Prometheus for metrics, Loki for logs, Tempo for traces, all visualized in Grafana dashboards.
- **Honeycomb** excels at high-cardinality queries — you can filter by any attribute (user ID, feature flag, region) without pre-defining indexes.

## Read the Source

- [OpenTelemetry Go SDK — `sdk/trace/span.go`](https://github.com/open-telemetry/opentelemetry-go/blob/main/sdk/trace/span.go): See how spans store attributes, events, and status — the data model behind our tracing implementation.
- [Prometheus `client_golang` — `counter.go`](https://github.com/prometheus/client_golang/blob/main/prometheus/counter.go): See how counters are incremented and exported via the exposition format.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`observability_reference.md`** — A one-page reference card covering the three pillars, RED/USE methodologies, structured log schema, golden signals, and SLO error budget math.

## Exercises

1. **Easy** — Add structured logging to an existing HTTP handler you've written. Include `trace_id`, `span_id`, `route`, `status_code`, and `latency_ms` in every log entry.
2. **Medium** — Instrument a two-service system where Service A calls Service B. Propagate trace context across the HTTP boundary (inject `traceparent` header, extract on the other side). Verify in your traces that both spans share the same `trace_id`.
3. **Hard** — Implement tail-based sampling: buffer all spans for a trace, then decide to keep the trace only if it contains an error span or exceeds a latency threshold. Write tests that verify interesting traces are kept and boring traces are dropped.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Observability | "We have monitoring" | The ability to ask arbitrary questions about system state from external outputs, without deploying new code |
| Monitoring | "Observability" | Collecting and displaying known signals — only answers questions you planned for |
| Structured logging | "JSON logs" | Emitting log entries as machine-parseable records with consistent schemas, not free-text |
| RED method | "Service metrics" | Rate, Errors, Duration — the three metrics every request-driven service must expose |
| USE method | "Resource metrics" | Utilization, Saturation, Errors — the three metrics every infrastructure resource must expose |
| Distributed tracing | "Tracing" | Following a single request across service boundaries using trace IDs and context propagation |
| Correlation ID | "Request ID" | A unique identifier that travels with a request across all services, enabling log correlation |
| SLI | "SLA" | Service Level Indicator — a quantitative measure (e.g., P99 latency, error rate) |
| SLO | "SLA" | Service Level Objective — a target value for an SLI (e.g., P99 < 200ms) |
| SLA | "SLO" | Service Level Agreement — a contractual commitment about SLOs with consequences |
| Error budget | "Downtime budget" | The inverse of an SLO — how much failure you can tolerate before breaching the SLO |
| OpenTelemetry | "OTel" | A vendor-neutral framework for instrumenting, generating, collecting, and exporting telemetry |
| Context propagation | "Trace headers" | The mechanism for passing trace context (trace ID, span ID, flags) across service boundaries |
| Tail-based sampling | "Smart sampling" | Deciding after a trace completes whether to keep it, enabling preservation of error and slow traces |

## Further Reading

- Google SRE Book — Chapter on Service Level Objectives: https://sre.google/sre-book/service-level-objectives/
- OpenTelemetry Documentation: https://opentelemetry.io/docs/
- Charity Majors — Observability: A 3-Legged Stool: https://www.honeycomb.io/blog/observability-3-legged-stool
- Cindy Sridharan — Monitoring and Observability: https://medium.com/@copyconstruct/monitoring-and-observability-8417d1952e5c
- USE Method (Brendan Gregg): http://www.brendangregg.com/usemethod.html
- RED Method (Tom Wilkie): https://grafana.com/blog/2018/08/02/the-red-method-how-to-instrument-your-services/
- W3C Trace Context Specification: https://www.w3.org/TR/trace-context/