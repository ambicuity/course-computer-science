# CQRS & Event Sourcing — Quick Reference

## CQRS at a Glance

| Aspect | Command Side (Write) | Query Side (Read) |
|--------|----------------------|-------------------|
| Input | Commands (imperative intent) | Queries (declarative requests) |
| Purpose | Validate business rules, emit events | Return optimized views |
| Model | Normalized, domain-focused | Denormalized, query-focused |
| Consistency | Strong within aggregate | Eventually consistent |
| Storage | Event store (append-only) | Projections / materialized views |
| Scaling | Vertical (complex logic) | Horizontal (read replicas) |

## Event Sourcing Core

```
State = fold(apply, initialState, events)
```

- **Events** are immutable facts — never updated or deleted
- **Event Store** is append-only — the source of truth
- **Aggregate** rebuilds state by replaying events
- **Projection** consumes events to build read-optimized views
- **Snapshot** saves computed state at a version to avoid full replays

## Decision Framework

### Use CQRS When
- Read/write ratio is high (100:1 or more)
- Multiple read representations with different shapes
- Complex business rules benefit from a focused write model
- Teams can own read and write sides independently

### Use Event Sourcing When
- Audit trail is required (finance, healthcare, legal)
- Temporal queries needed ("what was the state at time X?")
- Complex domain logic with many transitions
- Bug investigation requires understanding how state was reached

### Skip Both When
- Simple CRUD with no audit requirements
- Small team that cannot maintain two models
- Real-time consistency is mandatory
- Relational database already solves the problem

## Event Types for Banking Example

| Event | Meaning | Effect on Balance |
|-------|---------|-------------------|
| `AccountOpened` | Account created with initial deposit | balance = initialDeposit |
| `MoneyDeposited` | Money added to account | balance += amount |
| `MoneyWithdrawn` | Money removed from account | balance -= amount |
| `OverdraftLimitSet` | Overdraft allowance configured | overdraftLimit = limit |
| `AccountClosed` | Account closed, no further operations | isOpen = false |

## Key Patterns

### Command → Event Flow
```
Command → Handler → Validate → Emit Events → Append to Store
                                                   ↓
                                              Projection → Read Model
```

### Snapshot Load
```
1. Load latest snapshot (version N)
2. Load events from version N to current
3. Apply events on top of snapshot state
4. Return current state
```

### Multiple Projections
```
Events ──┬──► BalanceProjection → { balance, available }
         ├──► HistoryProjection → [transaction list]
         └──► DailySumProjection → { date: net change }
```

## Complexity Costs

| Cost | Mitigation |
|------|-----------|
| Eventual consistency | Document SLAs, add read-your-writes guarantees where needed |
| Schema evolution | Use upcasters, version events, never delete old formats |
| Debugging across boundaries | Distributed tracing, correlation IDs on events |
| Operational overhead | Monitoring projections for lag, alert on stuck consumers |
| Learning curve | Start with one aggregate, expand gradually |

## Production Systems

| System | Type | Key Feature |
|--------|------|-------------|
| EventStoreDB | Dedicated event store | Built-in projections, subscriptions, stream metadata |
| Apache Kafka | Distributed log | Partition-based scalability, retention policies, compaction |
| Axon Framework | CQRS+ES framework | Command bus, event store, sagas, snapshotting |
| Marten (PostgreSQL) | Document + event store | Uses PostgreSQL as event store with LINQ projections |

## Glossary

- **Command** — Intent to change state; can be rejected
- **Event** — Fact that happened; immutable once stored
- **Aggregate** — Entity enforcing business rules, rebuilt from events
- **Projection** — Function that builds a read model from events
- **Snapshot** — Saved aggregate state at a version for performance
- **Eventual Consistency** — Read model lags behind write model briefly
- **Upcaster** — Function that transforms old event formats to new formats
- **Stream** — Ordered sequence of events for one aggregate