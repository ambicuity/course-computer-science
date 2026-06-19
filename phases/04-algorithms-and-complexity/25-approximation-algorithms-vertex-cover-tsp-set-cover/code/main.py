"""
Approximation Algorithms — Vertex Cover, TSP, Set Cover
Phase 04 — Algorithms & Complexity Analysis

Implements greedy approximation algorithms with ratio verification
against brute-force optimal solutions on small instances.
"""

from __future__ import annotations

import itertools
import math
import random
from heapq import heappush, heappop


# ---------------------------------------------------------------------------
# Vertex Cover — 2-approx via maximal matching
# ---------------------------------------------------------------------------

def greedy_vertex_cover(edges: list[tuple[int, int]]) -> set[int]:
    """2-approximate vertex cover via maximal matching.

    Greedily pick uncovered edges, take both endpoints.
    Proof: |cover| = 2|M| where M is maximal matching.
    Since OPT must pick >= 1 vertex per matching edge, |OPT| >= |M|.
    So |cover| = 2|M| <= 2|OPT|.
    """
    remaining = set(edges)
    cover = set()
    while remaining:
        u, v = next(iter(remaining))
        cover.add(u)
        cover.add(v)
        remaining = {(a, b) for a, b in remaining if a != u and a != v and b != u and b != v}
    return cover


def brute_force_vertex_cover(edges: list[tuple[int, int]], n: int) -> set[int]:
    """Exact minimum vertex cover via brute force over subsets. O(2^n)."""
    vertices = set()
    for a, b in edges:
        vertices.add(a)
        vertices.add(b)
    best = None
    for r in range(1, len(vertices) + 1):
        for combo in itertools.combinations(vertices, r):
            cover = set(combo)
            if all(u in cover or v in cover for u, v in edges):
                if best is None or len(combo) < len(best):
                    best = set(combo)
        if best is not None:
            break
    return best if best is not None else set()


# ---------------------------------------------------------------------------
# Metric TSP — MST + preorder walk (2-approx)
# ---------------------------------------------------------------------------

def mst_prim(n: int, dist: list[list[float]]) -> list[tuple[int, int, float]]:
    """Prim's MST on a complete graph. O(n^2 log n) with heap."""
    visited = {0}
    edges = []
    heap: list[tuple[float, int, int]] = [(dist[0][j], 0, j) for j in range(1, n)]
    # heapq.heapify not needed — heappush handles it below
    heap_list = []
    for item in heap:
        heappush(heap_list, item)
    while heap_list and len(visited) < n:
        w, u, v = heappop(heap_list)
        if v in visited:
            continue
        visited.add(v)
        edges.append((u, v, w))
        for j in range(n):
            if j not in visited:
                heappush(heap_list, (dist[v][j], v, j))
    return edges


def preorder_walk(n: int, mst_edges: list[tuple[int, int, float]]) -> list[int]:
    """DFS preorder traversal of MST rooted at 0."""
    adj: list[list[int]] = [[] for _ in range(n)]
    for u, v, _ in mst_edges:
        adj[u].append(v)
        adj[v].append(u)
    order = []
    visited = set()
    stack = [0]
    while stack:
        node = stack.pop()
        if node in visited:
            continue
        visited.add(node)
        order.append(node)
        for nb in reversed(adj[node]):
            if nb not in visited:
                stack.append(nb)
    return order


def metric_tsp_approx(dist: list[list[float]]) -> tuple[list[int], float]:
    """2-approximate metric TSP via MST preorder walk.

    Returns (tour, cost).
    Proof: tour_cost <= 2 * MST_cost <= 2 * TSP_opt.
    """
    n = len(dist)
    mst_edges = mst_prim(n, dist)
    order = preorder_walk(n, mst_edges)
    cost = tour_cost(order, dist)
    return order, cost


def tour_cost(order: list[int], dist: list[list[float]]) -> float:
    """Compute round-trip tour cost."""
    cost = 0.0
    for i in range(len(order)):
        cost += dist[order[i]][order[(i + 1) % len(order)]]
    return cost


# ---------------------------------------------------------------------------
# Christofides — 1.5-approx for metric TSP (small n)
# ---------------------------------------------------------------------------

def _odd_degree_vertices(n: int, mst_edges: list[tuple[int, int, float]]) -> list[int]:
    """Find vertices with odd degree in the MST."""
    deg = [0] * n
    for u, v, _ in mst_edges:
        deg[u] += 1
        deg[v] += 1
    return [i for i in range(n) if deg[i] % 2 == 1]


def _min_weight_perfect_matching(
    vertices: list[int], dist: list[list[float]]
) -> list[tuple[int, int]]:
    """Minimum-weight perfect matching via bitmask DP. O(2^k * k^2).

    vertices: list of vertex indices (even count).
    Returns list of matched pairs (original vertex indices).
    """
    k = len(vertices)
    if k == 0:
        return []
    assert k % 2 == 0

    INF = float("inf")
    dp = [INF] * (1 << k)
    dp[0] = 0.0
    parent = [-1] * (1 << k)

    for mask in range(1 << k):
        if dp[mask] >= INF:
            continue
        # find first unmatched vertex
        i = 0
        while i < k and (mask >> i) & 1:
            i += 1
        if i >= k:
            continue
        for j in range(i + 1, k):
            if (mask >> j) & 1:
                continue
            new_mask = mask | (1 << i) | (1 << j)
            w = dist[vertices[i]][vertices[j]]
            if dp[mask] + w < dp[new_mask]:
                dp[new_mask] = dp[mask] + w
                parent[new_mask] = mask

    # reconstruct pairs
    pairs = []
    mask = (1 << k) - 1
    while mask:
        prev = parent[mask]
        diff = mask ^ prev
        bits = [i for i in range(k) if (diff >> i) & 1]
        pairs.append((vertices[bits[0]], vertices[bits[1]]))
        mask = prev
    return pairs


def _euler_tour(n: int, multiedges: list[tuple[int, int]]) -> list[int]:
    """Find Euler tour in an Eulerian multigraph via Hierholzer's algorithm."""
    adj: list[list[int]] = [[] for _ in range(n)]
    for u, v in multiedges:
        adj[u].append(v)
        adj[v].append(u)

    tour = []
    stack = [0]
    while stack:
        v = stack[-1]
        if adj[v]:
            u = adj[v].pop()
            adj[u].remove(v)
            stack.append(u)
        else:
            tour.append(stack.pop())
    tour.reverse()
    return tour


def _shortcut(euler_tour: list[int]) -> list[int]:
    """Shortcut Euler tour to Hamiltonian tour (first-visit order)."""
    visited = set()
    order = []
    for v in euler_tour:
        if v not in visited:
            visited.add(v)
            order.append(v)
    return order


def christofides_tsp(dist: list[list[float]]) -> tuple[list[int], float]:
    """1.5-approximate metric TSP via Christofides' algorithm.

    Steps:
    1. Compute MST T
    2. Find odd-degree vertices O in T
    3. Min-weight perfect matching M on O
    4. Euler tour of T ∪ M
    5. Shortcut to Hamiltonian tour

    For small n (<= 12) since matching step is O(2^k * k^2).
    """
    n = len(dist)
    mst_edges = mst_prim(n, dist)

    # Odd-degree vertices
    odd_verts = _odd_degree_vertices(n, mst_edges)
    assert len(odd_verts) % 2 == 0

    # Min-weight perfect matching on odd vertices
    matching_pairs = _min_weight_perfect_matching(odd_verts, dist)

    # Build Eulerian multigraph
    multiedges = [(u, v) for u, v, _ in mst_edges]
    multiedges.extend(matching_pairs)

    # Euler tour then shortcut
    euler = _euler_tour(n, multiedges)
    order = _shortcut(euler)
    cost = tour_cost(order, dist)
    return order, cost


def brute_force_tsp(dist: list[list[float]]) -> tuple[list[int], float]:
    """Exact TSP via brute force. O(n!)."""
    n = len(dist)
    best_cost = float("inf")
    best_order = None
    for perm in itertools.permutations(range(1, n)):
        order = [0] + list(perm)
        c = tour_cost(order, dist)
        if c < best_cost:
            best_cost = c
            best_order = order
    return best_order, best_cost


# ---------------------------------------------------------------------------
# Set Cover — Greedy O(log n)-approximation
# ---------------------------------------------------------------------------

def greedy_set_cover(universe: set, sets: list[tuple[str, set]]) -> list[str]:
    """Greedy O(log n)-approximate set cover.

    Each step: pick set covering the most uncovered elements.
    Guarantee: |cover| <= (ln n + 1) * OPT.
    """
    uncovered = set(universe)
    cover = []
    available = list(sets)

    while uncovered:
        best_idx = max(range(len(available)), key=lambda i: len(available[i][1] & uncovered))
        name, s = available[best_idx]
        newly_covered = s & uncovered
        if not newly_covered:
            break
        cover.append(name)
        uncovered -= newly_covered
    return cover


def brute_force_set_cover(universe: set, sets: list[tuple[str, set]]) -> list[str]:
    """Exact minimum set cover via brute force. O(2^m)."""
    m = len(sets)
    best = None
    for r in range(1, m + 1):
        for combo in itertools.combinations(range(m), r):
            covered = set()
            for idx in combo:
                covered |= sets[idx][1]
            if covered >= universe:
                names = [sets[idx][0] for idx in combo]
                if best is None or len(names) < len(best):
                    best = names
        if best is not None:
            break
    return best if best is not None else []


# ---------------------------------------------------------------------------
# Metric distance matrix generation
# ---------------------------------------------------------------------------

def random_metric_matrix(n: int, seed: int = 42) -> list[list[float]]:
    """Generate a random metric (Euclidean) distance matrix."""
    rng = random.Random(seed)
    points = [(rng.uniform(0, 100), rng.uniform(0, 100)) for _ in range(n)]
    dist = [[0.0] * n for _ in range(n)]
    for i in range(n):
        for j in range(i + 1, n):
            d = math.hypot(points[i][0] - points[j][0], points[i][1] - points[j][1])
            dist[i][j] = d
            dist[j][i] = d
    return dist


# ---------------------------------------------------------------------------
# Main demonstration
# ---------------------------------------------------------------------------

def main() -> None:
    print("=" * 65)
    print("  Approximation Algorithms — Vertex Cover, TSP, Set Cover")
    print("=" * 65)

    # --- Vertex Cover ---
    print("\n--- Vertex Cover (2-approx via maximal matching) ---\n")
    edges_vc = [(0, 1), (0, 3), (1, 2), (1, 4), (2, 3), (3, 4)]
    n_vc = 5
    approx_vc = greedy_vertex_cover(edges_vc)
    opt_vc = brute_force_vertex_cover(edges_vc, n_vc)
    ratio_vc = len(approx_vc) / len(opt_vc) if opt_vc else float("inf")
    print(f"  Edges:        {edges_vc}")
    print(f"  Approx cover: {sorted(approx_vc)}  (size {len(approx_vc)})")
    print(f"  Optimal cover:{sorted(opt_vc)}  (size {len(opt_vc)})")
    print(f"  Ratio:        {ratio_vc:.2f}x  (guaranteed <= 2.0x)")

    # --- Metric TSP (MST walk) ---
    print("\n--- Metric TSP (MST + preorder walk, 2-approx) ---\n")
    n_tsp = 6
    dist = random_metric_matrix(n_tsp, seed=7)
    tour_approx, cost_approx = metric_tsp_approx(dist)
    tour_opt, cost_opt = brute_force_tsp(dist)
    ratio_tsp = cost_approx / cost_opt if cost_opt > 0 else float("inf")
    print(f"  n = {n_tsp} cities")
    print(f"  Approx tour:  {tour_approx}  cost = {cost_approx:.2f}")
    print(f"  Optimal tour: {tour_opt}  cost = {cost_opt:.2f}")
    print(f"  Ratio:        {ratio_tsp:.2f}x  (guaranteed <= 2.0x)")

    # --- Christofides ---
    print("\n--- Christofides TSP (1.5-approx) ---\n")
    n_chr = 6
    dist_chr = random_metric_matrix(n_chr, seed=42)
    tour_chr, cost_chr = christofides_tsp(dist_chr)
    tour_opt_chr, cost_opt_chr = brute_force_tsp(dist_chr)
    ratio_chr = cost_chr / cost_opt_chr if cost_opt_chr > 0 else float("inf")
    print(f"  n = {n_chr} cities")
    print(f"  Christofides tour: {tour_chr}  cost = {cost_chr:.2f}")
    print(f"  Optimal tour:      {tour_opt_chr}  cost = {cost_opt_chr:.2f}")
    print(f"  Ratio:             {ratio_chr:.2f}x  (guaranteed <= 1.5x)")

    # --- Set Cover ---
    print("\n--- Set Cover (greedy ln(n)-approx) ---\n")
    universe = set(range(1, 9))  # {1..8}
    sets_sc = [
        ("S1", {1, 2, 3, 4}),
        ("S2", {3, 4, 5, 6}),
        ("S3", {5, 6, 7, 8}),
        ("S4", {1, 5}),
        ("S5", {2, 6}),
        ("S6", {3, 7}),
        ("S7", {4, 8}),
        ("S8", {1, 2, 7, 8}),
    ]
    approx_sc = greedy_set_cover(universe, sets_sc)
    opt_sc = brute_force_set_cover(universe, sets_sc)
    ln_n = math.log(len(universe))
    ratio_sc = len(approx_sc) / len(opt_sc) if opt_sc else float("inf")
    print(f"  Universe:       {sorted(universe)}")
    print(f"  Greedy cover:   {approx_sc}  (size {len(approx_sc)})")
    print(f"  Optimal cover:  {opt_sc}  (size {len(opt_sc)})")
    print(f"  Ratio:          {ratio_sc:.2f}x  (guaranteed <= ln(n)+1 = {ln_n + 1:.2f}x)")

    # --- Empirical Christofides study ---
    print("\n--- Empirical: Christofides vs MST-walk (100 random instances) ---\n")
    chr_wins = 0
    mst_wins = 0
    total_chr_ratio = 0.0
    total_mst_ratio = 0.0
    trials = 100
    for trial in range(trials):
        n_emp = 7
        d = random_metric_matrix(n_emp, seed=trial * 100 + 7)
        _, c_chr = christofides_tsp(d)
        _, c_mst = metric_tsp_approx(d)
        _, c_opt = brute_force_tsp(d)
        total_chr_ratio += c_chr / c_opt
        total_mst_ratio += c_mst / c_opt
        if c_chr < c_mst:
            chr_wins += 1
        elif c_mst < c_chr:
            mst_wins += 1
    print(f"  Christofides avg ratio: {total_chr_ratio / trials:.4f}")
    print(f"  MST-walk avg ratio:     {total_mst_ratio / trials:.4f}")
    print(f"  Christofides wins: {chr_wins}  MST-walk wins: {mst_wins}  Tie: {trials - chr_wins - mst_wins}")

    print("\nDone.")


if __name__ == "__main__":
    main()
