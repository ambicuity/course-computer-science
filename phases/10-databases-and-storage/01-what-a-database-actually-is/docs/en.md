# What a Database Actually Is

> A database is not a "big Excel file." It is a layered piece of systems software that guarantees your data survives crashes, stays consistent under concurrent access, and answers questions without scanning every row.

**Type:** Build & Learn
**Languages:** Python
**Prerequisites:** Phase 03 (data structures — hash tables, B-trees), Phase 09 (OS — files, buffers, crash safety)
**Time:** ~45 minutes

## Learning Objectives

- Distinguish a database from a flat file or CSV across five dimensions: query language, ACID, concurrency, crash recovery, and access methods.
- Describe the seven-layer architecture of a database engine (disk manager → buffer pool → access methods → executor → planner → optimizer → transaction manager).
- Explain the RUM conjecture and why no single storage engine can optimize all three of read, update, and memory.
- Build a persistent append-only key-value store (Bitcask model) in under 100 lines of Python.
- Write a minimal SQL query planner that parses SELECT/INSERT/CREATE TABLE and emits a plan tree.
- Identify the core PostgreSQL subsystems (processes, shared buffers, WAL, planner) and map them to the layered model.

## The Problem

You have a Python script that writes customer orders to a CSV file. It works fine — until two people order the same item at the same instant, and the second write corrupts the file. Then the server loses power and the last 200 orders vanish. Then your dataset hits 10 GB and every query requires scanning the entire file, taking 30 seconds.

Each of these is a hard systems problem: concurrency control, crash recovery, and fast access paths. A flat file solves none of them. A database engine solves all three — but the solutions are not free. Understanding the architecture behind them is what separates "I know SQL" from "I understand how this machine works."

The entire Phase 10 builds toward writing a full MVCC key-value store with a SQL frontend. This first lesson gives you the map of the territory.

## The Concept

### What Makes a DB Different from a File?

| Dimension | CSV file | Database |
|-----------|----------|----------|
| Query language | Scan + grep in shell | Declarative (SQL) with optimizer |
| Concurrency | Lock the whole file | MVCC, fine-grained locks |
| Crash recovery | fsync at app's whim | Write-ahead log (WAL) + ARIES |
| Access methods | Full scan only | Indexes (B-tree, hash, GiST) |
| ACID | None of the four | All four (when configured) |

**ACID** is the big one:
- **Atomicity**: a transaction either commits fully or aborts fully — no partial writes.
- **Consistency**: constraints and invariants hold before and after.
- **Isolation**: concurrent transactions appear to run one at a time.
- **Durability**: committed data survives power loss.

A CSV file gives you exactly zero of these. A database gives you all four (at a cost in throughput and complexity).

### The Seven-Layer Architecture

Every database engine — PostgreSQL, MySQL, SQLite, DuckDB — has the same logical layers:

```
                     ┌──────────────────────────────────┐
                     │    SQL client (psql, driver)      │
                     └────────────┬─────────────────────┘
                                  │ queries / results
                     ┌────────────▼─────────────────────┐
                     │         Parser / Planner           │
                     │  (SQL → parse tree → plan tree)   │
                     └────────────┬─────────────────────┘
                                  │ plan tree
                     ┌────────────▼─────────────────────┐
                     │          Optimizer                 │
                     │  (cost-based: pick join order,     │
                     │   index choice, pushdown)         │
                     └────────────┬─────────────────────┘
                                  │ optimized plan
                     ┌────────────▼─────────────────────┐
                     │          Executor                  │
                     │   (iterator model: next() → row)  │
                     └────────────┬─────────────────────┘
                                  │ get/put/scan
                     ┌────────────▼─────────────────────┐
                     │      Access Methods (Indexes)     │
                     │   (B-tree, Hash, GiST, BRIN)     │
                     └────────────┬─────────────────────┘
                                  │ page read/write
                     ┌────────────▼─────────────────────┐
                     │         Buffer Pool               │
                     │   (cache pages in memory,         │
                     │    eviction: LRU/clock)           │
                     └────────────┬─────────────────────┘
                                  │ block I/O
                     ┌────────────▼─────────────────────┐
                     │         Disk Manager               │
                     │   (read/write pages, files,        │
                     │    raw block device)              │
                     └──────────────────────────────────┘
```

On top of all of this sits the **Transaction Manager** which crosscuts everything — it coordinates WAL writes, lock acquisition, and MVCC visibility.

### Historical Evolution

```
1960s  ──  Hierarchical (IMS)         — tree of records, fast parent-child
1970s  ──  Network (CODASYL)          — graph of records with owner-member sets
1980s  ──  Relational (System R,      — tables, SQL, declarative queries
            Ingres, Oracle)
2000s  ──  NoSQL (Bigtable, Dynamo,   — schemaless, horizontal scale, BASE
            MongoDB, Redis)
2010s  ──  NewSQL (Spanner, Cockroach, — SQL + horizontal scale + strong
            TiDB)                        consistency
```

Each generation relaxed a constraint the previous one took for granted: IMS assumed hierarchical access patterns; the relational model threw that out and required a query optimizer; NoSQL threw out joins and strong consistency for scalability; NewSQL tried to get both back.

### The RUM Conjecture

You can optimize a storage engine for two of:
- **R**ead speed
- **U**pdate speed
- **M**emory overhead

Pick two. You cannot have all three.

| Engine | Read-optimized | Update-optimized | Memory-optimized |
|--------|:-:|:-:|:-:|
| B-tree (InnoDB) | ✓ | | | ✓ (compact pages) |
| LSM-tree (LevelDB) | | ✓ | ✓ (bloom filters) |
| Hash index (Bitcask) | ✓ | ✓ | |

The B-tree trades update cost (splits, writes) for fast reads and compact storage. The LSM-tree trades read cost (merge) for fast writes and moderate memory. A hash index trades range scans and memory for point-reads and fast writes. There is no free lunch.

## Build It

We build two pieces: a persistent key-value store using an append-only log (the Bitcask model from Riak), and a minimal SQL query planner.

### Part A: Append-Only Key-Value Store

The idea is dead simple: when you PUT a key, append it to a file. When you GET a key, look it up in an in-memory hash index that maps key → file offset. When the database starts, rebuild the index by replaying the log.

```
PUT x = 42    →   append "x:42\n" to db.log
                   index["x"] = <file position of this entry>

GET x         →   offset = index["x"]
                   seek to offset, read the value from db.log

Startup        →   scan db.log from beginning
                   rebuild index["x"] = latest offset for each key
```

This is the **Bitcask** storage model. It makes writes blindingly fast (sequential append), point reads fast (one hash lookup + one disk seek), but range scans slow (must scan the whole index).

```python
import os
import struct

class BitcaskKV:
    def __init__(self, path):
        self.path = path
        self.index = {}          # key → (file_offset, value_length)
        self.fd = open(path, "ab+")

    def _rebuild_index(self):
        self.index.clear()
        self.fd.seek(0)
        while True:
            offset = self.fd.tell()
            header = self.fd.read(9)
            if len(header) < 9:
                break
            klen, vlen = struct.unpack(">II", header[:8])
            tombstone = header[8]
            key = self.fd.read(klen)
            if len(key) < klen:
                break
            if tombstone:
                self.index.pop(key, None)
            else:
                val = self.fd.read(vlen)
                if len(val) < vlen:
                    break
                self.index[key] = (offset, vlen, val)
            if tombstone:
                self.fd.seek(vlen, os.SEEK_CUR)

    def put(self, key, value):
        if isinstance(key, str):
            key = key.encode()
        if isinstance(value, str):
            value = value.encode()
        self.fd.seek(0, os.SEEK_END)
        offset = self.fd.tell()
        klen = len(key)
        vlen = len(value)
        self.fd.write(struct.pack(">II", klen, vlen))
        self.fd.write(b"\x00")  # tombstone=0 means alive
        self.fd.write(key)
        self.fd.write(value)
        self.fd.flush()
        os.fsync(self.fd.fileno())
        self.index[key] = (offset, vlen, value)

    def get(self, key):
        if isinstance(key, str):
            key = key.encode()
        entry = self.index.get(key)
        if entry is None:
            return None
        offset, vlen, val = entry
        return val

    def delete(self, key):
        if isinstance(key, str):
            key = key.encode()
        self.fd.seek(0, os.SEEK_END)
        offset = self.fd.tell()
        klen = len(key)
        self.fd.write(struct.pack(">II", klen, 0))
        self.fd.write(b"\x01")  # tombstone
        self.fd.write(key)
        self.fd.flush()
        os.fsync(self.fd.fileno())
        self.index.pop(key, None)

    def close(self):
        self.fd.close()


if __name__ == "__main__":
    db = BitcaskKV("/tmp/test_kv.log")
    db.put("name", "Alice")
    db.put("age", "30")
    print(db.get("name"))   # b'Alice'
    print(db.get("age"))    # b'30'
    db.delete("age")
    print(db.get("age"))    # None
    db.close()

    # Reopen and rebuild index
    db2 = BitcaskKV("/tmp/test_kv.log")
    db2._rebuild_index()
    print(db2.get("name"))  # b'Alice'
    print(db2.get("age"))   # None
    db2.close()
```

### Part B: Minimal SQL Query Planner

A real database parser is thousands of lines of yacc/bison grammar. We build a toy that handles three statement types and emits a plan tree.

```python
import re
import shlex

class PlanNode:
    def __init__(self, op, children=None, **params):
        self.op = op
        self.children = children or []
        self.params = params

    def __repr__(self, indent=0):
        pad = "  " * indent
        s = f"{pad}{self.op}"
        for k, v in self.params.items():
            s += f" {k}={v}"
        s += "\n"
        for c in self.children:
            s += c.__repr__(indent + 1)
        return s


def parse_create(tokens):
    # CREATE TABLE name (col1 TYPE, col2 TYPE, ...)
    tokens.pop(0)  # TABLE
    name = tokens.pop(0)
    columns = []
    # everything in parens
    cols_str = " ".join(tokens)
    cols_str = cols_str.strip("()")
    for col in cols_str.split(","):
        col = col.strip()
        parts = col.split()
        if parts:
            columns.append((parts[0], parts[1] if len(parts) > 1 else "TEXT"))
    return PlanNode("CreateTable", table=name, columns=columns)


def parse_select(tokens):
    # SELECT col1, col2 FROM table WHERE cond
    table = None
    columns = []
    where_clause = None

    idx = 0
    if tokens[idx].upper() == "SELECT":
        idx += 1
    # read columns until FROM
    cols = []
    while idx < len(tokens) and tokens[idx].upper() != "FROM":
        cols.append(tokens[idx])
        idx += 1
    columns = [c.strip(",") for c in cols if c != ","]

    if idx < len(tokens) and tokens[idx].upper() == "FROM":
        idx += 1
        table = tokens[idx]
        idx += 1

    if idx < len(tokens) and tokens[idx].upper() == "WHERE":
        idx += 1
        where_clause = " ".join(tokens[idx:])

    node = PlanNode("SeqScan", table=table, columns=columns)
    if where_clause:
        node = PlanNode("Filter", children=[node], condition=where_clause)
    return node


def parse_insert(tokens):
    # INSERT INTO table (col1, col2) VALUES (v1, v2)
    tokens.pop(0)  # INTO
    table = tokens.pop(0)
    # skip parens for columns and values
    text = " ".join(tokens)

    m = re.search(r"\((.*?)\)\s*VALUES\s*\((.*?)\)", text, re.IGNORECASE)
    if not m:
        m = re.search(r"VALUES\s*\((.*?)\)", text, re.IGNORECASE)
        if m:
            return PlanNode("Insert", table=table, columns=[], values=m.group(1).split(","))
        return PlanNode("Insert", table=table)

    columns = [c.strip() for c in m.group(1).split(",")]
    values = [v.strip() for v in m.group(2).split(",")]
    return PlanNode("Insert", table=table, columns=columns, values=values)


def parse_sql(sql):
    sql = sql.strip().rstrip(";")
    tokens = shlex.split(sql)
    if not tokens:
        return None
    keyword = tokens[0].upper()
    if keyword == "CREATE":
        return parse_create(tokens[1:])
    elif keyword == "SELECT":
        return parse_select(tokens[1:])
    elif keyword == "INSERT":
        return parse_insert(tokens[1:])
    else:
        return PlanNode("Unknown", text=sql)


if __name__ == "__main__":
    queries = [
        "CREATE TABLE users (id INT, name TEXT, age INT);",
        "SELECT name, age FROM users WHERE age > 30;",
        "INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30);",
    ]
    for q in queries:
        print(f"SQL: {q}")
        plan = parse_sql(q)
        print(plan)
        print("---")
```

This produces:
```
SQL: CREATE TABLE users (id INT, name TEXT, age INT);
CreateTable table=users columns=[('id', 'INT'), ('name', 'TEXT'), ('age', 'INT')]

---
SQL: SELECT name, age FROM users WHERE age > 30;
Filter condition=age > 30
  SeqScan table=users columns=['name', 'age']

---
SQL: INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30);
Insert table=users columns=['id', 'name', 'age'] values=['1', "'Alice'", '30']
```

## Use It: PostgreSQL Architecture

PostgreSQL is the best production reference for everything in this lesson. Here is how its architecture maps to the seven-layer model:

```
┌───────────────────────┐
│   postmaster (daemon) │  ← listens on port, forks backends
├───────────────────────┤
│   backend process     │  ← one per client connection
│   ┌───────────────┐   │
│   │ Parser         │   │  ← raw SQL → parse tree (gram.y, gram.c)
│   │ Rewriter       │   │  ← rule rewriting
│   │ Planner        │   │  ← cost-based, join order, index selection
│   │ Optimizer      │   │  ← plan tree with estimated costs
│   │ Executor       │   │  ← iterator model (next() from each plan node)
│   └───────────────┘   │
├───────────────────────┤
│   Shared Buffers       │  ← buffer pool, 8 KB pages, clock-sweep eviction
│   WAL Buffers          │  ← write-ahead log, flushed on commit
│   SLRU Caches          │  ← clog, subtransaction, multixact
├───────────────────────┤
│   Storage              │
│   ├─ base/ (databases) │  ← each DB is a subdirectory
│   ├─ pg_wal/           │  ← WAL segments (16 MB each)
│   └─ pg_xact/          │  ← transaction commit status
└───────────────────────┘
```

Key Postgres subsystems:

- **Backend process**: each connection gets a forked process (not a thread). This is the executor context.
- **Shared buffers**: the buffer pool shared across all backends. Pages are 8 KB. Eviction uses an improved clock-sweep algorithm.
- **WAL (Write-Ahead Log)**: before any page modification hits the data file, a record is written to `pg_wal/`. On crash, the WAL is replayed. This is the **D** in ACID.
- **Planner/Optimizer**: `src/backend/optimizer/` — about 50,000 lines. It generates all possible join orders, estimates cardinality from table statistics, and picks the cheapest plan.
- **MVCC**: every row has `xmin` (creating transaction) and `xmax` (deleting/updating transaction). Readers see a snapshot of rows whose `xmin` committed before the snapshot started. This is the **I** in ACID.

## Read the Source

- **PostgreSQL parser**: `src/backend/parser/gram.y` — the yacc grammar that defines the SQL dialect. Look at how `SELECT` is reduced to a `SelectStmt` node.
- **PostgreSQL planner**: `src/backend/optimizer/plan/planner.c` — `subquery_planner()` is the entry point for turning a parsed query into a plan tree.
- **SQLite Bitcask-style storage**: `src/main.c` in SQLite — SQLite can use an append-only journal mode (PERSIST or TRUNCATE). Compare it to the Bitcask model.
- **Bitcask paper**: `https://riak.com/assets/bitcask-intro.pdf` — the original 2010 paper from Basho Technologies. ~10 pages, extremely readable.

## Exercises

1. **Easy** — Add a `scan()` method to `BitcaskKV` that iterates over all key-value pairs by scanning the log file. What is the asymptotic cost?

2. **Medium** — Modify the query planner to also handle `UPDATE table SET col = val WHERE condition` by emitting an `Update` plan node with a `Filter` child.

3. **Hard** — Add a simple cost-based optimizer: assign statistical costs (e.g., `SeqScan` cost = 100, `Filter` cost = 10 per condition, `IndexScan` cost = 5) and print the total estimated cost of each plan. Extend the parser to accept `EXPLAIN SELECT ...` and print the plan with costs.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| ACID | "My database is ACID compliant." | Atomicity, Consistency, Isolation, Durability — four properties that guarantee reliable transactions. No database enforces all four at the highest level by default. |
| RUM | "Pick two." | Read-Optimized, Update-Optimized, Memory-Optimized — you can optimize a storage engine for at most two of these three. |
| Buffer pool | "Cache for disk pages." | An in-memory region that caches database pages (typically 8 KB or 16 KB) to avoid disk reads. Managed by a page replacement policy (LRU, clock, ARC). |
| WAL | "Write-ahead log." | Before any data page is modified, a record of the change is written to a sequential log. On crash, replay the WAL to restore durability. |
| MVCC | "Multi-version concurrency control." | Each modification creates a new version of a row instead of overwriting it. Readers see a consistent snapshot of the database as of a point in time. |
| Query plan | "The database's execution recipe." | A tree of operators (SeqScan, IndexScan, NestedLoop, HashJoin, etc.) that the executor walks to produce query results. |
| Optimizer | "The part that makes queries fast." | A cost-based engine that enumerates alternative plan trees, estimates their cost from table statistics, and picks the cheapest. |
| Storage engine | "The part that actually stores data." | The component responsible for page layout, indexing, concurrency, and recovery. Examples: InnoDB (B-tree), RocksDB (LSM-tree), Bitcask (hash + append-only log). |

## Further Reading

- [Architecture of a Database System](http://db.cs.berkeley.edu/papers/fntdb07-architecture.pdf) — Hellerstein, Stonebraker, Hamilton. The canonical survey of database internals (~100 pages, covers all seven layers).
- [Readings in Database Systems (Red Book)](http://www.redbook.io/) — the curated collection of seminal database papers.
- [Bitcask: A Log-Structured Hash Table for Fast Key/Value Data](https://riak.com/assets/bitcask-intro.pdf) — Basho Technologies. The paper that inspired Part A of this lesson.
- [PostgreSQL Documentation: Chapter 18. Server Architecture](https://www.postgresql.org/docs/current/tutorial-arch.html) — the official architecture overview.
- [Designing Data-Intensive Applications](https://dataintensive.net/) — Martin Kleppmann. Chapters 2–5 cover storage engines, encoding, replication, and partitions with exceptional clarity.
- [The RUM Conjecture](http://daslab.seas.harvard.edu/rum-conjecture/) — Harvard DASLab. The original paper formalizing the read-update-memory tradeoff.
