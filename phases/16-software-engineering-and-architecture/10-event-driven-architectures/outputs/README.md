# Event-Driven Architecture Reference Card

## Events vs Commands vs Queries

| | Query | Command | Event |
|---|---|---|---|
| **Intent** | Get data | Make something happen | Notify something happened |
| **Tense** | N/A | Imperative (`ReserveInventory`) | Past (`OrderCreated`) |
| **Audience** | One handler | One handler | Zero or more subscribers |
| **Returns** | Data | Acknowledgment | Nothing (fire & forget) |
| **Failure mode** | Error response | Rejection | Nobody listens |

**Design rule:** If the sender cares whether the receiver processes it, it's a command. If the sender only cares about recording the fact, it's an event.

## Saga Pattern

### Choreography (Decentralized)
```
OrderCreated → InventoryService reacts → InventoryReserved
                                   → PaymentService reacts → PaymentProcessed
                                                            → ShippingService reacts
On failure: PaymentFailed → InventoryService releases stock
```

**Pros:** Easy to extend, no single point of failure  
**Cons:** Hard to debug, implicit flow across services

### Orchestration (Centralized)
```
Orchestrator calls: reserveInventory() → chargePayment() → shipOrder()
On failure: Orchestrator calls compensating actions in reverse
```

**Pros:** Explicit flow, easy to monitor  
**Cons:** Orchestrator is bottleneck and coupling point

## Event Bus

```
Publish(event) → Bus routes to all subscribed handlers
Subscribe(eventType, handler) → Register interest
```

**Production equivalents:** Kafka (commit log), RabbitMQ (AMQP routing), SQS (managed queues), NATS (lightweight pub/sub)

## Idempotency Techniques

| Technique | How it works | Example |
|-----------|-------------|---------|
| Deduplication key | Track processed event IDs, skip duplicates | `if processed[event.id] { skip }` |
| Natural idempotency | Operation is inherently safe to repeat | `SET status = 'ACTIVE'` |
| Conditional write | Database-level guard against double-processing | `INSERT ... IF NOT EXISTS` |

**Rule:** Assume at-least-once delivery. Design every handler to be idempotent.

## Eventual Consistency

```
Time →  t0        t1           t2              t3
Order:  Created   Pending      Pending         Confirmed
Inventory: Full   Reserved     Reserved        Reserved
Payment:  None    None         Processing      Charged
```

Between t0 and t3, different services have different views. This is expected, not a bug.

**Mitigate with:** compensating actions, idempotent handlers, saga timeouts.

## Schema Evolution

| Change | Safe? | Why |
|--------|-------|-----|
| Add optional field with default | Yes | Old consumers ignore it |
| Add new event type | Yes | Old consumers never subscribed |
| Remove existing field | No | Consumers may depend on it |
| Rename a field | No | Deserialization breaks |
| Narrow a type (string→enum) | No | Old values may not fit |

**Strategy:** Version event types (`OrderCreatedV2`) or use a schema registry.

## When to Use Events vs Request-Response

| Use events when... | Use request-response when... |
|--------------------|------------------------------|
| Multiple services need the same data | You need an immediate answer |
| Producers should not know about consumers | Caller needs confirmation of success |
| You need replay and audit trail | The interaction is a simple query |
| Services must remain available during partial failures | Debugging simplicity matters most |
| You need temporal decoupling (async processing) | You need strong consistency guarantees |

## Compensating Transactions

Every saga step must have a semantic inverse:

| Action | Compensating Action |
|--------|---------------------|
| `ReserveInventory` | `ReleaseInventory` |
| `ChargePayment` | `RefundPayment` |
| `CreateShippingLabel` | `CancelShippingLabel` |
| `SendConfirmationEmail` | *(best-effort, no compensation needed)* |

## Quick Decision Framework

```
Need immediate response? → Request-response
Need fan-out to N systems? → Events
Need replay/recovery? → Events (with persistent log)
Need ACID across services? → Saga (not distributed transaction)
Need to add consumers later? → Events (decoupled)
Need simple debugging? → Request-response (or orchestration)
```