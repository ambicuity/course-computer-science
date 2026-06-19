# ADR Bundle — Capstone Refactoring

This bundle contains three ADRs documenting the architectural decisions for refactoring the `OrderService` god class. Each ADR references specific lessons from Phase 16.

---

# ADR-001: Adopt Hexagonal Architecture

**Status:** Accepted

**Date:** 2026-05-14

**Lesson References:** L02 (Coupling), L03 (SOLID — SRP, DIP), L09 (Hexagonal Architecture), L08 (DDD — Aggregates)

## Context

The current `OrderService` is a god class spanning ~200 lines. It mixes:

- **Business logic** — pricing calculations, discount rules, order validation
- **Persistence** — direct SQL queries to a `DatabaseConnection`
- **Notification** — direct SMTP calls for order confirmation emails
- **External integration** — HTTP calls to an `InventoryService`

This violates Single Responsibility (L03 — one class, four reasons to change). The domain logic is tightly coupled to infrastructure (L02 — `OrderService` imports concrete `DatabaseConnection`, `SmtpClient`, and `InventoryApi`). Any change to the database schema, email provider, or inventory API requires modifying the order processing code.

Currently, testing `OrderService` requires a real database, SMTP server, and inventory API — making unit tests impossible and integration tests slow and flaky.

## Decision

We will adopt hexagonal architecture (Alistair Cockburn's ports and adapters pattern, as covered in L09) with three layers:

### Domain Layer
- Pure business logic with **zero** infrastructure imports
- Contains `Order` entity, `Money` and `OrderId` value objects
- Domain events (`OrderCreatedEvent`) live here
- Depends on nothing outside the domain

### Ports Layer
- Interfaces defining contracts the domain needs
- `OrderRepository` — abstract persistence contract
- `NotificationPort` — abstract notification contract
- `InventoryPort` — abstract inventory contract
- `EventPublisher` — abstract event publishing contract
- Ports are owned by the domain; implemented by adapters

### Adapters Layer
- Concrete implementations of ports for specific technologies
- `InMemoryOrderRepository` — for testing
- `PostgresOrderRepository` — for production (future)
- `SmtpNotificationAdapter` — real email sending
- `HttpInventoryAdapter` — real inventory API calls
- `InProcessEventBus` — synchronous event dispatching

### Dependency Rule
```
domain → ports → adapters
         ↑         ↑
         │         │
  adapters implement ports
  domain never imports adapters
```

The domain layer can only import from ports. Adapters can import ports (to implement them) but the domain never imports adapters. This inverts the dependency (L03 — Dependency Inversion Principle).

## Consequences

**Positive:**
- Domain logic is testable in isolation with in-memory adapters — no database, no network
- Swapping infrastructure (e.g., Postgres → DynamoDB) requires only a new adapter, no domain changes
- Clear module boundaries make the codebase navigable (L21)
- Each class has a single responsibility (L03 — SRP)
- New team members can understand the system by reading ports first, then domain, then adapters
- Adapters can be developed and tested independently

**Negative:**
- More files and directories to navigate initially
- Requires discipline to keep domain imports pure — linter rules recommended
- Overkill for trivially small projects (but appropriate for anything with real business logic)
- The indirection through ports adds a layer of abstraction that must be documented

**Neutral:**
- This is the foundational ADR. ADR-002 and ADR-003 build upon it.

---

# ADR-002: Separate Commands from Queries (CQRS Lite)

**Status:** Accepted

**Date:** 2026-05-14

**Lesson References:** L03 (SOLID — ISP), L11 (CQRS and Event Sourcing), L09 (Hexagonal Architecture)

## Context

With hexagonal architecture in place (ADR-001), the application layer needs orchestration. Currently, `OrderService` has methods like:

- `processOrder()` — **command** (creates order, modifies state, has side effects)
- `getOrder()` — **query** (reads state, no side effects)
- `listOrders()` — **query** (reads state, no side effects)
- `cancelOrder()` — **command** (modifies state, publishes event)

Mixing commands and queries in one service violates Interface Segregation (L03 — ISP). Consumers that only read orders still depend on the full `OrderService` interface including write methods. This also makes it impossible to:
- Optimize reads independently from writes (e.g., cache read models)
- Scale read and write sides differently
- Test commands and queries with different strategies
- Introduce eventual consistency for reads without affecting writes (L11)

## Decision

We will adopt CQRS Lite (L11) by splitting the application layer into two services:

### OrderCommandService
- `createOrder(customerId, items)` → `OrderId` — validates, prices, persists, publishes `OrderCreatedEvent`
- `cancelOrder(orderId)` — validates, updates status, publishes `OrderCancelledEvent`
- Commands mutate state and return minimal results (just an ID or void)
- Commands publish domain events via the `EventPublisher` port

### OrderQueryService
- `getOrder(orderId)` → `OrderDto` — returns a read-optimized view
- `listOrders(filter)` → `OrderDto[]` — returns filtered, paginated results
- `getOrderHistory(orderId)` → `OrderHistoryDto` — returns audit trail
- Queries are read-only and return DTOs, not domain entities
- Both services share the same `OrderRepository` port for now

### DTOs (Data Transfer Objects)
- `OrderDto` is a flat, serialized view of an order — no domain behavior
- Domain entities (`Order`) are never exposed outside the application layer
- This prevents leaky abstractions and allows read models to diverge from write models

### Future Considerations
- We may add separate read-model storage (materialized views) later
- Event sourcing (L11) could replace the write-side repository
- For now, both sides share one database — no eventual consistency concerns yet

## Consequences

**Positive:**
- Commands and queries have clearly different contracts (ISP satisfied)
- Read models can be optimized independently (add caching, different DB schema)
- Command handlers are simpler — they validate, mutate, and publish; no returning complex data
- Query handlers are simpler — they read and transform; no side effects
- Natural entry point for full CQRS + event sourcing later (L11)
- Test commands for state changes; test queries for correct projections

**Negative:**
- Two services to maintain instead of one
- Some duplication in order of operations (command creates, query reads)
- Must resist the temptation to query the write model directly (enforce through code review — L07)
- If we add separate read stores later, we need to handle eventual consistency

**Neutral:**
- This ADR builds on ADR-001 (hexagonal architecture). Commands and queries both use ports.
- ADR-003 (event-driven) enables commands to publish events that query side can consume.

---

# ADR-003: Adopt Event-Driven Order Processing

**Status:** Accepted

**Date:** 2026-05-14

**Lesson References:** L08 (DDD — Domain Events), L10 (Event-Driven Architectures), L11 (CQRS), L04 (Observer Pattern)

## Context

The current order processing flow is a synchronous chain inside `OrderService.processOrder()`:

```
validate → calculate pricing → check inventory → reserve inventory
→ persist order → send email notification → log audit entry
```

Problems with this approach:

1. **Fragility** — If the email server is down, order creation fails entirely. The customer can't place an order because a notification step failed.
2. **Coupling** — `OrderService` directly calls `InventoryApi` and `SmtpClient`. Adding a new side effect (e.g., webhook notification, analytics event) requires modifying `OrderService` (violates Open/Closed — L03 OCP).
3. **Performance** — The customer waits for every step including external API calls and email delivery.
4. **No extensibility** — New concerns (fraud detection, analytics, shipping) require modifying the core order flow.

With hexagonal architecture (ADR-001) and CQRS (ADR-002) in place, we have the structural foundation to decouple side effects through events.

## Decision

We will adopt event-driven processing using domain events (L08, L10):

### Domain Events
- `OrderCreatedEvent` — emitted when an order is successfully placed
  - Payload: `{ orderId, customerId, items, total, timestamp }`
- `OrderCancelledEvent` — emitted when an order is cancelled
  - Payload: `{ orderId, reason, timestamp }`

### Event Flow
```
OrderCommandService.createOrder()
    → validates order
    → calculates pricing
    → persists order via repository
    → publishes OrderCreatedEvent via EventPublisher

OrderCreatedEvent subscribers:
    → InventorySubscriber reserves inventory via InventoryPort
    → NotificationSubscriber sends confirmation via NotificationPort
    → AuditSubscriber logs via AuditPort
```

### Event Publisher Port
```typescript
interface EventPublisher {
  publish(event: DomainEvent): void;
  subscribe(eventType: string, handler: EventHandler): void;
}
```

### InProcessEventBus (Adapter)
- Synchronous dispatching for now (simple, no message broker)
- Subscribers execute in registration order
- Subscriber errors are caught and logged, not propagated to the command
- This matches the Observer pattern from L04

### Eventual Considerations
- If we need asynchronous processing, we can swap `InProcessEventBus` for an `AsyncMessageBus` adapter (RabbitMQ, Kafka, etc.)
- Event payloads are immutable records of what happened
- Events enable new subscribers without modifying the publisher (OCP satisfied)

## Consequences

**Positive:**
- Order creation succeeds even if email or inventory API is slow/down (eventual handling)
- New side effects are added by subscribing to events — no core changes (OCP)
- Natural fit for CQRS: command side publishes, other subscribers build read models
- Domain events are part of the domain model (L08 — DDD) — they express business meaning
- Enables future event sourcing (L11) — events as source of truth
- Simplifies `OrderCommandService` — it validates, persists, and publishes; that's it

**Negative:**
- Debugging event flows requires tracing through subscribers (harder than synchronous call chains)
- Must handle eventual consistency — inventory might not be reserved instantly
- Subscriber errors need dead-letter queues or retry logic (not yet implemented)
- Event ordering matters — `InProcessEventBus` is synchronous for now, but async changes ordering

**Neutral:**
- This ADR depends on ADR-001 (ports and adapters for `EventPublisher`) and ADR-002 (command service publishes, subscribers handle side effects).
- The event schema is part of the domain's public contract (L14 — versioning considerations apply).