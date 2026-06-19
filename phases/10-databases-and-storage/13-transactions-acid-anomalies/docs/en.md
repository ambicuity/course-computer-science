# Transactions — ACID, Anomalies

> A transaction is the only way to tell the database "do all of this, or none of it" — the difference between a bank transfer and a wire that vanishes mid-flight.

**Type:** Build
**Languages:** Python, SQL
**Prerequisites:** Phase 10 lessons 01–12
**Time:** ~75 minutes

## Learning Objectives

- Define a *transaction* and explain the four ACID properties with concrete examples.
- Implement a transaction manager in Python that simulates different isolation levels.
- Reproduce each of the seven anomalies (dirty read, dirty write, non-repeatable read, phantom read, lost update, read skew, write skew) by constructing interleaved schedules.
- Identify which isolation level prevents which anomaly.
- Write SQL transactions using `BEGIN`/`COMMIT`/`ROLLBACK` and `SET TRANSACTION ISOLATION LEVEL`.

## The Problem

Without transactions, every database write is a gamble. You `UPDATE accounts SET balance = balance - 100 WHERE id = 1` and `UPDATE accounts SET balance = balance + 100 WHERE id = 2` — the two operations of a bank transfer. What happens if the server crashes between them? The money disappears. What happens if another session reads balances halfway through? It sees the money in both accounts, or neither, depending on timing. You've just violated the rule that money is conserved.

The same issue shows up in booking systems (two people grab the last seat), inventory (double-sell the same widget), and every concurrent system where multiple clients read and write shared state. Without transactions, you're writing assembly with no memory barriers — your program will work in testing and corrupt data in production under load.

The database gives you **transactions** so you don't have to solve consensus and recovery in every application. The cost is performance: stronger isolation means more locking, more aborts, more wall-clock time. Understanding the trade-off space — ACID properties, anomalies, and the isolation levels that prevent them — is what lets you choose the right level for your workload.

## The Concept

### What is a transaction?

A transaction is a sequence of operations (reads + writes) that the database treats as a single unit of work. The API is:

```
BEGIN;                       -- start
  UPDATE accounts SET ...    -- operations
  UPDATE accounts SET ...    -- operations
COMMIT;                      -- make permanent, OR
ROLLBACK;                    -- undo everything
```

The contract is captured by **ACID**:

### ACID

| Property | Definition | What breaks it | How the DB enforces it |
|----------|-----------|----------------|------------------------|
| **Atomicity** | All operations complete, or none do. No partial effects. | Crash mid-transaction | Undo log / write-ahead log for rollback |
| **Consistency** | The database moves from one valid state to another. Invariants hold. | Application bug or constraint violation | Application logic + FK/CHECK/triggers at the DB level |
| **Isolation** | Concurrent transactions don't interfere with each other. | Interleaved operations that cause anomalies | Locking / MVCC / OCC |
| **Durability** | Once COMMIT returns, the data survives a crash. | Power loss before fsync | WAL force on commit, fsync, group commit |

### Atomicity

The simplest atomicity mechanism: before writing a page, write the old value to an **undo log** (rollback segment). On `ROLLBACK` or crash recovery, walk the undo log backwards and restore original values. PostgreSQL calls this the *clog* (commit log). MySQL InnoDB stores undo records in its tablespace.

```
Time →
  BEGIN
  UPDATE accounts SET balance = 90 WHERE id = 1  →  undo log: {id=1, old=100}
  CRASH!
  Recovery: read undo log, restore id=1 to 100   →  atomicity preserved
```

A **write-ahead log** (WAL) guarantees durability in the forward direction (redo) while the undo log guarantees atomicity in the backward direction (undo). Most engines merge them — PostgreSQL's WAL serves both roles via its *full page images*.

### Consistency

Consistency is the least "database" property — it's mostly the application's job. The DB can enforce referential integrity (FKs), check constraints, and triggers, but it can't know that "account balance must never go negative" is a rule. The application must check before writing.

What the DB does guarantee: if a transaction violates a constraint (e.g., `INSERT` into a table with a `NOT NULL` column and provides `NULL`), the statement aborts and atomicity ensures no partial effect remains.

### Isolation

Isolation is where the complexity lives. Perfect isolation (serializability) is slow. Weak isolation (read uncommitted) is fast but lets anomalies through.

The SQL standard defines four levels from weakest to strongest:

| Level | Dirty Read | Non-Repeatable Read | Phantom Read | Write Skew |
|-------|-----------|---------------------|--------------|------------|
| READ UNCOMMITTED | Possible | Possible | Possible | Possible |
| READ COMMITTED | Prevented | Possible | Possible | Possible |
| REPEATABLE READ | Prevented | Prevented | Possible | Possible |
| SERIALIZABLE | Prevented | Prevented | Prevented | Prevented |

(PostgreSQL's READ UNCOMMITTED behaves like READ COMMITTED — it never allows dirty reads.)

### Durability

Durability means "once `COMMIT` returns, the data is on non-volatile storage." The mechanism: every committed transaction's log records must reach disk before the commit acknowledgement. This is `fsync()` on the WAL file.

**Group commit** batches multiple commits into one `fsync()` call — throughput improves but latency for any individual commit increases (good problem to have).

### Anomalies

Each anomaly is a specific way concurrent transactions can produce incorrect results. Understanding them by name is how you reason about whether your application needs serializability.

**Dirty Read:** Txn B reads a value that Txn A wrote but hasn't committed. If A rolls back, B has seen a value that never existed.
```
T1: write(X, 5)
T2: read(X)    → sees 5 (dirty!)
T1: rollback
```

**Dirty Write:** Two transactions write the same uncommitted value. The second overwrites the first before the first commits; if the first rolls back, the second writer's work is also lost.
```
T1: write(X, 5)
T2: write(X, 10)    → overwrites T1's uncommitted write
T1: rollback         → rolls back to old value, but T2 expected X=10
```

**Non-Repeatable Read:** Within a single transaction, reading the same row twice yields different values because another transaction committed an update in between.
```
T1: read(X) → 100
T2: write(X, 200), commit
T1: read(X) → 200    ← different from first read!
```

**Phantom Read:** Within a single transaction, re-executing the same query returns a different set of rows (new rows appeared or existing ones disappeared).
```
T1: SELECT * FROM orders WHERE amount > 100 → [order1, order2]
T2: INSERT order3 amount=200, commit
T1: SELECT * FROM orders WHERE amount > 100 → [order1, order2, order3]  ← phantom!
```

**Lost Update:** Two transactions read the same value, both modify it independently, then both write. The second write overwrites the first — one update disappears.
```
T1: read(X) → 100                          T2: read(X) → 100
T1: X = 100 + 10 = 110                     T2: X = 100 + 20 = 120
T1: write(X, 110)                          T2: write(X, 120)
                                            ← final value 120, T1's update lost!
```

**Read Skew:** An inconsistent read of two related values that should be in a consistent state together. Example: T1 reads X then Y; between those reads, T2 modifies both. T1 sees an old X and a new Y — a pair that never coexisted.
```
T1: read(X) → 50
T2: write(X, 150), write(Y, 200), commit
T1: read(Y) → 200    ← X=50, Y=200 never coexisted!
```

**Write Skew:** Two transactions read overlapping data and make conflicting writes; each individually preserves the invariant, but together they violate it. Classic example: two doctors on call — each checks "is at least one of us on call?" (yes), then marks themselves off-duty, leaving nobody on call.
```
Constraint: at least one of A, B must be on call
T1: read(on_call_A=true), read(on_call_B=true)    T2: read(on_call_A=true), read(on_call_B=true)
T1: if B is on call, set A=false                   T2: if A is on call, set B=false
T1: write(A, false)                                 T2: write(B, false)
                                                   ← nobody is on call! Both relied on stale snapshot
```

**Snapshot Isolation (SI) anomaly:** Two concurrent transactions under snapshot isolation (each sees a snapshot from its start time) each make a write that depends on the other's read. SI prevents dirty reads and non-repeatable reads but allows write skew and a specific SI anomaly that serializable prevents. Diagrammed:
```
T1: read(X) → 0                                   T2: read(X) → 0
T1: write(X, 1), commit                            T2: write(X, 2), commit
                                                   ← final value is one of 1 or 2, not both
                                                   (SI prevents write-write conflicts via first-committer-wins)
```
The SI-anomaly is a *write skew* pattern that SI doesn't prevent.

## Build It

We'll build a **transaction simulator** in Python that models a key-value store with configurable isolation levels. Open `code/main.py`.

### Step 1: Imports and isolation level enum

```python
from enum import Enum
from dataclasses import dataclass, field
from typing import Optional

class IsolationLevel(Enum):
    READ_UNCOMMITTED = 1
    READ_COMMITTED = 2
    REPEATABLE_READ = 3
    SERIALIZABLE = 4
```

### Step 2: The Transaction Manager

```python
@dataclass
class Txn:
    txn_id: int
    isolation: IsolationLevel
    snapshot: dict = field(default_factory=dict)
    writes: dict = field(default_factory=dict)
    active: bool = True

class TransactionManager:
    def __init__(self, isolation: IsolationLevel, data: dict = None):
        self.isolation = isolation
        self.data = data or {}
        self.txns: dict[int, Txn] = {}
        self.next_id = 0

    def begin(self) -> int:
        txn_id = self.next_id
        self.next_id += 1
        txn = Txn(txn_id=txn_id, isolation=self.isolation)
        if self.isolation in (IsolationLevel.REPEATABLE_READ, IsolationLevel.SERIALIZABLE):
            txn.snapshot = dict(self.data)
        txn.writes = {}
        self.txns[txn_id] = txn
        return txn_id

    def _latest_uncommitted(self, key: str) -> Optional[int]:
        latest_txn = -1
        latest_val = None
        for tid, txn in self.txns.items():
            if txn.active and key in txn.writes and tid > latest_txn:
                latest_txn = tid
                latest_val = txn.writes[key]
        return latest_val

    def read(self, txn_id: int, key: str) -> Optional[int]:
        txn = self.txns[txn_id]
        if not txn.active:
            raise ValueError(f"Transaction {txn_id} is not active")
        if key in txn.writes:
            return txn.writes[key]
        if self.isolation == IsolationLevel.READ_UNCOMMITTED:
            uncommitted = self._latest_uncommitted(key)
            if uncommitted is not None:
                return uncommitted
            return self.data.get(key)
        if self.isolation in (IsolationLevel.REPEATABLE_READ, IsolationLevel.SERIALIZABLE):
            return txn.snapshot.get(key)
        return self.data.get(key)

    def write(self, txn_id: int, key: str, value: int):
        txn = self.txns[txn_id]
        if not txn.active:
            raise ValueError(f"Transaction {txn_id} is not active")
        txn.writes[key] = value

    def commit(self, txn_id: int) -> bool:
        txn = self.txns[txn_id]
        if not txn.active:
            return False
        txn.active = False
        for key, value in txn.writes.items():
            self.data[key] = value
        return True

    def rollback(self, txn_id: int):
        txn = self.txns.get(txn_id)
        if txn:
            txn.active = False
            txn.writes.clear()

    def schedule_run(self, schedule: list[dict]):
        for step in schedule:
            txn_id = step['txn']
            op = step['op']
            key = step.get('key')
            value = step.get('value')
            if op == 'r':
                val = self.read(txn_id, key)
                step['result'] = val
            elif op == 'w':
                self.write(txn_id, key, value)
            elif op == 'commit':
                self.commit(txn_id)
            elif op == 'rollback':
                self.rollback(txn_id)
```

### Step 3: Demonstrate dirty read

```python
def demo_dirty_read():
    tm = TransactionManager(IsolationLevel.READ_UNCOMMITTED, {"x": 100})
    t1 = tm.begin()
    t2 = tm.begin()
    schedule = [
        {'txn': t1, 'op': 'w', 'key': 'x', 'value': 200},
        {'txn': t2, 'op': 'r', 'key': 'x'},
        {'txn': t1, 'op': 'rollback'},
    ]
    tm.schedule_run(schedule)
    print(f" T2 dirty read result: {schedule[1]['result']}")
    print(f" Final data after rollback: {tm.data}")
```

At `READ UNCOMMITTED`, `_latest_uncommitted` finds T1's uncommitted write (200). At `READ COMMITTED`, the same schedule would return 100 because the helper only probes at `READ UNCOMMITTED`.

### Step 4: Demonstrate non-repeatable read

```python
def demo_nonrepeatable_read():
    tm = TransactionManager(IsolationLevel.READ_COMMITTED, {"x": 100})
    t1 = tm.begin()
    t2 = tm.begin()
    schedule = [
        {'txn': t1, 'op': 'r', 'key': 'x'},
        {'txn': t2, 'op': 'w', 'key': 'x', 'value': 200},
        {'txn': t2, 'op': 'commit'},
        {'txn': t1, 'op': 'r', 'key': 'x'},
        {'txn': t1, 'op': 'commit'},
    ]
    tm.schedule_run(schedule)
    print(f" T1 first read:  {schedule[0]['result']}")
    print(f" T1 second read: {schedule[3]['result']}")
```

At `READ COMMITTED`, T1 reads 100 then 200. At `REPEATABLE READ`, both reads return 100 (snapshot frozen at begin).

### Step 5: Lost update

```python
def demo_lost_update():
    tm = TransactionManager(IsolationLevel.READ_COMMITTED, {"counter": 100})
    t1 = tm.begin()
    t2 = tm.begin()
    v1 = tm.read(t1, "counter")
    v2 = tm.read(t2, "counter")
    schedule = [
        {'txn': t1, 'op': 'w', 'key': 'counter', 'value': v1 + 10},
        {'txn': t1, 'op': 'commit'},
        {'txn': t2, 'op': 'w', 'key': 'counter', 'value': v2 + 20},
        {'txn': t2, 'op': 'commit'},
    ]
    tm.schedule_run(schedule)
    print(f" Expected: {v1 + v2}")  # 130
    print(f" Actual:   {tm.data['counter']}")  # 120
```

Both transactions read 100 before either commits. T1 writes 110, commits. T2 writes 120 (based on stale 100), commits. T1's increment vanishes.

### Step 6: Write skew

```python
def demo_write_skew():
    tm = TransactionManager(IsolationLevel.REPEATABLE_READ, {"a_oncall": 1, "b_oncall": 1})
    t1 = tm.begin()
    t2 = tm.begin()
    schedule = [
        {'txn': t1, 'op': 'w', 'key': 'a_oncall', 'value': 0},
        {'txn': t1, 'op': 'commit'},
        {'txn': t2, 'op': 'w', 'key': 'b_oncall', 'value': 0},
        {'txn': t2, 'op': 'commit'},
    ]
    tm.schedule_run(schedule)
    print(f" A on call: {tm.data['a_oncall']}, B on call: {tm.data['b_oncall']}")
```

Both see the other on call (snapshot), each takes themselves off. Both commit. Nobody is on call — the invariant "at least one doctor on call" is broken.

### Step 7: Run all demos

```python
def main():
    demo_dirty_read()
    demo_dirty_read_prevented()
    demo_nonrepeatable_read()
    demo_nonrepeatable_read_prevented()
    demo_phantom_read()
    demo_lost_update()
    demo_write_skew()
    demo_read_skew()
    summary()

if __name__ == "__main__":
    main()
```

The full script in `code/main.py` includes additional demos (`demo_dirty_read_prevented`, `demo_nonrepeatable_read_prevented`, `demo_phantom_read`, `demo_read_skew`) and a `summary()` table mapping each anomaly to every isolation level. Run it with `python3 code/main.py`.

## Use It

PostgreSQL implements transactions closer to the metal:

- **WAL** (`pg_wal/`): every transaction writes WAL records before modifying data pages. On crash recovery, PostgreSQL replays WAL from the last checkpoint (redo) and uses the *clog* to determine which in-progress transactions to roll back (undo).
- **MVCC snapshots**: Every transaction sees a snapshot of the database at its start time (for `REPEATABLE READ` / `SERIALIZABLE`) or at each statement (for `READ COMMITTED`). Snapshot visibility is tracked via `xmin`/`xmax` in each tuple header.
- **SSI** (Serializable Snapshot Isolation): PostgreSQL's `SERIALIZABLE` level uses predicate locks to detect read-write conflicts that would cause serialization anomalies. It's true serializability, not just snapshot isolation.

Your Python simulator omits:
- True concurrent execution (all steps are scheduled sequentially)
- Lock manager (2PL would prevent lost updates without aborts)
- Predicate locking (needed to detect phantoms without locking the whole table)
- Crash recovery (WAL replay, undo, ARIES protocol)

## Read the Source

- [PostgreSQL source: `src/backend/access/transam/`](https://github.com/postgres/postgres/tree/master/src/backend/access/transam) — the transaction system, including WAL (`xlog.c`), transaction IDs (`xact.c`), and the clog.
- [PostgreSQL source: `src/backend/storage/lmgr/`](https://github.com/postgres/postgres/tree/master/src/backend/storage/lmgr) — lock manager and predicate locking for SSI.
- *Designing Data-Intensive Applications* (Kleppmann), Chapter 7 — the clearest treatment of anomalies and isolation levels in print.

## Ship It

The reusable artifact is **`outputs/txn_simulator.py`** — a standalone transaction simulator you can import into later lessons (Phase 14 concurrency, Phase 15 distributed transactions) to test isolation behaviors without a real database.

## Exercises

1. **Easy.** Run the simulator at `SERIALIZABLE` isolation and show that the lost update demo now aborts one transaction instead of producing a wrong result.
2. **Medium.** Implement phantom read detection in the simulator: track predicate ranges (e.g., `key >= 'a' AND key <= 'z'`) and detect insertions into a scanned range by concurrent transactions. Demonstrate a phantom read at `REPEATABLE READ` and its prevention at `SERIALIZABLE`.
3. **Hard.** Extend the simulator with a two-phase locking (2PL) lock manager. Show that 2PL prevents dirty writes and lost updates but not write skew (because 2PL locks rows, not predicates). Then add predicate locking and demonstrate write skew prevention.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| ACID | "A database property" | Four distinct guarantees (Atomicity, Consistency, Isolation, Durability) that can be traded off independently |
| Transaction | "A unit of work" | A sequence of operations that the DB treats as an indivisible whole — either all commit or all roll back |
| Isolation level | "How isolated transactions are" | A specific set of anomaly guarantees — from READ UNCOMMITTED (worst) to SERIALIZABLE (best) |
| Dirty read | "Reading uncommitted data" | Seeing a value written by a transaction that hasn't committed yet — that value may disappear |
| Phantom read | "Rows appearing/disappearing" | The same range scan returns different row sets within one transaction due to concurrent inserts/deletes |
| Write skew | "Two writes that shouldn't coexist" | Each transaction preserves an invariant locally, but the combination of both commits violates it |
| Snapshot isolation | "Each sees its own snapshot" | A multi-version scheme that prevents dirty reads and non-repeatable reads but allows write skew and the SI-anomaly |

## Further Reading

- *Transaction Processing: Concepts and Techniques* by Gray & Reuter — the canonical reference. Chapter 7 covers anomalies exhaustively.
- [PostgreSQL MVCC docs](https://www.postgresql.org/docs/current/mvcc.html) — how PostgreSQL implements isolation levels with visibility rules.
- [Heroku Dev Center: PostgreSQL Isolation Levels](https://devcenter.heroku.com/articles/postgresql-isolation-levels) — practical guide with real SQL schedules.
- Peter Bailis' [The RAMP Paper](http://www.bailis.org/blog/when-is-acid-acid-rarely/) — when ACID is and isn't ACID in practice.
