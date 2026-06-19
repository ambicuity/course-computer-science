# Isolation Levels — Read Committed → Serializable

> Isolation levels are the dial that trades consistency for concurrency — turning it too far toward consistency and your database stalls, too far toward concurrency and your data corrupts.

**Type:** Learn
**Languages:** SQL, Python
**Prerequisites:** Phase 10 lessons 01–13
**Time:** ~75 minutes

## Learning Objectives

- Name the four SQL isolation levels and the anomalies each prevents.
- Implement a transaction scheduler in Python that enforces each isolation level via versioned reads and conflict detection.
- Explain the trade-off between consistency and concurrency using concrete anomaly schedules.
- Choose the right isolation level for a given production workload and identify which database implements it how.

## The Problem

Alice has $1000, Bob has $500. A bank transfer moves $100 from Alice to Bob. The correct sequence: subtract 100 from Alice (→ $900), add 100 to Bob (→ $600). Now suppose two transactions run concurrently. Transaction T1 reads Alice's balance ($1000) and is about to subtract $100. Transaction T2 simultaneously reads Alice's balance — what does it see?

If T2 sees $1000 (the pre-transfer value), it's reading a state that's about to change. If T2 sees $900 (T1's mid-flight write), it's reading a value that may vanish if T1 aborts. If T2 sees $1000 again after T1 committed — even though the real balance is now $900 — the system has a non-repeatable read. Without controlled isolation, concurrent transactions produce every wrong answer you can imagine.

In production, this matters everywhere: booking systems double-sell seats, inventory systems report negative stock, analytics dashboards show sums that never existed. The database provides isolation levels to let you pick your poison: how much inconsistency can your application tolerate before the data breaks?

## The Concept

### The four SQL isolation levels

The SQL standard defines four levels from weakest to strongest:

| Level | Dirty Read | Non-Repeatable Read | Phantom Read | Write Skew |
|-------|-----------|---------------------|--------------|------------|
| READ UNCOMMITTED | Possible | Possible | Possible | Possible |
| READ COMMITTED | Prevented | Possible | Possible | Possible |
| REPEATABLE READ | Prevented | Prevented | Possible | Possible |
| SERIALIZABLE | Prevented | Prevented | Prevented | Prevented |

### Anomalies by example

- **Dirty read**: T1 writes X=5, T2 reads X and sees 5, T1 rolls back. T2 saw a value that never existed in any committed state.
- **Non-repeatable read**: T1 reads X=100, T2 writes X=200 and commits, T1 reads X again and gets 200. T1's two reads disagree.
- **Phantom read**: T1 scans `WHERE amount > 100` and gets 2 rows. T2 inserts a third row matching the predicate and commits. T1 rescans and gets 3 rows. The new row is a phantom.
- **Lost update**: T1 reads counter=100, adds 10 locally. T2 reads counter=100, adds 20 locally. T1 writes 110, commits. T2 writes 120, commits. The final value is 120 — T1's +10 is lost.
- **Read skew**: T1 reads X then Y. Between those reads, T2 modifies both X and Y. T1 sees an old X paired with a new Y — a combination that never existed.
- **Write skew**: Two doctors must always have at least one on call. Both check the schedule, see the other is on call, and each takes themself off. Both commit. Nobody is on call.

### How each level is implemented

- **Read Uncommitted**: No isolation. Reads return the latest value from any in-flight transaction.
- **Read Committed**: Statement-level snapshot. Each query sees the latest committed data at the moment the query starts.
- **Repeatable Read**: Transaction-level snapshot. The first query freezes a snapshot; all subsequent reads in the same transaction see that snapshot.
- **Serializable**: Transaction-level snapshot plus conflict detection. The system detects read-write dependencies and aborts transactions that would produce a non-serializable outcome. PostgreSQL uses **Serializable Snapshot Isolation (SSI)**: a graph-based conflict detector that tracks r/w dependencies and aborts one transaction in a cycle.

### The isolation-concurrency trade-off

```
Higher consistency                  Higher concurrency
     SERIALIZABLE ← ← ← ← ← → → → → →  READ UNCOMMITTED
        Fewer anomalies                 More anomalies
        Lower throughput                Higher throughput
        More aborts                     Fewer aborts
```

Snapshot Isolation (SI) is not an SQL standard level but a common implementation technique used by PostgreSQL (for REPEATABLE READ), Oracle, and SQL Server. SI prevents dirty reads, non-repeatable reads, and phantoms, but allows write skew and the SI-anomaly (a specific write-skew pattern).

## Build It

### Step 1: Python Transaction Scheduler

The scheduler models a key-value database where each transaction operates under a configurable isolation level. Open `code/main.py`.

```python
class IsolationLevel(Enum):
    READ_UNCOMMITTED = "READ UNCOMMITTED"
    READ_COMMITTED = "READ COMMITTED"
    REPEATABLE_READ = "REPEATABLE READ"
    SNAPSHOT = "SNAPSHOT"
    SERIALIZABLE = "SERIALIZABLE"
```

Each transaction has a snapshot (frozen at `BEGIN` for RR/Snapshot/Serializable), a write-set, and a status (active/committed/aborted). The scheduler dispatches reads and writes according to the isolation level:

- **Read Uncommitted**: reads bypass all visibility checks — return the latest value from any active write-set.
- **Read Committed**: reads check only the committed version history.
- **Repeatable Read / Snapshot**: reads consult the snapshot taken at transaction start.
- **Serializable**: same as Snapshot, but the scheduler maintains a conflict graph. On `COMMIT`, if the graph has a cycle, one transaction is aborted.

```python
class Scheduler:
    def read(self, txn_id, key):
        if self.isolation_level == IsolationLevel.READ_UNCOMMITTED:
            # Check active write-sets first
            for tid, writes in self.active_writes.items():
                if key in writes:
                    return writes[key][0]
            return self.db.read(key, self.isolation_level)

        elif self.isolation_level == IsolationLevel.READ_COMMITTED:
            return self.db.read(key, self.isolation_level)
            # reads latest committed value only

        elif self.isolation_level in (IsolationLevel.REPEATABLE_READ,
                                      IsolationLevel.SNAPSHOT,
                                      IsolationLevel.SERIALIZABLE):
            snapshot = {"snapshot_version": self.txn_versions[txn_id]}
            return self.db.read(key, IsolationLevel.REPEATABLE_READ, snapshot)
```

### Step 2: SQL Demonstration

Open `code/main.sql`. Each section demonstrates one isolation level in PostgreSQL with expected output as comments.

```sql
-- Read Committed: T1 sees T2's committed update on second read
BEGIN ISOLATION LEVEL READ COMMITTED;
    SELECT balance FROM bank_accounts WHERE account_id = 1;
    -- Returns 1000.00 (committed)
    -- Meanwhile, T2 updates id=1 to 900.00 and commits
    SELECT balance FROM bank_accounts WHERE account_id = 1;
    -- Returns 900.00 — non-repeatable read!
COMMIT;
```

```sql
-- Repeatable Read: T1 sees same snapshot regardless of T2's commits
BEGIN ISOLATION LEVEL REPEATABLE READ;
    SELECT balance FROM bank_accounts WHERE account_id = 1;
    -- Returns 900.00
    -- Meanwhile, T2 updates id=1 to 0 and commits
    SELECT balance FROM bank_accounts WHERE account_id = 1;
    -- Still returns 900.00 — frozen snapshot
COMMIT;
```

```sql
-- Serializable: prevents write skew via SSI conflict detection
BEGIN ISOLATION LEVEL SERIALIZABLE;
    UPDATE on_call SET on_call = false WHERE doctor_id = 1;
    -- If T2 ran the same logic for doctor_id = 2,
    -- one of the two commits gets:
    -- ERROR: could not serialize access due to read/write dependencies
COMMIT;
```

## Use It

Each major database implements isolation levels differently:

| Database | Default Level | Read Uncommitted | Serializable Implementation |
|----------|--------------|------------------|---------------------------|
| PostgreSQL | Read Committed | Treated as RC | SSI (predicate locks + conflict graph) |
| MySQL/InnoDB | Repeatable Read | Real RU | Next-key locks (locking, not SSI) |
| Oracle | Read Committed (snapshot) | Not supported | No real serializable (snapshot only) |
| SQL Server | Read Committed | Real RU | Lock-based serializability |

Key differences:
- PostgreSQL's `READ UNCOMMITTED` behaves identically to `READ COMMITTED` because the MVCC architecture cannot expose uncommitted writes without breaking the snapshot model. Everything is a snapshot at some point in time.
- MySQL's `REPEATABLE READ` uses next-key locking, which prevents phantoms via locks but reduces concurrency compared to MVCC snapshots.
- Oracle's `SERIALIZABLE` is actually Snapshot Isolation — it prevents dirty reads and non-repeatable reads but allows write skew.
- PostgreSQL's `SERIALIZABLE` is true serializability via SSI, which tracks r/w dependencies using predicate locks (stored in shared memory, not on every tuple).

## Read the Source

- PostgreSQL `src/backend/access/transam/` — MVCC visibility rules (`heapam_visibility.c`), transaction IDs (`xact.c`), commit log (`clog.c`).
- PostgreSQL `src/backend/storage/lmgr/predicate.c` — predicate locking for Serializable Snapshot Isolation.
- PostgreSQL `src/backend/utils/time/snapmgr.c` — snapshot creation and management for each isolation level.
- "Serializable Snapshot Isolation in PostgreSQL" (Ports & Grittner, VLDB 2012) — the paper that SSI is based on.

## Ship It

The reusable artifact is a configurable transaction scheduler in `outputs/` that enforces any isolation level. It can be imported into later lessons (Phase 14 concurrency, Phase 15 distributed systems) to test isolation behaviors without a real database.

- `outputs/isolation_scheduler.py` — standalone scheduler class with versioned reads and SSI conflict detection.
- `outputs/README.md` — usage guide.

## Exercises

1. **Easy** — Given this schedule, identify which anomaly occurs at READ COMMITTED: T1 reads X, T2 writes X and commits, T1 reads X again. Then show why REPEATABLE READ prevents it.
2. **Medium** — Implement snapshot isolation in the Python scheduler: each transaction reads from a snapshot taken at its start time. Demonstrate that write skew still occurs under SI but not under Serializable.
3. **Hard** — Implement serializable snapshot isolation (SSI): maintain a conflict graph of r/w dependencies between transactions. On commit, run cycle detection and abort one transaction per cycle to enforce serializability.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Isolation level | "How isolated transactions are" | A specific set of anomaly guarantees — from READ UNCOMMITTED (worst) to SERIALIZABLE (best) |
| Dirty read | "Reading uncommitted data" | Seeing a value written by a transaction that hasn't committed — that value may disappear on rollback |
| Non-repeatable read | "Two reads disagree" | The same row read twice within one transaction returns different values because another transaction committed an update in between |
| Phantom read | "Rows appearing out of nowhere" | Re-executing a range query returns different rows because another transaction inserted/deleted rows matching the predicate |
| Snapshot isolation | "Each transaction sees a point-in-time snapshot" | An MVCC-based scheme that prevents dirty reads, non-repeatable reads, and phantoms but allows write skew |
| Serializable | "Transactions appear sequential" | The strongest isolation level; guarantees that the result is equivalent to some serial execution of the same transactions |
| MVCC | "Multi-version concurrency control" | The database keeps multiple versions of each row so readers don't block writers and writers don't block readers |
| Predicate lock | "Locks the condition, not the row" | A lock on a search condition (e.g., `WHERE balance > 100`) to prevent phantoms without locking every matching row |
| SSI | "Serializable Snapshot Isolation" | PostgreSQL's algorithm for true serializability: snapshot isolation + cycle detection on the r/w dependency graph |
| Lost update | "My write disappeared" | Two concurrent transactions both read the same value, modify it, and write; the second write overwrites the first |
| Write skew | "Two wrongs make a wrong" | Two transactions read overlapping data and make conflicting writes; each preserves an invariant alone, but together they violate it |

## Further Reading

- "A Critique of ANSI SQL Isolation Levels" by Berenson et al. (SIGMOD 1995) — the paper that defined the modern understanding of isolation anomalies.
- PostgreSQL Documentation, Chapter 13: "Concurrency Control" — definitive reference for MVCC and isolation in PostgreSQL.
- *Designing Data-Intensive Applications* by Martin Kleppmann, Chapter 7 — the clearest book-level treatment of transactions and isolation.
- "Serializable Snapshot Isolation in PostgreSQL" by Ports & Grittner (VLDB 2012) — the paper describing PostgreSQL's SSI implementation.
- *Readings in Database Systems* (the Red Book), Section on Isolation — curated collection of the most important papers.
