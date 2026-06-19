# Message Queues & Streams — Kafka, NATS

> Decouple now, deliver later — message queues turn synchronous chains into asynchronous pipelines.

**Type:** Learn
**Languages:** Go
**Prerequisites:** Phase 11 lessons 01–18
**Time:** ~75 minutes

## Learning Objectives

- Explain why message queues exist: decoupling producers from consumers, buffering bursty traffic, and enabling asynchronous communication patterns.
- Distinguish point-to-point (queue) from publish/subscribe (topic) messaging and state when to use each.
- Describe Kafka's architecture: topics partitioned across brokers, offset-based consumption, consumer groups, and log-based retention.
- Explain Kafka's delivery semantics: at-most-once, at-least-once, and exactly-once — and what mechanisms enable each.
- Describe NATS's subject-based addressing with wildcards and contrast core NATS (at-most-once) with JetStream (durable, at-least-once/exactly-once).
- Compare RabbitMQ, Kafka, NATS, and Redis Streams along the dimensions of durability, ordering, throughput, and latency.
- Build a message broker in Go with topics, key-based partitioning, consumer groups with offset tracking, and partition rebalancing on consumer failure.

## The Problem

You run an e-commerce platform. When a customer places an order, five things must happen: charge the credit card, reserve inventory, send a confirmation email, update analytics, and notify the warehouse. If you wire these as synchronous HTTP calls, every service must be online for an order to succeed. A slow analytics service blocks the confirmation email. A warehouse outage prevents payment processing. The system is fragile — any single downstream failure cascades upstream.

Worse, traffic is bursty. Black Friday generates 100× normal load. Your payment service can handle 10×, but not 100×. Without a buffer, requests pile up, time out, and the entire system collapses.

You need **decoupling**: producers should fire messages without knowing or caring who consumes them. You need **buffering**: a burst of messages should queue up rather than overwhelm receivers. You need **asynchronous delivery**: the producer moves on immediately; the consumer catches up at its own pace. Message queues and event streams solve all three problems.

## The Concept

### Producer → Broker → Consumer

```
Without a broker:                    With a broker:

Producer ──HTTP──► Service A         Producer ──► Broker ──► Service A
        │                                          ──► Service B
        ├──HTTP──► Service B                       ──► Service C
        │
        └──HTTP──► Service C

Any service down = producer fails.   Services consume independently at their own pace.
Synchronous, fragile, coupled.      Async, resilient, decoupled.
```

The broker sits between producers and consumers. It does three things:

1. **Decouples** — producers don't know who reads their messages or how many consumers exist.
2. **Buffers** — if consumers are slow, messages queue up in the broker rather than overwhelming receivers.
3. **Delivers asynchronously** — the producer sends and forgets; the consumer reads when ready.

### Point-to-Point vs. Publish/Subscribe

```
Point-to-Point (Queue):              Publish/Subscribe (Topic):

  Producer ──► [Queue]                Producer ──► [Topic]
                 │                                   ├──► Subscriber A (gets every message)
                 └──► Consumer (one consumer         ├──► Subscriber B (gets every message)
                     gets each message)              └──► Subscriber C (gets every message)

One message → one consumer.           One message → all subscribers.
Work distribution pattern.             Event notification pattern.
```

- **Point-to-Point**: A message is delivered to exactly one consumer in a group. Use for work queues — order processing, job dispatch, task distribution. Only one worker should process each job.
- **Publish/Subscribe**: Every subscriber receives every message. Use for event notification — order-placed events that trigger email, analytics, and warehouse updates independently.

### Durability: Persistent vs. Non-Persistent

| Mode | Where stored | What happens on crash | Latency | Use case |
|---|---|---|---|---|
| Persistent | Written to disk before ack | Survives broker restart | Higher (disk I/O) | Financial transactions, orders |
| Non-persistent | In-memory only | Lost on broker crash | Lower (no disk wait) | Real-time telemetry, live scores |

Kafka is **always persistent** — every message is written to a commit log on disk. NATS core is **non-persistent** — if no subscriber is connected, the message is dropped. NATS JetStream adds persistence as an opt-in layer.

### Ordering: FIFO Per Partition

```
Global ordering requires a single partition:

  Topic "orders" (1 partition):
  ┌───────────────────────────────────────────────┐
  │ msg0 → msg1 → msg2 → msg3 → msg4 → msg5 →   │
  └───────────────────────────────────────────────┘
  ✅ Total order   ❌ Throughput limited to one broker

Partitioned ordering (common in Kafka):

  Topic "orders" (3 partitions):
  Partition 0: msg0 → msg3 → msg6 → msg9          (FIFO within partition)
  Partition 1: msg1 → msg4 → msg7 → msg10         (FIFO within partition)
  Partition 2: msg2 → msg5 → msg8 → msg11         (FIFO within partition)

  ✅ Parallel throughput   ❌ No global order across partitions
```

Kafka guarantees FIFO ordering **per partition**. Messages with the same key always land in the same partition, so per-key ordering is preserved. If you need total order across all messages, you must use a single partition — which limits throughput to what one broker can handle.

### Kafka Architecture

```
                    ┌────────────────────┐
                    │     ZooKeeper /     │
                    │   KRaft Controller   │
                    └─────────┬──────────┘
                              │
           ┌──────────────────┼──────────────────┐
           │                  │                  │
    ┌──────┴──────┐   ┌──────┴──────┐   ┌──────┴──────┐
    │   Broker 0   │   │   Broker 1   │   │   Broker 2   │
    │              │   │              │   │              │
    │ P0 (leader)  │   │ P0 (follower)│   │ P1 (leader)  │
    │ P2 (leader)  │   │ P1 (follower)│   │ P2 (follower)│
    └──────┬──────┘   └──────┬──────┘   └──────┬──────┘
           │                  │                  │
           ▼                  ▼                  ▼
     ┌─────────────────────────────────────────────────┐
     │              Producers & Consumers               │
     └─────────────────────────────────────────────────┘
```

**Topic**: A named stream of messages — like a table in a database, but append-only.

**Partition**: Each topic is split into N partitions. Partitions are the unit of parallelism:
- Each partition is an ordered, append-only log.
- Partitions are distributed across brokers.
- One broker is the partition leader; others are followers (replicas).

**Offset**: A monotonically increasing integer assigned to each message within a partition. Consumers track their position using offsets. Unlike queue systems, Kafka doesn't delete messages after consumption — it uses time-based or size-based retention.

**Consumer Group**: A set of consumers that cooperate to consume a topic. Each partition is consumed by exactly one consumer within the group. If a group has 3 consumers and a topic has 6 partitions, each consumer gets 2 partitions.

```
Consumer Group "analytics":
  Consumer A ← Partition 0, 1
  Consumer B ← Partition 2, 3
  Consumer C ← Partition 4, 5

If Consumer B crashes:
  Consumer A ← Partition 0, 1, 2    (took B's first partition)
  Consumer C ← Partition 3, 4, 5    (took B's second partition)
```

This is **rebalancing** — the group coordinator reassigns partitions among surviving consumers. During rebalancing, consumption pauses briefly.

### Kafka Delivery Semantics

| Semantics | How | What you handle | When to use |
|---|---|---|---|
| **At-most-once** | Producer sends, doesn't wait for ack. Consumer reads, commits offset immediately. | Duplicates possible on producer, messages lost on consumer crash | Telemetry, metrics (losing a data point is OK) |
| **At-least-once** | Producer sends with `acks=all`. Consumer processes, then commits offset. If consumer crashes before commit, it reprocesses. | You must make consumers idempotent | Most common default — orders, events |
| **Exactly-once** | Idempotent producer + transactional consumer (read-process-write as atomic unit). Consumer sees only committed transaction results. | Complex, lower throughput | Financial systems, exactly-once processing |

**At-least-once is the practical default.** You design consumers to be idempotent (processing the same message twice produces the same result) rather than paying the complexity cost of exactly-once.

### NATS: Lightweight Pub/Sub

NATS takes the opposite design stance from Kafka. Kafka is a durable commit log — heavy, persistent, replayable. NATS core is a lightweight, in-memory message bus: fast, simple, at-most-once.

```
Core NATS:

  Publisher ──► Subject "order.created" ──► Subscriber A (receives it)
                                      ──► Subscriber B (receives it)

  If no subscriber is connected → message is dropped.
  If subscriber disconnects → it misses messages.
  Latency: sub-millisecond.
```

**Subject-based addressing** replaces Kafka's topic hierarchy:

```
Kafka:       topic = "orders"
NATS:        subject = "orders.created.electronics"

NATS wildcards:
  * matches a single token:      "orders.*.electronics" matches "orders.created.electronics"
  > matches one or more tokens:  "orders.>" matches "orders.created" and "orders.created.electronics"

Subscriptions:
  Sub A subscribes to "orders.*.electronics"   → gets orders.created.electronics
  Sub B subscribes to "orders.>"               → gets everything under "orders"
  Sub C subscribes to "orders.created.book"    → gets only book orders
```

**NATS JetStream** adds persistence:

```
JetStream Stream: "ORDERS" (durable, replayable — like a Kafka topic)
  ┌─────────────────────────────────────────────────┐
  │ msg1 → msg2 → msg3 → msg4 → msg5 → msg6 →      │
  └─────────────────────────────────────────────────┘

JetStream Consumer: pulls from stream with ack-model choices:
  - At-most-once:  deliver, don't wait for ack
  - At-least-once: deliver, wait for ack, redeliver on nak/timeout
  - Exactly-once:  idempotent deduplication window
```

### Comparison

| System | Model | Durability | Ordering | Throughput | Latency | Best for |
|---|---|---|---|---|---|---|
| **RabbitMQ** | AMQP, rich routing (exchanges, bindings) | Persistent or transient | FIFO per queue | Moderate | Low-ms | Task queues, request-reply, complex routing |
| **Kafka** | Partitioned commit log | Always persistent (disk) | FIFO per partition | Very high | Low-ms to seconds | Event streaming, log aggregation, replay |
| **NATS Core** | Pub/sub, fire-and-forget | In-memory only | FIFO per subject | Very high | Sub-ms | Real-time fanout, microservice RPC |
| **NATS JetStream** | Durable streams + consumers | Persistent (disk) | FIFO per stream | High | Low-ms | Event streaming with simpler ops than Kafka |
| **Redis Streams** | Append-only log in Redis | In-memory (optional AOF/RDB) | FIFO per stream | Very high | Sub-ms | Simple streaming, rate limiting, short-lived data |

## Build It

### Step 1: Message and Topic Core

The broker needs messages with keys for partition routing, and topics that group partitions:

```go
type Message struct {
    Key       string
    Value     []byte
    Timestamp time.Time
    Offset    int64
}

type Partition struct {
    messages    []Message
    nextOffset  int64
    subscribers map[string]int64 // consumer ID → last committed offset
}

type Topic struct {
    Name        string
    Partitions  []*Partition
    RetentionMs int64 // 0 = infinite
}
```

### Step 2: Broker with Key-Based Partitioning

The broker routes messages to partitions using a hash of the key. Consumers subscribe and get assigned partitions from their consumer group:

```go
type Broker struct {
    topics map[string]*Topic
    mu     sync.RWMutex
}

func (b *Broker) Publish(topicName, key string, value []byte) (int64, int, error)
func (b *Broker) Subscribe(topicName, groupID, consumerID string) (*Subscription, error)
func (b *Broker) Consume(sub *Subscription, maxMsgs int) ([]Message, error)
func (b *Broker) CommitOffset(sub *Subscription, partition int, offset int64) error
```

### Step 3: Consumer Groups and Rebalancing

When a consumer in a group fails, its partitions are redistributed among surviving consumers. This is the core of Kafka-style fault tolerance:

```go
type ConsumerGroup struct {
    GroupID      string
    TopicName    string
    Consumers    map[string]*ConsumerState // consumer ID → state
    Assignments  map[string][]int           // consumer ID → partition list
}

func (g *ConsumerGroup) Rebalance(broker *Broker)
func (g *ConsumerGroup) RemoveConsumer(consumerID string, broker *Broker)
```

### Step 4: Full Demo

A producer sends 100 messages across 4 partitions. Two consumer groups process them. One consumer fails, triggering rebalancing. Offset tracking shows exactly where each consumer is.

See `code/main.go` for the complete, compilable implementation.

## Use It

**Kafka** — Apache Kafka stores each partition as a sequence of segment files on disk. Each segment contains the message data and an index for offset-based lookup. Consumers read from segments using `seek()` to their committed offset. See `kafka/log/LogSegment.scala` in the Kafka source for how segments are rolled over based on size and time, and `kafka/coordinator/group/GroupCoordinator.scala` for how consumer group rebalancing is managed.

**NATS** — NATS server routes subjects using a subscription trie that enables O(1) wildcard matching. See `nats-server/server/sublist.go` for the subject-matching trie implementation. JetStream stores streams as sequential files with a write-ahead log, similar to Kafka's segment design but with a simpler API surface.

**What production systems add beyond our implementation:**
- **Replication** — Kafka replicates each partition across multiple brokers. If the partition leader fails, a follower takes over. Our implementation is single-broker.
- **Log compaction** — Kafka can retain only the latest value per key, turning the log into a changelog. Our implementation uses simple time-based retention.
- **Consumer group coordination** — Kafka uses a group coordinator (a dedicated broker) that manages join/sync/heartbeat protocols. Our implementation uses a simple in-memory map.
- **Backpressure and flow control** — Production systems throttle fast producers when consumers are slow. Our implementation assumes unbounded in-memory buffers.
- **Exactly-once semantics** — Kafka's idempotent producer assigns sequence numbers per producer-partition pair, and transactional consumers read only committed transaction markers. Our implementation covers at-least-once.

## Read the Source

- Kafka `core/src/main/scala/kafka/log/LogSegment.scala` — how Kafka stores partition data as segment files with offset indexes.
- Kafka `core/src/main/scala/kafka/coordinator/group/GroupCoordinator.scala` — how Kafka manages consumer group membership and partition assignment.
- NATS `nats-server/server/sublist.go` — the subject-matching trie that enables wildcard subscriptions.

## Ship It

The reusable artifact from this lesson:

- **`code/main.go`** — A self-contained message broker with topics, key-based partitioning, consumer groups, offset tracking, and rebalancing. Reusable as a reference for understanding Kafka internals and as a starting point for custom message routing.

## Exercises

1. **Easy** — Add a round-robin partitioning strategy (no key hash) and compare message distribution across partitions against key-based hashing. How does skew differ?
2. **Medium** — Implement time-based retention: messages older than the retention window are purged from partitions. Show that consumers with committed offsets beyond the retention boundary reset to the earliest available offset.
3. **Hard** — Add leader-follower replication for partitions. Each partition has a leader and N followers. Writes go to the leader and replicate to followers (ack on quorum). On leader failure, promote a follower. Demonstrate that consumers continue reading without data loss.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Message queue | "A queue for messages" | An asynchronous broker that decouples producers from consumers. Can be point-to-point (one consumer per message) or pub/sub (all subscribers receive each message). |
| Partition | "A shard of a topic" | An ordered, append-only log within a topic. Partitions are the unit of parallelism in Kafka. Each partition is consumed by exactly one consumer in a group. |
| Offset | "A message ID" | A monotonically increasing integer position within a partition. Consumers track their progress by committing offsets. Unlike sequence numbers, offsets are per-partition, not global. |
| Consumer group | "A group of consumers" | A set of consumers that cooperatively consume a topic — each partition is assigned to exactly one consumer in the group. If a consumer fails, its partitions are reassigned (rebalanced). |
| At-least-once | "Messages are never lost" | The broker guarantees every message is delivered, but duplicates are possible because the consumer may reprocess messages after a crash before it commits its offset. |
| Exactly-once | "No duplicates, no losses" | The most expensive delivery guarantee. Requires idempotent producers and transactional consumer-read-process-write cycles. Rarely worth the complexity outside financial systems. |
| Rebalancing | "Reshuffling consumers" | When consumers join or leave a group, partitions are reassigned. During rebalancing, consumption pauses. Frequent rebalances are a common performance issue in Kafka. |
| Subject (NATS) | "A NATS topic" | A dot-separated hierarchical address like "orders.created.electronics". Supports wildcards: `*` matches one token, `>` matches one or more. More flexible than Kafka's flat topic names. |

## Further Reading

- [Kafka Documentation — Core Concepts](https://kafka.apache.org/documentation/#coreConcepts) — The canonical reference for topics, partitions, consumer groups, and delivery semantics.
- [NATS Documentation — JetStream](https://docs.nats.io/nats-concepts/jetstream) — How JetStream adds durability and exactly-once delivery to NATS core.
- [Jay Kreps, "The Log" (2013)](https://engineering.linkedin.com/distributed-systems/log-what-every-software-engineer-should-know-about-real-time-datas-unifying) — The foundational essay on log-based messaging that inspired Kafka's design.
- [Martin Kleppmann, "Designing Data-Intensive Applications" Ch. 11](https://dataintensive.net/) — Stream processing, message brokers, and the distinction between log-based and traditional messaging.
- [NATS Subject-Based Messaging](https://docs.nats.io/nats-concepts/subjects) — How wildcard subscriptions and hierarchical addressing enable flexible routing patterns.