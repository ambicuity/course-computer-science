# Query Optimization — Plans, Cost, Cardinality

> Given a SQL query, the optimizer must find the cheapest execution plan among millions of equivalent alternatives — without executing any of them.

**Type:** Build
**Languages:** Python, SQL
**Prerequisites:** Phase 10 Lessons 01–11 (relational algebra, SQL execution, B-trees, storage)
**Time:** ~90 minutes

## Learning Objectives

- Explain why query optimization is necessary and why exhaustive enumeration is infeasible
- Apply relational algebra equivalences (selection pushdown, projection pushdown, join reordering) to rewrite query plans
- Implement a cost model that accounts for I/O, CPU, and memory
- Build a cardinality estimator using histograms, NDV, and MCV lists
- Implement System R dynamic programming for join ordering
- Diagnose how bad cardinality estimates lead to catastrophically suboptimal plans

## The Problem

A SQL query describes *what* data to retrieve, not *how* to retrieve it. Consider:

```sql
SELECT c_name, o_totalprice
FROM customer
JOIN orders ON c_custkey = o_custkey
JOIN lineitem ON o_orderkey = l_orderkey
WHERE o_totalprice > 100000 AND l_shipdate < '1995-01-01';
```

The relational algebra expression tree for this query can be rewritten in hundreds of algebraically equivalent ways. Should you filter `lineitem` before joining? Should you join `customer` with `orders` first, or `orders` with `lineitem`? Should you use a hash join or a nested-loop join? The difference between a good plan and a bad plan is often **1000x or more** in execution time.

Without query optimization, the database would evaluate the query as written — typically left-deep, in the order tables appear in the FROM clause — which can be disastrous. The optimizer's job is to search the space of equivalent plans efficiently and pick the one with the lowest estimated cost.

## The Concept

### Relational Algebra Equivalences

The optimizer rewrites the query tree using proven equivalences:

| Equivalence | Rule | Example |
|---|---|---|
| **Selection pushdown** | σₚ(R ⨝ S) → σₚ(R) ⨝ S (if p references only R) | Filter before join reduces join input size |
| **Projection pushdown** | πₐ(R ⨝ S) → πₐ(πₐ'(R) ⨝ πₐ''(S)) | Remove unneeded columns early |
| **Join reordering** | R ⨝ S → S ⨝ R | Commutativity; pick smaller build side |
| **Join associativity** | (R ⨝ S) ⨝ T → R ⨝ (S ⨝ T) | Change join tree shape |
| **Predicate migration** | σₚ∧q(R) → σₚ(σ_q(R)) | Split predicates; push parts closer to base tables |

These rewrites preserve the query result but dramatically change execution cost.

### Cost-Based Optimization

Instead of applying heuristic rules blindly, a cost-based optimizer:

1. **Enumerates** a subset of the plan space (pruned via dynamic programming)
2. **Estimates** the cost of each candidate plan using a cost model
3. **Picks** the plan with the lowest estimated cost

### Cost Model

```
Total Cost = I/O Cost + CPU Cost + Memory Cost

I/O Cost  = seq_page_cost × seq_page_reads + random_page_cost × random_page_reads
CPU Cost  = cpu_tuple_cost × tuple_count + cpu_operator_cost × op_count
```

Typical PostgreSQL cost constants: `seq_page_cost = 1.0`, `random_page_cost = 4.0`,
`cpu_tuple_cost = 0.01`, `cpu_operator_cost = 0.0025`.

For a sequential scan of a 1,500-page table: `Cost = 1.0 × 1500 + 0.01 × 150000 = 3000`.

### Cardinality Estimation — The Hard Part

The cost model needs row counts at every operator output. These come from **cardinality estimation**:

```
output_rows = input_rows × selectivity(predicate)
```

The challenge: selectivity depends on data distribution, which the optimizer doesn't know exactly. It only knows **statistics** collected from the table.

**Table-level statistics:**
- Row count (`reltuples`)
- Page count (`relpages`)

**Column-level statistics:**
- Number of distinct values (NDV)
- Minimum and maximum values
- Null fraction
- Most Common Values (MCV) — values + frequencies
- Histogram buckets — equi-depth boundaries representing the distribution

### Histograms

An **equi-depth histogram** divides the data into N buckets, each containing approximately the same number of rows. The bucket boundaries are stored, not the counts:

```
Bucket:  [0─10000)  [10000─50000)  [50000─100000)  [100000─150000)
Rows:       ~37,500      ~37,500        ~37,500          ~37,500
```

To estimate `col > 30000`:
1. Find the bucket containing 30000: bucket 1 (`10000─50000`)
2. `fraction_within_bucket = (50000 - 30000) / (50000 - 10000) = 0.5`
3. `selectivity = (1 bucket before × 0.25) + (0.5 × 0.25) = 0.375`

An **equi-width histogram** divides the value range into equal-width buckets (bad for skewed data). An **equi-depth histogram** divides so each bucket has equal rows (much better for skewed data).

### Join Cardinality Estimation

For a join `R ⨝_{R.x = S.y} S`:

```
estimated_cardinality = |R| × |S| × selectivity
selectivity ≈ 1 / max(NDV(R.x), NDV(S.y))
```

This assumes **uniform distribution** and **independence** — both frequently violated in real data.

### The Independence Assumption

Most optimizers assume that predicates on different columns are independent:

```
selectivity(col_a = 1 AND col_b = 2) = selectivity(col_a = 1) × selectivity(col_b = 2)
```

This fails when columns are correlated. Example: `make = 'Toyota' AND model = 'Camry'` — the true selectivity might be 0.01, but assuming independence might give 0.0001 (if make=0.01 and model=0.01). This is known as the **correlated column problem** and is an active research area (multi-column statistics, samplers, etc.).

### System R Dynamic Programming

The seminal System R optimizer (1979) introduced DP for join ordering:

1. Compute the optimal access path for each single table (base scans)
2. For each pair of tables, compute the optimal join
3. For each triple, build on the optimal pair results
4. Continue until all tables are joined

For N tables, full enumeration is O(N! ) but DP reduces it to O(2^N). For N ≤ 10 (typical query size), this is tractable. System R restricted the search to **left-deep trees** (the right child is always a base table), reducing the space further to O(N × 2^N).

A **bushy tree** allows both sides to be joins, which can be better for parallelism but grows the search space to O(3^N).

## Build It

We'll build a complete query optimizer in Python with statistics, selectivity estimation, a cost model, and System R DP.

### Step 1: Column Statistics and Histograms

```python
@dataclass
class ColumnStats:
    ndv: int
    min_val: float
    max_val: float
    null_frac: float
    most_common_vals: list[tuple]
    histogram_bounds: list[float]

@dataclass
class TableStats:
    row_count: int
    page_count: int
    columns: dict[str, ColumnStats]
```

The `histogram_bounds` list stores equi-depth bucket boundaries. For NDV=150000 with 15 buckets:
`[0, 10000, 20000, ..., 150000]`.

### Step 2: Selectivity Estimation

For an equality predicate `col = value`:

```python
def estimate_selectivity_eq(col, value):
    for v, freq in col.most_common_vals:
        if v == value:
            return freq                       # exact match from MCV
    remaining_frac = 1.0 - sum(f for _, f in col.most_common_vals)
    remaining_ndv = col.ndv - len(col.most_common_vals)
    return remaining_frac / remaining_ndv      # uniform fallback
```

For a range predicate `col > value`, walk the histogram buckets to find what fraction of values exceed the threshold.

### Step 3: Cost Model

```python
def scan_cost(self, page_count):
    return page_count * seq_page_cost + page_count * 100 * cpu_tuple_cost

def join_cost(self, outer_card, inner_card, outer_pages, inner_pages, join_type):
    if join_type == "hash":
        return (outer_pages + inner_pages) * seq_page_cost + ...
    elif join_type == "nested_loop":
        return outer_card * inner_pages * random_page_cost + ...
```

### Step 4: System R Join Enumerator

```python
class JoinEnumerator:
    def __init__(self, tables, join_preds, estimator, cost_model, base_cards, base_pages):
        self.best = {}               # DP table: frozenset -> JoinTree

    def enumerate(self):
        # Phase 1: seed with single-table accesses
        for t in self.tables:
            self.best[frozenset([t])] = self._build_scan(t)

        # Phase 2: DP over increasing subset sizes
        for size in range(2, len(self.tables) + 1):
            for subset in itertools.combinations(self.tables, size):
                self.best[subset] = self._find_best_join(subset)

        return self.best[frozenset(self.tables)]
```

For each subset, consider all ways to split it into a left part and a single-table right part (left-deep trees). Evaluate each as: `cost(left) + cost(right) + join_cost(left, right)` and keep the minimum.

## Use It

Run the optimizer on a TPC-H style 5-table query:

```
$ python3 code/main.py
```

The output shows:
- Table statistics (row counts, page counts)
- Selectivity estimates for each query predicate
- The optimal left-deep plan and its cost
- The optimal bushy plan (can be cheaper by allowing parallel execution)
- How different join orderings compare in cost
- What happens when cardinality estimates are wrong (30,000x error!)

Compare this with PostgreSQL's `EXPLAIN` output:

```sql
EXPLAIN (ANALYZE, COSTS, BUFFERS)
SELECT c_name, o_totalprice
FROM customer
JOIN orders ON c_custkey = o_custkey
WHERE c_nationkey = 0 AND o_totalprice > 100000;
```

PostgreSQL's optimizer uses the same techniques but with dramatically more sophistication: multi-column statistics, extended statistics for correlated columns, parameterized paths, parallel query, JIT compilation, and more. The core algorithm remains System R DP.

## Read the Source

- [PostgreSQL `src/backend/optimizer/path/costsize.c`](https://github.com/postgres/postgres/blob/master/src/backend/optimizer/path/costsize.c) — the cost model. 2000+ lines of carefully calibrated constants.
- [PostgreSQL `src/backend/utils/adt/selfuncs.c`](https://github.com/postgres/postgres/blob/master/src/backend/utils/adt/selfuncs.c) — selectivity estimation functions for every operator type.
- [PostgreSQL `src/backend/optimizer/path/allpaths.c`](https://github.com/postgres/postgres/blob/master/src/backend/optimizer/path/allpaths.c) — query path generation entry point.
- [System R paper](http://mitz.ca/mirror/ibmrd journal/systemR.pdf) — Selinger et al., "Access Path Selection in a Relational Database Management System" (1979). The original DP join ordering algorithm.
- [SQLite's query planner](https://www.sqlite.org/optoverview.html) — a simpler, heuristic-heavy optimizer for comparison.

## Ship It

The reusable artifact is `code/main.py` — a self-contained query optimizer with cost model, selectivity estimator, and System R DP join enumerator. Drop it into any project that needs plan costing logic, or use it as a reference for understanding PostgreSQL's `EXPLAIN` output.

## Exercises

1. **Easy** — Add a new join type (merge join) to the `CostModel` and use it when both inputs are already sorted. Run the demo to see if the optimizer ever picks a merge join over hash join.

2. **Medium** — Extend the `JoinEnumerator` to handle pushed-down predicates. Currently, the demo estimates filter selectivity but doesn't push filters through joins. Implement selection pushdown so that filters on `nation.n_name` are applied before the join with `customer`.

3. **Hard** — Implement a simple query parser that takes SQL text and produces the predicate list. Connect it to the optimizer so the demo reads: `python main.py "SELECT ..."` instead of hardcoded predicates.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Cardinality | "How many rows come out of this operator" | The *estimated* output row count — off by 1000x is common with bad statistics |
| Selectivity | "What fraction of rows pass the filter" | A number between 0 and 1; 0.01 means 1% of rows survive |
| NDV | "Number of distinct values" | Used in join cardinality: 1/max(NDV) is the join selectivity under uniformity |
| Equi-depth histogram | "Histogram with equal row counts per bucket" | Bucket boundaries are chosen so each bucket has ~N/|buckets| rows; handles skew well |
| System R | "The original optimizer" | IBM's 1979 paper that defined DP join ordering; still the foundation of every major optimizer |
| Independence assumption | "Columns are independent" | The optimizer assumes p(a) × p(b) = p(a AND b); false for correlated columns, but nobody has a universally better alternative |

## Further Reading

- [Selinger et al. (1979) — Access Path Selection in a Relational Database Management System](http://mitz.ca/mirror/ibmrd_journal/systemR.pdf) — The paper that invented cost-based optimization. Still worth reading 45 years later.
- [Graefe (1993) — Query Evaluation Techniques for Large Databases](https://dl.acm.org/doi/10.1145/152610.152611) — The survey that defined modern query execution.
- [Leis et al. (2015) — How Good Are Query Optimizers, Really?](http://www.vldb.org/pvldb/vol9/p204-leis.pdf) — A modern evaluation showing how far optimizers still have to go.
- [PostgreSQL Documentation: Chapter 14 — Performance Tips](https://www.postgresql.org/docs/current/performance-tips.html) — Practical guidance for reading `EXPLAIN` output.
- [SQL Performance Explained](https://use-the-index-luke.com/) — Markus Winand's practical guide to understanding execution plans.
