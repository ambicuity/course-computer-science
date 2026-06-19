# Recovery — WAL, ARIES

> A crash is not a question of if, but when. WAL and ARIES are how a database answers: "What was the last committed state?"

**Type:** Build | Learn
**Languages:** Rust
**Prerequisites:** Phase 10 lessons 01–15 (buffer pool, pages, transactions, concurrency)
**Time:** ~90 minutes

## Learning Objectives

- Explain why crash recovery is needed and how the WAL invariant enables it
- Describe ARIES's three-phase recovery protocol: Analysis, REDO, UNDO
- Implement a working ARIES recovery simulator in Rust from scratch
- Trace LSN, DPT, CLR, and fuzzy checkpoint interactions through a crash scenario

## The Problem

You are transferring \$100 from account A to account B. The database debits A on page 7, then credits B on page 12. Halfway through, the power dies. Page 7 was written to disk. Page 12 was still in the buffer pool. The \$100 is gone — the database is inconsistent.

This isn't a hardware problem. It's a *coordination* problem. The storage engine manages a volatile buffer pool (fast, lost on crash) and stable disk (slow, survives). At any instant, some pages in the buffer pool are dirty (modified but not yet flushed), and some pages on disk are stale. After a crash, the disk has an arbitrary mix of old and new data — some committed, some uncommitted, some partially written.

Without a recovery protocol, the database administrator's only option is to restore yesterday's backup and replay a day's worth of transaction logs by hand. No production database does this. Instead, every serious database engine (PostgreSQL, MySQL/InnoDB, SQL Server, DB2) implements **ARIES**: the industry-standard crash recovery protocol that guarantees the database will reach a consistent state reflecting all committed transactions and no uncommitted ones.

Concretely this means: after recovery, every effect of a committed transaction is visible, and no effect of an uncommitted (or aborted) transaction is visible — even though during normal operation, dirty pages from uncommitted transactions may have been flushed to disk (STEAL), and dirty pages from committed transactions may still be in the buffer pool (NO-FORCE).

## The Concept

### The Write-Ahead Log (WAL)

The foundation of every crash recovery scheme is the **Write-Ahead Log** — an append-only sequence of records on stable storage that describes every modification to the database. The critical invariant:

> **WAL invariant**: A log record must reach stable storage **before** the corresponding dirty page is written to disk.

This means the log is always "ahead" of the data. If the data page gets corrupted or goes missing, the log contains enough information to reconstruct it. If the log record itself is lost (crash during log write), the corresponding page write will never happen — the system crashes forward, not backward.

A WAL record typically contains:

```
┌──────────────────────────────────────────────┐
│ LSN        : Log Sequence Number (monotonic) │
│ prevLSN    : previous LSN of this transaction │
│ transID    : which transaction               │
│ type       : BEGIN | UPDATE | COMMIT | ABORT │
│             | CLR | CHECKPOINT               │
│ pageID     : which page was modified         │
│ beforeImage: value before the change         │
│ afterImage : value after the change          │
└──────────────────────────────────────────────┘
```

### LSN — Log Sequence Number

Every log record gets a monotonically increasing LSN. Each page also stores an LSN in its header (`pageLSN`) — the LSN of the most recent change applied to that page. This is the key comparison point during REDO: if `pageLSN >= recordLSN`, the change is already on disk; if `pageLSN < recordLSN`, it must be reapplied.

### ARIES — Algorithm for Recovery and Isolation Exploiting Semantics

ARIES decomposes recovery into three phases that run in order:

```
┌─────────────────────────────────────────────────┐
│                  CRASH                           │
│                                                   │
│  1. ANALYSIS (forward scan)                       │
│     Build Transaction Table + Dirty Page Table    │
│         │                                         │
│         ▼                                         │
│  2. REDO (forward scan from min DPT LSN)          │
│     Reapply all changes, committed or not         │
│         │                                         │
│         ▼                                         │
│  3. UNDO (backward scan from end)                 │
│     Roll back uncommitted transactions            │
│     Write CLRs for each undo action               │
│         │                                         │
│         ▼                                         │
│             CONSISTENT STATE                      │
└─────────────────────────────────────────────────┘
```

#### Analysis Phase

Scan the log forward from the last checkpoint (or from LSN 0). Build two structures:

**Transaction Table**: maps each transaction ID to its current status (`InProgress`, `Committed`, `Aborted`) and the LSN of its most recent log record (`lastLSN`).

| transID | status | lastLSN |
|---------|--------|---------|
| T1 | Committed | 14 |
| T2 | InProgress | 22 |
| T3 | Aborted | 18 |

**Dirty Page Table (DPT)**: maps each dirty page ID to the LSN of the **first** record that dirtied it (`recLSN`). This is the starting point for REDO.

| pageID | recLSN |
|--------|--------|
| 7 | 12 |
| 12 | 15 |
| 19 | 22 |

The DPT only records the *first* dirty LSN per page. If multiple transactions dirty the same page, the earliest LSN wins. This is safe because REDO must replay every change starting from the earliest possible missing change.

#### REDO Phase

Scan forward from `min(recLSN in DPT)` to the end of the log. For every UPDATE and CLR record:

1. Read the page from disk (or fetch it into the buffer pool).
2. If `pageLSN < recordLSN`, the change is missing — reapply `afterImage` and set `pageLSN = recordLSN`.
3. If `pageLSN >= recordLSN`, the change is already present — skip.

REDO is **idempotent**: reapplying the same change twice produces the same result. This matters because (a) the system might crash during REDO and restart REDO, and (b) some pages may have been flushed by the STEAL policy while others weren't.

#### UNDO Phase

Collect all transactions that are still `InProgress` after Analysis. Walk their log chains **backward** (using `prevLSN` links). For each UPDATE record encountered:

1. Write a **Compensation Log Record (CLR)** to the log recording the undo action. The CLR has `undoNextLSN` set to the `prevLSN` of the record being undone.
2. Apply the inverse change: restore `beforeImage` to the page.

When a CLR is encountered during the backward walk (from a previous, interrupted UNDO), skip to `undoNextLSN` — the chain was already handled.

UNDO also produces an overall ABORT record for each loser transaction.

### CLR — Exactly-Once Undo

The CLR is what makes ARIES resilient to crashes *during recovery itself*. Without CLRs, a crash during UNDO would cause the next recovery attempt to re-undo already-undone changes, producing incorrect state. With CLRs:

1. First recovery: UNDO processes records, writes CLRs, then crashes.
2. Next recovery: REDO replays the CLRs (reapplying the undo actions), so the undone state persists. UNDO sees the CLRs, follows their `undoNextLSN` to skip the already-processed chain, and continues from where it left off.

This guarantees **exactly-once undo** regardless of how many times the system crashes during recovery.

### STEAL / NO-FORCE

Buffer management policies describe when dirty pages move between the buffer pool and disk:

| Policy | Meaning | Recovery implication |
|--------|---------|---------------------|
| STEAL | Dirty pages from uncommitted transactions may be flushed to disk before commit | UNDO needs before-images to roll back (stored in WAL) |
| NO-FORCE | Dirty pages from committed transactions may remain in the buffer pool after commit | REDO needs after-images to reapply (stored in WAL) |
| NO-STEAL | Dirty pages from uncommitted transactions must stay in the buffer pool | UNDO is unnecessary (nothing to undo on disk), but buffer pool must be huge |
| FORCE | Dirty pages from committed transactions must be flushed to disk at commit | REDO is unnecessary (everything is on disk), but commit latency is high |

ARIES assumes **STEAL + NO-FORCE**, which is the most flexible and performant combination. It trades recovery complexity for runtime efficiency — and the WAL makes recovery tractable.

### Fuzzy Checkpoint

A checkpoint periodically saves the DPT and transaction table to the log so that Analysis doesn't have to scan from LSN 0 every time. It's "fuzzy" because the database continues processing transactions while the checkpoint is being written — the snapshot might be slightly stale, but Analysis will catch up by scanning the log after the checkpoint.

## Build It

We'll build an ARIES recovery simulator in Rust. The full code is in `code/main.rs`. The simulator models a log manager, an in-memory "disk", and the three recovery phases.

### Step 1: Core Data Structures

The simulator defines type aliases and the `LogRecord` struct:

```rust
type LSN = u64;
type PageID = u64;
type TransID = u64;

enum RecordType { Begin, Update, Commit, Abort, CLR, Checkpoint }

struct LogRecord {
    lsn: LSN,
    prev_lsn: LSN,
    trans_id: TransID,
    rtype: RecordType,
    page_id: PageID,
    before_image: Vec<u8>,
    after_image: Vec<u8>,
    undo_next_lsn: LSN,  // used only by CLR records
}
```

Each record carries a before-image and after-image so that REDO can reapply the change and UNDO can reverse it. `prev_lsn` chains records of the same transaction, enabling the backward walk during UNDO.

### Step 2: Log Manager

The `LogManager` assigns monotonically increasing LSNs as records are appended:

```rust
struct LogManager {
    records: Vec<LogRecord>,
    next_lsn: LSN,
}

impl LogManager {
    fn append(&mut self, mut r: LogRecord) -> LSN {
        r.lsn = self.next_lsn;
        self.next_lsn += 1;
        self.records.push(r);
        self.next_lsn - 1
    }
}
```

In a real database, `append` would write to an OS file buffer, and `flush` would call `fsync`. In the simulator, the log is kept in memory (which survives the simulated "crash" because we explicitly control what state is discarded).

### Step 3: Page and Disk

A `Page` stores data and the `page_lsn` — the LSN of the last change applied to it. The disk is a `HashMap<PageID, Page>` that represents the state surviving a crash:

```rust
struct Page {
    page_id: PageID,
    data: Vec<u8>,
    page_lsn: LSN,
}
```

Before recovery, the disk contains whatever pages were flushed before the crash. Some pages may have uncommitted writes (STEAL), others may be missing committed writes (NO-FORCE).

### Step 4: The Recovery Engine

The `Recovery` struct owns the log and implements the three ARIES phases:

```rust
struct Recovery {
    log: Vec<LogRecord>,
}

impl Recovery {
    fn recover(&self, disk: &mut HashMap<PageID, Page>) {
        let (trans_table, dpt) = self.analysis();
        self.redo(&dpt, disk);
        self.undo(&trans_table, disk);
    }
}
```

#### Analysis

Scans every log record forward. For BEGIN records, creates an `InProgress` entry in the transaction table. For UPDATE/CLR records, updates the transaction's `lastLSN` and records the page in the DPT (if not already present). For COMMIT/ABORT, updates the transaction's status:

```rust
fn analysis(&self) -> (HashMap<TransID, TransState>, HashMap<PageID, LSN>) {
    let mut tt: HashMap<TransID, TransState> = HashMap::new();
    let mut dpt: HashMap<PageID, LSN> = HashMap::new();

    for r in &self.log {
        match r.rtype {
            RecordType::Begin => {
                tt.insert(r.trans_id, TransState::in_progress(r.lsn));
            }
            RecordType::Update | RecordType::CLR => {
                if let Some(t) = tt.get_mut(&r.trans_id) {
                    t.last_lsn = r.lsn;
                }
                dpt.entry(r.page_id).or_insert(r.lsn);
            }
            RecordType::Commit => { /* mark committed */ }
            RecordType::Abort => { /* mark aborted */ }
            RecordType::Checkpoint => { /* restore from snapshot */ }
        }
    }
    (tt, dpt)
}
```

#### REDO

Finds `min(recLSN in DPT)` and scans forward from there. For each UPDATE or CLR, checks `page_lsn < record_lsn` — if true, reapplies `after_image`:

```rust
fn redo(&self, dpt: &HashMap<PageID, LSN>, disk: &mut HashMap<PageID, Page>) {
    let min_lsn = dpt.values().min().copied().unwrap_or(0);
    if min_lsn == 0 { return; }

    for r in &self.log {
        if r.lsn < min_lsn { continue; }
        if r.rtype != Update && r.rtype != CLR { continue; }

        let page = disk.entry(r.page_id).or_insert_with(|| Page::new(r.page_id, ""));
        if page.page_lsn < r.lsn {
            page.data = r.after_image.clone();
            page.page_lsn = r.lsn;
        }
    }
}
```

#### UNDO

Finds "loser" transactions (still `InProgress`). Walks each loser's log chain backward via `prev_lsn`. Collects all UPDATE records, sorts them by LSN descending (LIFO order), and applies each one's `before_image` to reverse the change:

```rust
fn undo(&self, trans_table: &HashMap<TransID, TransState>, disk: &mut HashMap<PageID, Page>) {
    let losers: Vec<TransID> = /* filter InProgress */;

    let mut to_undo: Vec<(LSN, &LogRecord)> = Vec::new();
    for &tid in &losers {
        let mut lsn = trans_table[&tid].last_lsn;
        while lsn > 0 {
            let r = &self.log[(lsn - 1) as usize];
            match r.rtype {
                Update => { to_undo.push((lsn, r)); lsn = r.prev_lsn; }
                CLR    => { lsn = r.undo_next_lsn; }
                _      => break,
            }
        }
    }

    to_undo.sort_by(|a, b| b.0.cmp(&a.0)); // highest LSN first

    for (_, r) in &to_undo {
        let page = disk.entry(r.page_id).or_insert_with(|| Page::new(r.page_id, ""));
        page.data = r.before_image.clone();
    }
}
```

A production implementation would also write CLRs to the log during undo. The simulator skips this for brevity, but the `undo_next_lsn` field is present in the data structures and the CLR type is handled during the backward walk.

### Step 5: Simulation

The simulator runs a concrete scenario with two transactions:

```
T1: BEGIN -> UPDATE(page1, "111") -> UPDATE(page2, "111") -> COMMIT
T2: BEGIN -> UPDATE(page1, "222") -> UPDATE(page3, "222")
CRASH (T2 never committed)
```

Before the crash:
- **STEAL**: page1 was flushed to disk with T2's uncommitted "222"
- **NO-FORCE**: page2 was NOT flushed — T1's committed "111" is in the buffer pool only

The disk at crash time has:
- page1 = "222" (page_lsn = LSN of T2's update)
- page2 = "" (page_lsn = 0, never flushed)
- page3 = "" (page_lsn = 0, never flushed)

After ARIES recovery:
- page1 = "111" (T2's "222" undone, T1's committed "111" restored via undo of T2's before-image)
- page2 = "111" (T1's change redone from log)
- page3 = "" (T2's change undone)

Run the simulator:

```
cd code && cargo run
```

You'll see the WAL contents, disk state before recovery, and the corrected state after each phase. Five test cases in the `#[cfg(test)]` module cover different crash scenarios; run them with:

```
cd code && cargo test
```

## Use It

### PostgreSQL WAL (`src/backend/access/transam/xlog.c`)

PostgreSQL's WAL implementation is ARIES-based. Key details:
- LSNs are `XLogRecPtr` values (64-bit offset into the WAL file)
- `pageLSN` is stored in `PageHeaderData.pd_lsn`
- Full-page images are written for the first page modification after a checkpoint (to handle torn pages)
- Checkpoints write a `CHECKPOINT_SHUTDOWN` or `CHECKPOINT_ONLINE` record containing the DPT and transaction table
- WAL segments are 16 MB by default, recycled in a circular fashion
- `wal_level = replica` or `logical` enables the WAL for replication

### ARIES in SQL Server and DB2

ARIES was developed at IBM by C. Mohan and colleagues and first implemented in DB2. SQL Server's recovery is also directly based on ARIES, using the same LSN, DPT, CLR, and three-phase structure. The "A" in ARIES stands for "Algorithm" — it was the first algorithm to correctly handle STEAL + NO-FORCE + fine-grained locking + partial rollbacks + fuzzy checkpoints all at once.

## Read the Source

- **PostgreSQL xlog.c**: `src/backend/access/transam/xlog.c` in the PostgreSQL source — 20,000+ lines implementing WAL write, flush, checkpoint, and recovery. Start with `XLogInsertRecord` (write path) and `REDO` / `undo` in the recovery loop.
- **PostgreSQL xlogdefs.h**: `src/include/access/xlogdefs.h` — defines `XLogRecPtr` and related types.
- **PostgreSQL bufpage.h**: `src/include/storage/bufpage.h` — defines `pd_lsn` in `PageHeaderData`.
- **Original ARIES paper**: "ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollbacks Using Write-Ahead Logging" by Mohan, Haderle, Lindsay, Pirahesh, Schwarz (1992).

## Ship It

The reusable artifact is the ARIES recovery simulator in `code/main.rs`. It demonstrates all three recovery phases and includes test cases for common crash scenarios. Reuse it in:
- **Phase Capstone (MVCC KV Store)**: the storage engine needs a real WAL and recovery implementation
- **Phase 17 (Formal Methods)**: model-check the ARIES algorithm with TLA+ to verify safety properties

## Exercises

1. **Easy** — Extend the simulator to write CLRs during UNDO. Verify that running UNDO twice produces the same result as running it once (idempotency).
2. **Medium** — Add a `Checkpoint` record type that snapshots the DPT and transaction table. Modify Analysis to start from the last checkpoint instead of LSN 0. Verify it produces the same result.
3. **Hard** — Simulate a crash during UNDO. Insert a deliberate panic in the middle of undo processing, then run recovery again. Verify that CLRs (if you implemented them) ensure the second recovery produces the correct state. Without CLRs, verify that the second recovery corrupts the database.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| WAL | "Write-Ahead Log" | An append-only sequence of log records on stable storage. Every database modification is recorded here before the corresponding page is written to disk. |
| LSN | "Log Sequence Number" | A monotonically increasing integer assigned to each log record. Also stored in page headers (pageLSN) to track which changes have been applied to each page. |
| ARIES | "A recovery algorithm" | The industry-standard crash recovery protocol with three phases (Analysis, REDO, UNDO) that supports STEAL + NO-FORCE, fine-grained locking, and partial rollbacks. |
| DPT | "Dirty Page Table" | A table mapping each dirty page ID to the LSN of the first log record that dirtied it. Used by REDO to determine the starting point for reapplication. |
| CLR | "Compensation Log Record" | A log record that records an undo action. Contains undoNextLSN to skip already-undone chains during restarted recovery. Guarantees exactly-once undo. |
| STEAL | "Page written before commit" | The buffer manager may flush dirty pages from uncommitted transactions to disk. Requires UNDO to roll back these changes if the transaction aborts. |
| NO-FORCE | "Page not written at commit" | The buffer manager may keep dirty pages from committed transactions in memory after commit. Requires REDO to reapply these changes after a crash. |
| Fuzzy checkpoint | "Checkpoint without quiesce" | A checkpoint taken while transactions are still running. Writes the DPT and transaction table to the log without stopping normal processing. |

## Further Reading

- **ARIES: A Transaction Recovery Method Supporting Fine-Granularity Locking and Partial Rollbacks Using Write-Ahead Logging** (Mohan et al., 1992) — The canonical paper; still the definitive reference after 30+ years.
- **Database Systems: The Complete Book** (Garcia-Molina, Ullman, Widom), Chapter 17 — Crash recovery with ARIES.
- **Architecture of a Database System** (Hellerstein, Stonebraker, Hamilton), Section 5 — Recovery and logging in modern database systems.
- **PostgreSQL WAL internals**: `src/backend/access/transam/xlog.c` in the PostgreSQL source tree — the implementation closest to the ARIES paper in active use.
- **Readings in Database Systems (the "Red Book")**, Chapter on Recovery — curated by Peter Bailis, includes ARIES and alternative recovery protocols.
