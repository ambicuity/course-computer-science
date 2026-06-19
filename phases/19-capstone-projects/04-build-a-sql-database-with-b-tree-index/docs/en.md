# Build a SQL Database with B+-Tree Index

> Storage engines work when logical semantics and page-level mechanics stay aligned.

**Type:** Build
**Languages:** Rust, SQL
**Prerequisites:** Phase 19 lessons 01-03
**Time:** ~840 minutes

## Learning Objectives

- Design table/index storage boundaries for a toy SQL engine.
- Implement minimal B+-tree insert/search semantics.
- Validate SQL-facing operations against storage invariants.
- Plan milestone-driven database capstone execution.

## The Problem

Database capstones fail when parser, planner, and storage are all developed at once. Someone starts with a SQL parser, gets the grammar working, realizes they need a storage engine, designs a page format, discovers the parser's assumptions about row layout don't match the storage format, and starts over.

The fix: a small vertical slice (schema + insert/select + indexed lookup) provides faster feedback and clearer debugging. The first milestone is: define a table schema, serialize rows into pages, and insert/retrieve them by primary key. The second milestone: add a B+-tree index for fast key lookups. The third: add a SQL parser that translates `SELECT * FROM t WHERE id = 5` into an index lookup.

Each milestone is independently testable. You can verify that B+-tree insert maintains balance without caring about SQL parsing. You can verify that row serialization is correct without caring about indexes.

## The Concept

A minimal SQL database has three layers:

```
SQL statement: SELECT * FROM users WHERE id = 42
        │
        ▼
┌───────────────────┐
│  SQL Parser        │  Parse into AST, extract table/columns/predicate
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Query Executor    │  Choose scan vs index lookup
│  (planner)         │  Produce iterator over matching rows
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Storage Engine    │  Pages + B+-tree index
│  (B+-tree)         │  Row serialization/deserialization
└───────────────────┘
```

**B+-tree invariants**: all data lives in leaf nodes. Internal nodes contain separator keys and child pointers. Leaves are linked for efficient range scans. The tree is balanced: all leaves are at the same depth. When a node overflows (exceeds its capacity), it splits into two nodes and propagates the median key upward.

```
B+-tree structure (order 3):

         [30 | 60]                    ← Internal node
        /    |    \
   [10|20] [30|40|50] [60|70|80]     ← Leaf nodes (data lives here)
    ←→      ←→         ←→            ← Linked list for range scans
```

The leaf-level linked list is what makes B+-trees superior to B-trees for databases: range queries (`WHERE age BETWEEN 20 AND 30`) just walk the linked list instead of traversing the tree for each key.

## Build It

We build an in-memory B+-tree with insert, point lookup, and range scan. Then we add row serialization and a simple query executor.

### Step 1: B+-Tree Node Structure (Rust)

```rust
const ORDER: usize = 4; // Max keys per node
const MAX_KEYS: usize = ORDER;
const MAX_CHILDREN: usize = ORDER + 1;

#[derive(Debug, Clone)]
enum Node {
    Internal {
        keys: Vec<i64>,
        children: Vec<Box<Node>>,
    },
    Leaf {
        keys: Vec<i64>,
        values: Vec<String>, // Row data stored as serialized strings
        next: Option<usize>, // Leaf linked list (index into a node store)
    },
}

struct BPlusTree {
    root: Box<Node>,
    leaf_count: usize,
}

impl BPlusTree {
    fn new() -> Self {
        BPlusTree {
            root: Box::new(Node::Leaf {
                keys: Vec::new(),
                values: Vec::new(),
                next: None,
            }),
            leaf_count: 1,
        }
    }

    // Search for a key, return the value if found
    fn get(&self, key: i64) -> Option<String> {
        self.search_node(&self.root, key)
    }

    fn search_node(&self, node: &Node, key: i64) -> Option<String> {
        match node {
            Node::Leaf { keys, values, .. } => {
                match keys.binary_search(&key) {
                    Ok(idx) => Some(values[idx].clone()),
                    Err(_) => None,
                }
            }
            Node::Internal { keys, children } => {
                // Find the child to descend into
                let idx = keys.partition_point(|&k| k <= key);
                self.search_node(&children[idx], key)
            }
        }
    }

    // Insert a key-value pair
    fn insert(&mut self, key: i64, value: String) {
        let split = self.insert_node(&mut self.root, key, value);
        if let Some((sep_key, new_node)) = split {
            // Root split: create new root
            let old_root = std::mem::replace(
                &mut self.root,
                Box::new(Node::Internal {
                    keys: vec![sep_key],
                    children: vec![],
                }),
            );
            if let Node::Internal { keys: _, ref mut children } = *self.root {
                children.push(old_root);
                children.push(new_node);
            }
        }
    }

    // Returns Some((separator_key, new_right_node)) if split occurred
    fn insert_node(&mut self, node: &mut Box<Node>, key: i64, value: String)
        -> Option<(i64, Box<Node>)>
    {
        match node.as_mut() {
            Node::Leaf { keys, values, .. } => {
                // Insert in sorted order
                match keys.binary_search(&key) {
                    Ok(idx) => {
                        // Key exists, update value
                        values[idx] = value;
                        None
                    }
                    Err(idx) => {
                        keys.insert(idx, key);
                        values.insert(idx, value);
                        // Check if we need to split
                        if keys.len() > MAX_KEYS {
                            let split_idx = keys.len() / 2;
                            let right_keys = keys.split_off(split_idx);
                            let right_values = values.split_off(split_idx);
                            let sep = right_keys[0];
                            let new_leaf = Box::new(Node::Leaf {
                                keys: right_keys,
                                values: right_values,
                                next: None,
                            });
                            Some((sep, new_leaf))
                        } else {
                            None
                        }
                    }
                }
            }
            Node::Internal { keys, children } => {
                // Find the child to descend into
                let idx = keys.partition_point(|&k| k <= key);
                if let Some((sep, new_child)) = self.insert_node(&mut children[idx], key, value) {
                    // Insert separator key and new child
                    keys.insert(idx, sep);
                    children.insert(idx + 1, new_child);
                    // Check if internal node overflows
                    if keys.len() > MAX_KEYS {
                        let split_idx = keys.len() / 2;
                        let right_keys = keys.split_off(split_idx + 1);
                        let up_key = keys.pop().unwrap();
                        let right_children = children.split_off(split_idx + 1);
                        let new_internal = Box::new(Node::Internal {
                            keys: right_keys,
                            children: right_children,
                        });
                        Some((up_key, new_internal))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    // Range scan: all keys in [lo, hi]
    fn range_scan(&self, lo: i64, hi: i64) -> Vec<(i64, String)> {
        let mut results = Vec::new();
        self.collect_range(&self.root, lo, hi, &mut results);
        results
    }

    fn collect_range(&self, node: &Node, lo: i64, hi: i64, results: &mut Vec<(i64, String)>) {
        match node {
            Node::Leaf { keys, values, .. } => {
                for (k, v) in keys.iter().zip(values.iter()) {
                    if *k >= lo && *k <= hi {
                        results.push((*k, v.clone()));
                    }
                }
            }
            Node::Internal { keys, children } => {
                for (i, child) in children.iter().enumerate() {
                    // Prune: skip children whose key range doesn't overlap [lo, hi]
                    if i < keys.len() && keys[i] < lo { continue; }
                    if i > 0 && keys[i-1] > hi { break; }
                    self.collect_range(child, lo, hi, results);
                }
            }
        }
    }
}
```

### Step 2: Row Serialization and Table Layer

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct Column {
    name: String,
    col_type: String, // "INTEGER", "TEXT"
}

#[derive(Debug, Clone)]
struct TableSchema {
    name: String,
    columns: Vec<Column>,
    primary_key: String,
}

// Serialize a row (HashMap<String, String>) to a string
fn serialize_row(row: &HashMap<String, String>) -> String {
    let mut parts: Vec<String> = row.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();
    parts.sort();
    parts.join("|")
}

// Deserialize a string back to a row
fn deserialize_row(data: &str) -> HashMap<String, String> {
    data.split('|')
        .filter_map(|part| {
            let mut split = part.splitn(2, '=');
            Some((split.next()?.to_string(), split.next()?.to_string()))
        })
        .collect()
}

struct Table {
    schema: TableSchema,
    index: BPlusTree, // Primary key index
}

impl Table {
    fn new(schema: TableSchema) -> Self {
        Table {
            schema,
            index: BPlusTree::new(),
        }
    }

    fn insert_row(&mut self, row: HashMap<String, String>) -> Result<(), String> {
        let pk = row.get(&self.schema.primary_key)
            .ok_or("Missing primary key")?
            .parse::<i64>()
            .map_err(|_| "Primary key must be integer")?;
        let serialized = serialize_row(&row);
        self.index.insert(pk, serialized);
        Ok(())
    }

    fn get_by_key(&self, key: i64) -> Option<HashMap<String, String>> {
        self.index.get(key).map(|data| deserialize_row(&data))
    }

    fn scan_range(&self, lo: i64, hi: i64) -> Vec<HashMap<String, String>> {
        self.index.range_scan(lo, hi)
            .into_iter()
            .map(|(_, data)| deserialize_row(&data))
            .collect()
    }
}
```

### Step 3: Demo

```rust
fn main() {
    let schema = TableSchema {
        name: "users".to_string(),
        columns: vec![
            Column { name: "id".to_string(), col_type: "INTEGER".to_string() },
            Column { name: "name".to_string(), col_type: "TEXT".to_string() },
            Column { name: "age".to_string(), col_type: "INTEGER".to_string() },
        ],
        primary_key: "id".to_string(),
    };

    let mut table = Table::new(schema);

    // Insert rows
    for (id, name, age) in &[
        (10, "Alice", 30),
        (20, "Bob", 25),
        (30, "Charlie", 35),
        (40, "Diana", 28),
        (50, "Eve", 32),
    ] {
        let mut row = HashMap::new();
        row.insert("id".to_string(), id.to_string());
        row.insert("name".to_string(), name.to_string());
        row.insert("age".to_string(), age.to_string());
        table.insert_row(row).unwrap();
    }

    // Point lookup
    println!("=== Point lookup: id=30 ===");
    if let Some(row) = table.get_by_key(30) {
        println!("  {:?}", row);
    }

    // Range scan
    println!("\n=== Range scan: id in [20, 40] ===");
    for row in table.scan_range(20, 40) {
        println!("  {:?}", row);
    }
}
```

## Use It

Patterns apply to embedded stores and production databases:

- **SQLite**: uses a B+-tree for every table and index. The page cache sits between the B+-tree and the disk. SQLite's B+-tree supports both integer keys (rowid tables) and text keys (WITHOUT ROWID tables). The pager handles page splits and free-page management.
- **PostgreSQL**: uses B-tree indexes (technically B+-trees) as its default index type. The `nbtree` access method implements insertion, deletion, and vacuum (garbage collection of dead tuples). PostgreSQL's buffer manager handles page I/O.
- **LMDB**: Lightning Memory-Mapped Database uses a copy-on-write B+-tree over a memory-mapped file. Every write creates new pages; old pages are reclaimed when no transaction references them. This gives MVCC for free.

The key production lesson: **page split strategy determines write performance**. A naive split divides a full page 50/50. Production systems use more sophisticated strategies: B-link trees (PostgreSQL) add a high-key and right-link to each node for concurrent access. Lazy splitting (Bw-tree) batches splits to reduce write amplification.

## Read the Source

- [SQLite architecture](https://sqlite.org/arch.html) — How SQLite's B-tree, pager, and VDBE (virtual database engine) fit together. The B-tree module documentation explains the balancing algorithm.
- [Database Internals](https://www.databass.dev/) — Petrov. Chapter 3 (B-Tree variants) covers B+-trees, B-link trees, and copy-on-write trees. Chapter 7 (Buffer Management) covers the page cache layer.
- [CMU 15-445 B+ Tree lectures](https://15445.courses.cs.cmu.edu/) — Andy Pavlo's database course covers B+-tree implementation in detail, including concurrent access and buffer pool integration.

## Ship It

- `code/main.rs`: in-memory B+-tree with insert, point lookup, range scan, plus a table layer with row serialization.
- `code/main.sql`: schema and query samples showing the SQL interface we're targeting.
- `outputs/README.md`: DB capstone checklist covering storage engine, index, parser, and executor milestones.

## Exercises

1. **Easy** — Add delete with node merge. When a leaf drops below half capacity, merge it with its sibling. Propagate the merge upward if the parent also drops below capacity. Test with a sequence of inserts followed by deletes that trigger multiple merges.
2. **Medium** — Add a WAL (write-ahead log) sketch. Before modifying any page, write the intended change to an append-only log. On startup, replay the log to recover uncommitted changes. This is the foundation of crash recovery in all production databases.
3. **Hard** — Add range scan benchmarks. Insert 100,000 rows with random keys, then benchmark point lookups vs range scans of various sizes. Measure how performance scales with tree depth and compare against a HashMap baseline. Report the crossover point where the B+-tree's ordered access becomes faster.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| B+-tree | "index structure" | A balanced search tree where all data lives in leaf nodes. Internal nodes contain separator keys and child pointers. Leaves are linked for range scans. The standard index structure in relational databases. |
| Leaf split | "node overflow handling" | When a leaf exceeds its capacity, it divides into two nodes and propagates the median key to the parent. If the parent overflows, it splits too, potentially increasing the tree height. |
| Predicate pushdown | "early filtering" | Applying filter conditions at the storage layer rather than the executor layer. Instead of scanning all rows and filtering in the executor, push the predicate into the index lookup to scan fewer pages. |
| WAL | "durability log" | Write-Ahead Log: an append-only log of intended changes written before the changes are applied to the actual data pages. Enables crash recovery: replay the log to restore the last consistent state. |
| Page split | "node division" | The physical operation of dividing a full B+-tree page into two pages. The split strategy (50/50, prefix, suffix) affects write amplification and space utilization. |

## Further Reading

- [SQLite architecture](https://sqlite.org/arch.html) — The most widely deployed SQL database. Its architecture is compact enough to understand end-to-end.
- [Database Internals](https://www.databass.dev/) — Petrov. Deep dive into storage engines, B-tree variants, and distributed database internals.
- [CMU 15-445](https://15445.courses.cs.cmu.edu/) — Database systems course with projects building a buffer pool manager and B+-tree index.
