# Concurrency Control — 2PL, OCC, MVCC

> Serializing chaos: three ways to keep concurrent transactions from stepping on each other

**Type:** Build
**Languages:** Python, Rust
**Prerequisites:** Phase 10 lessons 13 (Transactions & ACID), 14 (Isolation Levels)
**Time:** ~90 minutes

## Learning Objectives

- Implement a two-phase locking deadlock detector in Python
- Explain why Strict 2PL prevents cascading aborts (and vanilla 2PL doesn't)
- Build an OCC validator in Rust and measure when it beats 2PL
- Trace an MVCC read in PostgreSQL using xmin/xmax and the commit log

## The Problem

Two transactions run at the same time. T1 transfers $100 from A to B. T2 reads A and B to compute a report. If T2 reads A after the debit but before the credit, the books don't balance — A + B < 2000. That's a non-serializable interleaving.

Serial execution would fix it, but serial execution defeats the purpose of a database: you want throughput. Concurrency control is the art of scheduling interleaved transactions so the result equals *some* serial execution, without actually serializing them. Three families of solutions exist:

1. **Pessimistic (2PL):** lock everything you touch, block others out.
2. **Optimistic (OCC):** charge ahead without locks, check for conflicts at commit.
3. **Versioned (MVCC):** give every reader a snapshot of the past; writers never block readers.

Each has a sweet spot. This lesson builds all three from scratch.

## The Problem

Three transactions run concurrently on a bank account table `(id, balance)`. T1: `UPDATE accounts SET balance = balance - 100 WHERE id = 1; UPDATE accounts SET balance = balance + 100 WHERE id = 2;`. T2: `SELECT SUM(balance) FROM accounts`. T3: `UPDATE accounts SET balance = balance * 1.05 WHERE id = 1;`.

Without concurrency control, T2 might see the $100 removed from account 1 but not yet added to account 2 — a non-serializable read. T3 might see the old balance on account 1 and overwrite T1's debit with a stale value — a lost update.

The goal: schedule these interleavings so the outcome is equivalent to *some* serial order (T1→T2→T3, T2→T1→T3, etc.), without actually executing them one at a time.

## The Concept

### Two-Phase Locking (2PL)

Every transaction has two phases:

1. **Growing phase:** acquire locks, never release.
2. **Shrinking phase:** release locks, never acquire.

If every transaction follows 2PL, the schedule is serializable. Proof sketch: the first unlock orders the transaction in a serialization graph — any cycle would require a lock acquisition after a release, which 2PL forbids.

```
T1: LOCK(A) → LOCK(B) → UNLOCK(A) → UNLOCK(B)    [serializable]
T1: LOCK(A) → UNLOCK(A) → LOCK(B) → UNLOCK(B)    [not 2PL — not serializable]
```

**Strict 2PL** strengthens this: hold **all** locks (especially write locks) until commit/abort. This prevents cascading aborts — if T2 reads uncommitted data from T1 and T1 aborts, T2 must abort too. With Strict 2PL, no transaction reads uncommitted data because writers hold locks until commit. InnoDB uses Strict 2PL + MVCC.

### Deadlock in 2PL

Two transactions waiting for each other: T1 holds A and wants B; T2 holds B and wants A. Cycle in the wait-for graph.

**Detection:** Build the wait-for graph; run DFS for cycles. Abort the youngest transaction in the cycle (minimal wasted work).

**Prevention:** Wound-wait (older tx wounds younger tx — forces it to abort and restart) or wait-die (older tx waits for younger; younger tx kills itself if it needs a lock held by older). Both are deadlock-free but may abort unnecessarily.

### Lock Modes & Granularity

| Mode | What it means | Compatible with |
|------|--------------|-----------------|
| S (shared) | Read lock | S, IS |
| X (exclusive) | Write lock | None |
| IS (intention shared) | "I might read something below" | IS, S, IX, SIX, X |
| IX (intention exclusive) | "I might write something below" | IS, IX |
| SIX (shared + intention exclusive) | "I'm reading here but writing below" | IS |

Intention locks sit at a higher granularity (e.g., table level) to signal that a transaction holds finer-grained locks (e.g., rows). The hierarchy: table → page → row. A transaction must acquire an IS or IX lock on the table before acquiring an S or X lock on a row. This lets the table-level lock manager quickly check compatibility without scanning every row.

**Granularity tradeoff:** Row-level locking gives high concurrency (two txs can update different rows) but costs memory per lock. Table-level locking is cheap but serializes everything. Most real databases default to row-level and escalate to table-level when a single transaction locks many rows.

### Optimistic Concurrency Control (OCC)

Assumes conflicts are rare. Three phases:

1. **Read phase:** Execute the transaction on a private copy. Track read-set (everything read) and write-set (everything written).
2. **Validation phase:** Check whether the transaction conflicts with concurrent transactions.
3. **Write phase:** If validation passes, atomically apply write-set to the database.

**Backward validation** (Kung-Robinson): check the transaction's read-set against the write-sets of transactions that already committed. If any read item was modified by a committed transaction after this one started, abort.

**Forward validation** (most common in practice): check the transaction's write-set against the read-sets of transactions still running. If this transaction's writes overlap another's reads, abort one of them (usually the one with fewer resources invested).

**Abort rate:** OCC shines under low contention (0–5% conflicts). Under high contention, abort rates skyrocket — wasted work snowballs. 2PL blocks early, which wastes less work on conflicts.

### Multi-Version Concurrency Control (MVCC)

Instead of locking — or instead of aborting on conflict — keep multiple versions of every row.

Each write creates a new version tagged with the writer's transaction ID (xid). Readers see a **snapshot** of the database at a point in time. A read never blocks a write, and a write never blocks a read.

**InnoDB:**
- Rollback segment stores old versions in undo logs.
- Each row has a `DB_TRX_ID` (latest modifying transaction) and `DB_ROLL_PTR` (pointer to undo log entry).
- Purge thread asynchronously deletes versions no visible transaction can see.
- Read view: at transaction start, InnoDB captures the list of active transaction IDs. A row version is visible iff its xid is committed before the read view, or is the transaction's own xid.

**PostgreSQL:**
- Each table has implicit `xmin` (creating xid) and `xmax` (deleting xid) system columns.
- `xmin` = the xid that inserted this row version. Visible if `xmin` is committed.
- `xmax` = the xid that deleted/updated this row version (0 if active). Visible if `xmax` is committed and the reading tx started after it.
- Commit log (clog, now called `pg_xact`): bit array mapping xid → committed/aborted.
- Hint bits: once a tuple's visibility is determined, set flags on the tuple to avoid consulting the clog next time.
- Autovacuum: freezes tuples older than `vacuum_freeze_min_age` to prevent xid wraparound, and removes dead row versions.

**MVCC vs 2PL vs OCC:**

| Property | 2PL | OCC | MVCC |
|----------|-----|-----|------|
| Readers block writers | Yes | No | No |
| Writers block readers | Yes | No | No |
| Writers block writers | Yes | Yes | Yes |
| Abort on conflict | No (wait) | Yes (retry) | Rare (serialization anomaly) |
| Memory overhead | Lock table | Read/write sets | Version chain |
| Best for | High contention | Low contention | Mixed OLTP |

## Build It: 2PL Deadlock Detector (Python)

### Step 1: Lock Manager with S/X Modes

Every resource can be locked in shared (S) or exclusive (X) mode. Multiple S locks are compatible; X locks conflict with everything. We maintain a lock queue per resource: when the head transaction releases, the next compatible set is granted.

The `LockManager` keeps a `HashMap[resource → deque[(txn_id, mode)]]`. `acquire` enqueues the request and checks if it can be granted immediately. `release` pops the head and wakes the next compatible batch.

### Step 2: Wait-For Graph from Lock Queues

If T2 is waiting for a lock held by T1, we add an edge T1 → T2 in the wait-for graph. We can build the graph by scanning every lock queue: for each resource, for every waiter behind a holder, add an edge from each holder to that waiter.

### Step 3: DFS Cycle Detection

Run DFS from every node in the wait-for graph. If we encounter a back-edge (a node already on the current path), we have a cycle. Extract the cycle by walking the parent pointers. Abort the youngest transaction (smallest wasted work).

### Step 4: Demo

Spawn threads that acquire locks in conflicting orders. The detector runs on a timer or after each lock wait. When a cycle forms, the youngest victim is aborted, its locks released, and the blocked transactions wake up.

## Build It: OCC Validator (Rust)

### Step 1: Key-Value Store and Transaction

A `DatabaseTable` stores `HashMap<u64, u64>`. An `OCCTransaction` records:
- `read_set: Vec<(u64, u64)>` — (key, observed value at read time)
- `write_set: Vec<(u64, u64)>` — (key, new value to write)

`begin()` allocates a transaction ID. `read(key)` checks the write-set first (own updates), then the table. `write(key, value)` buffers in the write-set.

### Step 2: Validation

On `commit()`:
1. Check every key in this transaction's read-set: if any concurrent transaction has written to that key and committed, abort.
2. More precisely: for each concurrently-running transaction, if this transaction's read-set intersects that transaction's write-set, abort this one (forward validation).
3. If validation passes, atomically apply the write-set to the table.

### Step 3: Retry

On abort, retry from the beginning. In practice you'd limit retries and surface an error to the client.

### Step 4: Benchmark

Compare 2PL (simulated via `std::sync::Mutex` per key, acquired in a fixed order) against OCC under:
- **Low contention (10 transactions, 100 keys each, 0.5% overlap):** OCC wins — almost no aborts, no lock overhead.
- **High contention (10 transactions, 3 keys each, 100% overlap):** 2PL wins — OCC aborts and retries repeatedly.

## Use It

**MySQL/InnoDB** uses Strict 2PL + MVCC. The `innodb_lock_wait_timeout` (default 50s) and `innodb_deadlock_detect` (default ON) control deadlock handling. You can see current locks with `SHOW ENGINE INNODB STATUS` and `performance_schema.data_locks`.

**PostgreSQL** uses MVCC with a Snapshot Isolation implementation. No 2PL for reads — readers never block. Serializable isolation in PG adds Serializable Snapshot Isolation (SSI) which detects serialization anomalies via predicate locks on index pages.

**FoundationDB** uses OCC at the storage layer. Read and write sets track conflicts at the key-range granularity. If validation fails (conflict with another committed transaction), the entire transaction retries. FDB's secret sauce is that the directory layer partitions data to minimize conflict probability.

## Read the Source

- [PostgreSQL `heapam_visiblity.c`](https://github.com/postgres/postgres/blob/master/src/backend/access/heap/heapam_visibility.c) — the core visibility check: `HeapTupleSatisfiesMVCC` compares xmin/xmax against the snapshot.
- [MySQL `lock0lock.cc`](https://github.com/mysql/mysql-server/blob/trunk/storage/innobase/lock/lock0lock.cc) — InnoDB's lock manager: `lock_rec_lock` and `lock_table_lock` with intention locks, wait queues, and deadlock detection.
- [FoundationDB `ConflictSet.cpp`](https://github.com/apple/foundationdb/blob/main/fdbclient/ConflictSet.cpp) — OCC conflict checking at key-range granularity.

## Ship It

The reusable artifact is the deadlock detector (`outputs/deadlock_detector.py`) and the OCC validator library (`outputs/occ_validator.rs`). Both are standalone and testable.

## Exercises

1. **Easy** — Add `IS`, `IX`, and `SIX` lock modes to the Python lock manager. Verify that IS + IS are compatible, IS + X are not, etc.
2. **Medium** — Implement wound-wait deadlock prevention in the Python 2PL. Compare abort counts against detection-based resolution in the demo.
3. **Hard** — Implement Snapshot Isolation in the OCC Rust validator: store the commit timestamp with each write and let readers see the most recent version committed before their start time. This is MVCC on top of OCC.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| 2PL | "Lock everything you need" | Two-phase protocol: grow (acquire) then shrink (release). Strict 2PL holds locks until commit. |
| OCC | "No locks, just retry" | Read phase (private copy + tracking), validation phase (conflict check), write phase (atomic apply). |
| MVCC | "Multiple versions of data" | Every write creates a new row version tagged with transaction ID; readers see a snapshot. |
| Wait-for graph | "Who's waiting for whom" | Directed graph: edge X → Y means X is waiting for a lock Y holds. Cycle = deadlock. |
| Snapshot Isolation | "Read a consistent snapshot" | A form of MVCC where readers see the latest version committed before their snapshot time. Not fully serializable (write skew is possible). |
| Cascading abort | "One abort causes others" | T2 reads uncommitted data from T1; T1 aborts → T2 must abort too. Prevented by Strict 2PL or MVCC. |

## Further Reading

- [Kung & Robinson (1981) "On Optimistic Methods for Concurrency Control"](https://dl.acm.org/doi/10.1145/319566.319567) — the original OCC paper, still relevant.
- [Gray & Reuter "Transaction Processing: Concepts and Techniques"](https://www.amazon.com/Transaction-Processing-Concepts-Techniques-Management/dp/1558601902) — Chapters 7–9, the definitive treatment.
- [PostgreSQL MVCC docs](https://www.postgresql.org/docs/current/mvcc-intro.html) — official docs with system column details.
- [CMU 15-445 Concurrency Control slides](https://15445.courses.cs.cmu.edu/fall2024/slides/16-concurrencycontrol.pdf) — Andy Pavlo's lecture, excellent diagrams.
