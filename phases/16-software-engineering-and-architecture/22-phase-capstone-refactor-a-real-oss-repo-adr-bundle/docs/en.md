# Phase Capstone — Refactor a Real OSS Repo + ADR Bundle

> The capstone that ties every lesson in Phase 16 together: read a real codebase, smell the debt, write ADRs, refactor, test, and document.

**Type:** Build
**Languages:** TypeScript, Markdown
**Prerequisites:** Phase 16 lessons 01–21
**Time:** ~150 minutes

## Learning Objectives

- Read and navigate an unfamiliar codebase using systematic strategies from L21.
- Identify technical debt and architecture smells using frameworks from L19 and L20.
- Write Architecture Decision Records (ADRs) that document both current state and planned changes.
- Apply SOLID principles (L03), naming/cohesion/coupling heuristics (L02), and refactoring mechanics (L06) to real code.
- Restructure toward hexagonal architecture (L09), separate commands from queries (L11), and adopt event-driven patterns (L10).
- Write tests that protect refactored code, connecting forward to Phase 17.
- Produce commit messages and ADRs that tell the architectural story.

## The Problem

You've spent 21 lessons learning individual skills: how to name things (L02), how SOLID keeps classes honest (L03), which patterns still matter (L04–L05), how to refactor safely (L06), how to review code (L07), how DDD shapes domains (L08), how hexagonal ports-and-adapters separate concerns (L09), how events decouple systems (L10), how CQRS splits reads from writes (L11), when microservices help or hurt (L12), how APIs evolve (L13–L14), how monorepos and dependency management work (L15–L16), how CI/CD should look (L17), why observability is a design concern (L18), how to measure and pay down debt (L19), how to write ADRs (L20), and how to read large codebases (L21).

But skills in isolation don't make you a software engineer. The moment you face a real open-source project — thousands of lines, tangled dependencies, no tests, unclear boundaries — you need to **combine every lesson** into a single workflow: read, diagnose, decide, refactor, test, document.

That's what this capstone does.

## The Capstone Workflow

The capstone follows seven stages, each tied to specific phase lessons:

```
┌─────────────────────────────────────────────────────────────────┐
│                    CAPSTONE WORKFLOW                            │
│                                                                 │
│  1. Pick a real OSS project         ← connects to L21           │
│  2. Read and understand it          ← L21 reading techniques    │
│  3. Identify debt & smells          ← L19 debt + L02 coupling   │
│  4. Write ADRs for changes          ← L20 ADRs                  │
│  5. Refactor with SOLID+patterns    ← L03, L06, L09, L10, L11   │
│  6. Write tests for new code        ← bridges to Phase 17       │
│  7. Document via ADRs + commits     ← L20, L14                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Stage 1: Pick a Small but Real Open-Source Project

**Connecting to:** L21 (Reading Large Codebases)

You need a project that is:
- **Real** — actually used, not a toy
- **Small enough** — under ~3,000 lines so you can hold it in your head
- **Smelly enough** — has genuine design problems worth fixing

Good candidates on GitHub:
- A small Express middleware library with a god-class index.ts
- A CLI tool where every command is in one file
- A data-processing script with no type safety and hardcoded constants

**Exercise:** Find a project with <3K LOC that has at least two of: god class, tight coupling, missing tests, mixed concerns. Clone it and run it locally. Confirm you can build and run the existing (possibly minimal) test suite.

### Stage 2: Read and Understand It

**Connecting to:** L21 (Reading Large Codebases)

Use the three-pass reading method from L21:

**Pass 1 — Forest (10 min):** Read the README, scan the directory tree, identify entry points (main, index, app), and trace the top-level dependency graph. Draw a box-and-arrow diagram showing module relationships.

**Pass 2 — Trees (20 min):** Read each module's public API (exported types, classes, functions). For our companion code example, the "before" code in `code/main.ts` has:
- `OrderService` — a 200-line god class handling orders, pricing, inventory, notifications, and persistence
- Direct database calls scattered through business logic
- No interfaces — everything concretely coupled

**Pass 3 — Leaves (15 min):** Read the internal implementation of critical paths. Follow a sample request from entry point to side effect. You'll notice:
- Business rules mixed with I/O
- No error handling discipline
- No domain types — primitives everywhere (L02)

**Checkpoint:** Write a one-page "Codebase Inventory" listing every module, its responsibilities, its dependencies, and its smells. This inventory becomes the foundation for your ADRs.

### Stage 3: Identify Technical Debt and Architecture Issues

**Connecting to:** L19 (Technical Debt: Measure, Pay Down, Negotiate), L02 (Naming, Cohesion, Coupling)

Use the debt quadrant from L19:

| Quadrant | Example in Smelly Codebase |
|----------|---------------------------|
| **Reckless + Deliberate** | "We knew we should use interfaces but shipped without them" |
| **Reckless + Inadvertent** | "We didn't realize business logic was coupled to the DB driver" |
| **Prudent + Deliberate** | "We chose a simple JSON file for config, knowing we'd need a real store later" |
| **Prudent + Inadvertent** | "We didn't know the domain model needed events until we saw the coupling" |

Apply L02's cohesion/coupling lens to each module:

**Cohesion problems** (module does too many things):
- `OrderService.processOrder()` computes pricing, checks inventory, sends emails, writes to DB — 4 reasons to change (violates Single Responsibility from L03)
- Discount calculations are embedded in the order flow rather than isolated
- Notification logic (email templates, SMTP config) lives next to business rules

**Coupling problems** (module knows too much about others):
- `OrderService` directly imports and instantiates `DatabaseConnection`, `SmtpClient`, and `InventoryApi`
- No dependency inversion — you can't swap the database without touching order logic (L03 Dependency Inversion Principle)
- Domain types are primitives — `string` for OrderId, `number` for Money — so type errors surface at runtime

**Debt inventory template:**

```
| # | Smell | Location | Severity | Quadrant | Lesson Ref |
|---|-------|----------|----------|----------|-------------|
| 1 | God class | OrderService | High | Reckless+Inadvertent | L02, L03 |
| 2 | Tight coupling to DB | OrderService.processOrder | High | Reckless+Deliberate | L02, L09 |
| 3 | No tests | * | Critical | Reckless+Deliberate | L06 |
| 4 | Mixed concerns | OrderService.notify | Medium | Reckless+Inadvertent | L03, L09 |
| 5 | Primitive obsession | All domain types | Medium | Inadvertent | L02, L08 |
```

### Stage 4: Write ADRs for the Planned Changes

**Connecting to:** L20 (Architecture Decision Records)

ADRs are the bridge between diagnosis and action. You write them **before** refactoring so the reasoning is captured, not retrofitted.

**ADR-001: Adopt Hexagonal Architecture**

```
# ADR-001: Adopt Hexagonal Architecture

## Status: Proposed

## Context
OrderService is a god class with ~200 lines mixing business logic,
database access, SMTP notifications, and inventory API calls. This
violates SRP (L03) and makes changes ripple unpredictably (L02 coupling).

We need to separate the domain (what the business does) from
infrastructure (how the business communicates with the outside world).

## Decision
We will adopt hexagonal architecture (L09) with:
- A `domain` layer containing pure business logic with no I/O
- A `ports` layer defining interfaces (abstract contracts)
- An `adapters` layer implementing those interfaces for specific
  technologies (Postgres, SMTP, REST API)

## Consequences
+ Domain logic becomes testable in isolation
+ Swapping infrastructure requires only new adapters
+ Clear boundaries enforce dependency rule: domain → ports → adapters
- More files and directories to navigate
- Requires discipline to keep domain imports pure
```

**ADR-002: Separate Commands from Queries**

```
# ADR-002: Separate Commands from Queries (CQRS Lite)

## Status: Proposed

## Context
Currently, OrderService.processOrder() both modifies state (creating
orders, updating inventory) and returns computed results (order totals,
confirmation numbers). Read operations like getOrder and listOrders
go through the same persistence layer as writes.

This mixed read/write model (L11) makes caching impossible, complicates
testing, and conflates two fundamentally different concerns.

## Decision
We will adopt CQRS Lite (L11):
- Command side: OrderCommandService handles creates, updates, cancels
- Query side: OrderQueryService handles reads and lookups
- Each side has its own model and can evolve independently
- Both sides share the same database for now (no event sourcing yet)

## Consequences
+ Read models can be optimized for display without affecting write models
+ Commands have clear, void-ish intent — easier to reason about
+ Enables future event sourcing (L10, L11) without re-architecting
- Two models to maintain
- Eventual consistency needed if we add separate read stores later
```

**ADR-003: Adopt Event-Driven Order Processing**

```
# ADR-003: Adopt Event-Driven Order Processing

## Status: Proposed

## Context
Order processing currently chains synchronous calls:
validate → price → check inventory → persist → notify
If any step fails, the whole operation fails. If notification is slow,
the user waits. The system is brittle (L10).

## Decision
We will adopt an event-driven approach (L10):
- Order creation emits an OrderCreatedEvent
- Inventory reservation, notification, and audit logging subscribe
  to this event
- Each subscriber handles failures independently
- The domain uses domain events (L08 aggregates can emit events)

## Consequences
+ Steps are decoupled — a downstream failure doesn't crash the order
+ New subscribers (analytics, webhooks) attach without modifying core
+ Natural fit for CQRS command handlers to publish events
- Requires an event bus or message dispatcher
- Debugging event flows is harder than synchronous call chains
- Must handle eventual consistency
```

### Stage 5: Refactor Using SOLID, Patterns, and Architecture

**Connecting to:** L03 (SOLID), L06 (Refactoring Mechanics), L09 (Hexagonal), L10 (Events), L11 (CQRS), L08 (DDD)

#### Refactoring Mechanics (from L06)

Refactoring is **not** "rewrite everything." Follow the L06 mechanics:

1. **Ensure tests exist first.** If none exist, write characterization tests that pin current behavior.
2. **Take small steps.** Each refactoring step should be one mechanical transformation.
3. **Run tests after every step.** Green = safe to continue.
4. **Commit after each coherent step.** Commit messages reference ADRs.

The refactoring sequence for this capstone:

```
Step 1: Extract method — pull pricing logic out of processOrder()
         → commit: "extract: isolate pricing calculation (ADR-001)"

Step 2: Extract interface — create OrderRepository port
         → commit: "refactor: introduce OrderRepository port (ADR-001)"

Step 3: Move method — move DB calls into InMemoryOrderRepository adapter
         → commit: "refactor: move persistence to adapter (ADR-001)"

Step 4: Extract class — create OrderPricingService from extracted pricing
         → commit: "extract: OrderPricingService (SRP, ADR-001)"

Step 5: Introduce domain event — OrderCreatedEvent
         → commit: "feat: add OrderCreatedEvent (ADR-003)"

Step 6: Split command/query — OrderCommandService vs OrderQueryService
         → commit: "refactor: split command/query services (ADR-002)"

Step 7: Wire event bus — subscribers for inventory, notification, audit
         → commit: "feat: event-driven processing (ADR-003)"
```

#### SOLID in Action (from L03)

| Principle | Before | After |
|-----------|--------|-------|
| **S** — Single Responsibility | OrderService does everything | Each class has one reason to change |
| **O** — Open/Closed | Adding notification channel requires modifying OrderService | Add new NotificationAdapter, no core changes |
| **L** — Liskov Substitution | No interfaces, can't substitute | Repositories are interchangeable via ports |
| **I** — Interface Segregation | OrderService is one big interface | Command and Query interfaces are separate |
| **D** — Dependency Inversion | OrderService depends on concrete DB/SMTP | Domain depends on abstractions (ports) |

#### Hexagonal Architecture (from L09)

The "after" structure in `code/main.ts` demonstrates:

```
src/
├── domain/           ← Pure business logic, zero I/O imports
│   ├── Order.ts      ← Entity with domain behavior
│   ├── Money.ts      ← Value object replacing primitive number
│   ├── OrderId.ts    ← Value object replacing primitive string
│   └── events/       ← Domain events
│       └── OrderCreatedEvent.ts
├── ports/            ← Interfaces defining contracts
│   ├── OrderRepository.ts
│   ├── NotificationPort.ts
│   ├── InventoryPort.ts
│   └── EventPublisher.ts
├── adapters/         ← Infrastructure implementations
│   ├── InMemoryOrderRepository.ts
│   ├── SmtpNotificationAdapter.ts
│   ├── HttpInventoryAdapter.ts
│   └── InProcessEventBus.ts
├── application/      ← Use case orchestration
│   ├── OrderCommandService.ts
│   └── OrderQueryService.ts
└── tests/            ← Unit tests for domain + integration for adapters
    ├── domain/
    │   ├── Order.test.ts
    │   ├── Money.test.ts
    │   └── events/
    │       └── OrderCreatedEvent.test.ts
    ├── application/
    │   ├── OrderCommandService.test.ts
    │   └── OrderQueryService.test.ts
    └── adapters/
        └── InMemoryOrderRepository.test.ts
```

#### Domain-Driven Design (from L08)

The refactored code demonstrates DDD concepts:
- **Entities** — `Order` has identity (`OrderId`) and lifecycle
- **Value Objects** — `Money` is immutable, compared by value
- **Aggregates** — `Order` is the aggregate root; items belong to it
- **Domain Events** — `OrderCreatedEvent` signals that an order was placed
- **Bounded Context** — The order context is isolated from inventory/notification contexts

### Stage 6: Write Tests for the Refactored Code

**Connecting to:** Bridges to Phase 17

Tests serve two roles here:

1. **Characterization tests** (before refactoring) — Pin existing behavior so you can refactor safely (L06).
2. **Specification tests** (after refactoring) — Test the new architecture's contracts.

For domain objects, tests are pure and fast — no database, no network:

```typescript
describe('Order', () => {
  it('calculates total from line items', () => {
    const order = Order.create(customerId, [
      { productId: 'p1', quantity: 2, unitPrice: Money.from(10) },
      { productId: 'p2', quantity: 1, unitPrice: Money.from(25) },
    ]);
    expect(order.total).toEqual(Money.from(45));
  });

  it('emits OrderCreatedEvent on creation', () => {
    const order = Order.create(customerId, items);
    expect(order.domainEvents).toHaveLength(1);
    expect(order.domainEvents[0]).toBeInstanceOf(OrderCreatedEvent);
  });
});
```

For application services, tests use in-memory adapters (no real DB needed):

```typescript
describe('OrderCommandService', () => {
  it('creates an order and publishes event', async () => {
    const repo = new InMemoryOrderRepository();
    const eventBus = new InProcessEventBus();
    const service = new OrderCommandService(repo, eventBus, inventoryPort, notificationPort);

    const orderId = await service.createOrder(customerId, items);

    const saved = await repo.findById(orderId);
    expect(saved).toBeDefined();
    expect(eventBus.publishedEvents).toHaveLength(1);
  });
});
```

**Key testing principle:** Domain tests have zero dependencies. Application tests use fakes. Adapter tests test one infrastructure concern at a time. This testability is a **result** of good architecture, not a luxury you add later.

### Stage 7: Document via ADRs and Commit Messages

**Connecting to:** L20 (ADRs), L14 (Versioning and Deprecation)

Each commit in the refactoring sequence (Stage 5) references its ADR. This creates a navigable history:

```
git log --oneline
a3f7c2e feat: event-driven processing (ADR-003)
b1e4d8a refactor: split command/query services (ADR-002)
9c2f1a0 feat: add OrderCreatedEvent (ADR-003)
7d5b3e7 extract: OrderPricingService (SRP, ADR-001)
4a1c9f3 refactor: move persistence to adapter (ADR-001)
2e8f6d0 refactor: introduce OrderRepository port (ADR-001)
1b3a5c7 extract: isolate pricing calculation (ADR-001)
```

Commit message conventions (from L14 thinking):
- `refactor:` for mechanical transformations that preserve behavior
- `feat:` for new capabilities (events, CQRS split)
- `extract:` for pulling methods/classes out of larger units
- Always reference the ADR number so future readers can find the reasoning

After completing the refactoring, update each ADR's status from "Proposed" to "Accepted" or "Superseded."

## The Companion Code

The `code/main.ts` file contains the complete before-and-after example:

1. **Before section** — A smelly `OrderService` god class that violates SOLID, has tight coupling, no domain types, and no tests.
2. **After section** — Refactored code with hexagonal architecture, domain types, CQRS split, event-driven processing, and comprehensive tests.
3. **Comparison** — A `demonstrate()` function that runs both versions side-by-side and shows the architectural improvements.

Run it with `npx ts-node code/main.ts` to see the before/after demonstration.

## ADR Bundle

The `code/notes.md` file contains three complete ADRs:
- **ADR-001:** Adopt Hexagonal Architecture
- **ADR-002:** Separate Commands from Queries (CQRS Lite)
- **ADR-003:** Adopt Event-Driven Order Processing

Each ADR follows the format from L20: Status, Context, Decision, Consequences.

## Concept Map: How This Capstone Ties Phase 16 Together

```
L01 What Makes SW Engineered        ← "Why are we doing all this?"
  │
  ├── L02 Naming/Cohesion/Coupling   ← Identify smells, enforce domain types
  │     │
  ├── L03 SOLID                       ← SRP for god class, DIP for ports
  │     │
  ├── L04 GoF Patterns               ← Strategy for adapters, Observer for events
  │     │
  ├── L05 Modern Patterns             ← Functional core / imperative shell
  │     │
  ├── L06 Refactoring Mechanics       ← Small steps, tests between each
  │     │
  ├── L07 Code Review                 ← Review each ADR-driven commit
  │     │
  ├── L08 DDD                         ← Entities, VOs, Aggregates, Events
  │     │
  ├── L09 Hexagonal Architecture      ← Ports, Adapters, Dependency Rule
  │     │
  ├── L10 Event-Driven                ← Domain events, pub/sub, eventual consistency
  │     │
  ├── L11 CQRS                        ← Command/Query split
  │     │
  └── L20 ADRs                        ← Document every decision above
```

## Exercises

1. **Easy** — Take the "before" code from `code/main.ts`. Write three characterization tests that pin the current behavior of `OrderService.processOrder()`. Run the "after" code and verify these tests still pass conceptually.

2. **Medium** — Choose a real open-source project (<3K LOC). Apply the capstone workflow: read it (L21), inventory the smells (L19), write three ADRs (L20), and outline (don't implement) the refactoring sequence with commit messages referencing your ADRs.

3. **Hard** — Extend the "after" code with a new bounded context (e.g., Shipping). Write ADR-004 proposing how it integrates with the Order context via domain events. Implement the port, adapter, and application service. Write tests. Submit the diff.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Capstone | "final project" | A multi-skill exercise that proves you can combine isolated lessons into an end-to-end workflow |
| God class | "big class that does a lot" | A class violating SRP that accumulates responsibilities because no one extracted them |
| Characterization test | "test that locks in existing behavior" | A test written before refactoring that describes what the code *does*, not what it *should* do |
| Refactoring sequence | "series of small changes" | An ordered list of mechanical transformations, each preserving behavior, referenced to ADRs |
| Debt inventory | "list of things that are wrong" | A structured catalog of smells, their locations, severity, and which lessons address them |
| ADR bundle | "collection of decision records" | A set of ADRs that together describe a coherent architectural change, cross-referencing each other |

## Connections to Other Phases

- **Phase 17 (Testing):** The tests written in Stage 6 use techniques from the testing phase. The key insight: testability is an architectural property, not an afterthought.
- **Phase 18 (Security):** Hexagonal architecture (L09) makes it easier to add security adapters (auth, validation) without touching domain logic.
- **Phase 19 (Performance):** CQRS (L11) enables separate optimization of read vs. write paths.

## Further Reading

- **"Working Effectively with Legacy Code"** — Michael Feathers. The canonical guide to adding tests to untested code before refactoring.
- **"Refactoring"** — Martin Fowler. The catalogue of mechanical transformations used in Stage 5.
- **"Clean Architecture"** — Robert C. Martin. The dependency rule and why domain logic must not import infrastructure.
- **"Domain-Driven Design"** — Eric Evans. Entities, value objects, aggregates, and bounded contexts.
- **"ADR GitHub repository"** — https://adr.github.io — The ADR template and examples used in L20.