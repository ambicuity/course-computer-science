# Event-Driven Architectures

> When your system needs to react, not just respond.

**Type:** Learn
**Languages:** TypeScript, Go
**Prerequisites:** Phase 16 lessons 01–09
**Time:** ~60 minutes

## Learning Objectives

- Distinguish events from commands and queries, and know when each is appropriate.
- Implement a publish/subscribe event bus with typed handlers.
- Model a choreography-based saga for distributed transactions.
- Reason about eventual consistency, idempotency, and schema evolution.
- Decide when event-driven architecture helps and when it hurts.

## The Problem

You're building an e-commerce platform. When a customer places an order, five things must happen: inventory reserved, payment charged, shipping scheduled, confirmation emailed, analytics recorded. In a request-response world, the order service calls each downstream service directly. This works—until one service is down and the whole chain fails, or until you need to add a sixth step without touching the order service code.

The core tension: **coupling vs. resilience**. Synchronous chains couple the caller to every downstream service's availability. Event-driven architectures break that coupling by replacing direct calls with asynchronous notifications—events—that interested parties react to independently.

## The Concept

### Events vs Commands vs Queries

These three message types serve fundamentally different purposes:

| Property | Query | Command | Event |
|----------|-------|---------|-------|
| Intent | Ask for data | Tell someone to do something | Notify that something happened |
| Direction | Request → Response | Sender → Receiver (imperative) | Producer → Anyone listening (declarative) |
| Return value | Yes (the data) | Maybe (acknowledgment) | No (fire and forget) |
| Audience | One specific handler | One specific handler | Zero or more subscribers |
| Reversibility | N/A | Sender expects outcome | Sender doesn't care who reacts |
| Naming | `GetOrderDetails` | `ReserveInventory` | `OrderCreated` |
| Tense | N/A | Imperative present | Past tense (it already happened) |

A **query** asks for information: "What is the current inventory for SKU-1234?" It expects a response and is idempotent by nature.

A **command** instructs: "Reserve 5 units of SKU-1234 for order 9942." It targets a specific handler. The sender cares whether it succeeded. Commands can fail—the handler might reject them.

An **event** states a fact: "OrderCreated(orderId=9942, sku=SKU-1234, qty=5)." It has already happened. The producer doesn't know or care who processes it. Events are the past tense of your system's state changes.

**Mistaking commands for events** is the most common design error. If you publish `ReserveInventory` as an event, you're implying the inventory service *must* subscribe—but events carry no guarantee of handling. If you publish `OrderCreated` and the inventory service reacts by reserving stock, the event correctly encapsulates "something happened" without mandating the reaction.

### Publish/Subscribe Pattern

The publish/subscribe (pub/sub) pattern decouples producers from consumers through a intermediary:

```
Producer → [Event Bus / Message Broker] → Consumer A
                                    └──→ Consumer B
                                    └──→ Consumer C
```

**Key properties:**
- **Producers don't know consumers exist.** The order service publishes `OrderCreated` without knowing about inventory, shipping, or analytics.
- **Consumers subscribe independently.** The analytics team can start listening to `OrderCreated` without any code change on the producer side.
- **Multiple consumers receive the same event.** All subscribers process the same event independently.

The event bus (or message broker) sits in the middle, handling routing, delivery guarantees, and sometimes persistence. In its simplest form, it's an in-process dispatcher. At production scale, it's Apache Kafka, RabbitMQ, or AWS SQS.

### Event Bus

An event bus manages the lifecycle of events:

1. **Registration:** Handlers subscribe to event types.
2. **Publication:** Producers emit events to the bus.
3. **Dispatch:** The bus routes each event to all registered handlers.
4. **Error handling:** Failed handlers can retry, dead-letter, or be skipped.

A minimal in-process event bus is a map of event type → list of handler functions. Production buses add persistence, ordering, partitioning, and replay.

### Saga Pattern for Distributed Transactions

In a monolith, a database transaction ensures atomicity: either all steps commit or none do. In a distributed system, there is no single database, so you need a different strategy.

A **saga** is a sequence of local transactions where each step publishes an event that triggers the next step. If any step fails, compensating transactions undo the previous steps.

There are two saga coordination styles:

**Choreography** — Each service listens for events and decides what to do:

```
OrderService → publishes OrderCreated
  InventoryService → reacts, publishes InventoryReserved
    PaymentService → reacts, publishes PaymentProcessed
      ShippingService → reacts, publishes OrderShipped
```

No central coordinator. Services are autonomous. Easy to add new participants. Hard to see the full flow by reading any single service's code.

**Orchestration** — A central orchestrator tells each service what to do:

```
Orchestrator → calls InventoryService → calls PaymentService → calls ShippingService
                (on failure) → calls compensating transactions in reverse
```

Explicit control flow. Easier to debug and monitor. The orchestrator becomes a single point of failure and coupling.

Choreography scales better for simple flows. Orchestration scales better for complex, multi-branch flows.

### Eventual Consistency

When services react to events asynchronously, the system is **eventually consistent**—the state converges to consistency over time, but there are windows where different services have different views of the world.

Example: After `OrderCreated`, the order service considers the order "pending" immediately, but the inventory service hasn't reserved stock yet. For a brief window, the customer sees "order confirmed" while inventory still shows full availability.

This is not a bug—it's a design tradeoff. You gain availability and partition tolerance (the AP in CAP) but sacrifice immediate consistency (the C). The question is never "should we have eventual consistency?" but "how long is the convergence window, and what happens during it?"

**Mitigations:**
- **Compensating actions:** If inventory reservation fails after the order is confirmed, emit `OrderCancelled` to roll back.
- **Idempotent handlers:** Processing the same event twice produces the same result, so redelivery is safe.
- **Saga timeouts:** If a step doesn't complete within a deadline, trigger compensation.

### When Events Help

| Scenario | Why events help |
|----------|----------------|
| **Decoupling** | Producers don't depend on consumers. Add/remove subscribers without code changes. |
| **Audit trail** | Events are immutable facts. You can replay them to reconstruct any past state. |
| **Replay & recovery** | After a failure, replay events from a log to rebuild state without restoring from backup. |
| **Fan-out** | One event triggers N independent reactions. No need for the producer to know about all N. |
| **Temporal decoupling** | Producer and consumer don't need to be running simultaneously. The bus buffers events. |
| **Load leveling** | Spikes in production are absorbed by the bus and processed at the consumer's pace. |

### When Events Hurt

| Problem | Why it hurts |
|---------|--------------|
| **Debugging difficulty** | A single business action flows through multiple services asynchronously. Tracing the full path requires correlation IDs and distributed tracing. |
| **Ordering** | Events may arrive out of order. Consumers must handle `OrderShipped` arriving before `OrderCreated`. Mitigate with sequence numbers or causal timestamps. |
| **Exactly-once processing** | Networks fail, retries happen. At-least-once delivery means you must design for duplicates. At-most-once means you may lose events. Exactly-once requires idempotence + transactions, which is expensive. |
| **Schema evolution** | Consumer code changes over time. Events written months ago must still be processable. You need backward-compatible schemas or a migration strategy. |
| **Cognitive overhead** | "Where does this event go?" requires searching multiple codebases. The implicit flow is harder to reason about than a call stack. |
| **Testing** | Integration tests must set up event buses, simulate timing, and handle eventual consistency assertions. |

### Event Schema Evolution

Events are data contracts that live as long as your system produces them. Changing an event's schema is not like changing an internal struct—it affects every consumer that has ever processed that event type.

**Backward-compatible changes (safe):**
- Adding a new optional field with a default value
- Adding a new event type (old consumers ignore it)
- Widening a field type (e.g., `int32` → `int64`)

**Breaking changes (dangerous):**
- Removing a field that consumers depend on
- Renaming a field
- Narrowing a field type (e.g., `string` → `enum`)
- Changing the semantics of an existing field

**Strategies:**
- **Version in the event type:** `OrderCreatedV2` alongside `OrderCreated`. Old consumers handle V1, new ones handle V2.
- **Schema registry:** Enforce compatibility rules (e.g., Avro with CONFLUENT compatibility modes).
- **Consumer-driven contracts:** Consumers declare what they need; producers must satisfy all registered contracts.

### Idempotency

An operation is **idempotent** if performing it multiple times has the same effect as performing it once. In event-driven systems, idempotency is not optional—it's a survival strategy.

Events can be delivered more than once. Networks fail mid-acknowledgment. Consumers crash after processing but before committing. The only safe assumption is **at-least-once delivery**.

**Idempotency techniques:**

1. **Idempotency keys:** Include a unique key (e.g., `orderId`) in every event. Consumers track which keys they've processed and skip duplicates.
2. **Natural idempotency:** "Set account status to ACTIVE" is naturally idempotent—running it 10 times changes nothing after the first.
3. **Conditional writes:** Database upserts or conditional updates ("UPDATE inventory SET qty = qty - 5 WHERE order_id IS NULL") prevent double-processing.

```
Non-idempotent:  PaymentService.charge(orderId, $50)
                  → Second delivery charges $100 total

Idempotent:      PaymentService.chargeIfNotProcessed(orderId, $50)
                  → Second delivery sees orderId already processed, skips
```

### Real-World Event Systems

| System | Model | Strengths | Weaknesses |
|--------|-------|-----------|------------|
| **Apache Kafka** | Distributed commit log | High throughput, replay, persistence, partitioning | Complex ops, ordering only within partition |
| **RabbitMQ** | Traditional message broker | Flexible routing, AMQP protocol, dead-letter queues | Lower throughput, no native replay |
| **AWS SQS** | Managed queue | Simple, serverless, auto-scaling | No fan-out (use SNS + SQS), limited ordering |
| **NATS** | Lightweight pub/sub | Ultra-low latency, simple deployment, JetStream for persistence | Less ecosystem tooling |

**Kafka** treats events as an append-only log. Consumers read at their own offset. This enables replay: rewind the offset, reprocess all events. Kafka is ideal when you need durability and replay.

**RabbitMQ** routes messages through exchanges to queues. Once a message is acked and removed from the queue, it's gone. Ideal for task queues and request-reply patterns.

**SQS** provides managed queues with at-least-once delivery. Combine with SNS for fan-out. Ideal when you don't want to operate infrastructure.

**NATS** prioritizes speed and simplicity. Core NATS is fire-and-forget (at-most-once). JetStream adds persistence and at-least-once. Ideal for lightweight, high-frequency events.

### Event-Driven vs Request-Response: When to Use Which

| Factor | Request-Response | Event-Driven |
|--------|------------------|--------------|
| **You need an immediate answer** | Yes | No |
| **Multiple systems need the same data** | Call each one manually | Publish once, fan out automatically |
| **The producer should not know about consumers** | No—tight coupling | Yes—full decoupling |
| **You need to replay historical events** | No—state is ephemeral | Yes—events are the log of truth |
| **You need ACID transactions across services** | Consider a monolith or distributed transaction | Saga with compensations |
| **Debugging simplicity matters more than scalability** | Call stack is explicit | Events flow implicitly |
| **The system must remain available during partial failures** | One service down blocks the call | Other services continue independently |

**Pragmatic guidance:** Start with request-response. Introduce events when you hit a specific pain point: too many direct dependencies, need for replay, need for fan-out, or need for temporal decoupling. Most production systems are hybrid—synchronous for queries and commands, asynchronous for events and notifications.

## Build It

We'll implement an event bus with pub/sub in both Go and TypeScript, then model a choreography-based saga for order processing: `OrderCreated → ReserveInventory → ProcessPayment → ShipOrder`, with compensating transactions on failure.

See `code/main.go` and `code/main.ts` for the full implementations.

### Step 1: Minimal Event Bus

The smallest correct version: a map of event type → handler list, a `Publish` method that calls all handlers, and simple structs for events.

### Step 2: Saga with Compensation

The realistic version: add saga orchestration with compensating transactions. If `ProcessPayment` fails, the saga publishes `PaymentFailed`, which triggers `ReleaseInventory` (the compensating action for `ReserveInventory`). Each handler is idempotent—it tracks processed event IDs and skips duplicates.

## Use It

**Apache Kafka's partition model:** Kafka partitions topics by key, guaranteeing ordering within a partition but not across partitions. This is the production answer to the ordering problem we discussed. Read the Kafka source at `core/src/main/scala/kafka/` to see how the commit log and consumer group protocols work.

**NATS JetStream's ack model:** NATS uses a three-state acknowledgment: `ack` (processed), `nak` (rejected, redeliver), and `term` (dead-letter). This is more nuanced than the simple success/failure our event bus handles. See `server/server.go` in the NATS source.

**Confluent Schema Registry:** In production, you'd use a schema registry to enforce backward-compatible evolution of Avro/Protobuf schemas. Our code uses raw structs, but the schema evolution strategies outlined above would be enforced by the registry.

## Read the Source

- **Kafka:** `core/src/main/scala/kafka/log/Log.scala` — how Kafka appends events to the commit log and manages offsets.
- **NATS:** `server/server.go` — the NATS server's routing and delivery logic.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **events_reference.md** — A quick-reference card covering event vs command vs query, saga patterns, idempotency techniques, and when to choose event-driven vs request-response.

## Exercises

1. **Easy** — Reimplement the Go or TypeScript event bus from scratch without looking at the lesson code. Verify it produces the same output.
2. **Medium** — Extend the saga to include an `EmailConfirmation` step after `ShipOrder`. Handle failure: if the email service is down, the order should still be considered shipped (this is a best-effort notification, not a critical path).
3. **Hard** — Implement event sourcing on top of the event bus: instead of storing current state, store the sequence of events. Add a `Replay` method that reconstructs state by replaying all events from the beginning. Compare the order of events after replay with the original order to verify correctness.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Event | "We'll just send an event" | An immutable record of something that already happened, named in past tense, with no expectation of a response |
| Command | "Send a message to the inventory service" | An imperative instruction to a specific handler that expects the handler to attempt the action and report success or failure |
| Event bus | "The message queue" | The infrastructure that routes events from producers to subscribers, handling dispatch, delivery, and sometimes persistence |
| Saga | "A distributed transaction" | A sequence of local transactions where each step publishes an event triggering the next, with compensating transactions for rollback |
| Choreography | "Just let services react" | Saga coordination where each service decides independently what to do based on events—no central controller |
| Orchestration | "Have a coordinator" | Saga coordination where a central orchestrator explicitly calls each step and manages compensating actions |
| Idempotency | "Make it safe to retry" | Processing the same event twice produces the same result as processing it once—essential for at-least-once delivery |
| Eventual consistency | "It'll sync eventually" | The system guarantees convergence to a consistent state, but temporarily different services may see different data |
| Compensating transaction | "Undo that step" | An action that semantically reverses a previous step in a saga (e.g., `ReleaseInventory` reverses `ReserveInventory`) |
| Schema evolution | "Just add the field" | Managing changes to event structure over time so old consumers don't break when producers emit new event versions |

## Further Reading

- Martin Fowler, ["Sagas" pattern](https://martinfowler.com/articles/patterns-of-distributed-systems/saga.html)
- Confluent, [Kafka Streams documentation](https://docs.confluent.io/platform/current/streams/)
- Chris Richardson, *Microservices Patterns* (Manning, 2018), Chapter 7: "Using Sagas to Maintain Data Consistency"
- Hewitt, et al., "The Saga Pattern" (original 1987 paper)
- AWS Architecture Blog, ["Using Amazon SQS and SNS for Fanout"](https://aws.amazon.com/blogs/compute/building-loosely-coupled-architectures-with-amazon-sqs-and-amazon-sns/)
- NATS Documentation, [JetStream](https://docs.nats.io/nats-concepts/jetstream)