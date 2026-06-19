# Notes — Microservices: When and When Not

## Decision Framework: Monolith vs Microservices

### Architecture Comparison

| Dimension | Monolith | Microservices |
|-----------|----------|---------------|
| Deployment | Single unit, all-or-nothing | Independent per service |
| Scaling | Whole application together | Per-service, proportional to need |
| Data | Shared database, transactions | Each service owns its data, eventual consistency |
| Team structure | Small team owns whole app | Teams own services end-to-end |
| Complexity | Low initial, grows with size | High initial, grows with number of services |
| Debugging | Single process, single log | Distributed tracing, centralized logging |
| Technology | Single stack | Polyglot — each service chooses |
| Latency | In-process calls (microseconds) | Network calls (milliseconds) |
| Refactoring | Move code between packages | Rewrite service, migrate data, version APIs |
| Failure mode | Single point of failure | Cascading failures, partial degradation |
| Operational cost | One CI/CD pipeline, one monitoring setup | Per-service: CI/CD, monitoring, on-call, secrets |

### When to Start with Microservices (Rare)

| Condition | Why |
|-----------|-----|
| You already have a well-understood domain with clear bounded contexts | Boundaries are data-driven, not guessed |
| Multiple teams need independent deployment cadences | Organizational scaling drives the split |
| Specific modules have 10x+ different scaling profiles | Cost optimization is provable |
| You have platform engineering support | CI/CD, observability, service mesh already exist |
| Regulatory / compliance isolation required | Data sovereignty, multi-tenant isolation |

### When to Start with a Monolith (Common)

| Condition | Why |
|-----------|-----|
| Team < 10 people | Operational overhead of services exceeds benefit |
| Domain not yet understood | You'll guess boundaries wrong |
| Pre-product-market fit | Architecture changes faster than services can be migrated |
| Uniform scaling requirements | No need for independent scaling |
| Limited operational budget | Each service has fixed infrastructure cost |

### The Distributed Monolith Checklist

If you check 3 or more, you likely have a distributed monolith:

- [ ] A single feature change requires deploying 3+ services
- [ ] Services share a database or database schema
- [ ] Services communicate via synchronous call chains of 4+ hops
- [ ] A "common" library is shared across all services and upgraded in lockstep
- [ ] No team owns a service end-to-end (ownership is fragmented across teams)
- [ ] You can't deploy a service without checking with other teams first
- [ ] Integration tests require all services running simultaneously

### Service Boundary Identification

```
BOUNDARY SIGNALS (where to split):

1. Language boundaries ── same word, different meaning
   "Product" in Catalog ≠ "Product" in Shipping ≠ "Product" in Billing

2. Change rate boundaries ── modules that change at different speeds
   Catalog changes daily, Billing changes monthly

3. Scaling boundaries ── modules with different resource profiles
   Search is CPU-heavy, Payment is I/O-heavy, Reporting is memory-heavy

4. Data boundaries ── data accessed together, rarely with other data
   Order + OrderLines are always queried together

5. Team boundaries ── where a team can own something end-to-end
   The Notifications team doesn't need to coordinate with the Billing team

ANTI-SIGNALS (where NOT to split):

✗ Splitting by technical layer (UI service, DB service, auth service)
  → Creates technical coupling, not business boundaries
✗ Splitting by CRUD entity (User CRUD, Product CRUD, Order CRUD)
  → Creates anemic services with no business logic
✗ Splitting because "the codebase is big"
  → A module system within a monolith solves this
✗ Splitting because "we want to use different languages"
  → Technology diversity should follow boundary need, not drive it
```

### Communication Patterns

```
SYNCHRONOUS (REST / gRPC)

  Service A ──HTTP──► Service B ──HTTP──► Service C
                (blocks)          (blocks)

  Use when: caller needs the response now
  Risk: cascading failures, latency stacking
  Mitigation: circuit breakers, timeouts, fallbacks


ASYNCHRONOUS (Events / Messages)

  Service A ──publish──► Event Bus ──┬──► Service B (subscribes)
                                     └──► Service C (subscribes)
  (returns immediately)

  Use when: processing can be deferred
  Risk: eventual consistency, event ordering
  Mitigation: idempotent consumers, partitioned streams, DLQ
```

| Concern | Synchronous | Asynchronous |
|---------|-------------|--------------|
| Response needed now | REST / gRPC | N/A |
| Process can be deferred | N/A | Events / Messages |
| Coupling | Tighter (caller must know callee) | Looser (publisher doesn't know subscribers) |
| Failure handling | Retry + circuit breaker | Retry + DLQ + compensating actions |
| Ordering | Not guaranteed across hops | Via partitioned streams (Kafka) |
| Debugging | Request tracing across hops | Correlation IDs across event chains |

### Data Consistency Patterns

```
SHARED-NOTHING PRINCIPLE:
  Each service owns its data.
  Access only through the owning service's API.
  No cross-service database access.

SAGA PATTERN (cross-service consistency):

  Choreographed:
    Service A acts → emits event → Service B acts → emits event → ...

  Orchestrated:
    Central coordinator calls each service step
    On failure: executes compensating actions in reverse

  Compensating actions:
    ReserveStock  → ReleaseStock
    ChargeCard    → RefundCard
    CreateOrder   → CancelOrder
```

### Infrastructure Patterns

```
API GATEWAY:
  Client ──► Gateway ──► Service A
                 ──► Service B
  Handles: auth, rate limiting, TLS, routing, aggregation
  Risk: becomes "smart pipe" that absorbs business logic

SERVICE MESH:
  Service A ──sidecar──► sidecar ──► Service B
  Handles: mTLS, retries, circuit breaking, tracing
  Tools: Istio, Linkerd
  When: 10+ services need consistent infra-level concerns

OBSERVABILITY:
  Distributed tracing (Jaeger, Zipkin) — correlate requests across services
  Centralized logging (ELK, CloudWatch, Datadog) — aggregate logs by trace ID
  Metrics (Prometheus + Grafana) — RED method: Rate, Errors, Duration
```

### Team Topology Guidance

```
CONWAY'S LAW:
  Architecture mirrors organization structure.
  Fragmented teams → fragmented, coupled services.

INVERSE CONWAY:
  Design team structure to produce the architecture you want.

TEAM TYPES:
  Stream-aligned:  Owns a service end-to-end (e.g., Order team)
  Enabling:        Helps stream teams adopt capabilities (Platform team)
  Complicated:     Owns a specialized subsystem (ML team)
  Platform:        Provides internal platform (DevOps / Infra team)

RULE OF THUMB:
  One team per service, one service per team.
  If a team owns portions of multiple services, boundaries are wrong.
```

### Real-World Scale Reference

| Company | Architecture | Services | When to Extract |
|---------|-------------|----------|----------------|
| Netflix | Microservices | ~700 (consolidated from 1000+) | Had scale need (200M+ users), built platform team first |
| Amazon | Microservices | Thousands | Started from monolith, extracted based on team scaling |
| Shopify | Modular Monolith | 1 (Rails) with clear module boundaries | Extracts only proven services (billing) — scale to millions of req/min |
| Basecamp | Monolith | 1 (Rails) | Deliberately monolithic — small team, clear boundaries within |
| Stripe | Microservices | ~50+ | Extracted from monolith as domain boundaries clarified |

### Evaluation Checklist

Score each factor 1-5. Weight by importance to your project.

| Factor | Monolith Favors (1) ↔ Microservices Favors (5) | Weight | Score | Weighted |
|--------|:---:|:---:|:---:|:---:|
| Team size (small=1, large=5) | | | | |
| Domain clarity (unclear=1, clear=5) | | | | |
| Scaling variance (uniform=1, diverse=5) | | | | |
| Deployment independence need (low=1, high=5) | | | | |
| Operational maturity (low=1, high=5) | | | | |
| Technology diversity need (low=1, high=5) | | | | |
| Regulatory isolation (no=1, yes=5) | | | | |

Total weighted score < 15 → monolith. 15-25 → monolith with extraction points. > 25 → microservices justified.