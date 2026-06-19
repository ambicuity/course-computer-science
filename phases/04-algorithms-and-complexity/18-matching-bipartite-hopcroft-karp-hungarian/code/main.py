"""
Matching — Bipartite, Hopcroft-Karp, Hungarian
Phase 04 — Algorithms & Complexity Analysis, Lesson 18

Implements:
  1. bipartite_max_matching — DFS augmenting-path baseline O(V·E)
  2. hopcroft_karp           — BFS+DFS layered augmenting O(E√V)
  3. hungarian               — minimum-weight perfect matching O(V³)
  4. gale_shapley            — deferred-acceptance stable matching O(n²)
"""

from collections import deque


# ---------------------------------------------------------------------------
# 1. Augmenting-path bipartite matching (baseline)
# ---------------------------------------------------------------------------

def bipartite_max_matching(graph, left, right):
    """DFS augmenting-path maximum bipartite matching. O(V * E).

    Args:
        graph: dict  left-node -> list of adjacent right-nodes
        left:  iterable of left-side vertex labels
        right: iterable of right-side vertex labels (unused but kept for API symmetry)

    Returns:
        (size, match_r) where match_r[right_node] = left_node
    """
    match_r = {}

    def dfs(u, visited):
        for v in graph[u]:
            if v not in visited:
                visited.add(v)
                if v not in match_r or dfs(match_r[v], visited):
                    match_r[v] = u
                    return True
        return False

    result = 0
    for u in left:
        if dfs(u, set()):
            result += 1
    return result, match_r


# ---------------------------------------------------------------------------
# 2. Hopcroft-Karp  O(E sqrt(V))
# ---------------------------------------------------------------------------

def hopcroft_karp(graph, left, right):
    """Hopcroft-Karp maximum bipartite matching. O(E * sqrt(V)).

    Args:
        graph: dict  left-node -> list of adjacent right-nodes
        left:  list/set of left-side vertices
        right: list/set of right-side vertices

    Returns:
        (size, pair_u) where pair_u[left_node] = matched right_node or None
    """
    pair_u = {u: None for u in left}
    pair_v = {v: None for v in right}
    dist = {}

    def bfs():
        queue = deque()
        for u in left:
            if pair_u[u] is None:
                dist[u] = 0
                queue.append(u)
            else:
                dist[u] = float("inf")
        found = False
        while queue:
            u = queue.popleft()
            for v in graph[u]:
                pu = pair_v[v]
                if pu is None:
                    found = True
                elif dist.get(pu, float("inf")) == float("inf"):
                    dist[pu] = dist[u] + 1
                    queue.append(pu)
        return found

    def dfs(u):
        for v in graph[u]:
            pu = pair_v[v]
            if pu is None or (dist.get(pu) == dist[u] + 1 and dfs(pu)):
                pair_u[u] = v
                pair_v[v] = u
                return True
        dist[u] = float("inf")
        return False

    matching = 0
    while bfs():
        for u in left:
            if pair_u[u] is None and dfs(u):
                matching += 1
    return matching, pair_u


# ---------------------------------------------------------------------------
# 3. Hungarian algorithm  O(n^3)
# ---------------------------------------------------------------------------

def hungarian(cost):
    """Minimum-weight perfect matching (assignment problem). O(n^3).

    Args:
        cost: n×n list of lists — cost[i][j] = assigning left-i to right-j.

    Returns:
        (min_cost, assignment) where assignment[i] = j means left-i → right-j.
    """
    n = len(cost)
    u = [0] * (n + 1)
    v = [0] * (n + 1)
    p = [0] * (n + 1)
    way = [0] * (n + 1)

    for i in range(1, n + 1):
        p[0] = i
        j0 = 0
        min_v = [float("inf")] * (n + 1)
        used = [False] * (n + 1)
        while True:
            used[j0] = True
            i0 = p[j0]
            delta = float("inf")
            j1 = 0
            for j in range(1, n + 1):
                if not used[j]:
                    cur = cost[i0 - 1][j - 1] - u[i0] - v[j]
                    if cur < min_v[j]:
                        min_v[j] = cur
                        way[j] = j0
                    if min_v[j] < delta:
                        delta = min_v[j]
                        j1 = j
            for j in range(n + 1):
                if used[j]:
                    u[p[j]] += delta
                    v[j] -= delta
                else:
                    min_v[j] -= delta
            j0 = j1
            if p[j0] == 0:
                break
        while j0:
            p[j0] = p[way[j0]]
            j0 = way[j0]

    assignment = [-1] * n
    for j in range(1, n + 1):
        if p[j] != 0:
            assignment[p[j] - 1] = j - 1
    return -v[0], assignment


# ---------------------------------------------------------------------------
# 4. Gale-Shapley stable matching  O(n^2)
# ---------------------------------------------------------------------------

def gale_shapley(men_prefs, women_prefs):
    """Deferred-acceptance stable matching. O(n^2).

    Args:
        men_prefs:   dict  man -> list of women in preference order
        women_prefs: dict  woman -> list of men in preference order

    Returns:
        dict  man -> matched woman (proposer-optimal stable matching)
    """
    n = len(men_prefs)
    women_rank = {}
    for w, prefs in women_prefs.items():
        women_rank[w] = {m: rank for rank, m in enumerate(prefs)}

    next_proposal = {m: 0 for m in men_prefs}
    engaged = {}  # woman -> man
    free = deque(men_prefs.keys())

    while free:
        m = free.popleft()
        w = men_prefs[m][next_proposal[m]]
        next_proposal[m] += 1
        if w not in engaged:
            engaged[w] = m
        else:
            m2 = engaged[w]
            if women_rank[w][m] < women_rank[w][m2]:
                engaged[w] = m
                free.append(m2)
            else:
                free.append(m)

    return {engaged[w]: w for w in engaged}


# ---------------------------------------------------------------------------
# Demo
# ---------------------------------------------------------------------------

def main():
    # --- Bipartite graph example ---
    #
    #  L = {A, B, C, D}    R = {1, 2, 3}
    #  A: 1, 2
    #  B: 1, 3
    #  C: 2
    #  D: 2, 3
    #
    graph = {
        "A": ["1", "2"],
        "B": ["1", "3"],
        "C": ["2"],
        "D": ["2", "3"],
    }
    left = ["A", "B", "C", "D"]
    right = ["1", "2", "3"]

    print("=" * 60)
    print("1. Augmenting-path bipartite matching (baseline)")
    size1, match1 = bipartite_max_matching(graph, left, right)
    print(f"   Matching size: {size1}")
    print(f"   Edges: {match1}")

    print()
    print("2. Hopcroft-Karp O(E√V)")
    size2, pair_u = hopcroft_karp(graph, left, right)
    print(f"   Matching size: {size2}")
    print(f"   Pairs: { {u: v for u, v in pair_u.items() if v is not None} }")

    assert size1 == size2, "Both algorithms must agree on maximum matching size"

    # --- Hungarian (assignment problem) ---
    print()
    print("3. Hungarian algorithm — minimum-weight assignment")
    cost_matrix = [
        [4, 1, 3],
        [2, 0, 5],
        [3, 2, 2],
    ]
    min_cost, assignment = hungarian(cost_matrix)
    print(f"   Cost matrix: {cost_matrix}")
    print(f"   Min cost:    {min_cost}")
    print(f"   Assignment:  {assignment}")

    # --- Gale-Shapley stable matching ---
    print()
    print("4. Gale-Shapley stable matching")
    men_prefs = {
        "m1": ["w1", "w2", "w3"],
        "m2": ["w2", "w1", "w3"],
        "m3": ["w1", "w2", "w3"],
    }
    women_prefs = {
        "w1": ["m2", "m1", "m3"],
        "w2": ["m1", "m2", "m3"],
        "w3": ["m1", "m2", "m3"],
    }
    stable = gale_shapley(men_prefs, women_prefs)
    print(f"   Stable matching: {stable}")

    # Verify stability
    women_rank = {w: {m: r for r, m in enumerate(p)} for w, p in women_prefs.items()}
    men_rank = {m: {w: r for r, w in enumerate(p)} for m, p in men_prefs.items()}
    for m, w in stable.items():
        for other_w in men_prefs[m]:
            if men_rank[m][other_w] < men_rank[m][w]:
                other_m = [mm for mm, ww in stable.items() if ww == other_w][0]
                assert women_rank[other_w][m] >= women_rank[other_w][other_m], \
                    f"Blocking pair: ({m}, {other_w})"
    print("   Stability verified — no blocking pairs.")

    print()
    print("All demonstrations passed.")


if __name__ == "__main__":
    main()
