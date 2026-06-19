# Microservices — When and When Not

> Microservices are a powerful architectural style — but they solve a specific set of problems, and applying them to the wrong problems creates distributed systems that are harder to build, deploy, and debug than the monolith they replaced.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 16 lessons 01–11
**Time:** ~60 minutes

## Learning Objectives

- Define what microservices are and how they differ from monolithic architectures.
- Articulate the monolith-first approach and explain why most systems should start monolithic.
- List the conditions under which microservices provide genuine benefits (independent scaling, deployment autonomy, team ownership, technology diversity).
- List the conditions under which microservices increase cost and complexity beyond their benefits (distributed system overhead, network latency, data consistency, debugging difficulty, operational burden).
- Recognize the distributed monolith anti-pattern and explain how it arises.
- Find service boundaries using bounded contexts from Domain-Driven Design.
- Compare inter-service communication styles: synchronous (REST, gRPC) vs asynchronous (events, message queues).
- Apply the shared-nothing data principle and the saga pattern for cross-service consistency.
- Describe the API gateway and service mesh patterns and when each is useful.
- Explain observability challenges unique to microservices (distributed tracing, centralized logging) and how to address them.
- Apply Conway's Law and the inverse Conway maneuver to team topology decisions.
- Evaluate real-world case studies (Netflix, Amazon, Shopify) and extract lessons.
- Use a decision framework to choose between monolith and microservices for a given project.

## The Problem

You are building a SaaS product. The team is four developers. Someone read a blog post about Netflix and now every hallway conversation ends with "we should break this into microservices." The codebase is six months old, hasn't shipped to production yet, and the team is already debating whether to split the user service from the billing service.

This is the most common architectural mistake in modern software: adopting microservices before the monolith has taught you where the boundaries are. Microservices solve real problems — independent deployment, independent scaling, team autonomy — but they introduce distributed-system complexity that dwarfs the problems they solve if the organization and domain are not ready.

Without a clear framework for when microservices help and when they hurt, teams either over-engineer early (ending with a distributed monolith that is worse than a plain monolith) or under-engineer late (stuck with a tangled monolith that no single team can deploy independently).

## The Concept

### What Microservices Are

A microservice architecture structures an application as a collection of **independently deployable services**, each organized around a business capability. Each service:

- Owns its data — no shared databases between services
- Deploys independently — no协调 coordinated deployments
- Communicates over well-defined APIs — no in-process function calls
- Can be built with different technology stacks — polyglot persistence and languages
- Is owned by a single team — team has full stack responsibility

```
Monolith:

┌─────────────────────────────────────────┐
│                  App                     │
│  ┌─────────┐ ┌──────────┐ ┌───────────┐ │
│  │ Users   │ │ Billing  │ │ Catalog   │ │
│  │         │ │          │ │           │ │
│  │ (shared │ │ (shared  │ │ (shared   │ │
│  │  DB)    │ │   DB)    │ │    DB)    │ │
│  └─────────┘ └──────────┘ └───────────┘ │
└─────────────────────────────────────────┘
  Deploys as one unit. Scales as one unit.

Microservices:

┌────────┐    ┌─────────┐    ┌──────────┐
│ User   │    │ Billing │    │ Catalog  │
│Service │    │ Service │    │ Service  │
│        │    │         │    │          │
│User DB │    │Bill. DB │    │Cat. DB   │
└────────┘    └─────────┘    └──────────┘
  Deploys       Deploys        Deploys
  independently independently  independently
```

**Independently deployable** is the defining characteristic. If you must coordinate deployments between services, you do not have microservices — you have a distributed monolith.

### The Monolith-First Approach

**Start monolithic. Extract services as needed.** This is the dominant recommendation among experienced practitioners, including Martin Fowler, Sam Newman, and the engineers who built Amazon and Shopify.

The reasoning:

1. **You don't know the boundaries yet.** A monolithic codebase reveals which modules change together and which can be separated. Before you've operated the system, boundary guesses are informed by theory, not data.

2. **YAGNI (You Ain't Gonna Need It).** Most systems never reach the scale where microservices pay off. Premature decomposition is premature optimization at the architecture level.

3. **Refactoring within a monolith is cheap.** Moving code between packages in a monolith is a rename operation. Moving code between services is a distributed system migration with data migration, API versioning, and deployment orchestration.

4. **The monolith-first extraction path is well-understood:**
   ```
   Monolith ──► Strangler Fig ──► Extracted Services
                     │
                     ▼
              Identify seams:
              - Which modules change together?
              - Which modules have independent scaling needs?
              - Which modules can be owned by one team?
              - Which modules share data that can be separated?
   ```

**When to start extracting from the monolith:**
- A module has significantly different scaling requirements from the rest
- A module is deployed far more frequently than the rest
- A team needs to own a module end-to-end without blocking on other teams
- A module has clearer boundaries than others and causes deployment conflicts

### When Microservices Help

**Independent Scaling**

The billing service processes a few hundred transactions per day. The catalog service serves tens of thousands of product page views per second. In a monolith, you scale the entire application for the catalog's needs, wasting resources on the billing code. With microservices, you scale the catalog service horizontally and leave billing at one instance.

```
Monolith scaling:     10 instances (all running billing + catalog)
                       = 10× billing cost for 1× billing need

Microservice scaling: 1 billing instance + 10 catalog instances
                       = cost proportional to actual need
```

**Independent Deployment**

The mobile team needs to ship catalog updates daily. The billing team ships monthly. In a monolith, a billing deployment blocks the mobile deployment queue, or vice versa. Microservices let each team deploy on their own schedule without coordination.

**Team Autonomy**

A team owns a service end-to-end: code, data, deployment, monitoring, on-call. They make decisions within their service boundary without cross-team meetings. This reduces coordination overhead and increases ownership.

**Technology Diversity**

The recommendation engine benefits from Python's ML ecosystem. The payment service benefits from Rust's memory safety. The real-time notification service benefits from Go's goroutines. Microservices allow each service to use the best tool for its specific problem.

### When Microservices Hurt

**Distributed System Complexity**

A monolith makes an in-process function call. A microservice makes a network call. Network calls can fail in ways in-process calls cannot: timeouts, partial failures, DNS issues, network partitions. Every inter-service call needs retry logic, circuit breakers, and timeout configuration. This complexity is quadratic in the number of services.

**Network Latency**

What was a microsecond in-process call becomes a millisecond network call. Across a request chain of five services, you've added 5–50ms of latency. For user-facing requests, this is noticeable. For internal processing, this compounds.

**Data Consistency**

In a monolith, a transaction spans multiple tables atomically. In a microservice architecture, each service owns its data. An operation that touches multiple services cannot use a database transaction. You need the saga pattern, eventual consistency, or compensating transactions — each of which adds significant complexity.

```
Monolith:     BEGIN; INSERT orders; UPDATE inventory; COMMIT;
              (atomic — both succeed or both fail)

Microservices: Order Service ──► Inventory Service
              (separate databases — partial failures possible)
              Need: saga, compensating transaction, or reconciliation
```

**Debugging Difficulty**

A single user request that errors might have traversed four services, each with its own logs, its own trace ID format, and its own error handling. Reproducing the bug requires collecting logs from all four services and correlating them by request ID. This is orders of magnitude harder than reading a single log file.

**Operational Overhead**

Each service needs: CI/CD pipeline, deployment infrastructure, monitoring, alerting, on-call rotation, secrets management, database operations, and capacity planning. Ten services means ten of each. This overhead is fixed per service — it doesn't decrease as services mature.

### The Distributed Monolith Anti-Pattern

A **distributed monolith** is a system that has the disadvantages of both monoliths and microservices and the advantages of neither:

- **Coupled deployments:** You must deploy Service A, B, and C together because they share a database schema or have synchronized API contracts.
- **Shared databases:** Services reach into each other's databases, creating hidden coupling that breaks the "independently deployable" contract.
- **Chatty communication:** Services make dozens of synchronous calls per request, creating tight temporal coupling. If Service B is down, Service A cannot function.
- **No team ownership:** Three teams each own pieces of five services, creating coordination overhead that matches — or exceeds — monolithic deployment.

**How distributed monoliths form:**
1. Premature decomposition — splitting into microservices before understanding boundaries
2. Leaking abstractions — services exposing internal schema through their API
3. Synchronous chains — request paths that must traverse six services to complete
4. Shared libraries — a "common" JAR/npm package used by all services that forces coordinated upgrades

**Sign you have a distributed monolith:** A single feature change requires deploying three or more services simultaneously.

### Service Boundaries: Finding Them

**Bounded Contexts from Domain-Driven Design**

A bounded context is a linguistic boundary within a domain where a term has one, unambiguous meaning. In an e-commerce system:

```
Sales Context          Inventory Context       Shipping Context
─────────────────      ──────────────────      ─────────────────
"Product" = item       "Product" = SKU          "Product" = parcel
 being purchased        in warehouse            being shipped
 
"Order" = a sales      "Order" = a pick list   "Order" = shipment
 contract               for warehouse           manifest
```

The word "Product" means different things in each context. A bounded context boundary becomes a natural service boundary because:
- The language is consistent within the boundary
- The data model is coherent within the boundary
- Communication across boundaries is explicit (via APIs or events)
- Teams can work within a boundary without constant coordination

**Heuristics for finding boundaries:**

1. **Language boundaries:** Where the same word means different things to different groups, you have a context boundary.
2. **Change rate boundaries:** Modules that change at different rates should be in different services. The payment processing module changes monthly; the user profile module changes weekly.
3. **Scaling boundaries:** Modules with different resource profiles (CPU-heavy, I/O-heavy, memory-heavy) can be separated.
4. **Team boundaries:** If a team can own a module end-to-end without blocking on other teams, it's a service candidate.
5. **Data boundaries:** If a set of data is always accessed together and rarely with other data, it belongs in one service.

### Inter-Service Communication

**Synchronous: REST and gRPC**

```
Client ──► Service A ──► Service B ──► Service C
           (waits for   (waits for
            response)    response)
```

REST:
- Simple, widely understood, human-readable
- HTTP/1.1 latency overhead per call
- Good for request-response patterns
- Tolerant of polyglot environments

gRPC:
- Binary protocol (Protocol Buffers), lower latency
- Bidirectional streaming
- Strong type contracts via .proto files
- Better for internal high-throughput service-to-service calls

**Downsides of synchronous communication:**
- Temporal coupling — the caller must know the callee is available
- Cascading failures — if Service B is slow, Service A's threads are consumed waiting
- Latency stacking — each hop adds network round-trip time

**Asynchronous: Events and Messages**

```
Client ──► Service A ──► Event Bus ──┬──► Service B (processes event)
                                     └──► Service C (processes event)
           (returns immediately)           
```

Event-driven (pub/sub):
- Services publish events without knowing who consumes them
- Decoupled — publisher doesn't need to know about subscribers
- Good for "something happened" notifications (OrderPlaced, PaymentReceived)
- Requires an event broker (Kafka, RabbitMQ, SNS/SQS, EventBridge)

Message-driven (queue-based):
- Producer sends a message to a queue; consumer processes it
- Good for commands (ProcessRefund, GenerateInvoice)
- Supports retry, dead-letter queues, and idempotency
- Requires a message broker (SQS, RabbitMQ, Azure Service Bus)

**Choosing between sync and async:**

| Concern | Synchronous | Asynchronous |
|---------|-------------|--------------|
| Caller needs response now | Use REST/gRPC | Don't use |
| Process can be deferred | Don't use | Use events/messages |
| Loose coupling needed | Partial | Strong |
| Error handling complexity | Cascading failures | Retry + DLQ |
| Debugging traceability | Request chains | Event chains + correlation IDs |
| Ordering guarantees | N/A | Requires partitioned event streams |

### Data Management: Shared-Nothing and the Saga Pattern

**The Shared-Nothing Principle**

Each microservice owns its data exclusively. No service reads or writes another service's database. The only way to access another service's data is through its API.

```
WRONG:  Service A ──SQL──► Service B's Database
RIGHT:  Service A ──API──► Service B ──SQL──► Service B's Database
```

Why:
- Shared databases create hidden coupling (schema changes break consumers)
- Service B loses control over its own data access patterns (can't optimize queries, can't add caching, can't change schema without breaking Service A)
- Deployment independence breaks (can't change Service B's schema without coordinating with Service A)

**The Saga Pattern for Cross-Service Consistency**

When an operation spans multiple services, you cannot use a distributed transaction (two-phase commit is impractical at scale). Instead, use a saga: a sequence of local transactions, each publishing an event that triggers the next step.

```
Choreographed Saga (event-driven):

Order Service:     CreateOrder → emit OrderCreated
Inventory Service: Receive OrderCreated → ReserveStock → emit StockReserved
Payment Service:   Receive StockReserved → ChargeCard → emit PaymentProcessed  
Shipping Service:  Receive PaymentProcessed → ScheduleShipment → emit ShipmentScheduled

If Payment Fails:
Payment Service:   emit PaymentFailed
Inventory Service: Receive PaymentFailed → ReleaseStock
Order Service:     Receive PaymentFailed → CancelOrder
```

```
Orchestrated Saga (central coordinator):

Saga Orchestrator:
  1. → Order Service: CreateOrder
  2. → Inventory Service: ReserveStock
  3. → Payment Service: ChargeCard
  4. → Shipping Service: ScheduleShipment
  
  If step 3 fails:
  3. ← Payment Service: ChargeCardFailed
  2. → Inventory Service: ReleaseStock (compensating action)
  1. → Order Service: CancelOrder (compensating action)
```

**Compensating actions** are the saga equivalent of rollback. Each step has a corresponding undo operation. If the saga fails at step 3, you execute compensating actions for steps 2 and 1 in reverse order.

### API Gateway Pattern

An API gateway is the single entry point for all client requests. It:

- Routes requests to the appropriate service
- Handles cross-cutting concerns: authentication, rate limiting, TLS termination
- Aggregates responses from multiple services into a single response
- Translates protocols (external REST → internal gRPC)
- Shields internal service topology from clients

```
Clients
  │
  ▼
┌─────────────────┐
│   API Gateway   │  ← Authentication, rate limiting, routing
└────────┬────────┘
    ┌─────┼─────┐
    ▼     ▼     ▼
  User   Order  Catalog
 Service Service Service
```

Trade-offs:
- Pro: Single entry point simplifies client integration
- Pro: Centralized auth, rate limiting, logging
- Con: Single point of failure; must be highly available
- Con: Another deployment unit with its own lifecycle
- Con: Can become a "smart pipe" that absorbs business logic (anti-pattern)

### Service Mesh

A service mesh provides infrastructure-level concerns as a sidecar proxy alongside each service:

- **Traffic management:** Retries, circuit breaking, load balancing
- **Security:** mTLS between services, authorization policies
- **Observability:** Distributed tracing metrics, access logs

```
Service A          Service B
┌──────────┐      ┌──────────┐
│ App Code │      │ App Code │
│   +      │      │   +      │
│ Sidecar  │◄────►│ Sidecar  │
│ (Envoy)  │      │ (Envoy)  │
└──────────┘      └──────────┘
```

**Istio** and **Linkerd** are the most widely used service meshes. They inject Envoy proxies as sidecars and manage them via a control plane.

When you need a service mesh:
- You have 10+ services communicating over the network
- You need mTLS without modifying application code
- You need fine-grained traffic control (canary, mirroring, fault injection)
- You want observability without instrumenting every service

When you don't:
- You have fewer than 10 services — the overhead isn't worth it
- Your API gateway handles routing adequately
- You can add observability via libraries (OpenTelemetry SDK) instead of sidecars

### Observability Challenges

In a monolith, a single log file and a stack trace tell you what went wrong. In a microservice system, a single request may traverse five services, each with its own:

- Log format and aggregation
- Trace ID propagation
- Error handling

**Three pillars of observability in microservices:**

1. **Distributed Tracing:** A single trace ID is propagated across all service boundaries. Tools like Jaeger, Zipkin, and AWS X-Ray collect spans from each service and reconstruct the full request path. Without distributed tracing, debugging cross-service latency is essentially guessing.

2. **Centralized Logging:** All services ship logs to a central aggregation point (ELK Stack, Splunk, CloudWatch Logs, Datadog). Logs are tagged with trace IDs so you can search across services for a specific request.

3. **Metrics and Alerting:** Each service emits metrics (request rate, error rate, latency — the RED method). Prometheus scrapes them; Grafana visualizes them; Alertmanager pages on-call when thresholds are breached.

```
Request Flow (with observability):

Client ──► API Gateway ──► Order Service ──► Inventory Service ──► Payment Service
  │            │                  │                   │                    │
  │         trace:abc         trace:abc           trace:abc           trace:abc
  │         span:1            span:2              span:3              span:4
  │            │                  │                   │                    │
  ▼            ▼                  ▼                   ▼                    ▼
         ┌────────────────────────────────────────────────────────────────┐
         │                 Centralized Observability                      │
         │  Traces (Jaeger) | Logs (ELK) | Metrics (Prometheus)         │
         └────────────────────────────────────────────────────────────────┘
```

### Team Topology: Conway's Law and Inverse Conway

**Conway's Law:** "Organizations which design systems [...] produce designs which are copies of the communication structures of these organizations."

If three teams each own parts of five services, the architecture will reflect that communication structure: coupled, hard to change, requiring cross-team coordination for every feature.

**Inverse Conway Maneuver:** Deliberately structure your organization to produce the architecture you want. If you want services with clear boundaries, create teams with clear boundaries that own services end-to-end.

```
CONWAY'S LAW (passive):
  Organization structure ──► Architecture structure
  (3 teams, each scattered)    (5 coupled services)

INVERSE CONWAY (deliberate):
  Desired architecture ──► Team structure
  (3 bounded contexts)       (3 teams, each owning one service)
```

**Team topologies (from Matthew Skelton and Manuel Pais):**

- **Stream-aligned team:** Owns a service end-to-end, aligned to a value stream (e.g., the "Order" team)
- **Enabling team:** Helps stream-aligned teams adopt capabilities (e.g., platform engineering, observability)
- **Complicated-subsystem team:** Owns a specialized component (e.g., ML recommendation engine)
- **Platform team:** Provides internal platform capabilities (e.g., deployment pipeline, service mesh, auth)

### Real Examples

**Netflix**

Netflix is the most cited microservices success story. They migrated from a monolithic data center architecture to hundreds of microservices on AWS after a major database corruption incident in 2008. Key points:
- They had 1000+ microservices at peak (later consolidating back toward hundreds)
- They built an entire platform team to handle operational concerns (Eureka for discovery, Hystrix for circuit breaking, Zuul for gateway)
- Their scale (200M+ subscribers, massive traffic spikes) justified the investment
- They explicitly warn against adopting their architecture at smaller scale

**Amazon**

Amazon's transition from monolith to services is the origin story of microservices. In the early 2000s, their monolithic C++ application became a deployment bottleneck. They decomposed into services, which led to the creation of AWS (internal infrastructure became a product). Key points:
- The "two-pizza team" concept — each service owned by a team small enough to feed with two pizzas (6–10 people)
- Their service decomposition was driven by organizational scaling, not just technical needs
- They invented API gateway patterns, service discovery, and eventually AWS services to manage the complexity

**Shopify**

Shopify deliberately chose a monolithic architecture ("Shopify is a monolith" is a blog post title). Key points:
- They scaled a Rails monolith to handle Black Friday traffic (millions of requests per minute)
- They use modular monolith patterns: well-defined boundaries within the monolith, no cross-module database access
- They extract services only when they have a proven, recurring need (e.g., their billing service)
- Their position: "If you can't build a well-structured monolith, you won't be able to build well-structured microservices either"

### Decision Framework: Monolith vs Microservices Checklist

Use this checklist to evaluate whether microservices are appropriate for your situation:

**Lean toward Monolith when:**
- [ ] Team size < 10 developers
- [ ] Domain is not yet well understood (you haven't identified bounded contexts)
- [ ] Deployment frequency is manageable (once per day or less)
- [ ] Scaling requirements are uniform across modules
- [ ] No regulatory requirement for independent deployability
- [ ] Operational budget doesn't support 10+ deployment pipelines
- [ ] You can't articulate why a specific module needs to be a separate service
- [ ] The product is pre-product-market fit

**Lean toward Microservices when:**
- [ ] Teams are blocked by each other's deployment schedules
- [ ] Specific modules have 10x different scaling requirements
- [ ] Different parts of the system need different technology stacks
- [ ] Bounded contexts are well-defined and stable
- [ ] You have platform engineering support (CI/CD, observability, service mesh)
- [ ] Independent deployment is a competitive advantage (not just "nice to have")
- [ ] The organization is structured around service boundaries (inverse Conway)
- [ ] You've already extracted at least one service from a monolith successfully

**Yellow flags — reconsider:**
- [ ] You're building microservices "because Netflix does it" (cargo-culting)
- [ ] You have more services than engineers
- [ ] A single feature change regularly requires deploying 3+ services
- [ ] Services share a database
- [ ] You can't trace a request end-to-end across services

## Build It

### Step 1: Minimal Version

Draw the service boundary diagram for a familiar system (e.g., an e-commerce platform). Identify which modules belong in which bounded context and justify boundaries based on language consistency, change rate, and data ownership.

```
E-Commerce Bounded Contexts:

┌──────────────────────────────────────────────────────┐
│  Catalog Context       │  Order Context     │  Shipping Context
│  ─────────────────     │  ─────────────     │  ─────────────────
│  Product (for display) │  Order (contract)  │  Shipment (logistics)
│  Category              │  OrderLine         │  Tracking
│  Inventory (stock)     │  Payment           │  Carrier
│                        │  Refund            │
└──────────────────────────────────────────────────────┘
    Owned by:              Owned by:            Owned by:
    Catalog team           Order team           Logistics team
    Owns DB: yes           Owns DB: yes         Owns DB: yes
```

### Step 2: Realistic Version

Create a decision matrix for your current (or imagined) project. Score each factor, and use the total to guide your architecture decision:

| Factor | Weight (1-5) | Monolith Score | Microservices Score | Weighted Monolith | Weighted Microservices |
|--------|:---:|:---:|:---:|:---:|:---:|
| Team size < 10 | 5 | 5 | 1 | 25 | 5 |
| Domain maturity | 4 | 5 | 2 | 20 | 8 |
| Scaling variance | 3 | 2 | 5 | 6 | 15 |
| Deployment independence | 4 | 2 | 5 | 8 | 20 |
| Operational budget | 3 | 5 | 1 | 15 | 3 |
| ... | ... | ... | ... | ... | ... |

If "Weighted Monolith" >> "Weighted Microservices," start monolithic. If the reverse, consider microservices. If close, start monolithic and extract services as needs become clear.

## Use It

**Kubernetes** (kubernetes.io) is the production system that manages microservice deployment at scale. Key concepts from our lesson that map to real infrastructure:

- **Service discovery** — Kubernetes Services provide DNS-based discovery, eliminating the need for hard-coded service addresses
- **API Gateway** — Ingress controllers (NGINX, Envoy) route external traffic to services, mirroring our API Gateway pattern
- **Service Mesh** — Istio and Linkerd inject sidecar proxies, implementing the service mesh pattern we described
- **Observability** — OpenTelemetry provides distributed tracing, metrics, and logging as a unified standard

Look at the Istio project at `istio.io` for how a production service mesh implements traffic management, security, and observability across microservices.

## Read the Source

- **Istio** — `pilot/pkg/networking/core/` — how a service mesh control plane configures Envoy sidecars for routing, retries, and circuit breaking
- **Envoy** — `source/common/router/` — how a production sidecar proxy handles request routing, weighted load balancing, and retry policies

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`microservices_decision.md`** — A decision framework document you can use for any project to evaluate monolith vs microservices.

## Exercises

1. **Easy** — Draw the bounded context map for a university registration system (Student, Course, Enrollment, Billing). Identify where the same term means different things in different contexts.
2. **Medium** — Take a monolithic application you work on (or imagine one). Identify two modules that would benefit from extraction into separate services and two modules that should stay in the monolith. Justify each decision using the framework from this lesson.
3. **Hard** — Design a choreographed saga for a food delivery system (Order placed → Restaurant accepts → Driver assigned → Food delivered → Payment processed). Write out the events, the compensating actions for each step, and what happens if the driver is never assigned.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Microservices | "Just split it into services" | Independently deployable services organized around business capabilities, each owning its data. The key word is "independently." |
| Monolith-first | "Start big then split" | Build a monolith first, learn where the boundaries are, then extract services only when you have evidence they're needed. |
| Distributed Monolith | "Our microservices are coupled" | A system deployed as multiple services but coupled in deployment, data, or communication — the worst of both worlds. |
| Bounded Context | "A service boundary" | A linguistic and data boundary from DDD where a term has one unambiguous meaning. The foundation for finding service boundaries. |
| Saga Pattern | "Distributed transactions" | A sequence of local transactions with compensating actions for rollback — not a transaction, not atomic, but eventually consistent. |
| API Gateway | "The front door" | A single entry point for external clients that routes, authenticates, and aggregates across internal services. |
| Service Mesh | "Network middleware" | Infrastructure that provides traffic management, security (mTLS), and observability as sidecar proxies — without modifying application code. |
| Shared-Nothing | "Each service has its own DB" | The principle that each service exclusively owns its data. No service reads or writes another service's database directly. |
| Conway's Law | "Org structure = architecture" | The observation that system architectures mirror the communication structures of the organizations that build them. |
| Inverse Conway | "Design teams first" | Deliberately structuring teams to produce the architectural boundaries you want — team structure drives architecture, not the reverse. |

## Further Reading

- Martin Fowler — "Microservices" (martinfowler.com/articles/microservices.html) — the foundational article that defined the term
- Sam Newman — "Building Microservices" (O'Reilly, 2nd Edition) — the most comprehensive book on microservices design, deployment, and operation
- Martin Fowler — "MonolithFirst" (martinfowler.com/bliki/MonolithFirst.html) — the argument for starting monolithic
- "Team Topologies" by Matthew Skelton and Manuel Pais — organizing teams for fluid software delivery
- "Designing Data-Intensive Applications" by Martin Kleppmann — Chapter on consistency and consensus in distributed systems
- Shopify Engineering Blog — "Deconstructing the Monolith" — how Shopify scales a modular monolith
- Netflix TechBlog — microservices at scale with real production insights
- "Saga Pattern" by Chris Richardson — pattern comparison of choreography vs orchestration sagas
- Istio documentation (istio.io) — how service meshes implement observability, traffic management, and security