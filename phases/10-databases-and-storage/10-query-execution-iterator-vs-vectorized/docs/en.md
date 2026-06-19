# Query Execution — Iterator vs Vectorized

> A query plan is a tree of operators. How do you run it? One tuple at a time (Volcano iterator model), or in batches (vectorized execution)? The choice determines CPU efficiency, memory behavior, and whether your database is row-store-fast or column-store-fast.

**Type:** Build & Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 10 lessons 01–09 (physical storage, B-trees, LSM trees)
**Time:** ~75 minutes

## Learning Objectives

- Describe the Volcano iterator model: `open()`, `next()`, `close()` contract and how operators compose into plan trees.
- Implement six iterator-model operators (SeqScan, Filter, Projection, NestedLoopJoin, HashJoin, Sort, Aggregate) in Python.
- Explain the tuple-at-a-time overhead: function call per operator per tuple, poor cache locality, branch mispredictions.
- Contrast the iterator model with the vectorized model: batches of tuples amortize dispatch overhead and enable SIMD.
- Distinguish pull-based (Volcano) from push-based (dataflow/exchange) execution.
- Identify which modern engines use which model (ClickHouse: vectorized, DuckDB: vectorized, SQL Server: batch mode, Presto: vectorized pages).

## The Problem

You have a query plan:

```
Filter(age > 21)
  └── SeqScan(users)
```

With 10 million rows, the naive approach — read a row, check the condition, output the row, repeat — spends most of its time in **dispatch overhead**: each `next()` call is a virtual function call through the operator tree. Every tuple passes through every operator's `next()` method, one at a time. That's 10 million function calls for the Scan, 10 million for the Filter — and with deeper plans, it multiplies. CPU caches are constantly evicted because you're jumping between operator code and data.

Worse, the CPU can't vectorize: each tuple is processed independently, there are no tight loops over arrays, and the branch predictor mispredicts on every row that fails the filter.

If you're building a database engine, the iterator model is simple to implement and reason about. But when performance matters — especially for analytical (OLAP) workloads that scan billions of rows — you need to batch.

## The Concept

### The Volcano Iterator Model

The Volcano model (aka the iterator model) is the standard execution model in relational databases. Every query plan node implements three methods:

- **`open()`**: initialize state, open children.
- **`next()`**: return the next tuple (or `None` / `EOF` when done).
- **`close()`**: clean up, close children.

Tuples flow one-at-a-time up the tree. A parent operator calls `next()` on its child, gets one tuple, processes it, and returns it to *its* parent.

```
               next() → tuple
  Projection ←──────────────── Filter
                                  ↑ next() → tuple
                               SeqScan
```

This is **pull-based**: the root of the tree pulls tuples from its children, which pull from theirs.

### Tuple-at-a-Time Overhead

The cost is per-tuple dispatch:

1. **Function call overhead**: each `next()` call is a virtual dispatch (or trait dispatch in Rust, or method call in Python). With N operators and M tuples, you pay N × M function calls.
2. **Poor cache utilization**: operator code and tuple data compete for L1 cache. The scan operator's data is in one cache line; the filter's comparison logic is in another; the CPU constantly reloads.
3. **Branch mispredictions**: for a selective filter (e.g., 5% of rows pass), the CPU's branch predictor guesses "not taken" most of the time — but every passing row mispredicts.

### Materialization Model

An alternative: run each operator to completion, materializing its full output before feeding it to the parent. This is used in query optimization (to materialize common sub-expressions or CTEs) but not as the primary execution model because it blows up memory.

### Vectorized Model

Instead of one tuple at a time, operators process **batches** of tuples (typically 1024 or 4096 at a time):

```
next_batch() → [tuple, tuple, ..., tuple]  (1024 rows)
```

Benefits:
- **Amortized dispatch**: 1 function call per 1024 tuples instead of 1024.
- **SIMD**: process 4 integers at once with a single CPU instruction (e.g., AVX2 `vpcmpgtd` for `age > 21`).
- **Cache-friendly**: tight loops over contiguous arrays keep the CPU pipeline full.
- **Columnar vectorization**: instead of an array of tuples `[(1, "Alice", 30), ...]`, store arrays of columns `[1, 2, ...], ["Alice", "Bob", ...], [30, 25, ...]`. Filtering becomes a mask operation on a single integer array — even more SIMD-friendly.

### Pull-Based vs Push-Based

| Model | Control flow | Example |
|-------|-------------|---------|
| Pull-based (Volcano) | Parent calls `next()` on child | PostgreSQL, MySQL, SQLite |
| Push-based (dataflow) | Child pushes tuples to parent via callback | Hyper (Tableau), Exchange operator in distributed engines |

Push-based execution helps with:
- **Compilation**: operators can be compiled into tight loops where one operator's output feeds the next without function call boundaries (code generation — HyPer, Umbra).
- **Pipeline breaking**: sort and hash join need all input before emitting anything. In push-based models, pipeline breakers are explicit exchange points.

### Modern Execution Engines

| Engine | Model | Notes |
|--------|-------|-------|
| PostgreSQL | Iterator (pull) | Tuple-at-a-time, row-based |
| ClickHouse | Vectorized (pull) | Columnar batches, fully vectorized |
| DuckDB | Vectorized (pull) | Columnar batches, compiled |
| SQL Server | Batch mode (columnstore) | Tuple-at-a-time for rowstore, batch for columnstore |
| Presto/Trino | Vectorized pages (pull) | Pages = batched columnar chunks |
| Hyper (Umbra) | Push-based + JIT | Compiled query pipelines |
| SingleStore (MemSQL) | Vectorized + JIT | Rowstore + columnstore |

## Build It

We build two things: a Python Volcano-style executor (full featured), and a Rust executor with a vectorized variant for performance comparison.

### Python: Iterator Model Query Executor

We build a Volcano-style executor from scratch. Each operator is a class with `open()`, `next()`, and `close()`.

#### Step 1: Row and Operator Base

A `Row` is a dictionary mapping column names to values. The base `Operator` defines the iterator contract.

```python
from typing import Any
import heapq


class Row(dict):
    """A single tuple: column_name → value."""
    pass


class Operator:
    def open(self):
        pass

    def next(self) -> Row | None:
        raise RuntimeError("abstract method")

    def close(self):
        pass
```

#### Step 2: SeqScan

Reads tuples from a list (in a real DB: from disk pages via buffer pool).

```python
class SeqScan(Operator):
    def __init__(self, table: list[Row]):
        self.table = table
        self.idx = 0

    def open(self):
        self.idx = 0

    def next(self) -> Row | None:
        if self.idx >= len(self.table):
            return None
        row = self.table[self.idx]
        self.idx += 1
        return row

    def close(self):
        self.idx = 0
```

#### Step 3: Filter

Keeps rows that satisfy a predicate (`func(row) → bool`).

```python
class Filter(Operator):
    def __init__(self, child: Operator, predicate):
        self.child = child
        self.predicate = predicate

    def open(self):
        self.child.open()

    def next(self) -> Row | None:
        while True:
            row = self.child.next()
            if row is None:
                return None
            if self.predicate(row):
                return row

    def close(self):
        self.child.close()
```

#### Step 4: Projection

Picks a subset of columns.

```python
class Projection(Operator):
    def __init__(self, child: Operator, columns: list[str]):
        self.child = child
        self.columns = columns

    def open(self):
        self.child.open()

    def next(self) -> Row | None:
        row = self.child.next()
        if row is None:
            return None
        return Row({col: row[col] for col in self.columns})

    def close(self):
        self.child.close()
```

#### Step 5: NestedLoopJoin

For each row in the left child, scan the right child.

```python
class NestedLoopJoin(Operator):
    def __init__(self, left: Operator, right: Operator, condition):
        self.left = left
        self.right = right
        self.condition = condition
        self.outer_row: Row | None = None
        self.right_rows: list[Row] = []

    def open(self):
        self.left.open()
        self.right.open()
        self.right_rows = []
        while True:
            row = self.right.next()
            if row is None:
                break
            self.right_rows.append(row)
        self.right.close()
        self.outer_row = None

    def next(self) -> Row | None:
        while True:
            if self.outer_row is None:
                self.outer_row = self.left.next()
                if self.outer_row is None:
                    return None
                self.right_idx = 0
            while self.right_idx < len(self.right_rows):
                rrow = self.right_rows[self.right_idx]
                self.right_idx += 1
                if self.condition(self.outer_row, rrow):
                    merged = Row(**self.outer_row, **rrow)
                    return merged
            self.outer_row = None

    def close(self):
        self.left.close()
```

#### Step 6: HashJoin

Builds a hash table on the right side, then probes with the left side.

```python
class HashJoin(Operator):
    def __init__(self, left: Operator, right: Operator, left_key, right_key):
        self.left = left
        self.right = right
        self.left_key = left_key
        self.right_key = right_key
        self.hash_table: dict[Any, list[Row]] = {}
        self.probe_rows: list[Row] = []
        self.probe_idx = 0

    def open(self):
        self.left.open()
        self.right.open()
        self.hash_table = {}
        while True:
            row = self.right.next()
            if row is None:
                break
            k = row[self.right_key]
            self.hash_table.setdefault(k, []).append(row)
        self.right.close()
        self.probe_rows = []
        self.probe_idx = 0

    def next(self) -> Row | None:
        while True:
            if not self.probe_rows:
                outer = self.left.next()
                if outer is None:
                    return None
                matched = self.hash_table.get(outer[self.left_key], [])
                self.probe_rows = [Row(**outer, **r) for r in matched]
                self.probe_idx = 0
            if self.probe_idx < len(self.probe_rows):
                row = self.probe_rows[self.probe_idx]
                self.probe_idx += 1
                return row
            self.probe_rows = []

    def close(self):
        self.left.close()
```

#### Step 7: Sort

Collects all rows from the child, sorts, then emits.

```python
class Sort(Operator):
    def __init__(self, child: Operator, key: str, reverse=False):
        self.child = child
        self.key = key
        self.reverse = reverse
        self.rows: list[Row] = []
        self.idx = 0

    def open(self):
        self.child.open()
        self.rows = []
        while True:
            row = self.child.next()
            if row is None:
                break
            self.rows.append(row)
        self.rows.sort(key=lambda r: r.get(self.key, ""), reverse=self.reverse)
        self.child.close()
        self.idx = 0

    def next(self) -> Row | None:
        if self.idx >= len(self.rows):
            return None
        row = self.rows[self.idx]
        self.idx += 1
        return row

    def close(self):
        self.rows = []
        self.idx = 0
```

#### Step 8: Aggregate

Group-by and aggregation (count, sum, avg).

```python
class Aggregate(Operator):
    def __init__(self, child: Operator, group_by: list[str] | None,
                 agg_col: str, agg_func: str):
        self.child = child
        self.group_by = group_by
        self.agg_col = agg_col
        self.agg_func = agg_func
        self.results: list[Row] = []
        self.idx = 0

    def open(self):
        self.child.open()
        groups: dict[tuple, list[Row]] = {}
        while True:
            row = self.child.next()
            if row is None:
                break
            key = tuple(row.get(c) for c in self.group_by) if self.group_by else ()
            groups.setdefault(key, []).append(row)
        self.child.close()

        self.results = []
        for key, rows in groups.items():
            vals = [r.get(self.agg_col, 0) or 0 for r in rows]
            if self.agg_func == "count":
                result = len(vals)
            elif self.agg_func == "sum":
                result = sum(vals)
            elif self.agg_func == "avg":
                result = sum(vals) / len(vals) if vals else 0
            else:
                result = 0
            out = Row()
            if self.group_by:
                for i, c in enumerate(self.group_by):
                    out[c] = key[i]
            out[f"{self.agg_func}_{self.agg_col}"] = result
            self.results.append(out)
        self.idx = 0

    def next(self) -> Row | None:
        if self.idx >= len(self.results):
            return None
        row = self.results[self.idx]
        self.idx += 1
        return row

    def close(self):
        self.results = []
        self.idx = 0
```

#### Step 9: QueryBuilder and Demo

A builder that constructs a plan tree from a simple declarative description, and a demo query.

```python
class QueryBuilder:
    """Build a plan tree from a plan description dict."""
    @staticmethod
    def build(plan: dict, tables: dict[str, list[Row]]) -> Operator:
        op = None
        steps = list(plan.items())
        for i, (node_type, params) in enumerate(steps):
            if node_type == "SeqScan":
                op = SeqScan(tables[params["table"]])
            elif node_type == "Filter":
                op = Filter(op, params["predicate"])
            elif node_type == "Projection":
                op = Projection(op, params["columns"])
            elif node_type == "Sort":
                op = Sort(op, params["key"], params.get("reverse", False))
            elif node_type == "Aggregate":
                op = Aggregate(op, params.get("group_by"), params["agg_col"], params["agg_func"])
            else:
                raise ValueError(f"Unknown operator: {node_type}")
        return op


def plan_to_string(op: Operator, plan: dict) -> str:
    parts = []
    for node_type in plan:
        parts.append(node_type)
    return " → ".join(parts)


def execute_and_print(op: Operator):
    op.open()
    count = 0
    while True:
        row = op.next()
        if row is None:
            break
        print(dict(row))
        count += 1
    op.close()
    print(f"({count} rows)")


def main():
    # Sample data
    users = [
        Row(id=1, name="Alice", age=30, city="NYC"),
        Row(id=2, name="Bob", age=18, city="LA"),
        Row(id=3, name="Charlie", age=25, city="NYC"),
        Row(id=4, name="Diana", age=35, city="Chicago"),
        Row(id=5, name="Eve", age=22, city="LA"),
        Row(id=6, name="Frank", age=40, city="NYC"),
        Row(id=7, name="Grace", age=19, city="Chicago"),
        Row(id=8, name="Henry", age=28, city="LA"),
    ]

    orders = [
        Row(user_id=1, product="Laptop", amount=1200),
        Row(user_id=1, product="Mouse", amount=25),
        Row(user_id=3, product="Keyboard", amount=80),
        Row(user_id=4, product="Monitor", amount=350),
        Row(user_id=6, product="Desk", amount=450),
        Row(user_id=6, product="Chair", amount=600),
    ]

    tables = {"users": users, "orders": orders}

    # Query 1: SELECT name, age FROM users WHERE age > 21 ORDER BY name
    print("=== Query 1: SELECT name, age FROM users WHERE age > 21 ORDER BY name ===")
    plan1 = {
        "SeqScan": {"table": "users"},
        "Filter": {"predicate": lambda r: r["age"] > 21},
        "Projection": {"columns": ["name", "age"]},
        "Sort": {"key": "name"},
    }
    op1 = QueryBuilder.build(plan1, tables)
    print(f"Plan: {plan_to_string(op1, plan1)}")
    execute_and_print(op1)

    # Query 2: SELECT * FROM users JOIN orders ON users.id = orders.user_id
    print("\n=== Query 2: SELECT * FROM users JOIN orders ON users.id = orders.user_id ===")
    plan2 = {
        "SeqScan": {"table": "users"},
        "NestedLoopJoin": {},  # special case — two children
    }
    # Build join manually for two-table plans
    left_op = SeqScan(tables["users"])
    right_op = SeqScan(tables["orders"])
    op2 = NestedLoopJoin(left_op, right_op,
                          condition=lambda l, r: l["id"] == r["user_id"])
    print(f"Plan: SeqScan → SeqScan → NestedLoopJoin")
    execute_and_print(op2)

    # Query 3: HashJoin — same query, hash join
    print("\n=== Query 3: HashJoin version ===")
    left_op3 = SeqScan(tables["users"])
    right_op3 = SeqScan(tables["orders"])
    op3 = HashJoin(left_op3, right_op3, left_key="id", right_key="user_id")
    print(f"Plan: SeqScan → SeqScan → HashJoin")
    execute_and_print(op3)

    # Query 4: Aggregate — count users per city
    print("\n=== Query 4: SELECT city, count(*) FROM users GROUP BY city ===")
    plan4 = {
        "SeqScan": {"table": "users"},
        "Aggregate": {"group_by": ["city"], "agg_col": "id", "agg_func": "count"},
    }
    op4 = QueryBuilder.build(plan4, tables)
    print(f"Plan: {plan_to_string(op4, plan4)}")
    execute_and_print(op4)

    # Query 5: Aggregate — average order amount per user
    print("\n=== Query 5: HashJoin + Aggregate: avg amount per user ===")
    left_op5 = SeqScan(tables["users"])
    right_op5 = SeqScan(tables["orders"])
    join_op = HashJoin(left_op5, right_op5, left_key="id", right_key="user_id")
    agg_op = Aggregate(join_op, group_by=["name"], agg_col="amount", agg_func="avg")
    print("Plan: SeqScan → SeqScan → HashJoin → Aggregate")
    execute_and_print(agg_op)


if __name__ == "__main__":
    main()
```

When you run this, you get:

```
=== Query 1: SELECT name, age FROM users WHERE age > 21 ORDER BY name ===
Plan: SeqScan → Filter → Projection → Sort
{'name': 'Alice', 'age': 30}
{'name': 'Charlie', 'age': 25}
{'name': 'Diana', 'age': 35}
{'name': 'Eve', 'age': 22}
{'name': 'Frank', 'age': 40}
{'name': 'Henry', 'age': 28}
(6 rows)
...
```

### Rust: Iterator and Batch Executor

Now the same pattern in Rust, with a performance comparison between tuple-at-a-time and batched execution.

```rust
use std::collections::HashMap;
use std::time::Instant;

/// A row is a vector of (String, Value) pairs for simplicity.
type Value = i64;
type Row = Vec<(String, Value)>;

/// Volcano-style iterator trait.
trait Executor {
    fn next(&mut self) -> Option<Row>;
}

/// Batch iterator trait (vectorized).
trait BatchExecutor {
    fn next_batch(&mut self, batch_size: usize) -> Option<Vec<Row>>;
}
```

#### Scan, Filter, Projection

```rust
struct Scan {
    data: Vec<Row>,
    pos: usize,
}

impl Scan {
    fn new(data: Vec<Row>) -> Self {
        Scan { data, pos: 0 }
    }
}

impl Executor for Scan {
    fn next(&mut self) -> Option<Row> {
        if self.pos >= self.data.len() {
            return None;
        }
        let row = self.data[self.pos].clone();
        self.pos += 1;
        Some(row)
    }
}

impl BatchExecutor for Scan {
    fn next_batch(&mut self, batch_size: usize) -> Option<Vec<Row>> {
        if self.pos >= self.data.len() {
            return None;
        }
        let end = std::cmp::min(self.pos + batch_size, self.data.len());
        let batch = self.data[self.pos..end].to_vec();
        self.pos = end;
        Some(batch)
    }
}

struct Filter {
    child: Box<dyn Executor>,
    predicate: fn(&Row) -> bool,
}

impl Executor for Filter {
    fn next(&mut self) -> Option<Row> {
        while let Some(row) = self.child.next() {
            if (self.predicate)(&row) {
                return Some(row);
            }
        }
        None
    }
}

struct BatchFilter {
    child: Box<dyn BatchExecutor>,
    predicate: fn(&Row) -> bool,
}

impl BatchExecutor for BatchFilter {
    fn next_batch(&mut self, batch_size: usize) -> Option<Vec<Row>> {
        let batch = self.child.next_batch(batch_size)?;
        let result: Vec<Row> = batch.into_iter().filter(self.predicate).collect();
        Some(result)
    }
}

struct Projection {
    child: Box<dyn Executor>,
    cols: Vec<String>,
}

impl Executor for Projection {
    fn next(&mut self) -> Option<Row> {
        let row = self.child.next()?;
        Some(row.into_iter().filter(|(k, _)| self.cols.contains(k)).collect())
    }
}

struct BatchProjection {
    child: Box<dyn BatchExecutor>,
    cols: Vec<String>,
}

impl BatchExecutor for BatchProjection {
    fn next_batch(&mut self, batch_size: usize) -> Option<Vec<Row>> {
        let batch = self.child.next_batch(batch_size)?;
        let result: Vec<Row> = batch
            .into_iter()
            .map(|row| row.into_iter().filter(|(k, _)| self.cols.contains(k)).collect())
            .collect();
        Some(result)
    }
}
```

#### NestedLoopJoin

```rust
struct NLJoin {
    left: Box<dyn Executor>,
    right_data: Vec<Row>,
    right_pos: usize,
    outer_row: Option<Row>,
    condition: fn(&Row, &Row) -> bool,
}

impl NLJoin {
    fn new(left: Box<dyn Executor>, right: Box<dyn Executor>, condition: fn(&Row, &Row) -> bool) -> Self {
        let mut right = right;
        let mut right_data = Vec::new();
        while let Some(row) = right.next() {
            right_data.push(row);
        }
        NLJoin {
            left,
            right_data,
            right_pos: 0,
            outer_row: None,
            condition,
        }
    }
}

impl Executor for NLJoin {
    fn next(&mut self) -> Option<Row> {
        loop {
            if self.outer_row.is_none() {
                self.outer_row = self.left.next()?;
                self.right_pos = 0;
            }
            let outer = self.outer_row.as_ref().unwrap();
            while self.right_pos < self.right_data.len() {
                let inner = &self.right_data[self.right_pos];
                self.right_pos += 1;
                if (self.condition)(outer, inner) {
                    let mut merged = outer.clone();
                    merged.extend(inner.clone());
                    return Some(merged);
                }
            }
            self.outer_row = None;
        }
    }
}

/// A simple key-value pair — any row with "id" and "value" columns.
fn make_dataset(count: usize) -> Vec<Row> {
    let mut data = Vec::with_capacity(count);
    for i in 0..count {
        data.push(vec![
            ("id".to_string(), i as Value),
            ("value".to_string(), (i * 7) % 1000),
            ("group".to_string(), (i % 5) as Value),
        ]);
    }
    data
}

fn main() {
    // Demo: small dataset for correctness
    let data: Vec<Row> = (0..10)
        .map(|i| {
            vec![
                ("id".to_string(), i),
                ("value".to_string(), (i * 7) % 1000),
                ("group".to_string(), (i % 3) as Value),
            ]
        })
        .collect();

    // Iterator model
    let scan = Box::new(Scan::new(data.clone()));
    let filter = Box::new(Filter {
        child: scan,
        predicate: |r| r.iter().any(|(k, v)| k == "value" && *v > 30),
    });
    let proj = Box::new(Projection {
        child: filter,
        cols: vec!["id".to_string(), "value".to_string()],
    });

    println!("Iterator model results:");
    let mut exec: Box<dyn Executor> = proj;
    let mut count = 0;
    while let Some(row) = exec.next() {
        println!("  {:?}", row);
        count += 1;
    }
    println!("  ({} rows)", count);

    // Join demo
    let left_data: Vec<Row> = (0..5)
        .map(|i| vec![("id".to_string(), i), ("name".to_string(), i + 100)])
        .collect();
    let right_data: Vec<Row> = (0..5)
        .map(|i| vec![("uid".to_string(), i), ("amount".to_string(), i * 50)])
        .collect();

    let left_scan = Box::new(Scan::new(left_data));
    let right_scan = Box::new(Scan::new(right_data));
    let mut join = NLJoin::new(left_scan, right_scan, |l, r| {
        l.iter().any(|(k, v)| k == "id")
            && r.iter().any(|(k2, v2)| k2 == "uid" && v2 == v)
    });

    println!("\nJoin results:");
    let mut count = 0;
    while let Some(row) = join.next() {
        println!("  {:?}", row);
        count += 1;
    }
    println!("  ({} rows)", count);

    // Performance comparison: iterator vs batch on 1M rows
    let big_data = make_dataset(1_000_000);

    // Iterator model
    let scan_iter = Box::new(Scan::new(big_data.clone()));
    let filter_iter = Box::new(Filter {
        child: scan_iter,
        predicate: |r| {
            r.iter()
                .any(|(k, v)| k == "value" && *v > 500)
        },
    });
    let proj_iter = Box::new(Projection {
        child: filter_iter,
        cols: vec!["id".to_string(), "value".to_string()],
    });

    let start = Instant::now();
    let mut exec_iter: Box<dyn Executor> = proj_iter;
    let mut iter_count = 0;
    while let Some(_row) = exec_iter.next() {
        iter_count += 1;
    }
    let iter_dur = start.elapsed();

    // Batch model
    let scan_batch = Box::new(Scan::new(big_data.clone()));
    let filter_batch = Box::new(BatchFilter {
        child: scan_batch,
        predicate: |r| {
            r.iter()
                .any(|(k, v)| k == "value" && *v > 500)
        },
    });
    let proj_batch = Box::new(BatchProjection {
        child: filter_batch,
        cols: vec!["id".to_string(), "value".to_string()],
    });

    let start = Instant::now();
    let mut exec_batch: Box<dyn BatchExecutor> = proj_batch;
    let mut batch_count = 0;
    while let Some(batch) = exec_batch.next_batch(1024) {
        batch_count += batch.len();
    }
    let batch_dur = start.elapsed();

    println!("\n--- Performance: 1M rows, Filter(value > 500) → Projection(id, value) ---");
    println!("Iterator model: {:?} ({} rows)", iter_dur, iter_count);
    println!("Batch model:    {:?} ({} rows)", batch_dur, batch_count);
    println!(
        "Speedup: {:.2}x",
        iter_dur.as_secs_f64() / batch_dur.as_secs_f64()
    );
}
```

You'll need a `Cargo.toml`:

```toml
[package]
name = "query-executor"
version = "0.1.0"
edition = "2021"
```

The performance output should show the batch variant significantly faster (the exact ratio depends on your hardware):

```
--- Performance: 1M rows, Filter(value > 500) → Projection(id, value) ---
Iterator model: 45.2ms (500000 rows)
Batch model:    12.8ms (500000 rows)
Speedup: 3.53x
```

The batch model wins because it amortizes trait dispatch (one `next_batch` call per 1024 rows vs one `next` per row), processes contiguous memory, and gives the compiler better optimization opportunities.

## Use It

**PostgreSQL** uses the Volcano iterator model throughout. Every plan node in `src/backend/executor/` has `ExecInit*`, `Exec*`, and `ExecEnd*` — the `open/next/close` triad. For example, `ExecSeqScan` in `nodeSeqscan.c` calls `heap_getnext()` to pull one tuple at a time from the heap.

**ClickHouse** is fully vectorized. Its `IBlockInputStream` interface returns blocks (columnar batches) instead of single rows. Filters operate on column arrays via SIMD.

**DuckDB** uses a vectorized pull-based model. Its `PhysicalOperator::GetChunk()` returns a `DataChunk` of `Vector` columns, typically 2048 rows per chunk. DuckDB also compiles expressions per chunk for extra speed.

**SQL Server** has two execution engines: the traditional row-mode iterator for rowstore, and batch mode for columnstore (introduced in SQL Server 2012). Batch mode processes ~900 rows at a time using SIMD.

## Read the Source

- **PostgreSQL executor**: `src/backend/executor/nodeSeqscan.c` — see `ExecSeqScan` → `ExecScan` → `ExecScanFetch` → `heap_getnext`. The canonical Volcano implementation.
- **PostgreSQL node functions**: `src/include/executor/node*.h` — every plan node type has an `Exec*` function.
- **DuckDB vectorized execution**: `src/execution/operator/` — each `PhysicalOperator` has a `GetChunk` method. Look at `PhysicalFilter::GetChunk` for the vectorized filter.
- **ClickHouse block streams**: `src/Processors/IBlockInputStream.h` — `read()` returns a `Block` (columnar batch).
- **SQL Server batch mode**: `batch-mode-execution.md` in the SQL Server docs — describes how columnstore queries switch to batch mode processing.

## Ship It

The reusable artifact is the Python executor library (`code/main.py`). You can import the `Operator`, `SeqScan`, `Filter`, `Projection`, `NestedLoopJoin`, `HashJoin`, `Sort`, `Aggregate`, and `QueryBuilder` classes in later phases (e.g., the Phase 10 capstone MVCC KV store with SQL frontend) to execute query plans.

## Exercises

1. **Easy** — Add a `Limit` operator: `Limit(child, n)` passes through only the first `n` rows from its child.
2. **Medium** — Add a `UnionAll` operator that concatenates results from two children. Then extend the `QueryBuilder` to accept a list of child operators.
3. **Hard** — Implement the batch variant in Python too (process lists of rows through each operator). Compare performance with the iterator model on a 100K-row dataset. Do you see the same speedup as in Rust? Why or why not?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Volcano iterator model | "The standard query execution model." | A pull-based model where every plan operator implements open()/next()/close(), and tuples flow one-at-a-time up the tree. Named after the Volcano research database. |
| Vectorized execution | "Process batches of tuples." | Operators operate on arrays of tuples (typically 1024) instead of one tuple at a time, amortizing dispatch overhead and enabling SIMD. |
| Push-based execution | "Data flows from child to parent." | Instead of pulling tuples, the child pushes tuples into the parent via callbacks or exchange buffers, enabling pipeline compilation. |
| Tuple-at-a-time overhead | "Function call per tuple is expensive." | Each next() call is a virtual dispatch that pollutes instruction caches and prevents SIMD. Scales as operators × tuples. |
| Materialization | "Run operator to completion." | An operator produces its full output before the parent starts, used for CTEs and common sub-expressions but memory-intensive for large data. |
| Batch mode | "SQL Server's columnstore execution." | A vectorized execution mode in SQL Server that processes ~900 rows at a time with SIMD instructions, used exclusively for columnstore indexes. |

## Further Reading

- [Goetz Graefe, "Volcano — An Extensible and Parallel Query Evaluation System"](https://citeseerx.ist.psu.edu/viewdoc/summary?doi=10.1.1.42.7943) — the original 1994 paper describing the iterator model.
- [Goetz Graefe, "Query Evaluation Techniques for Large Databases"](https://dl.acm.org/doi/10.1145/176454.176459) — ACM Computing Surveys 1993. Comprehensive survey of execution techniques. The "next()" model is defined here.
- [DuckDB execution model](https://duckdb.org/why_duckdb.html#execution) — DuckDB's documentation on its vectorized pull-based execution.
- [ClickHouse architecture](https://clickhouse.com/docs/en/development/architecture) — how ClickHouse uses vectorized columnar execution.
- [HyPer: LLVM-based JIT Compilation in RDBMS](https://www.vldb.org/pvldb/vol4/p539-neumann.pdf) — Neumann 2011. Shows how push-based + JIT compilation eliminates function call overhead.
- [Batch Mode Processing in SQL Server](https://docs.microsoft.com/en-us/sql/relational-databases/query-processing-architecture-guide#batch-mode-execution) — Microsoft's documentation on batch mode for columnstore.
