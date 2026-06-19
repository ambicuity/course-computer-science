# Distributed Transactions — 2PC, 3PC, Sagas

> Either all of it happens or none of it does — and in distributed systems, "none" is the easy part.

**Type:** Learn
**Languages:** Python, Go
**Prerequisites:** Phase 11 lessons 01–13
**Time:** ~90 minutes

## Learning Objectives

- Explain why distributed transactions are harder than local transactions: multiple participants, partial failures, network partitions.
- Describe the Two-Phase Commit (2PC) protocol: the Prepare and Commit/Abort phases, the coordinator's role, and what happens when participants vote No.
- Identify the 2PC blocking problem: if the coordinator crashes between Phase 1 and Phase 2, participants are locked indefinitely.
- Explain how Three-Phase Commit (3PC) adds a Pre-Commit phase to eliminate the blocking single point of failure — and why it still fails under network partitions.
- Define Sagas: long-running transactions decomposed into a sequence of local transactions, each with a compensating (undo) action.
- Implement compensating logic: forward execution (T1, T2, … Tn) and backward compensation (Cn, Cn-1, … C1) on failure.
- Compare choreography vs. orchestration patterns for sagas and state their trade-offs.
- Build a 2PC coordinator in Python with crash simulation and recovery.
- Build a Saga orchestrator in Go with compensating transactions.

## The Problem

You're building a travel booking service. Reserving a car, booking a hotel, and booking a flight are three separate operations handled by three separate services. A customer expects all three to happen — or none of them. If the car and hotel are reserved but the flight booking fails, you need to undo the car and hotel reservations. This is a **distributed transaction**: operations spanning multiple independent services that must maintain atomicity across service boundaries.

Without a protocol, you get partial commits: money deducted, seats held, reservations made — but no way to roll back consistently. The customer sees a charge for a trip they can't take. The hotel has a ghost booking. The car sits reserved but unused.

Local transactions (BEGIN / COMMIT / ROLLBACK within a single database) can't help here because each service owns its own data store. You need a **protocol** that coordinates commitment across participants who can independently fail, crash, or become unreachable. That protocol is what this lesson builds.

## The Concept

### Two-Phase Commit (2PC)

2PC is the simplest distributed commitment protocol. A single **coordinator** drives the protocol with two rounds of communication:

```
Phase 1: Prepare (Voting)
  ┌──────────┐    PREPARE     ┌─────────────┐
  │Coordinator├───────────────►│ Participant A│
  │           ├───────────────►│ Participant B│
  │           ├───────────────►│ Participant C│
  └──────────┘                └─────────────┘
                              ▼
                     Each participant votes
                     YES (can commit) or
                     NO (must abort)

Phase 2: Commit / Abort (Decision)
  Coordinator decides:
    - All YES → COMMIT
    - Any NO (or timeout) → ABORT

  ┌──────────┐    COMMIT/     ┌─────────────┐
  │Coordinator├──ABORT────────►│ Participant A│
  │           ├───────────────►│ Participant B│
  │           ├───────────────►│ Participant C│
  └──────────┘                └─────────────┘
```

**Why it works:** Once a participant votes YES in Phase 1, it promises it *can* commit. It must hold locks on its resources until Phase 2 arrives. If the coordinator says COMMIT, the participant commits. If the coordinator says ABORT, the participant rolls back.

### The 2PC Blocking Problem

The coordinator is a single point of *decision*. If the coordinator crashes after all participants have voted YES in Phase 1 but before sending the Phase 2 decision, every participant that voted YES is **blocked** — it has promised to commit but doesn't know the decision. It cannot unilaterally commit (the coordinator may have decided ABORT) and cannot unilaterally abort (the coordinator may have decided COMMIT). It holds its locks and waits indefinitely.

```
Coordinator crashes here → 💥
                    ┌──────────┐
  Phase 1: ─────────│  YES YES  │
  Phase 2: ─ ─ ─ ─ │  BLOCKED! │
                    └──────────┘
  Participants hold locks forever.
```

**Recovery:** Participants log their vote to stable storage. When the coordinator recovers, it reads its decision log and resends. If a participant has no decision log entry, it remains blocked until the coordinator returns. There is no protocol-level escape — this is the fundamental limitation of 2PC.

### Three-Phase Commit (3PC)

3PC adds a **Pre-Commit** phase between Prepare and Commit/Abort:

```
Phase 1: Prepare     — participants vote YES/NO
Phase 2: Pre-Commit  — coordinator tells YES voters "I will commit"
Phase 3: Commit      — coordinator tells everyone to commit
```

The key insight: if the coordinator crashes after Phase 1 and before Phase 3, the remaining participants can decide among themselves:

- **If any participant is in the Pre-Commit state**, the decision must have been COMMIT (otherwise no one would have been told pre-commit). The surviving participants commit.
- **If all participants are in the Prepared or Aborted state**, no one was told pre-commit, so the decision must have been ABORT. The surviving participants abort.

**Why 3PC still fails:** 3PC assumes a *failure-free window* — the time between Phase 2 and Phase 3 must be long enough for pre-commit messages to reach at least one participant. More critically, 3PC assumes no network partitions. If the network splits and a minority partition has participants in Pre-Commit while the majority partition sees no Pre-Commit, you get **split-brain**: both partitions make different decisions.

```
Network partition during 3PC:

  [Partition A]          [Partition B]
  Participant X:         Participant Z:
    In Pre-Commit          In Prepared
    → Decides COMMIT       → Decides ABORT
    (splits brain)
```

3PC eliminates the *single coordinator* blocking point but not *partition* failures. In practice, 3PC is rarely used — the additional round-trip latency and the remaining partition vulnerability make it unappealing.

### Sagas

A **Saga** decomposes a long-running distributed transaction into a sequence of local transactions, each with a **compensating transaction** that undoes its effects:

```
Book-a-trip Saga:

  Forward path:          T1: Reserve Car → T2: Book Hotel → T3: Book Flight
                                                       ↓ fails!
  Compensation path:  C1: Cancel Car ← C2: Cancel Hotel ← ─┘
```

If T3 fails, the saga runs C2 (cancel hotel) then C1 (cancel car) in reverse order. Each Ti commits locally — there is no global lock. The saga provides **semantic atomicity**: the overall effect is either all steps completed or all completed steps are compensated.

**Saga Isolation:** Sagas are **not isolated** from each other. While a saga is executing its forward path, other sagas can observe intermediate states (e.g., a car reserved but a flight not yet booked). You get ACD (Atomicity, Consistency, Durability) but not I (Isolation). This is called the **A.I.D. without the I** property.

### Choreography vs. Orchestration

**Choreography:** Each service emits events. Other services react. No central coordinator.

```
Car Service ──(car_reserved)──► Hotel Service ──(hotel_booked)──► Flight Service
      │                              │                                  │
      │   ◄──(car_cancelled)─────────│◄───(hotel_cancelled)────────────│
```

Each service knows what to do when an event arrives and what event to emit next. Simple for 2–3 steps, but the flow becomes implicit and hard to trace with more participants.

**Orchestration:** A central orchestrator service calls each step and handles compensation.

```
Orchestrator ──► Car Service ──► Hotel Service ──► Flight Service
     │                                              │
     │◄─── Cancel Car ◄── Cancel Hotel ◄───────────│
```

The orchestrator has complete visibility into the saga's state. Easier to monitor and debug, but introduces a single point of failure (the orchestrator itself).

### Comparison

| Property | 2PC | 3PC | Saga |
|---|---|---|---|
| Consistency | Strong (all-or-nothing) | Strong (all-or-nothing) | Eventual (compensating) |
| Blocking | Yes — if coordinator crashes | No single-point blocking | No blocking |
| Partition tolerance | Blocks until coordinator recovers | Split-brain risk | Continues with compensation |
| Latency | 2 round trips | 3 round trips | 1 RT per step |
| Isolation | Full (ACID) | Full (ACID) | None (ACD only) |
| Use case | Short, lock-tolerant txns | Theoretical极少 used | Long-running business processes |

## Build It

### Step 1: 2PC Coordinator and Participants (Python)

See `code/main.py` for the full implementation. The key pieces:

1. **Participant** — holds local state (PREPARED, COMMITTED, ABORTED), logs votes to stable storage, responds to PREPARE and COMMIT/ABORT messages.
2. **Coordinator** — drives both phases, logs decisions, handles timeouts.
3. **TwoPhaseCommit** — wires coordinator and participants together, simulates crashes.

### Step 2: Saga Orchestrator (Go)

See `code/main.go` for the full implementation. The key pieces:

1. **SagaDefinition** — an ordered list of (transaction, compensating_transaction) pairs.
2. **SagaOrchestrator** — executes transactions forward. On failure, executes compensating transactions in reverse.
3. **Choreography** — each service emits events and reacts to others' events without a central coordinator.

## Use It

**PostgreSQL's 2PC:** PostgreSQL implements the SQL-standard `PREPARE TRANSACTION` and `COMMIT PREPARED` commands — this is 2PC. The `pg_two_phase` module manages prepared transaction recovery after coordinator crashes. See `src/backend/access/transam/twophase.c` in the PostgreSQL source — it implements the participant side: logging the prepare record, holding locks, and committing or aborting based on the coordinator's decision.

**Apache Camel's Saga:** Apache Camel implements saga orchestration using both choreography (event-based) and orchestration patterns. See the `camel-saga` module for a production-grade saga coordinator.

**AWS Step Functions:** AWS Step Functions implement saga orchestration as a managed service. Each state in the state machine is a local transaction, and error paths define compensating actions.

**What production systems add beyond our implementation:**
- **Timeouts and retries** with exponential backoff for each step.
- **Idempotency keys** so re-executing a compensating transaction is safe.
- **Durability** — saga state persisted to a database, not just in-memory.
- **Observability** — distributed tracing across saga steps (see Lesson 21).
- **Dead letter queues** — compensating transactions that fail end up for manual review.

## Read the Source

- PostgreSQL `src/backend/access/transam/twophase.c` — the participant side of 2PC: how it logs prepare records and manages the in-memory prepared-transaction table.
- etcd `server/etcdserver/txn.go` — how etcd wraps multi-key operations in a single Raft proposal (not 2PC, but achieves distributed atomicity via consensus).

## Ship It

The reusable artifacts from this lesson:

- **`code/main.py`** — A self-contained 2PC coordinator and participant implementation with crash simulation and recovery. Reusable as a reference for understanding and debugging 2PC in production.
- **`code/main.go`** — A Saga orchestrator and choreography framework. Reusable as the starting point for saga-based coordination in the Phase capstone.

## Exercises

1. **Easy** — Modify the 2PC coordinator to support a configurable timeout for participant responses. If a participant times out during Phase 1, treat it as a No vote and abort.
2. **Medium** — Extend the Saga orchestrator to support **semantic compensation**: instead of just undoing, a compensating action can resolve a conflict (e.g., if a hotel room was given to someone else, offer a different room instead of just canceling).
3. **Hard** — Implement 3PC by extending the 2PC coordinator. Add the Pre-Commit phase, implement recovery logic where surviving participants can decide based on each other's states, and demonstrate the split-brain failure mode under a simulated network partition.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| 2PC | "Two-phase commit" | A protocol where a coordinator first asks all participants if they can commit (Phase 1), then tells them the decision (Phase 2). Participants that vote YES are blocked until they receive the decision. |
| Blocking problem | "2PC is slow" | Specifically: if the coordinator crashes between phases, participants that voted YES hold locks indefinitely with no way to resolve. They can't commit or abort without the coordinator's decision. |
| 3PC | "Three-phase commit" | Adds a Pre-Commit phase so surviving participants can decide without the coordinator. Eliminates blocking but not partition failures. |
| Saga | "A long transaction" | A sequence of local transactions where each step has a compensating (undo) action. Not isolated — other sagas can see intermediate states. |
| Compensating transaction | "An undo" | A semantic rollback: it doesn't literally reverse the original transaction but achieves an equivalent effect (e.g., issuing a refund rather than deleting a charge record). |
| Choreography | "Event-driven saga" | No central coordinator — each service emits events that trigger the next step and compensations. The flow is implicit in the event handlers. |
| Orchestration | "Centralized saga" | A single orchestrator service manages the saga's forward and backward paths. Explicit state machine, easier to debug, but the orchestrator is a single point of failure. |

## Further Reading

- [Jim Gray, "Notes on Data Base Operating Systems" (1978)](https://www.cs.cmu.edu/~natassa/courses/764S01/Readings/gray78.pdf) — Original 2PC specification. Still the reference.
- [Dale Skeen, "Nonblocking Commit Protocols" (1981)](https://dl.acm.org/doi/10.1145/319632.319636) — Introduces 3PC and proves that non-blocking commit requires three phases in a crash-failure model.
- [Hector Garcia-Molina & Kenneth Salem, "Sagas" (1987)](https://dl.acm.org/doi/10.1145/38714.38742) — The original saga paper. Defines the concept of long-running transactions with compensating actions.
- [Caitie McCaffrey, "Distributed Sagas" (YouTube, 2015)](https://www.youtube.com/watch?v=1H6J3r9K8Fo) — Practical overview of sagas in production microservices.
- [Chris Richardson, "Microservices Patterns" (2018)](https://microservices.io/book.html) — Chapter 4 covers saga patterns (orchestration vs. choreography) in depth.