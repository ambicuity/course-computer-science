# Backtracking, Branch & Bound

> Systematic search that never explores dead ends twice.

**Type:** Learn
**Languages:** Python, C++
**Prerequisites:** Phase 04 lessons 01–11
**Time:** ~75 minutes

## Learning Objectives

- Implement backtracking with pruning for N-Queens, Sudoku, graph coloring, and subset sum.
- Apply branch & bound to optimization problems using bounding functions.
- Compare backtracking (find any/all solutions) vs branch & bound (find optimal solution).
- Instrument code to report pruning statistics and reason about search space reduction.

## The Problem

Lesson 10 taught bitmask DP — exhaustive search when subproblems overlap. But many problems have structured state spaces where partial solutions can be tested early and abandoned. N-Queens, Sudoku, graph coloring, and constraint satisfaction don't fit the DP model. TSP and knapsack do, but branch & bound explores only a fraction of the DP table by pruning with bounds.

## The Concept

### Backtracking: Systematic Search with Pruning

DFS over the space of partial solutions. At each step: **choose** a value, **constraint check** — if violated, **prune** (abandon this branch), otherwise **recurse**, then **un-choose** (backtrack).

```
                  root
               /    |    \
             a      b     c          ← choose first variable
           / | \    | \    |
          d  e  f   g  h   i         ← choose second variable
          ✗  |  ✗   ✗  |   ✗        ← pruned branches
             v         w             ← valid partial solutions
```

**Key insight:** pruning cuts the tree early. For N-Queens, brute force explores n! permutations. Backtracking prunes as soon as two queens share a diagonal, reducing explored nodes by orders of magnitude.

### Classic Problems

**1. N-Queens** — Place N queens on N×N board, no two attacking. Track `cols`, `diag1` (row−col), `diag2` (row+col). Prune when a candidate shares a line.

**2. Sudoku Solver** — Fill 9×9 grid. Use constraint propagation: compute allowed digits per cell, pick the cell with fewest options, try each, recurse.

**3. Graph Coloring** — k-color a graph. Prune when a neighbor already has the candidate color.

**4. Subset Sum** — Find subset summing to target. Prune when running sum exceeds target (positive numbers) or remaining elements can't reach it.

### Branch & Bound: Backtracking for Optimization

Backtracking finds *any valid* solution. Branch & bound finds the *optimal* solution. At each node, compute a **bounding function** — a cheap estimate of the best achievable value. If the bound cannot beat the current best, prune.

**Bounding functions** must be optimistic (minimization) or pessimistic (maximization). Common: relaxation (drop a constraint), or cost-so-far + admissible heuristic.

**DFS vs Best-First:** DFS updates a global best; best-first (priority queue) always expands the most promising node first — finds optimum faster but uses more memory.

**TSP example:** bound = cost-so-far + sum of minimum outgoing edges for unvisited cities. If bound ≥ current best tour cost, prune.

### Comparison

| Aspect | Backtracking | Branch & Bound |
|--------|-------------|----------------|
| Goal | Find valid solutions | Find optimal solution |
| Pruning | Constraint violation | Bound cannot beat best |
| Bound function | Not needed | Required |
| Output | One or all solutions | One optimal solution |

## Build It

### Step 1: N-Queens with Pruning Statistics

```python
def solve_nqueens(n):
    solutions, pruned = [], 0
    def backtrack(row, cols, diag1, diag2, board):
        nonlocal pruned
        if row == n: solutions.append(board[:]); return
        for col in range(n):
            if col in cols or (row-col) in diag1 or (row+col) in diag2:
                pruned += 1; continue
            cols.add(col); diag1.add(row-col); diag2.add(row+col); board.append(col)
            backtrack(row+1, cols, diag1, diag2, board)
            board.pop(); cols.remove(col); diag1.remove(row-col); diag2.remove(row+col)
    backtrack(0, set(), set(), set(), [])
    return solutions, pruned
```

### Step 2: Sudoku Solver with Constraint Propagation

```python
def solve_sudoku(board):
    rows, cols, boxes = [set() for _ in range(9)] * 3
    empty = []
    for r in range(9):
        for c in range(9):
            v = board[r][c]
            if v: rows[r].add(v); cols[c].add(v); boxes[(r//3)*3+c//3].add(v)
            else: empty.append((r, c))
    def bt(idx):
        if idx == len(empty): return True
        r, c = empty[idx]; box = (r//3)*3+c//3
        for d in set(range(1,10)) - rows[r] - cols[c] - boxes[box]:
            board[r][c]=d; rows[r].add(d); cols[c].add(d); boxes[box].add(d)
            if bt(idx+1): return True
            board[r][c]=0; rows[r].remove(d); cols[c].remove(d); boxes[box].remove(d)
        return False
    bt(0)
    return board
```

### Step 3: TSP with Branch & Bound

```python
def tsp_branch_bound(dist):
    n, best_cost, best_tour = len(dist), math.inf, None
    def bound(visited, cost_so_far):
        b = cost_so_far
        for i in range(n):
            if i not in visited: b += min(dist[i][j] for j in range(n) if j != i)
        return b
    heap = []; heappush(heap, (0, 0, [0], {0}))
    while heap:
        b, cost, path, visited = heappop(heap)
        if b >= best_cost: continue
        if len(path) == n:
            total = cost + dist[path[-1]][0]
            if total < best_cost: best_cost = total; best_tour = path[:]
            continue
        for j in range(n):
            if j not in visited:
                nc = cost + dist[path[-1]][j]; nv = visited|{j}; np = path+[j]
                b2 = bound(nv, nc)
                if b2 < best_cost: heappush(heap, (b2, nc, np, nv))
    return best_tour, best_cost
```

See `code/main.py` for the complete implementations including graph coloring, subset sum, and a generic backtracking framework.

## Use It

- **SAT solvers** (MiniSat, CaDiCaL) — CDCL backtracking with conflict-driven clause learning.
- **Constraint programming** (MiniZinc, OR-Tools CP-SAT) — declarative modeling with automatic backtracking + propagation.
- **Operations research** — Gurobi, CPLEX use branch & bound with LP relaxation as bounding function.

Production enhancements: arc consistency, symmetry breaking, nogood learning, restarts, parallel search.

## Read the Source

- **Python `itertools`:** `Lib/itertools.py` — `permutations`, `combinations` enumerate the same search spaces backtracking prunes.
- **MiniSat:** `minisat/core/Solver.cc` — CDCL backtracking (~2000 lines).

## Ship It

`outputs/` contains **a generic backtracking framework with pluggable choose/constrain/accept functions and pruning counters** — reuse for any constraint satisfaction problem.

## Exercises

1. **Easy** — Solve N-Queens using bitmask representation (integers instead of sets). Verify same solution count, compare node counts.
2. **Medium** — Implement branch & bound for 0/1 knapsack with fractional knapsack as bounding function. Compare nodes explored vs DP table size from lesson 08.
3. **Hard** — Solve a 6×6 KenKen puzzle using backtracking. Each cage has a target and operation (+, −, ×, ÷). Implement constraint propagation for cages.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Backtracking | "Try and undo" | DFS over partial solutions, pruning constraint violations |
| Pruning | "Skip bad branches" | Abandon a partial solution that provably cannot lead to valid/optimal result |
| Branch & Bound | "Smart exhaustive search" | Backtracking with bounding function pruning branches worse than current best |
| Bounding function | "Estimate of the best" | Cheap optimistic (min) or pessimistic (max) estimate of achievable value |
| Constraint propagation | "Reduce the search space" | Deduce forced values before branching |

## Further Reading

- Cormen et al., *Introduction to Algorithms*, Ch. 34.
- Russell & Norvig, *Artificial Intelligence: A Modern Approach*, Ch. 6 (CSP backtracking).
- Applegate et al., *The Traveling Salesman Problem* — the Concorde solver uses branch & cut.
