# CQRS and Event Sourcing

> When your read model and write model have fundamentally different shapes — or when every state change is a story worth telling — you need patterns beyond CRUD.

**Type:** Learn
**Languages:** TypeScript, Rust
**Prerequisites:** Phase 16 lessons 01–10
**Time:** ~75 minutes

## Learning Objectives

- Explain why separating reads from writes (CQRS) can improve scalability and model clarity.
- Implement an event-sourced domain model where state is derived from an append-only log of events.
- Build projections that reconstruct read-optimized views from raw events.
- Decide when CQRS and event sourcing are worth the complexity cost — and when they are not.
- Compare a hand-built event store against production systems like EventStoreDB and Kafka.

## The Problem

You are building a banking system. The write side must enforce business rules: you cannot withdraw more than your balance, accounts must be opened before deposits, and overdraft limits must be respected. The read side is completely different: the customer dashboard needs current balance, the compliance team needs a full transaction history, and the analytics team needs daily aggregates. A single relational model serving both roles forces compromises — the normalized schema is hard to query for reports, and the denormalized schema makes it easy to accidentally violate business rules.

This is the core tension that CQRS and Event Sourcing resolve. Without these patterns, either your write model gets polluted with read concerns (adding redundant columns "just for the dashboard"), or your read model becomes slow (joining ten tables for every page load). The moment you try to make code other people can read, change, and ship — at scale, over years — a single model becomes a bottleneck.

## The Concept

### CQRS: Command Query Responsibility Segregation

CQRS starts from a simple observation: **the shape of data you write is rarely the shape of data you read.**

In a traditional CRUD application, a single model serves double duty:

```
┌─────────────────────────┐
│      One Model          │
│  ┌─────┐    ┌─────┐    │
│  │Write│◄──►│Read │    │
│  └─────┘    └─────┘    │
└─────────────────────────┘
```

CQRS splits this into two models, each optimized for its purpose:

```
┌───────────────┐    ┌───────────────┐
│  Write Model  │    │  Read Model   │
│  (Commands)   │    │  (Queries)    │
│               │    │               │
│ Enforces      │    │ Optimized     │
│ business      │───►│ for queries   │
│ rules,        │    │ and display    │
│ invariants    │    │               │
└───────────────┘    └───────────────┘
```

**Command Model (Write Side):**
- Accepts commands (imperative: "Deposit $50", "Open Account")
- Validates business rules
- Emits events describing what happened
- Never answers queries — it only processes commands
- Optimized for consistency, not read performance

**Query Model (Read Side):**
- Accepts queries (declarative: "What is the balance of account X?")
- Built from projections of events
- Denormalized for fast reads
- May be stale (eventual consistency)
- Can have multiple read models for different use cases

**Eventual Consistency:** The read model is not updated synchronously. Events flow from the write model to update projections asynchronously. There is a window — usually milliseconds — where the read model may not yet reflect the latest write. This is acceptable for dashboards and reports but dangerous for processes that must read their own writes (read-your-writes consistency requires extra work).

**When CQRS helps:**
- High read/write disparity (100 reads per write) — you can scale read replicas independently
- Complex business rules that benefit from a focused domain model
- Multiple read representations (dashboard, audit log, analytics) with different shapes
- Teams can develop read and write sides independently

**When CQRS is overkill:**
- Simple CRUD apps where reads and writes have the same shape
- Small teams that cannot maintain two models
- Real-time consistency requirements that cannot tolerate eventual consistency
- No read/write disparity to justify the split

### Event Sourcing: Store Events, Not State

Traditional persistence stores **current state** — the latest row in a table. Event sourcing stores **every event that led to that state**.

```
State-based:    Account { balance: $150 }          ← only the present
Event-sourced:  AccountOpened($100)                 ← the full story
                MoneyDeposited($50)                  ← every change
                MoneyWithdrawn($20)                  ← traceable
```

**The Event Store** is an append-only log. You never update or delete events — you only append new ones. This gives you:

1. **Complete audit trail** — every change is recorded with who, what, and when
2. **Temporal queries** — "What was the balance on March 15?" is a first-class operation
3. **Replay capability** — rebuild state from scratch, fix bugs by replaying with corrected logic
4. **Natural fit for CQRS** — the write model emits events; projections consume them

**Replaying Events to Rebuild State:**

```
events: [Opened($100), Deposited($50), Withdrew($20)]
         ──────────────────────────────────────────────
replay: balance = 0
        + Opened($100)   → balance = 100
        + Deposited($50) → balance = 150
        + Withdrew($20)  → balance = 130
```

**Projections / Materialized Views:**

A projection is a function that consumes events and produces a read-optimized view. You can have multiple projections from the same event stream:

```
events: [Opened($100), Deposited($50), Withdrew($20)]
    │
    ├──► BalanceProjection    → { balance: 130 }
    ├──► HistoryProjection    → [Opened, Deposited, Withdrew]
    └──► DailySumProjection   → { "2024-01-15": +130 }
```

**Snapshots for Performance:**

When an aggregate has thousands of events, replaying all of them on every command becomes expensive. Snapshots solve this by periodically saving the computed state:

```
snapshot at event 1000: { balance: 50000 }
events 1001-1050: [Deposited($100), Withdrew($50), ...]

replay: load snapshot → apply events 1001-1050 → current state
        (50 events instead of 1050)
```

**Event Sourcing vs State-Based Persistence:**

| Aspect | State-Based | Event Sourced |
|--------|-------------|--------------|
| Stored | Current state only | Full history of changes |
| Audit | Needs extra logging | Built in |
| Temporal queries | Impossible without extra design | Natural |
| Debugging | "What is the state?" | "How did we get here?" |
| Schema change | Migrate in place | Replay with new logic |
| Complexity | Simple | Higher operational cost |
| Performance | Direct reads | Requires projections |

### Combining CQRS with Event Sourcing

These two patterns are independent but complementary. CQRS says "separate reads from writes." Event sourcing says "store events, not state." Together:

```
            Command                   Event                  Query
Client ──► Command Handler ──► Event Store ──► Projection ──► Read Model
              │                    │                               │
              │                    │    ┌─────────────┐            │
              │                    └───►│  Event Store │            │
              │                         │  (append only)│           │
              ▼                         └─────────────┘            ▼
         Aggregate                 ┌──────────────────┐      Read Model
         (rebuilds from            │   Projections    │      (denormalized)
          events)                  │   balance view   │
                                   │   history view   │
                                   │   daily summary  │
                                   └──────────────────┘
```

1. Client sends a **command** to the write side
2. The command handler loads the aggregate, replays events to get current state
3. The handler validates business rules against current state
4. If valid, the handler **emits events** to the event store
5. Projections consume events and update the **read model**
6. Client queries the read model for display

### The Complexity Cost

These patterns are powerful but expensive:

**Debugging** becomes harder. A bug might not appear until the projection processes an event. You need tracing across the command → event → projection pipeline.

**Schema evolution** is real. Events are stored forever, so old event formats must remain readable. You need upcasters or versioned deserializers to handle evolving schemas.

**Eventual consistency** means the read model can be stale. Users might see stale data after a write. If read-your-writes consistency matters, you need to route reads through the write model or use subscriptions.

**When to use:**
- Audit requirements (financial systems, healthcare, legal)
- Temporal queries ("what was the state at time X?")
- Complex business logic that benefits from a rich domain model
- Systems with dramatically different read and write patterns
- Regulatory compliance that demands full history

**When NOT to use:**
- Simple CRUD applications (blog posts, user profiles)
- Small teams without the bandwidth for two models
- No audit or temporal query requirements
- Real-time consistency is mandatory
- When relational databases already solve your problem

### Real Examples

**EventStoreDB** — A purpose-built event store. Stores events as immutable streams, handles projections server-side, provides subscriptions for real-time read model updates. Used in financial trading, healthcare, and logistics systems where audit is non-negotiable.

**Kafka as Event Store** — Apache Kafka's append-only log makes it a viable event store. Events are written to topics (streams), consumers build projections. Kafka adds replay, retention policies, and partition-based scalability. However, it lacks built-in projection support and requires more infrastructure compared to dedicated event stores.

**Axon Framework (Java)** — A full CQRS+ES framework that manages command buses, event stores, and projection updates. Shows how these patterns look in production with sagas, event replay, and snapshotting.

**Lagom (Scala/Java)** — Microservice framework built on CQRS+ES principles with persistent entities and read-side processors.

## Build It

We will build a bank account using event sourcing with CQRS. The domain is intentionally simple — banking is the classic teaching example because the business rules (no negative balances, overdraft limits) are universally understood, and the read/write split is natural (balance queries vs. transaction processing).

### Step 1: Minimal Version

The minimal version has:
- Three event types: `AccountOpened`, `MoneyDeposited`, `MoneyWithdrawn`
- An append-only event store (`Vec` in Rust, array in TypeScript)
- A command handler that validates and emits events
- A projection that replays events to compute current balance

### Step 2: Realistic Version

The realistic version adds:
- Overdraft protection with configurable limits
- Multiple projections (balance, transaction history, daily summary)
- Snapshots to skip replaying all events on every command
- Proper error handling with domain-specific error types
- A separate query model that reads from projections, never from the event store directly

## Use It

**EventStoreDB** (https://eventstore.com) is the production system that implements event sourcing natively. Key differences from our hand-built version:

- **Stream-based storage** — Events are grouped into streams (one per aggregate), not one global list. Our implementation uses a single Vec; EventStoreDB uses per-stream semantics with expected version checks for optimistic concurrency.
- **Projections engine** — EventStoreDB runs projections server-side in JavaScript, updating read models atomically. Our projections run in-process synchronously.
- **Subscriptions** — Real systems use persistent subscriptions to push events to projection handlers. Our pull-based model is simpler but misses events if the projection crashes mid-processing.
- **Snapshots** — EventStoreDB stores snapshots as special events in the same stream. Our snapshots are separate.

**Kafka** as an event store demonstrates the "infrastructure as event store" approach. Compare with our implementation:
- **Partitioning** — Kafka partitions events by key for scalability. Our single Vec has no partitioning.
- **Retention** — Kafka has time/size-based retention and compaction. Our Vec grows forever.
- **Consumer groups** — Kafka allows multiple consumers to share projection work. Our projections are single-threaded.

Look at EventStoreDB's source at `src/EventStore.Core/Data/` for how production event types are structured, and `src/EventStore.Core/Services/Storage/` for the append-only storage engine.

## Read the Source

- **EventStoreDB** — `src/EventStore.Core/Data/EventRecord.cs` — how a production event record is structured (with metadata, type info, and causation IDs)
- **Axon Framework** — `modules/messaging/src/main/java/org/axonframework/eventhandling/EventMessage.java` — production event message with tracking metadata

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`cqrs_reference.md`** — A quick-reference card comparing CQRS patterns, event sourcing decisions, and when to use each.

## Exercises

1. **Easy** — Implement a `CloseAccount` command and `AccountClosed` event. Ensure no operations can be performed on a closed account.
2. **Medium** — Add a `DailyLimitProjection` that enforces daily withdrawal limits. Commands should check this projection before allowing withdrawals.
3. **Hard** — Implement snapshotting at every 10 events. Load the snapshot first, then replay only events after the snapshot. Verify that the final state matches a full replay.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| CQRS | "Just use CQRS" | Separate your write model (commands, validation, invariants) from your read model (queries, denormalized views). Two models, not one. |
| Event Sourcing | "Store everything as events" | Persist state changes as an immutable sequence of events. Current state is derived by replaying — never stored directly. |
| Projection | "A read model builder" | A function that consumes events and produces a query-optimized view. Multiple projections can consume the same events. |
| Command | "A write operation" | An intent to change state: "Deposit $50." Commands can be rejected if business rules are violated. |
| Event | "Something that happened" | A fact that has occurred: "MoneyDeposited($50)." Events are immutable — once written, never changed or deleted. |
| Aggregate | "The domain object" | The entity that enforces business rules. Loads events, applies them to compute current state, then emits new events. |
| Snapshot | "A cached state" | A saved copy of the aggregate's computed state at a point in time, used to avoid replaying all events from the beginning. |
| Eventual Consistency | "The reads might be stale" | The read model catches up to the write model asynchronously. There is a delay between a write being committed and it becoming visible in reads. |
| Event Store | "The database for events" | An append-only log that stores events. Never updated or deleted — events are immutable facts. |
| Schema Evolution | "Changing event formats" | The challenge of handling old event formats when your domain model changes over time. Requires upcasters or versioned deserializers. |

## Further Reading

- Martin Fowler — "CQRS" (martinfowler.com/bliki/CQRS.html) — the original article that popularized the pattern
- Greg Young — "CQRS Documents" — the foundational paper by the pattern's creator
- EventStoreDB Documentation — docs.eventstore.com — production event store with built-in projections
- "Versioning in an Event Sourced System" by Greg Young — the definitive guide to schema evolution
- "Building Event-Driven Microservices" by Adam Bellemare — covers event modeling, streaming, and CQRS at scale
- Vaughn Vernon — "Implementing Domain-Driven Design" — Chapter on event sourcing and aggregates
- Apache Kafka Documentation — kafka.apache.org — for understanding Kafka as an event store alternative