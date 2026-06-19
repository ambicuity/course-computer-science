"""
DP III — Bitmask, Digit, Tree, DP on DAGs
Phase 04 — Algorithms & Complexity Analysis
"""

import math
from collections import deque
from functools import lru_cache


# ---------------------------------------------------------------------------
# 1. Bitmask DP — TSP
# ---------------------------------------------------------------------------

def tsp_bitmask(dist, n):
    """Shortest Hamiltonian tour using bitmask DP.  O(2^n · n^2)."""
    FULL = (1 << n) - 1
    INF = math.inf
    dp = [[INF] * n for _ in range(1 << n)]
    dp[1][0] = 0  # start at city 0

    for S in range(1, 1 << n):
        for u in range(n):
            if not (S >> u) & 1:
                continue
            if dp[S][u] == INF:
                continue
            for v in range(n):
                if (S >> v) & 1:
                    continue
                ns = S | (1 << v)
                dp[ns][v] = min(dp[ns][v], dp[S][u] + dist[u][v])

    ans = INF
    for u in range(n):
        ans = min(ans, dp[FULL][u] + dist[u][0])
    return ans


# ---------------------------------------------------------------------------
# 2. Digit DP — count numbers in [1, N] with no digit 4
# ---------------------------------------------------------------------------

def count_no_four(N):
    """Count integers in [1, N] whose decimal representation contains no '4'."""
    s = str(N)
    n = len(s)

    @lru_cache(maxsize=None)
    def dp(pos, tight, started):
        if pos == n:
            return 1 if started else 0
        limit = int(s[pos]) if tight else 9
        total = 0
        for d in range(0, limit + 1):
            if d == 4:
                continue
            total += dp(pos + 1, tight and d == limit, started or d != 0)
        return total

    return dp(0, True, False)


def count_no_adjacent_same(N):
    """Count integers in [1, N] with no two adjacent digits equal."""
    s = str(N)
    n = len(s)

    @lru_cache(maxsize=None)
    def dp(pos, tight, last, started):
        if pos == n:
            return 1 if started else 0
        limit = int(s[pos]) if tight else 9
        total = 0
        for d in range(0, limit + 1):
            if started and d == last:
                continue
            total += dp(
                pos + 1,
                tight and d == limit,
                d if started or d != 0 else -1,
                started or d != 0,
            )
        return total

    return dp(0, True, -1, False)


# ---------------------------------------------------------------------------
# 3. Tree DP — Maximum Independent Set + Rerooting
# ---------------------------------------------------------------------------

def tree_max_independent_set(tree, root=0):
    """MIS on tree.  tree: adjacency list.  Returns (dp0, dp1) arrays."""
    n = len(tree)
    dp0, dp1 = [0] * n, [0] * n

    def dfs(u, parent):
        dp1[u] = 1
        for v in tree[u]:
            if v == parent:
                continue
            dfs(v, u)
            dp0[u] += max(dp0[v], dp1[v])
            dp1[u] += dp0[v]

    dfs(root, -1)
    return dp0, dp1


def reroot_mis(tree):
    """Compute MIS size for every node as root.  O(n)."""
    n = len(tree)
    dp0, dp1 = [0] * n, [0] * n
    ans = [0] * n

    def dfs_down(u, parent):
        dp1[u] = 1
        for v in tree[u]:
            if v == parent:
                continue
            dfs_down(v, u)
            dp0[u] += max(dp0[v], dp1[v])
            dp1[u] += dp0[v]

    dfs_down(0, -1)

    def dfs_up(u, parent):
        ans[u] = max(dp0[u], dp1[u])
        for v in tree[u]:
            if v == parent:
                continue
            u0, u1, v0, v1 = dp0[u], dp1[u], dp0[v], dp1[v]
            dp0[u] -= max(dp0[v], dp1[v])
            dp1[u] -= dp0[v]
            dp0[v] += max(dp0[u], dp1[u])
            dp1[v] += dp0[u]
            dfs_up(v, u)
            dp0[u], dp1[u], dp0[v], dp1[v] = u0, u1, v0, v1

    dfs_up(0, -1)
    return ans


# ---------------------------------------------------------------------------
# 4. DP on DAG — Longest path
# ---------------------------------------------------------------------------

def dag_longest_path(adj, n):
    """adj: list of (u, v, weight).  Returns max path weight.  O(V+E)."""
    in_deg = [0] * n
    graph = [[] for _ in range(n)]
    for u, v, w in adj:
        graph[u].append((v, w))
        in_deg[v] += 1

    topo = deque(i for i in range(n) if in_deg[i] == 0)
    order = []
    while topo:
        u = topo.popleft()
        order.append(u)
        for v, _ in graph[u]:
            in_deg[v] -= 1
            if in_deg[v] == 0:
                topo.append(v)

    dp = [0] * n
    for u in order:
        for v, w in graph[u]:
            dp[v] = max(dp[v], dp[u] + w)
    return max(dp) if dp else 0


# ---------------------------------------------------------------------------
# Demos
# ---------------------------------------------------------------------------

def main():
    # --- Bitmask DP: TSP ---
    dist = [
        [0, 10, 15, 20],
        [10, 0, 35, 25],
        [15, 35, 0, 30],
        [20, 25, 30, 0],
    ]
    print(f"TSP min tour (4 cities): {tsp_bitmask(dist, 4)}")  # 80

    # --- Digit DP ---
    print(f"Numbers in [1,100] with no '4': {count_no_four(100)}")       # 81
    print(f"Numbers in [1,100] no adj same: {count_no_adjacent_same(100)}")  # 90

    # --- Tree DP ---
    #       0
    #      / \
    #     1   2
    #    / \
    #   3   4
    tree = [[1, 2], [0, 3, 4], [0], [1], [1]]
    dp0, dp1 = tree_max_independent_set(tree, 0)
    print(f"MIS (root=0): {max(dp0[0], dp1[0])}")  # 3  {0,3,4} or {2,3,4}
    print(f"Rerooting MIS: {reroot_mis(tree)}")     # [3, 3, 3, 3, 3]

    # --- DAG longest path ---
    adj = [(0, 1, 3), (0, 2, 2), (1, 3, 2), (2, 3, 4), (3, 4, 1)]
    print(f"DAG longest path: {dag_longest_path(adj, 5)}")  # 7


if __name__ == "__main__":
    main()
