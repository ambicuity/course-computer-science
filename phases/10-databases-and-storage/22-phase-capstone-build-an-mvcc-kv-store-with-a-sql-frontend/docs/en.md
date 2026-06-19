# Phase Capstone — Build an MVCC KV Store with a SQL Frontend

> Every lesson in Phase 10 converges here: one working database from slotted pages to SQL.

**Type:** Build
**Languages:** Rust (full implementation), SQL (demonstration)
**Prerequisites:** All Phase 10 lessons (01–21)
**Time:** ~150 minutes

## Learning Objectives

- Integrate slotted pages, buffer pool, B+ Tree, LSM-Tree, MVCC, WAL, and SQL parsing into a single working system
- Implement a recursive-descent SQL parser that produces an AST and a Volcano-style query executor
- Build an MVCC transaction manager with snapshot isolation and first-committer-wins write conflict detection
- Implement ARIES-style WAL with analysis/redo/undo crash recovery
- Trace a query through every layer: parser → binder → planner → executor → storage engine → disk

## The Problem

Every lesson in this phase teaches one piece of the puzzle. Slotted pages store records. The buffer pool caches them. B+ Trees index them. LSM-Trees accelerate writes. MVCC keeps transactions isolated. WAL/ARIES survives crashes. A SQL parser and planner let users query with a familiar language.

But a database is not a collection of parts — it's a **pipeline**. A `SELECT` starts as text, becomes an AST, becomes a logical plan, becomes a physical plan, and finally walks through buffer pool frames and slotted page slots to return rows. A `BEGIN TRANSACTION` pins a snapshot, and every subsequent read consults `begin_ts`/`end_ts` to decide which row version to show. A crash during any of this means the WAL must replay or undo the partial work.

The capstone forces you to wire every subsystem together. You cannot build one piece in isolation — the parser must produce plans the executor can run, the executor must call the MVCC manager, the MVCC manager must write WAL records, and the WAL must integrate with the buffer pool's flush-on-eviction policy. This is where database theory meets systems engineering.

## The Concept

### Architecture Overview

The system has nine layers, each corresponding to a Phase 10 lesson:

```
┌─────────────────────────────────────────────────────────┐
│                    SQL Frontend (CLI REPL)               │
├─────────────────────────────────────────────────────────┤
│  Parser (recursive descent)   ──►    AST                 │
├─────────────────────────────────────────────────────────┤
│  Binder (resolve names)       ──►    Logical Plan        │
├─────────────────────────────────────────────────────────┤
│  Planner (index selection)    ──►    Physical Plan       │
├─────────────────────────────────────────────────────────┤
│  Executor (Volcano iterator model)                      │
│  ┌──────────┐ ┌──────────┐ ┌──────┐ ┌───────────────┐  │
│  │ SeqScan  │ │IndexScan │ │Filter│ │ HashJoin      │  │
│  └────┬─────┘ └────┬─────┘ └──┬───┘ └───────┬───────┘  │
│       └────────────┼──────────┼──────────────┘          │
│                    ▼          ▼                          │
│              Storage Engine Layer                        │
│  ┌────────┐ ┌────────┐ ┌──────────┐ ┌───────────────┐  │
│  │Buffer  │ │ B+ Tree │ │ LSM-Tree │ │ MVCC Txn Mgr │  │
│  │Pool    │ │ Index   │ │ Engine   │ │ (snapshots)  │  │
│  └───┬────┘ └────────┘ └──────────┘ └───────┬───────┘  │
│      │                                       │          │
│      ▼                                       ▼          │
│  ┌──────────┐                         ┌──────────────┐  │
│  │ Slotted  │                         │ WAL / ARIES  │  │
│  │ Pages    │                         │ (crash rec)  │  │
│  └────┬─────┘                         └──────┬───────┘  │
│       │                                      │          │
│       ▼                                      ▼          │
│  ┌──────────────────────────────────────────────────┐   │
│  │              Disk (.data, .index, .wal)          │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Data Flow

**Write path (INSERT/UPDATE/DELETE):**
1. SQL text → Parser → `AST::Insert`
2. Binder resolves table/schema → `LogicalPlan::Insert`
3. Transaction manager assigns `txn_id` and snapshot
4. WAL record written (BEFORE any page modification)
5. Row stored in slotted page with `begin_ts = txn_id`
6. B+ Tree index updated with new RID
7. On commit: WAL commit record written; transaction becomes visible

**Read path (SELECT):**
1. SQL text → Parser → `AST::Select`
2. Binder resolves columns, detects WHERE clause on PK
3. Planner chooses IndexScan (if PK predicate) or SeqScan
4. Executor opens iterator: `IndexScan → Filter → Projection`
5. For each tuple: MVCC visibility check (begin_ts vs snapshot)
6. Rows returned to CLI

**Recovery path (after crash):**
1. Read WAL from disk into memory
2. Analysis phase: reconstruct transaction table and dirty page table
3. REDO phase: replay all updates (committed and uncommitted) from the last checkpoint
4. UNDO phase: roll back uncommitted transactions using CLRs

### MVCC Model

Every row version carries two timestamps:

```
┌──────────────────────────────────────────────────┐
│ Row Version                                      │
│ ┌────────┬────────┬──────────────────────────┐   │
│ │begin_ts│ end_ts │ column data ...          │   │
│ └────────┴────────┴──────────────────────────┘   │
│ begin_ts = transaction ID that created this row  │
│ end_ts   = transaction ID that deleted/updated   │
│            this row (0 = still visible)          │
└──────────────────────────────────────────────────┘
```

A row version is visible to transaction `T` with snapshot `S` iff:
- `begin_ts` is committed in `S` (or is T itself)
- `end_ts` is 0, or `end_ts` is not committed in `S`

**First-committer-wins:** If two concurrent transactions modify the same row, the second to commit detects the conflict (the other's version has `end_ts` set to the first committer's ID) and aborts.

## Build It

### Step 1: Storage Format — Slotted Pages

Every data file is a sequence of 4096-byte pages. Each page uses the slotted layout from Lesson 05:

```
Offset  Size  Field
──────  ────  ──────────────────────────────
  0      4    page_id (u32)
  4      2    free_start (u16) — end of slot array
  6      2    data_end (u16) — start of free space
  8      2    slot_count (u16)
 10      2    lsn (u16) — last WAL sequence number
 12     12    reserved
 24      N    slot array (grows forward)
 free   N     record data (grows backward from 4096)
```

A slot entry is 4 bytes: `offset (u16) | length (u16)`. A deleted slot is `(0, 0)`.

The page header stores the LSN so that during ARIES REDO we can skip re-applying log records that already made it to disk (the LSN on page >= the LSN in the log record).

```rust
struct SlottedPage {
    buffer: [u8; PAGE_SIZE],
}

impl SlottedPage {
    fn insert(&mut self, data: &[u8]) -> Option<u16> { /* ... */ }
    fn get(&self, slot: u16) -> Option<&[u8]> { /* ... */ }
    fn delete(&mut self, slot: u16) { /* ... */ }
    fn update(&mut self, slot: u16, data: &[u8]) -> bool { /* ... */ }
    fn defragment(&mut self) { /* ... */ }
}
```

### Step 2: Buffer Pool with Clock Eviction

The buffer pool keeps frequently accessed pages in memory. When a page is requested that isn't in the pool, we evict an unpinned page using the Clock (second-chance) algorithm from Lesson 06.

Each frame has:
- `page_id` (or `None` if empty)
- `pin_count` — number of current users
- `dirty` — was the page modified?
- `ref` — reference bit for Clock algorithm

```rust
struct BufferPool {
    frames: Vec<Frame>,
    clock_hand: usize,
}

impl BufferPool {
    fn pin(&mut self, page_id: u32, disk: &mut HeapFile) -> &mut SlottedPage { /* ... */ }
    fn unpin(&mut self, page_id: u32, dirty: bool) { /* ... */ }
    fn flush_page(&mut self, page_id: u32, disk: &mut HeapFile) { /* ... */ }
    fn flush_all(&mut self, disk: &mut HeapFile) { /* ... */ }
}
```

The Clock algorithm walks frames in a circle. If a frame has `ref = true`, we clear the bit and move on (second chance). If `ref = false` and `pin_count == 0`, we evict it (flush if dirty).

### Step 3: B+ Tree Index

The B+ Tree maps primary keys to RIDs `(page_id, slot)`. For this capstone we implement a simplified two-level B+ Tree that captures the essential concepts:

- **Internal nodes**: store separator keys and child page pointers
- **Leaf nodes**: store (key, RID, begin_ts, end_ts) pairs for MVCC visibility

The tree is persisted in a separate `.index` file. Internal operations (split, merge) follow the standard B+ Tree algorithm from Lesson 07.

```rust
struct BTree {
    root_page: u32,
    // leaf node entry: (key, page_id, slot, begin_ts, end_ts)
}
```

### Step 4: LSM-Tree Write Path

For write-heavy workloads, writes go through an LSM engine (Lesson 09) before reaching the B+ Tree:

1. **MemTable** (in-memory BTreeMap): all writes land here first
2. **Immutable MemTable**: when the memtable exceeds a size threshold, it's frozen and a new memtable takes over
3. **SSTable flush**: the frozen memtable is flushed to disk as a sorted string table
4. **Compaction**: background process merges SSTables and removes stale keys

```rust
struct LSMTree {
    memtable: BTreeMap<Vec<u8>, Vec<u8>>,
    immutable: Option<BTreeMap<Vec<u8>, Vec<u8>>>,
    levels: Vec<Vec<SSTable>>,
    threshold: usize,
}
```

The read path checks: memtable → immutable → L0 SSTables → L1 SSTables → ...

### Step 5: MVCC Transaction Manager

The transaction manager assigns monotonically increasing transaction IDs and maintains snapshots.

```rust
struct TransactionManager {
    next_txn_id: u64,
    active: Vec<u64>,
    committed: Vec<(u64, u64)>, // (txn_id, commit_ts)
}

struct Transaction {
    txn_id: u64,
    snapshot: Vec<u64>, // active transactions at start
    status: Status,     // Active | Committed | Aborted
}

impl TransactionManager {
    fn begin(&mut self) -> Transaction { /* ... */ }
    fn commit(&mut self, txn: &mut Transaction) { /* ... */ }
    fn rollback(&mut self, txn: &mut Transaction) { /* ... */ }
    fn visible(&self, begin_ts: u64, end_ts: u64, snapshot: &[u64]) -> bool {
        begin_ts <= self.max_committed(snapshot)
            && (end_ts == 0 || end_ts > self.max_committed(snapshot))
    }
}
```

The visibility check: a row version is visible if its `begin_ts` was committed before the snapshot and its `end_ts` hasn't been committed yet (or is 0, meaning still active).

### Step 6: WAL and ARIES Crash Recovery

Every page modification is preceded by a log record. The log is an append-only file of records. Each record has:

```
┌───────────────────────────────────────────┐
│ LSN (u64) — global sequence number        │
│ prev_lsn (u64) — previous LSN for this tx │
│ txn_id (u64) — which transaction          │
│ record_type (u8) — BEGIN|INSERT|COMMIT...│
│ page_id (u32) — affected page             │
│ payload (variable) — data (for undo/redo) │
└───────────────────────────────────────────┘
```

Three-phase recovery (ARIES):
1. **Analysis**: scan WAL from the last checkpoint to rebuild the transaction table (which txns were active) and the dirty page table (which pages might need REDO)
2. **REDO**: replay from the earliest LSN in the dirty page table. Skip pages whose `page_lsn >= record_lsn` — they already have this update.
3. **UNDO**: roll back all transactions that were active at crash time. Write CLRs (Compensation Log Records) for each undo to ensure idempotency.

```rust
fn recover(wal: &mut WalLog, heap: &mut HeapFile) {
    let (txn_table, dirty_pages, checkpoint_lsn) = analysis(wal);
    let min_dirty = dirty_pages.iter().map(|(_, lsn)| lsn).min().unwrap_or(0);
    redo(wal, heap, min_dirty, &dirty_pages);
    undo(wal, heap, &txn_table);
}
```

### Step 7: SQL Parser (Recursive Descent)

The parser converts SQL text into an AST. We support a minimal SQL dialect:

```sql
CREATE TABLE name (col1 TYPE, col2 TYPE, ...);
INSERT INTO name VALUES (val1, val2, ...);
SELECT col1, col2 FROM name WHERE condition;
UPDATE name SET col = val WHERE condition;
DELETE FROM name WHERE condition;
BEGIN [TRANSACTION];
COMMIT;
ROLLBACK;
```

The tokenizer produces a stream of tokens. The parser descends through grammar rules:

```rust
enum Token { Create, Table, Ident(String), Number(i64), String(String), /* ... */ }

enum Statement {
    CreateTable { name: String, columns: Vec<ColumnDef> },
    Insert { table: String, values: Vec<Vec<Value>> },
    Select { columns: Vec<String>, table: String, where_clause: Option<Expr> },
    Begin, Commit, Rollback,
}

fn parse(tokens: &[Token]) -> Statement { /* recursive descent */ }
```

### Step 8: Planner and Executor

The planner converts an AST into a physical plan. For `SELECT * FROM users WHERE id = 5` with a primary key on `id`, the planner chooses `IndexScan("users", 5)`. For a table scan without filter, it chooses `SeqScan("users")`.

The executor uses the Volcano iterator model (Lesson 10): every operator implements `next() -> Option<Row>`. Operators compose: a `Filter` wraps a `SeqScan`, a `Projection` wraps a `Filter`.

```rust
trait Executor {
    fn next(&mut self) -> Option<Vec<Value>>;
}

struct SeqScan { /* iterates pages */ }
struct Filter { child: Box<dyn Executor>, predicate: Expr }
struct Projection { child: Box<dyn Executor>, columns: Vec<usize> }
struct InsertExec { /* inserts rows from child into table */ }
```

### Step 9: Putting It Together — The REPL

The CLI reads SQL from stdin, executes it, and prints results. It initializes the database (loading pages from disk, recovering from WAL if needed), then enters a read-eval-print loop:

```
$ cargo run
db> CREATE TABLE users (id INT, name TEXT, email TEXT);
OK
db> INSERT INTO users VALUES (1, 'Alice', 'alice@example.com');
OK
db> INSERT INTO users VALUES (2, 'Bob', 'bob@example.com');
OK
db> SELECT * FROM users;
1 | Alice | alice@example.com
2 | Bob | bob@example.com
db> BEGIN;
OK
db> INSERT INTO users VALUES (3, 'Charlie', 'charlie@example.com');
OK
db> SELECT * FROM users;
1 | Alice | alice@example.com
2 | Bob | bob@example.com
3 | Charlie | charlie@example.com
db> ROLLBACK;
OK
db> SELECT * FROM users;
1 | Alice | alice@example.com
2 | Bob | bob@example.com
db> EXIT;
```

### Step 10: Tests

Each subsystem has unit tests. Run them with `cargo test`. The test suite covers:

- **SlottedPage**: insert, get, delete, update, defrag
- **BufferPool**: pin, unpin, eviction order, dirty flush
- **BTree**: insert, search, range scan
- **MVCC**: snapshot isolation, write-write conflict
- **WAL**: log append, recovery (analysis, redo, undo)
- **SQL Parser**: all statement types, expressions
- **Integration**: full CRUD with transactions

## Use It

**SQLite** is the closest production comparison — a single-binary embedded database with a C API. Like our capstone, SQLite uses slotted pages (of 4KB) and a B+ Tree for both table data and indexes. Unlike our capstone, SQLite does NOT use MVCC for isolation — it uses a coarse-grained reader-writer lock on the entire database file. A write in SQLite blocks all readers, and a read blocks all writers (except in WAL mode, which adds a limited form of concurrency). This is the #1 reason SQLite doesn't scale to high-concurrency workloads.

**PostgreSQL** uses a much more sophisticated architecture. It has a separate set of processes (not threads), a shared buffer pool with multiple replacement policies, full MVCC with `xmin`/`xmax`/`clog`/`hint bits`, a multi-phase query optimizer with cost-based join ordering, and ARIES-compatible WAL. PostgreSQL's MVCC allows concurrent readers and writers without blocking — the same model our capstone implements. PostgreSQL also supports serializable snapshot isolation (SSI), which detects serialization anomalies by tracking read-write conflicts at the predicate level.

**RocksDB** is the LSM-Tree extreme. It has no MVCC and no SQL frontend — it's a key-value store. But it's where the LSM portion of our system points: write-optimized storage with leveled compaction, bloom filters, and a memtable → immutable → SSTable pipeline.

| Feature | Our Capstone | SQLite | PostgreSQL | RocksDB |
|---------|-------------|--------|------------|---------|
| MVCC | Snapshot isolation | Reader-writer lock | Full MVCC + SSI | None |
| Storage | B+ Tree + LSM | B+ Tree | Heap + B+ Tree | LSM |
| SQL | Recursive descent | Lemon parser | Bison + custom | None |
| WAL/Recovery | ARIES-style | WAL (rollback journal) | Full ARIES | WAL (no MVCC undo) |
| Concurrency | First-committer-wins | Database-level lock | Row-level + SSI | Key-level locking |

## Ship It

The reusable artifact is a working embedded database library + CLI tool in `code/`. Build with `cargo build`, run with `cargo run`. The database stores data in binary files (`.data`, `.index`, `.wal`) in the current directory. It supports basic SQL with MVCC transactions and crash recovery.

Save the artifact to `outputs/` as a distributable binary:

```bash
cd code
cargo build --release
cp target/release/mvcc-sql ../outputs/
```

## Exercises

1. **Easy** — Add `CREATE INDEX` on non-primary-key columns. Implement `CREATE INDEX idx_name ON table(col)`. The index should be stored as a separate B+ Tree and used by the planner when a WHERE clause references the indexed column.

2. **Medium** — Implement `INSERT ... SELECT`. Parse `INSERT INTO table SELECT ...`, bind it, and execute as a single transaction: open a scan on the source table, iterate rows, and insert each into the destination table. Ensure MVCC consistency — both the scan and inserts share the same snapshot.

3. **Hard** — Add JOIN support. Implement `SELECT * FROM t1 JOIN t2 ON t1.id = t2.ref_id` with a nested loop join: for each row in the outer (t1), scan the inner (t2) for matching rows. If both tables have indexes, the planner should use an index scan for the inner table. Handle MVCC visibility for both sides of the join.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Slotted page | "A page with variable-length records" | Fixed-size buffer where metadata grows from the front and records grow from the back, meeting in the middle |
| Buffer pool | "Cache for disk pages" | In-memory array of page frames with replacement policy (Clock), pin/unpin tracking, and dirty-page management |
| Clock eviction | "Page replacement with second chance" | Circular scan of frames; clear reference bit on first pass, evict on second pass if still unreferenced |
| B+ Tree | "Balanced tree for indexing" | Height-balanced tree where internal nodes guide search and leaves contain (key, value) pairs, chained for range scans |
| LSM-Tree | "Write-optimized storage" | In-memory sorted tree flushed to disk as immutable sorted runs, compacted in the background |
| SSTable | "Sorted String Table" | Immutable file of sorted (key, value) pairs on disk, typically with an index and bloom filter |
| MVCC | "Multiple row versions" | Each write creates a new version tagged with begin_ts/end_ts; readers see a consistent snapshot |
| Snapshot isolation | "Read a point-in-time view" | Transaction sees all rows committed before its start time and none committed after; write-write conflicts are detected |
| First-committer-wins | "Whoever commits first keeps their write" | If two concurrent transactions modify the same row, the second to commit is aborted — no lost updates |
| WAL | "Write-ahead log" | Append-only sequence of log records on stable storage, written before every page modification |
| ARIES | "Algorithm for Recovery and Isolation Exploiting Semantics" | Three-phase recovery: Analysis (rebuild state), REDO (replay all changes), UNDO (roll back uncommitted) |
| Volcano iterator model | "Pull-based query execution" | Every operator exposes `next() -> Option<Row>`; operators compose by wrapping each other |

## Further Reading

- [Architecture of a Database System](https://dsf.berkeley.edu/papers/fntdb07-architecture.pdf) (Hellerstein, Stonebraker, Hamilton) — The canonical overview of every subsystem in a relational database
- [PostgreSQL System Catalogs](https://www.postgresql.org/docs/current/catalogs.html) — How PostgreSQL stores table/index metadata
- [SQLite Architecture](https://www.sqlite.org/arch.html) — Single-page description of SQLite's architecture, useful for comparison
- [CMU 15-445/645 Database Systems](https://15445.courses.cs.cmu.edu/fall2024/) — Andy Pavlo's full course with lecture videos and projects
- [RocksDB Wiki](https://github.com/facebook/rocksdb/wiki) — LSM-Tree internals, compaction strategies, bloom filters
- [ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollbacks](https://web.stanford.edu/class/cs345a/slions/aries.pdf) — The original ARIES paper by Mohan et al.
