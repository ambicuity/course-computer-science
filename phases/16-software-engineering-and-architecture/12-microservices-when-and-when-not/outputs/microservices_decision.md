# Microservices Decision Framework

Use this framework to evaluate whether microservices are appropriate for your project. Score honestly — most projects should start monolithic.

## Step 1: Evaluate Your Current State

### Organizational Factors

| Factor | Monolith (1) | Microservices (5) | Your Score |
|--------|:---:|:---:|:---:|
| Team size | < 5 people | 10+ separate teams | ___ |
| Domain understanding | We're still learning boundaries | Bounded contexts are well-defined and stable | ___ |
| Deployment urgency | Deploy once/day is fine | Teams need independent deploy multiple times/day | ___ |
| Operational maturity | Basic CI/CD | Full platform team with service mesh, observability | ___ |

### Technical Factors

| Factor | Monolith (1) | Microservices (5) | Your Score |
|--------|:---:|:---:|:---:|
| Scaling variance | All modules scale similarly | Specific modules need 10x different resources | ___ |
| Technology requirements | Single stack is fine | Different modules need different stacks | ___ |
| Data isolation | Shared DB is natural | Data has natural boundaries per business capability | ___ |
| Latency tolerance | In-process calls (microseconds) | Network calls (milliseconds) are acceptable | ___ |

### Risk Factors

| Factor | Monolith (1) | Microservices (5) | Your Score |
|--------|:---:|:---:|:---:|
| Team experience | New to distributed systems | Deep experience with distributed systems | ___ |
| Regulatory isolation | No special data isolation needs | Data sovereignty or compliance requires isolation | ___ |
| Product maturity | Pre-product-market fit | Stable product with clear feature trajectory | ___ |

## Step 2: Calculate Your Score

**Total (sum of all scores):** ___ / 45

| Score Range | Recommendation |
|-------------|---------------|
| 9–18 | **Monolith.** Start monolithic. Re-evaluate when score increases. |
| 19–27 | **Modular Monolith.** Build well-structured module boundaries inside a monolith. Prepare extraction points but don't split yet. |
| 28–36 | **Consider Extraction.** Start monolith, extract 1–2 services whose need is proven. Re-evaluate after extraction. |
| 37–45 | **Microservices Justified.** The scale, team structure, and domain clarity justify the investment. |

## Step 3: Check for Distributed Monolith Risk

If you proceed with microservices, verify you're NOT falling into these traps:

- [ ] A single feature change regularly requires deploying 3+ services → **Redraw boundaries**
- [ ] Services share a database → **Each service must own its data exclusively**
- [ ] Synchronous call chains of 4+ hops → **Redesign with async events**
- [ ] A "common" library shared across all services → **Reduce to trivial utilities only**
- [ ] No team owns a service end-to-end → **Restructure teams (inverse Conway)**
- [ ] Integration tests require all services running → **Boundaries are wrong**

## Step 4: If Starting Monolith, Plan Extraction Points

Even in a monolith, design for future extraction:

| Module | Likely Service? | Current Seam Quality | Extraction Priority |
|--------|----------------|---------------------|-------------------|
| ________ | Yes / No / Maybe | Clean / Tangled / Unknown | High / Medium / Low |
| ________ | Yes / No / Maybe | Clean / Tangled / Unknown | High / Medium / Low |
| ________ | Yes / No / Maybe | Clean / Tangled / Unknown | High / Medium / Low |

Seam quality assessment:
- **Clean:** Module has a well-defined interface, no shared database tables, no circular dependencies
- **Tangled:** Module shares database tables with other modules, has circular imports
- **Unknown:** Not yet analyzed — will become clear through operation

## Quick Reference: Architecture Patterns

### Extract a Service When

1. A module has 10x different scaling needs from the rest
2. A module is deployed far more frequently than the rest
3. A team is blocked by another team's deployment schedule
4. A module has clear boundaries and can be owned end-to-end

### Inter-Service Communication Decision

| Need | Pattern | Technology |
|------|---------|------------|
| Caller needs response now | Synchronous (REST/gRPC) | HTTP/2, Protocol Buffers |
| Processing can be deferred | Async event (pub/sub) | Kafka, SNS, EventBridge |
| Command with retry semantics | Async message (queue) | SQS, RabbitMQ, Service Bus |
| Cross-service consistency | Saga (choreographed or orchestrated) | Custom orchestrator, Temporal |

### Data Consistency Decision

| Need | Pattern | Trade-off |
|------|---------|-----------|
| Single-service operation | Local transaction | ACID, simple |
| Cross-service, can be eventual | Event-driven saga | Eventually consistent, complex |
| Cross-service, must be immediate | Reconsider the boundary | Likely a monolith seam, not a service boundary |
| Read-your-writes required | Read from write model after write | Extra latency on write path |

### Observability Stack

| Pillar | Tool Category | Examples |
|--------|--------------|----------|
| Distributed tracing | Trace collector | Jaeger, Zipkin, AWS X-Ray, Datadog |
| Centralized logging | Log aggregation | ELK Stack, Splunk, CloudWatch Logs |
| Metrics | Time-series + alerting | Prometheus + Grafana, Datadog, CloudWatch |
| Correlation | Trace ID propagation | OpenTelemetry, W3C Trace Context |

## Anti-Patterns to Avoid

1. **Distributed Monolith** — Coupled services that must deploy together. Worse than a monolith.
2. **Shared Database** — Services reading each other's databases. Breaks independent deployability.
3. **Smart Pipe, Dumb Endpoint** — Business logic in the API gateway or service mesh instead of in services.
4. **Nano-service** — Services so small they have more infrastructure overhead than business logic.
5. **Premature Decomposition** — Splitting before understanding boundaries. Creates distributed monolith.
6. **God Service** — One service doing everything. You've built a monolith, just with network calls.