# Observability Reference Card

## The Three Pillars

| Pillar | Question | Grain | Cost |
|--------|----------|-------|------|
| **Logs** | *Why* did this happen? | Event-level | High (volume) |
| **Metrics** | *What* is happening? | Aggregated | Low (compact) |
| **Traces** | *Where* did this request go? | Request-scoped | Medium |

## Monitoring vs. Observability

- **Monitoring**: Answers questions you planned for (known-knowns).
- **Observability**: Lets you ask questions you never planned for (unknown-unknowns).
- Observable system = you can understand internal state from external outputs without new code.

## Structured Log Schema (JSON)

```json
{
  "timestamp": "2025-01-15T02:31:00.123Z",
  "level": "ERROR",
  "message": "payment processing failed",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "service": "payment-service",
  "route": "POST /payments",
  "status_code": 502,
  "latency_ms": 3204,
  "error": "upstream timeout: stripe-api"
}
```

**Required fields**: `timestamp`, `level`, `message`, `trace_id`, `service`
**Best practice**: Include `trace_id` in every log for cross-service correlation.

## RED Method (for services / APIs)

| Letter | Metric | Instrument As |
|--------|--------|---------------|
| **R** — Rate | Requests/sec | Counter |
| **E** — Errors | Failed requests/sec | Counter |
| **D** — Duration | Latency distribution | Histogram |

Apply RED to **every service**.

## USE Method (for infrastructure / resources)

| Letter | Metric | Instrument As |
|--------|--------|---------------|
| **U** — Utilization | % busy over time window | Gauge |
| **S** — Saturation | Work queued / deferred | Gauge |
| **E** — Errors | Error count | Counter |

Apply USE to **every resource** (CPU, disk, network, memory).

## Golden Signals (Google SRE)

| Signal | Measures |
|--------|----------|
| **Latency** | Time to serve request (split success vs error) |
| **Traffic** | Demand — req/s, concurrent sessions |
| **Errors** | Failed request rate |
| **Saturation** | How full the system is |

If you can only measure four things → these four.

## Distributed Tracing

- **Trace**: One request's full journey across services.
- **Span**: One unit of work within a trace (has `trace_id`, `span_id`, `parent_id`).
- **Context Propagation**: Trace context passed via headers (`traceparent` W3C standard).
- **Baggage**: User-defined key-values propagated across all spans.

```
Trace: 4bf92f3577b34da6...
├── Span A: API Gateway (8ms)
│   ├── Span B: Auth (2ms)
│   └── Span C: Order (6ms)
│       ├── Span D: Inventory (3ms)
│       └── Span E: Payment (50ms) ← bottleneck
```

## OpenTelemetry (OTel)

- **Instrument once, export everywhere.** Write against OTel API, switch backends via config.
- Provides: API + SDK, OTLP wire protocol, semantic conventions, auto-instrumentation, Collector.
- W3C `traceparent` header format: `{version}-{trace-id}-{span-id}-{flags}`

## Alerting Principles

1. Alert on **symptoms**, not causes.
2. Every alert must be **actionable**.
3. Set thresholds based on **SLOs**, not arbitrary percentages.
4. **Minimize alert fatigue** — snoozed alerts are broken alerts.

## SLIs / SLOs / SLAs + Error Budgets

| Term | What | Example |
|------|------|---------|
| **SLI** | Quantitative measure of service behavior | P99 latency, error rate |
| **SLO** | Target value for an SLI | P99 < 200ms, 99.9% availability |
| **SLA** | Contractual commitment with consequences | 99.9% or refund 10% |

**Error budget** = 100% − SLO target. At 99.9% SLO → 0.1% budget = 43.2 min/month.

- > 80% budget remaining → deploy freely, take risks.
- < 20% budget remaining → freeze deploys, prioritize reliability.

## Sampling Strategies

| Strategy | How | When |
|----------|-----|------|
| **Head-based** | Decide at trace start (random %) | Simple, but may miss rare errors |
| **Tail-based** | Decide after trace completes (keep errors, sample success) | Better, requires buffering |

## Quick Checklist: Observability-Driven Development

- [ ] SLIs and SLO targets defined in design doc
- [ ] RED metrics exported for every service endpoint
- [ ] Structured JSON logs with `trace_id`, `span_id`, `service`
- [ ] Trace context propagation across all service boundaries
- [ ] Alerts defined on symptoms (SLO burn rate), not causes
- [ ] Error budget policy documented (when to freeze deploys)