# DP III — Bitmask, Digit, Tree, DP on DAGs

> Four advanced DP paradigms that unlock problems standard tabulation cannot touch.

**Type:** Learn
**Languages:** Python, C++
**Prerequisites:** Phase 04 lessons 01–09
**Time:** ~90 minutes

## Learning Objectives

- Model subsets as integers and use bitmask DP to solve TSP and subset-enumeration problems.
- Design digit DP states to count or sum over numbers satisfying digit-level constraints.
- Aggregate subtree information with tree DP and apply rerooting to answer queries for every node.
- Exploit DAG structure for topological-order DP on shortest, longest, and counting tasks.

## The Problem

Standard 1D/2D DP works when the state is a range or a pair of indices. Many real problems live on
*subsets*, *digits of a number*, or *tree structures* — the state space is exponential or shaped by
the input topology. Without these four techniques you cannot solve the Traveling Salesman Problem,
count integers with digit constraints, find the maximum independent set on a tree, or find the longest
path in a DAG.

## The Concept

### 1. Bitmask DP

**Core idea.** A state is an integer whose bits encode a *subset* of elements. Bit `i` set means
element `i` is in the subset.

**Canonical example — TSP.** `n` cities, distance matrix, find shortest tour visiting all exactly once.

```
dp[S][i] = shortest path visiting exactly the cities in bitmask S, ending at city i
dp[S][i] = min over j in S\{i} of dp[S - {i}][j] + dist[j][i]
Base: dp[{0}][0] = 0
Answer: min over i of dp[full][i] + dist[i][0]
Complexity: O(2^n · n^2)
```

**Key bit tricks:** `S | (1<<i)` adds, `S & ~(1<<i)` removes, `S & (1<<i)` tests membership,
`sub = (sub-1) & S` iterates all submasks. Works for `n ≤ 20–25`.

### 2. Digit DP

**Core idea.** Count integers `1…N` satisfying a digit property by walking MSB→LSB, tracking whether
the prefix is still *tight* (equal to N's prefix so far).

```
State: dp[pos][tight][...flags]
  pos   — current digit position (MSB first)
  tight — 1 if prefix == N's prefix so far, 0 if already smaller
  flags — problem-specific: last digit, digit count, sum mod k, etc.
Transition: for d in [0, limit]: dp[pos][tight][...] += dp[pos+1][tight && d==limit][...]
Base: dp[len][...]=1 if number satisfies the property.
```

Example: count numbers in [1,N] containing no 4. State is just `(pos, tight, started)` — no extra
flags needed since the constraint is per-digit.

### 3. Tree DP

**Core idea.** Root the tree arbitrarily. The answer for a node depends on answers for its children.

**Maximum Independent Set:**
```
dp[u][0] = sum over children v of max(dp[v][0], dp[v][1])   // u not picked
dp[u][1] = 1 + sum over children v of dp[v][0]              // u picked
```

**Rerooting.** Compute answer for *every* node as root in O(n): one post-order DFS for "down"
values, one pre-order DFS to propagate "up" values from parent. Each edge crossed twice.

### 4. DP on DAGs

**Core idea.** Topological order guarantees each vertex is processed after all predecessors.

```
topo = topological_sort(adj)
for u in topo:
    for (v, w) in adj[u]:
        dp[v] = max(dp[v], dp[u] + w)    // longest path
```

Backbone of scheduling (PERT/CPM), longest increasing subsequence (index graph is a DAG).

### Summary

| Technique | State space | Key insight | Complexity |
|-----------|-------------|-------------|------------|
| Bitmask DP | 2^n × poly(n) | Subset = integer | O(2^n · n^2) |
| Digit DP | digits × tight × flags | Walk MSB→LSB, tight controls limit | O(digits · flags · 10) |
| Tree DP | n × constant | Post-order aggregation, rerooting | O(n) |
| DAG DP | V × constant | Topological order → acyclic deps | O(V + E) |

## Build It

See `code/main.py` for full implementations of:
- `tsp_bitmask(dist, n)` — TSP via bitmask DP
- `count_no_four(N)` / `count_no_adjacent_same(N)` — digit DP
- `tree_max_independent_set(tree, root)` + `reroot_mis(tree)` — tree DP with rerooting
- `dag_longest_path(adj, n)` — DAG longest path

See `code/main.cpp` for the C++ mirror of TSP bitmask DP and Hamiltonian path counting.

## Use It

- **Bitmask DP** — Constraint satisfaction: scheduling `n ≤ 20` jobs with pairwise conflicts.
  Network routing: find min-cost Hamiltonian path as a sub-problem of survivable routing.
- **Digit DP** — Competitive programming: "count numbers in [L, R] whose digit sum is prime."
  Cryptographic analysis: counting keys with structural properties.
- **Tree DP** — Network design: rerooting gives cost of removing each node in O(n). Biology:
  phylogenetic tree parsimony scoring.
- **DAG DP** — Build systems (make, Bazel): longest path = critical path = minimum build time.
  PERT charts in project management are literally longest-path-in-DAG.

## Read the Source

- Python `itertools` — `Lib/itertools.py` — subset enumeration via `combinations()`.
- C++ `<bitset>` — `bits/reference.cc` in libstdc++ — fixed-width bitmask operations.

## Ship It

The reusable artifact lives in `outputs/`:

- **Bitmask DP template** — `tsp_bitmask(dist, n)` generalises to any "visit each of n items
  exactly once" problem by swapping the transition cost function.

## Exercises

1. **Hamiltonian path count.** Given an adjacency matrix `adj[n][n]`, count Hamiltonian paths
   starting at vertex 0. Use bitmask DP with `dp[S][i]` = number of paths visiting subset `S`
   and ending at `i`.

2. **No two adjacent same digits.** Count integers in `[1, N]` (N ≤ 10^18) with no two adjacent
   digits equal. Use digit DP with state `(pos, tight, last_digit)`.

3. **Tree diameter via tree DP.** For each node `u`, compute the longest path through `u` (two
   deepest child-subtrees). Extend to rerooting to answer "diameter if node `u` is removed."

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Bitmask | "pack a subset into an int" | `n`-bit integer where bit `i` indicates presence of element `i` |
| Digit DP | "walk digits, track tight" | DP over digit positions; `tight` limits digit to N's digit |
| Rerooting | "answer for every root" | Two-pass DFS: compute down-values, propagate up-values in O(n) |
| Topological order | "linearize the DAG" | Permutation where every edge goes left→right; enables 1-pass DP |
| Hamiltonian path | "visit every node once" | Path visiting all `n` vertices exactly once |

## Further Reading

- "Competitive Programmer's Handbook" — Laaksonen, Ch. 7 (DP on subsets), Ch. 8 (tree DP).
- "Introduction to Algorithms" (CLRS) — Ch. 15 §15.3 (DAG shortest paths), Ch. 34 §34.5 (TSP).
- Codeforces EDU: "Dynamic Programming over Subsets" module.
