# Microservices vs Monolith — Real Trade-offs

> Start monolithic. Extract services when the pain of the monolith exceeds the pain of microservices.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 11 lessons 01–19
**Time:** ~60 minutes

## Learning Objectives

- Explain what a monolith and a microservice architecture are and why the choice between them is a trade-off, not a moral imperative.
- Quantify the latency cost of microservices: network calls are 10–100× slower than in-process calls, and each hop adds failure probability.
- List the conditions where a monolith wins (small team, early stage, simple domain, low-latency requirements, limited DevOps maturity) and where microservices win (large teams, independent scaling, different SLAs, polyglot persistence, clear bounded contexts).
- Describe Conway's Law and explain why team structure should drive service boundaries.
- Define bounded contexts from Domain-Driven Design and explain why a microservice should own one.
- Explain the purpose of a service mesh (Istio, Linkerd), how sidecar proxies work, and what they handle so application code doesn't have to.
- Describe the role of an API gateway (routing, rate limiting, auth, aggregation) and name production examples.
- Implement the strangler fig pattern: gradually replace monolith endpoints with microservice calls.
- Implement a circuit breaker with closed → open → half-open → closed state transitions.
- Build a side-by-side simulation comparing monolith and microservice request latency and failure handling.

## The Problem

You're the CTO of a startup. Your team of four just shipped v1 as a single deployable — one process, one database, one deployment pipeline. It works. It's fast. Deploys take 30 seconds.

Now the VP of Engineering at BigTech gives a conference talk: "We decomposed our monolith into 400 microservices and our velocity increased 10×." Your CEO sends you the YouTube link: "Why aren't we doing this?"

Here's the thing: BigTech has 400 engineers, a platform team of 30, dedicated SRE teams per service, and a custom service mesh. You have four people and a Postgres instance. If you decompose now, you'll spend 80% of your time on infrastructure — service discovery, distributed tracing, saga orchestration, deployment pipelines for N services — and 20% on product. Your monolith lets you spend 90% on product.

But there's a real inflection point. When your team grows to 15, when two teams keep stepping on each other's deploys, when the search service needs 3× more CPU than the auth service, when a deploy of the billing module takes down notifications — the monolith starts to hurt. The same simplicity that made it fast to build makes it painful to scale teams and independently scale components.

This lesson doesn't tell you which to pick. It gives you the framework to decide, the numbers to justify the decision, and the patterns to migrate when the time comes.

## The Concept

### The Monolith Model

A monolith is a single deployable unit. All code — authentication, billing, search, notifications — runs in one process. All modules share the same database (or at least the same database connection pool). An in-process function call handles a request end-to-end:

```
HTTP Request → Router → Auth → Business Logic → DB → Response
                     all in one process
                     call overhead: ~0.001ms (function call)
```

**Advantages:**
- Simple to develop, test, and debug — one process, one log stream, one stack trace.
- Simple to deploy — one artifact, one deploy pipeline.
- Low latency — function calls, not network calls.
- Strong consistency — one database, one transaction boundary.
- Easy transactional integrity — BEGIN/COMMIT across all tables.

**Disadvantages:**
- Scaling = scaling the whole app. The search service that needs 8 CPUs takes the auth service along for the ride.
- Deployment coupling — a one-line fix in billing requires redeploying everything. A bug in notifications takes down auth.
- Team coupling — merge conflicts, deploy conflicts, "who broke staging?"
- Tech lock-in — everything is one language, one framework, one database.

### The Microservice Model

Microservices decompose the application into independently deployable services, each owning its own data:

```
Client → API Gateway → Auth Service → Auth DB
                      → Billing Service → Billing DB
                      → Search Service → Search DB
                      → Notification Service → Notification DB
```

Each service:
- Has its own database (no shared tables).
- Deploys independently (separate pipeline, separate versioning).
- Scales independently (3 replicas of search, 1 replica of auth).
- Fails independently (billing down doesn't crash auth).

**The catch:** Every inter-service call goes over the network:

```
In-process call:           ~0.001ms  (function call)
Local network call:        ~0.5ms    (localhost TCP)
Datacenter network call:   ~2-5ms    (same region)
Cross-region call:         ~50-100ms (different region)
```

A single user request that touches 5 services = 5 network hops = 10–25ms minimum, vs. 1ms in the monolith. And that's the happy path. If any service is slow or down, the whole chain degrades.

### When the Monolith Wins

| Condition | Reason |
|-----------|--------|
| Team of 2–8 people ("two-pizza team") | Communication overhead of N services exceeds development overhead of one codebase |
| Early-stage startup | Speed of iteration matters more than scalability; you don't yet know the domain boundaries |
| Simple domain | Few bounded contexts → few natural service boundaries → forced decomposition creates accidental complexity |
| Low-latency requirements | Each network hop adds 0.5–5ms; a monolith with 5 in-process calls is 1ms vs. 10ms+ in microservices |
| Limited DevOps maturity | Microservices require CI/CD per service, monitoring per service, deployment orchestration, on-call rotation per service |

### When Microservices Win

| Condition | Reason |
|-----------|--------|
| Large teams (10+ devs) | Independent deploy pipelines let teams ship without blocking each other |
| Independent scaling needs | Search needs 10× more replicas than auth; monolith forces same scale for all |
| Different SLAs per component | Billing needs 99.999% uptime; search can tolerate 99.9% |
| Polyglot persistence | Search needs Elasticsearch; auth needs relational ACID; analytics needs columnar |
| Clear bounded contexts | When domain boundaries are stable and well-understood, decomposition is low-risk |

### The Real Costs of Microservices

**Network latency:** A request that touches 4 services pays 4 network round trips. At 2ms per hop, that's 8ms vs. <1ms in-process. With retries, circuit breakers, and load balancers, the p99 can be 50–100ms.

**Partial failure:** In a monolith, either the whole app works or it doesn't. In microservices, any subset can fail. Your auth service is up but billing is down — what do you return? Every call site needs timeout, retry, and fallback logic.

**Data consistency:** No shared database means no ACID transactions across services. You need eventual consistency and saga-based transactions (Lesson 14). A customer's order appears in the orders service but not yet in the inventory service. For 100ms, the data is inconsistent and every client must handle this.

**Operational complexity:** Monitoring 1 service → monitoring N services. Each service needs its own logs, metrics, traces, alerts, and on-call rotation. You need distributed tracing (Lesson 21) just to understand a single request's path.

**Deployment complexity:** N services with M possible versions means N×M compatibility surfaces. API versioning, contract testing, and canary deployments become mandatory.

**Debugging:** A single user request spans 4 services → 4 log files, 4 trace spans, 4 alert systems. Without distributed tracing (Lesson 21), you're correlating timestamps by hand.

### Conway's Law

> "Organizations which design systems produce designs that are copies of the communication structures of these organizations." — Mel Conway, 1967

If your team of four communicates constantly, your architecture will be tightly coupled regardless of how many services you deploy. If you split into three teams of five, each team will naturally produce a service boundary.

**The implication:** Don't decompose into microservices and then reorganize teams around them. Organize teams around business capabilities, and the services will follow. The service boundary should match the team boundary. If a team owns a service end-to-end (code, deploy, operate, on-call), they ship faster because they don't need cross-team coordination.

Conway's Law works in both directions:
- **Reverse Conway:** Design the team structure you want, and the architecture will follow.
- **Conway neglect:** Ignore team structure, and you'll get a distributed monolith — microservices that are so tightly coupled they might as well be one process, but now with network calls.

### Bounded Contexts (DDD)

A **bounded context** is a subdivision of a domain model with its own ubiquitous language. In an e-commerce system:

- **Catalog context:** Products, categories, pricing, search. Language: "product," "SKU," "listing."
- **Order context:** Cart, checkout, order, line item. Language: "order," "item," "fulfillment."
- **Billing context:** Invoice, payment, refund. Language: "charge," "invoice," "settlement."

A microservice should own one bounded context. Within the context, the model is consistent and cohesive. Across contexts, data is exchanged through well-defined APIs or events. "Product" in the Catalog context (name, description, price) is a different model than "Product" in the Order context (SKU, quantity, line item price).

**Why this matters for decomposition:** If you can't identify bounded contexts, you're not ready for microservices. Forced decomposition without domain boundaries produces the distributed monolith — all the costs of microservices with none of the benefits.

### Service Mesh

A **service mesh** provides infrastructure-level networking between services without requiring changes to application code:

```
┌──────────────┐         ┌──────────────┐
│  Service A   │         │  Service B   │
│  (app code)  │         │  (app code)  │
└──────┬───────┘         └───────┬──────┘
       │                         │
   ┌───┴───┐               ┌─────┴───┐
   │Sidecar│               │ Sidecar │
   │(Envoy)│               │(Envoy)  │
   └───┬───┘               └────┬────┘
       │    mTLS, retries,  │
       │    circuit breaking │
       └────────────────────┘
            handled by mesh
```

The **sidecar proxy** (Envoy, in Istio/Linkerd) intercepts all inbound and outbound traffic. It handles:
- **mTLS** — automatic encryption between services.
- **Retries with backoff** — transparent retry of failed requests.
- **Circuit breaking** — stop sending traffic to failing services.
- **Traffic shaping** — canary, A/B, blue-green deployments.
- **Observability** — metrics, traces, and access logs automatically emitted.
- **Rate limiting** — per-service, per-route.

**Istio** and **Linkerd** are the two most widely deployed service meshes. Both use Envoy as the data plane (the proxy) and provide a control plane for configuration.

**What the mesh does NOT do:** It doesn't eliminate network latency. It doesn't solve data consistency. It makes the infrastructure concerns transparent to app code, but the fundamental costs of distribution remain.

### API Gateway

An **API gateway** is the single entry point for all client traffic:

```
Client → API Gateway → /api/auth     → Auth Service
                      → /api/orders   → Order Service
                      → /api/search   → Search Service
                      → /api/billing  → Billing Service
```

The gateway handles:
- **Routing** — map paths to services.
- **Authentication** — validate JWT tokens once at the gateway, not in every service.
- **Rate limiting** — throttle per-client, per-route.
- **Aggregation** — combine responses from multiple services into one response.
- **SSL termination** — handle TLS at the edge.

Production examples: Kong, Ambassador, AWS API Gateway, Envoy (as edge proxy), NGINX (in gateway mode).

The gateway is NOT a substitute for service-to-service communication. Services still call each other directly (or via the mesh). The gateway handles client-to-service traffic.

### The Strangler Fig Pattern

Named after strangler fig trees that gradually envelop their host tree, the **strangler fig pattern** migrates from monolith to microservices by replacing one endpoint at a time:

```
Phase 1: 100% monolith
  Client → Monolith (all endpoints)

Phase 2: Extract auth service
  Client → API Gateway → /api/auth → Auth Service (new)
                      → /api/*    → Monolith (everything else)

Phase 3: Extract order service
  Client → API Gateway → /api/auth   → Auth Service
                      → /api/orders  → Order Service (new)
                      → /api/*       → Monolith (everything else)

Phase N: Monolith is gone
  Client → API Gateway → /api/auth   → Auth Service
                      → /api/orders  → Order Service
                      → /api/search  → Search Service
                      → /api/billing → Billing Service
```

Each extraction is production-validated before the next. If the new service has a bug, route traffic back to the monolith. No big-bang migration, no overnight rewrite.

## Build It

See `code/main.py` for the full implementation. The key pieces:

1. **MonolithSimulation** — single-process request handling with in-process calls and a shared database.
2. **MicroserviceSimulation** — N services calling each other over simulated network, with service discovery, circuit breaking, and retry with backoff.
3. **LatencyComparison** — side-by-side request latency measurement.
4. **CircuitBreaker** — three-state (closed → open → half-open → closed) failure protector.
5. **StranglerFig** — gradually redirect endpoints from monolith to microservice.

### Step 1: MonolithSimulation

The monolith is a single process. Function calls are effectively free. The shared database (in-process dict) has zero network cost. A request that touches 5 modules in sequence takes ~1ms total.

### Step 2: MicroserviceSimulation

Each service is an independent entity. Calls between services go over a simulated network with configurable latency (0.5–5ms per hop). Services register with a service discovery registry. Circuit breakers protect against cascading failures. Retries with exponential backoff handle transient faults.

### Step 3: Circuit Breaker

The circuit breaker has three states:
- **Closed:** Requests flow normally. Track failure rate. If failure rate exceeds threshold, transition to Open.
- **Open:** Requests are immediately rejected (fast fail). After a cooldown period, transition to Half-Open.
- **Half-Open:** Allow one request through. If it succeeds, transition to Closed. If it fails, transition back to Open.

### Step 4: Strangler Fig

The API gateway starts with all routes pointing at the monolith. One by one, routes are redirected to new microservices. Each migration is validated before the next endpoint is extracted.

### Step 5: Comparison

Run both architectures for the same workload and compare:
- Latency: monolith ~1ms vs. microservice ~10–50ms depending on how many services a request touches.
- Failure handling: monolith fails entirely; microservices fail partially (circuit breakers prevent cascading failure).

## Use It

**Envoy Proxy:** The sidecar proxy used by Istio and Linkerd. See the Envoy source at `source/common/http/` for how it implements circuit breaking, retry budgets, and outlier detection. The key difference from our simulation: Envoy's circuit breaker operates at the connection pool level, not per-request.

**Istio:** The most widely deployed service mesh. See `manifests/` for how it configures VirtualServices (routing rules), DestinationRules (circuit breakers, connection pools), and AuthorizationPolicies (mTLS, RBAC). Our simulation's service discovery and routing is what Istio's Pilot/istiod control plane does.

**Kong Gateway:** A production API gateway. See `kong/plugins/` for how it handles authentication, rate limiting, and request transformation. The strangler fig pattern is what Kong's route-by-route migration enables.

**What production systems add beyond our simulation:**
- **Observability:** Prometheus metrics, Jaeger/Zipkin traces, structured logs — all automatically emitted by the mesh.
- **Dynamic configuration:** Circuit breaker thresholds, retry budgets, and rate limits change at runtime without redeployment.
- **Canary deployments:** The mesh routes 5% of traffic to the new version, monitors error rates, and either promotes or rolls back automatically.
- **mTLS rotation:** Certificate rotation every 24 hours, managed by the mesh control plane.

## Read the Source

- Envoy `source/common/upstream/outlier_detection.cc` — how Envoy implements circuit breaking and outlier detection at the connection pool level.
- Istio `manifests/charts/istio-control/istio-discovery/templates/` — how Istio's control plane configures Envoy sidecars with routing rules, circuit breakers, and mTLS policies.

## Ship It

The reusable artifacts from this lesson:

- **`code/main.py`** — A self-contained service mesh simulation with monolith vs. microservice comparison, circuit breaker state machine, and strangler fig pattern. Reusable as a reference for understanding and debugging production microservice architectures.

## Exercises

1. **Easy** — Modify the `MonolithSimulation` to simulate a slow module (e.g., a reporting module that takes 50ms). Observe how one slow module slows down the entire monolith. Then increase that module to 500ms and observe the impact on overall request latency.
2. **Medium** — Add a **bulkhead** pattern to the `MicroserviceSimulation`: limit each service to a fixed number of concurrent requests. When the limit is hit, return 503 immediately instead of queuing. Compare the failure propagation pattern with and without bulkheads.
3. **Hard** — Implement a **saga** (Lesson 14) in the `MicroserviceSimulation`. When the order-service, inventory-service, and payment-service each commit locally, and payment fails after the first two succeed, the saga should trigger compensating transactions. Compare the latency and consistency properties with a monolithic transaction.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Monolith | "Outdated architecture" | A single deployable unit where all modules share one process and one database. Simple to develop, deploy, and operate — and the right choice for many teams and domains. |
| Microservices | "Modern architecture" | A decomposition into independently deployable services, each owning its own database. Provides team autonomy, independent scaling, and fault isolation — at the cost of network latency, operational complexity, and eventual consistency. |
| Bounded context | "Service boundary" | A subdivision of the domain model with its own ubiquitous language, persistent storage, and consistency guarantees. From DDD — a microservice should own one. |
| Service mesh | "Networking layer" | Infrastructure (sidecar proxies + control plane) that handles mTLS, retries, circuit breaking, traffic shaping, and observability between services — without application code changes. |
| API gateway | "Front door" | A single entry point for all client traffic that handles routing, authentication, rate limiting, and response aggregation. Not a substitute for service-to-service communication. |
| Strangler fig | "Gradual migration" | A pattern where endpoints are redirected one at a time from monolith to microservices. Each extraction is production-validated before the next. Named after the strangler fig tree that gradually envelops its host. |
| Conway's Law | "Teams mirror architecture" | Organizations produce system designs that mirror their communication structures. Team boundaries should drive service boundaries, not the other way. |
| Circuit breaker | "Automatic shutoff" | A state machine (closed → open → half-open → closed) that stops sending traffic to a failing service, preventing cascading failures. After a cooldown, it tests the service again. |

## Further Reading

- [Martin Fowler, "Microservices" (2014)](https://martinfowler.com/articles/microservices.html) — The article that popularized the term. Read it carefully — Fowler explicitly says microservices are a trade-off, not a default.
- [Sam Newman, "Building Microservices" 2nd Ed (2021)](https://www.oreilly.com/library/view/building-microservices-2nd/9781492034018/) — The most comprehensive book on the topic. Chapter 2 on decomposition and Chapter 5 on the strangler fig pattern are essential.
- [Conway's Law, IEEE (1968)](https://www.melconway.com/Home/Conways_Law.html) — The original article. Short, dense, and still relevant.
- [Eric Evans, "Domain-Driven Design" (2003)](https://domainlanguage.com/ddd/) — Chapter 3 on bounded contexts. The theoretical foundation for service decomposition.
- [Istio Architecture Docs](https://istio.io/latest/docs/ops/deployment/architecture/) — How Envoy sidecars, istiod control plane, and the data plane fit together.
- [Envoy Circuit Breaking](https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/upstream/circuit_breaking) — Production circuit breaking at the connection pool level.