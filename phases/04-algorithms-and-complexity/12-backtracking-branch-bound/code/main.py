"""
Backtracking, Branch & Bound
Phase 04 — Algorithms & Complexity Analysis

From-scratch implementations with pruning statistics and timing.
"""

import math
import time
from heapq import heappush, heappop


# ---------------------------------------------------------------------------
# N-Queens
# ---------------------------------------------------------------------------

def solve_nqueens(n: int) -> tuple[list[list[int]], int]:
    """Return all solutions and pruning count for N-Queens.

    Each solution is a list where board[r] = column of queen in row r.
    """
    solutions: list[list[int]] = []
    pruned = 0

    def backtrack(row: int, cols: set[int], diag1: set[int],
                  diag2: set[int], board: list[int]) -> None:
        nonlocal pruned
        if row == n:
            solutions.append(board[:])
            return
        for col in range(n):
            if col in cols or (row - col) in diag1 or (row + col) in diag2:
                pruned += 1
                continue
            cols.add(col)
            diag1.add(row - col)
            diag2.add(row + col)
            board.append(col)
            backtrack(row + 1, cols, diag1, diag2, board)
            board.pop()
            cols.remove(col)
            diag1.remove(row - col)
            diag2.remove(row + col)

    backtrack(0, set(), set(), set(), [])
    return solutions, pruned


def solve_nqueens_bitmask(n: int) -> tuple[int, int]:
    """N-Queens using bitmask representation. Returns (solution_count, pruned_count)."""
    count = 0
    pruned = 0
    all_mask = (1 << n) - 1

    def bt(row: int, cols: int, diag1: int, diag2: int) -> None:
        nonlocal count, pruned
        if row == n:
            count += 1
            return
        available = all_mask & ~(cols | diag1 | diag2)
        while available:
            bit = available & (-available)
            available ^= bit
            pruned += bin(all_mask & ~(cols | bit | ((diag1 | bit) << 1) |
                          ((diag2 | bit) >> 1))).count('1')
            bt(row + 1, cols | bit, (diag1 | bit) << 1, (diag2 | bit) >> 1)

    bt(0, 0, 0, 0)
    return count, pruned


# ---------------------------------------------------------------------------
# Sudoku Solver
# ---------------------------------------------------------------------------

def solve_sudoku(board: list[list[int]]) -> list[list[int]]:
    """Solve a 9x9 Sudoku in-place. 0 = empty cell."""
    rows: list[set[int]] = [set() for _ in range(9)]
    cols: list[set[int]] = [set() for _ in range(9)]
    boxes: list[set[int]] = [set() for _ in range(9)]
    empty: list[tuple[int, int]] = []

    for r in range(9):
        for c in range(9):
            v = board[r][c]
            if v:
                rows[r].add(v)
                cols[c].add(v)
                boxes[(r // 3) * 3 + c // 3].add(v)
            else:
                empty.append((r, c))

    def candidates(r: int, c: int) -> set[int]:
        return set(range(1, 10)) - rows[r] - cols[c] - boxes[(r // 3) * 3 + c // 3]

    def bt(idx: int) -> bool:
        if idx == len(empty):
            return True
        r, c = empty[idx]
        box = (r // 3) * 3 + c // 3
        for d in candidates(r, c):
            board[r][c] = d
            rows[r].add(d)
            cols[c].add(d)
            boxes[box].add(d)
            if bt(idx + 1):
                return True
            board[r][c] = 0
            rows[r].remove(d)
            cols[c].remove(d)
            boxes[box].remove(d)
        return False

    bt(0)
    return board


# ---------------------------------------------------------------------------
# Graph Coloring
# ---------------------------------------------------------------------------

def graph_color(adj: list[list[int]], k: int) -> tuple[list[int] | None, int]:
    """k-color a graph. Returns (coloring, pruned_count) or (None, pruned)."""
    n = len(adj)
    color = [-1] * n
    pruned = 0

    def valid(v: int, c: int) -> bool:
        return all(color[u] != c for u in adj[v])

    def bt(v: int) -> bool:
        nonlocal pruned
        if v == n:
            return True
        for c in range(k):
            if valid(v, c):
                color[v] = c
                if bt(v + 1):
                    return True
                color[v] = -1
            else:
                pruned += 1
        return False

    return (color, pruned) if bt(0) else (None, pruned)


# ---------------------------------------------------------------------------
# Subset Sum
# ---------------------------------------------------------------------------

def subset_sum(nums: list[int], target: int) -> list[list[int]]:
    """Find all subsets of nums summing to target."""
    result: list[list[int]] = []
    nums_sorted = sorted(nums)

    def bt(i: int, current: list[int], remaining: int) -> None:
        if remaining == 0:
            result.append(current[:])
            return
        if i >= len(nums_sorted) or remaining < 0:
            return
        current.append(nums_sorted[i])
        bt(i + 1, current, remaining - nums_sorted[i])
        current.pop()
        bt(i + 1, current, remaining)

    bt(0, [], target)
    return result


# ---------------------------------------------------------------------------
# TSP — Branch & Bound
# ---------------------------------------------------------------------------

def tsp_branch_bound(dist: list[list[float]]) -> tuple[list[int] | None, float]:
    """Solve TSP with branch & bound using best-first search.

    Returns (tour, cost) starting and ending at city 0.
    """
    n = len(dist)
    best_cost = math.inf
    best_tour: list[int] | None = None

    def bound(visited: set[int], cost_so_far: float) -> float:
        b = cost_so_far
        for i in range(n):
            if i not in visited:
                min_edge = min(dist[i][j] for j in range(n) if j != i)
                b += min_edge
        return b

    heap: list[tuple[float, float, list[int], set[int]]] = []
    heappush(heap, (0.0, 0.0, [0], {0}))

    while heap:
        b, cost, path, visited = heappop(heap)
        if b >= best_cost:
            continue
        if len(path) == n:
            total = cost + dist[path[-1]][0]
            if total < best_cost:
                best_cost = total
                best_tour = path[:]
            continue
        last = path[-1]
        for j in range(n):
            if j not in visited:
                new_cost = cost + dist[last][j]
                new_visited = visited | {j}
                new_path = path + [j]
                b2 = bound(new_visited, new_cost)
                if b2 < best_cost:
                    heappush(heap, (b2, new_cost, new_path, new_visited))

    return best_tour, best_cost


# ---------------------------------------------------------------------------
# Generic Backtracking Framework
# ---------------------------------------------------------------------------

class BacktrackSolver:
    """Generic backtracking framework with pruning statistics."""

    def __init__(self) -> None:
        self.nodes_explored = 0
        self.nodes_pruned = 0

    def solve(self, state, choose, constrain, accept, apply, undo):
        self.nodes_explored = 0
        self.nodes_pruned = 0
        results = []

        def bt(s):
            self.nodes_explored += 1
            if accept(s):
                results.append(s[:])
                return
            for choice in choose(s):
                if not constrain(s, choice):
                    self.nodes_pruned += 1
                    continue
                apply(s, choice)
                bt(s)
                undo(s, choice)

        bt(state)
        return results


# ---------------------------------------------------------------------------
# Demos
# ---------------------------------------------------------------------------

def main() -> None:
    print("=== Backtracking, Branch & Bound ===\n")

    # --- N-Queens ---
    print("--- N-Queens ---")
    for n in [4, 8, 12]:
        start = time.perf_counter()
        sols, pruned = solve_nqueens(n)
        elapsed = time.perf_counter() - start
        total_nodes = pruned + len(sols)
        print(f"  n={n:>2}: {len(sols):>3} solutions, "
              f"{pruned:>7} pruned branches, "
              f"{total_nodes:>7} total nodes, "
              f"{elapsed * 1000:.1f} ms")
    print()

    # --- Sudoku ---
    print("--- Sudoku Solver ---")
    puzzle = [
        [5, 3, 0, 0, 7, 0, 0, 0, 0],
        [6, 0, 0, 1, 9, 5, 0, 0, 0],
        [0, 9, 8, 0, 0, 0, 0, 6, 0],
        [8, 0, 0, 0, 6, 0, 0, 0, 3],
        [4, 0, 0, 8, 0, 3, 0, 0, 1],
        [7, 0, 0, 0, 2, 0, 0, 0, 6],
        [0, 6, 0, 0, 0, 0, 2, 8, 0],
        [0, 0, 0, 4, 1, 9, 0, 0, 5],
        [0, 0, 0, 0, 8, 0, 0, 7, 9],
    ]
    start = time.perf_counter()
    solved = solve_sudoku(puzzle)
    elapsed = time.perf_counter() - start
    print(f"  Solved in {elapsed * 1000:.2f} ms")
    for row in solved:
        print(f"  {row}")
    print()

    # --- Graph Coloring ---
    print("--- Graph Coloring (4-node cycle, k=2) ---")
    adj = [[1, 3], [0, 2], [1, 3], [0, 2]]
    start = time.perf_counter()
    coloring, pruned = graph_color(adj, 2)
    elapsed = time.perf_counter() - start
    print(f"  Coloring: {coloring}, pruned: {pruned}, "
          f"{elapsed * 1000:.2f} ms")

    print("--- Graph Coloring (Petersen graph, k=3) ---")
    petersen = [
        [1, 4, 5], [0, 2, 6], [1, 3, 7], [2, 4, 8], [0, 3, 9],
        [0, 7, 8], [1, 8, 9], [2, 5, 9], [3, 5, 6], [4, 6, 7],
    ]
    start = time.perf_counter()
    coloring, pruned = graph_color(petersen, 3)
    elapsed = time.perf_counter() - start
    print(f"  Coloring: {coloring}, pruned: {pruned}, "
          f"{elapsed * 1000:.2f} ms")
    print()

    # --- Subset Sum ---
    print("--- Subset Sum ---")
    nums = [3, 34, 4, 12, 5, 2]
    target = 9
    start = time.perf_counter()
    subs = subset_sum(nums, target)
    elapsed = time.perf_counter() - start
    print(f"  nums={nums}, target={target}")
    print(f"  Solutions: {subs} ({elapsed * 1000:.2f} ms)")
    print()

    # --- TSP Branch & Bound ---
    print("--- TSP Branch & Bound ---")
    dist = [
        [0, 10, 15, 20],
        [10, 0, 35, 25],
        [15, 35, 0, 30],
        [20, 25, 30, 0],
    ]
    start = time.perf_counter()
    tour, cost = tsp_branch_bound(dist)
    elapsed = time.perf_counter() - start
    print(f"  Tour: {tour}, cost: {cost}, {elapsed * 1000:.2f} ms")
    print()

    # --- Generic Framework Demo (N-Queens) ---
    print("--- Generic Backtrack Framework (8-Queens) ---")
    solver = BacktrackSolver()
    n = 8

    def choose(state):
        row = len(state)
        if row >= n:
            return []
        return range(n)

    def constrain(state, col):
        row = len(state)
        for r, c in enumerate(state):
            if c == col or abs(r - row) == abs(c - col):
                return False
        return True

    def accept(state):
        return len(state) == n

    def apply(state, choice):
        state.append(choice)

    def undo(state, choice):
        state.pop()

    start = time.perf_counter()
    results = solver.solve([], choose, constrain, accept, apply, undo)
    elapsed = time.perf_counter() - start
    print(f"  Solutions: {len(results)}, nodes explored: {solver.nodes_explored}, "
          f"pruned: {solver.nodes_pruned}, {elapsed * 1000:.1f} ms")


if __name__ == "__main__":
    main()
