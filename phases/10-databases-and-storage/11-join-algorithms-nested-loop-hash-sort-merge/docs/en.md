# Join Algorithms — Nested Loop, Hash, Sort-Merge

> Join a million rows with a million rows in under a second. The three algorithms that make relational joins fast — and when to pick each one.

**Type:** Build
**Languages:** Python, Rust
**Prerequisites:** Phase 10 lessons 01–10 (query execution, indexing, buffer pool)
**Time:** ~75 minutes

## Learning Objectives

- Implement Simple Nested Loop Join, Block Nested Loop Join, and Index Nested Loop Join
- Build Grace Hash Join with partitioning and Hybrid Hash Join
- Implement Sort-Merge Join with sorted inputs and merge phase
- Analyze I/O cost and memory requirements for each algorithm
- Determine which join algorithm is optimal for a given relation size, index, and memory budget
- Trace through PostgreSQL's join planner decision process

## The Problem

You're the query planner in a relational database. The user writes:

```sql
SELECT * FROM orders JOIN customers ON orders.cust_id = customers.id;
```

The `orders` table has 100,000 rows (1,000 pages). The `customers` table has 10,000 rows (200 pages). Your buffer pool has 256 pages of memory. How do you produce the joined result without spending an hour reading pages?

A naive approach — for each row in orders, scan the entire customers table — reads 100,000 × 200 = 20 million pages. At 10 μs per page I/O, that's 200 seconds. Unacceptable.

The answer: three fundamentally different join strategies, each with a sweet spot. Pick wrong and your query is 100× slower. Pick right and it finishes in milliseconds. Understanding join algorithms is what separates a toy database from a production query engine.

## The Concept

A join combines two relations R and S on a predicate, producing all matching pairs of tuples. The three canonical strategies differ in how they find matches:

### Nested Loop Join (NLJ)

The simplest idea: for each tuple in the outer relation, scan the inner relation looking for matches.

```
Simple NLJ:     for r in R: for s in S: if match(r,s): emit(r,s)
Block NLJ:      for each page of R: for each page of S: for r in page: for s in page: if match: emit
Index NLJ:      for r in R: use index on S to find matching s; emit(r,s)
```

- **Simple NLJ**: O(|R|·|S|) tuple comparisons. I/O: read every page of R once, every page of S once per tuple of R — catastrophic for large tables.
- **Block NLJ**: Outer loop iterates page-by-page instead of tuple-by-tuple. Inner relation scanned once per outer *page*. I/O: `P_R + P_R · P_S` page reads.
- **Index NLJ**: If S has an index on the join key, each outer tuple triggers an index lookup (O(log |S|) or O(1)). I/O: `P_R + |R| · depth(index)`.

**When NLJ wins**: Small outer relation (few tuples), inner relation has a cheap index, or the join predicate is non-equi (e.g., `R.a < S.b`) where hash join can't apply.

### Hash Join (HJ)

Hash join avoids scanning the inner relation multiple times by building a hash table on the smaller relation.

```
Build phase:  hash every tuple of R into buckets in memory
Probe phase:  for each tuple of S, hash it, look up bucket, check matches
```

**Grace Hash Join** handles the case where neither relation fits in memory:

1. **Partition phase**: hash both R and S into N partitions on disk using a hash function h1
2. **Build+Probe phase**: for each partition i, load Rᵢ into memory, build hash table, probe with Sᵢ

**Hybrid Hash Join** keeps the first partition in memory during the partitioning phase, writing only the remaining N-1 partitions to disk. This saves one pass of I/O for the first partition.

- I/O cost: ~3 × (|R| + |S|) for Grace Hash Join (partition both, then build+probe)
- Memory requirement: enough for the largest partition's build side
- Only works with equi-joins (= predicate)

### Sort-Merge Join (SMJ)

Sort both relations on the join key, then walk them in lockstep:

```
Sort R on key -> sorted_R
Sort S on key -> sorted_S
i = 0, j = 0
while i < len(R) and j < len(S):
    if sorted_R[i].key == sorted_S[j].key:
        // emit all matches in the matching group
        ...
    elif sorted_R[i].key < sorted_S[j].key: i++
    else: j++
```

- I/O cost: cost to sort both relations + one merge pass
- Sorting cost (external merge sort): `2 · P · (1 + ceil(log_M(P)))` page I/Os where M is memory in pages
- Merge phase: `P_R + P_S` page I/Os
- Works for equi-joins AND range joins (non-equi predicates)
- Ideal when one or both inputs are already sorted (e.g., from an index scan or `ORDER BY`)

### Cost Comparison

| Algorithm | I/O Cost | Memory | Predicate |
|-----------|----------|--------|-----------|
| Simple NLJ | P_R + |R|·P_S | ~2 pages | any |
| Block NLJ | P_R + P_R·P_S/B | B+2 pages | any |
| Index NLJ | P_R + |R|·depth(index) | ~2 pages | any |
| Hash Join (Grace) | 3·(P_R+P_S) | ~√(P_S)·fudge | equi only |
| Sort-Merge Join | sort(R)+sort(S)+P_R+P_S | B+2 pages | any |

Where P_R, P_S = pages in R, S; |R| = tuples in R; B = buffer pages available.

### PostgreSQL's Join Planner

PostgreSQL considers all three strategies for every join. Its decision process:

1. If an index exists on the inner relation's join key AND the outer is small → **Index NLJ** (called "Nested Loop" in EXPLAIN)
2. If work_mem is large enough to hold the smaller relation → **Hash Join** (in-memory hash)
3. If both relations are large and work_mem is limited → **Sort-Merge Join** or **Grace Hash Join** (external)
4. If the join predicate is non-equi (range, inequality) → **NLJ** or **SMJ** (hash join can't apply)

PostgreSQL's `EXPLAIN` shows the plan:

```sql
EXPLAIN SELECT * FROM orders JOIN customers ON orders.cust_id = customers.id;
--                                  QUERY PLAN
-- Hash Join  (cost=... rows=... width=...)
--   Hash Cond: (orders.cust_id = customers.id)
--   ->  Seq Scan on orders
--   ->  Hash
--         ->  Seq Scan on customers
```

## Build It

We'll implement all three join families in Python and Rust, then build a cost estimator that selects the optimal strategy.

### Step 1: Simple Nested Loop Join

The baseline: double loop, compare every pair.

```python
def simple_nested_loop_join(R, S, key):
    """O(|R| * |S|) — every tuple compared."""
    result = []
    for r in R:
        for s in S:
            if r[key] == s[key]:
                result.append({**r, **s})
    return result
```

### Step 2: Block Nested Loop Join

Group by pages. Each page of the outer is read once, then the inner is scanned page by page.

```python
def block_nested_loop_join(R_pages, S_pages, key):
    """P_R + P_R*P_S page reads."""
    result = []
    for r_page in R_pages:          # outer page loop
        for s_page in S_pages:      # inner page loop (full scan)
            for r in r_page:
                for s in s_page:
                    if r[key] == s[key]:
                        result.append({**r, **s})
    return result
```

### Step 3: Grace Hash Join with Partitioning

When neither relation fits in memory, partition both to disk, then build+probe per partition.

```python
def grace_hash_join(R, S, key, num_partitions=4):
    """Grace Hash Join — partitions to disk, then per-partition hash join."""

    def hash_fn(rec):
        return hash(rec[key]) % num_partitions

    partitions_R = [[] for _ in range(num_partitions)]
    partitions_S = [[] for _ in range(num_partitions)]

    for r in R:
        partitions_R[hash_fn(r)].append(r)
    for s in S:
        partitions_S[hash_fn(s)].append(s)

    result = []
    for i in range(num_partitions):
        # Build hash table for smaller side
        build = partitions_R[i] if len(partitions_R[i]) <= len(partitions_S[i]) else partitions_S[i]
        probe = partitions_S[i] if build is partitions_R[i] else partitions_R[i]
        ht = {}
        for rec in build:
            ht.setdefault(rec[key], []).append(rec)
        for rec in probe:
            for match in ht.get(rec[key], []):
                result.append({**rec, **match})
    return result
```

### Step 4: Sort-Merge Join

Sort both on the join key, then walk in lockstep handling groups of equal keys.

```python
def sort_merge_join(R, S, key):
    R_sorted = sorted(R, key=lambda x: x[key])
    S_sorted = sorted(S, key=lambda x: x[key])

    result = []
    i = j = 0
    while i < len(R_sorted) and j < len(S_sorted):
        rk, sk = R_sorted[i][key], S_sorted[j][key]
        if rk == sk:
            # Collect the matching group from S
            j_start = j
            while j < len(S_sorted) and S_sorted[j][key] == rk:
                j += 1
            # Emit all R-S pairs in this group
            for k in range(i, len(R_sorted)):
                if R_sorted[k][key] != rk:
                    break
                for m in range(j_start, j):
                    result.append({**R_sorted[k], **S_sorted[m]})
            while i < len(R_sorted) and R_sorted[i][key] == rk:
                i += 1
        elif rk < sk:
            i += 1
        else:
            j += 1
    return result
```

### Step 5: Cost Estimator

An I/O cost model that picks the optimal join strategy given relation sizes, memory budget, and index availability.

```python
def estimate_join_cost(pages_r, pages_s, tuples_r, memory_pages, has_index=False, pred_type="equi"):
    """Returns estimated I/Os for each algorithm, 0 = not applicable."""
    costs = {}
    # Simple NLJ
    costs["simple_nlj"] = pages_r + tuples_r * pages_s
    # Block NLJ: block size = memory_pages
    block_size = max(1, memory_pages // 2)
    costs["block_nlj"] = pages_r + (pages_r // block_size) * pages_s
    # Index NLJ (only if index available)
    costs["index_nlj"] = pages_r + tuples_r * 3 if has_index else float('inf')
    # Grace Hash Join
    fanout = min(memory_pages, 16)
    costs["grace_hash"] = 3 * (pages_r + pages_s) if pred_type == "equi" else float('inf')
    # Sort-Merge Join (external sort cost: 2P * ceil(log_M(P)))
    sort_cost_r = 2 * pages_r * max(1, (pages_r.bit_length() // memory_pages.bit_length()))
    sort_cost_s = 2 * pages_s * max(1, (pages_s.bit_length() // memory_pages.bit_length()))
    costs["sort_merge"] = sort_cost_r + sort_cost_s + pages_r + pages_s
    return costs

def pick_optimal_join(pages_r, pages_s, tuples_r, memory_pages, has_index=False, pred_type="equi"):
    costs = estimate_join_cost(pages_r, pages_s, tuples_r, memory_pages, has_index, pred_type)
    best = min(costs, key=costs.get)
    return best, costs
```

## Use It

PostgreSQL's join planner (`nodeHash.c`, `nodeNestloop.c`, `nodeMergejoin.c`) implements all three strategies with real-world refinements:

- **Batch-aware hash join**: PostgreSQL's `ExecHashJoin` builds hash tables in batches when work_mem fills up, spilling overflow to `temp_tablespaces`.
- **Materialized NLJ**: When the inner plan is expensive (e.g., a subquery), PostgreSQL materializes it first so it's scanned in memory.
- **Merge join with mark/restore**: PostgreSQL's merge join can save and restore positions within a group, handling duplicate keys more efficiently.

Your implementations capture the core algorithms. PostgreSQL adds: parallel join workers (`gatherMerge`), incremental sorting for merge joins, and adaptive memory management during hash build.

## Read the Source

- [`src/backend/executor/nodeHash.c`](https://github.com/postgres/postgres/blob/master/src/backend/executor/nodeHash.c) — Batched hash join with skew handling. Look at `ExecHashJoin` and `MultiExecHash`.
- [`src/backend/executor/nodeNestloop.c`](https://github.com/postgres/postgres/blob/master/src/backend/executor/nodeNestloop.c) — Nested loop with inner index support and materialization.
- [`src/backend/optimizer/path/joinpath.c`](https://github.com/postgres/postgres/blob/master/src/backend/optimizer/path/joinpath.c) — The planner that picks the join algorithm based on cost estimates. Look at `add_paths_to_joinrel`.

## Ship It

The reusable artifact is `outputs/join_bench.py` — a join algorithm benchmark that takes relation sizes and reports optimal strategy with estimated I/O cost. Useful for capacity planning and query optimization in later phases.

## Exercises

1. **Easy** — Run the cost estimator for: R=500 pages (50k tuples), S=200 pages (10k tuples), memory=64 pages, no index. Which join wins? Now add an index on S. How does the choice change?
2. **Medium** — Implement Hybrid Hash Join (keep one partition in memory during partitioning). Compare its I/O cost against Grace Hash Join.
3. **Hard** — Extend the cost estimator to model parallel hash join (divide partitions across N workers) and parallel sort-merge join. When does parallelism help most?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Nested Loop Join | "The dumb join" | Optimal when outer is tiny and inner has an index — used by PostgreSQL every day |
| Hash Join | "The fast join" | Best for equi-joins on large, unindexed tables with enough memory — limited to equality predicates |
| Sort-Merge Join | "The sorted join" | Wins when inputs are pre-sorted, or when the predicate is a range/inequality |
| Grace Hash Join | "The disk-based hash" | Handles any-size tables by partitioning to disk — 3-pass I/O cost |
| work_mem | "Sort/hash memory limit" | PostgreSQL parameter controlling in-memory hash table size and sort buffer — running out triggers disk spill |
| Probe Phase | "Lookup phase" | Second phase of hash join: scan one input, hash the key, and check the in-memory hash table from the build phase |

## Further Reading

- [CMU 15-445: Join Algorithms lecture](https://www.youtube.com/watch?v=9M0GUs8VdSs) — Andy Pavlo's excellent walkthrough with cost model worked examples
- [PostgreSQL NodeHash source](https://github.com/postgres/postgres/blob/master/src/backend/executor/nodeHash.c) — The real production hash join implementation
- [Join Planning in PostgreSQL](https://www.postgresql.org/docs/current/geqo-pg-intro.html) — How the planner enumerates join orders and picks algorithms
