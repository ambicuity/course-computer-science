"""
Graph Algorithms I — BFS, DFS, Topo, SCC
Phase 04 — Algorithms & Complexity Analysis

From-scratch implementations: Graph class, BFS, DFS, topological sort
(Kahn's + DFS-based), Tarjan SCC, Kosaraju SCC, bipartite check, cycle detection.
"""

from collections import deque


# ---------------------------------------------------------------------------
# Graph
# ---------------------------------------------------------------------------

class Graph:
    """Adjacency-list graph supporting directed and undirected modes."""

    def __init__(self, n: int, directed: bool = False) -> None:
        self.n = n
        self.directed = directed
        self.adj: list[list[int]] = [[] for _ in range(n)]

    def add_edge(self, u: int, v: int) -> None:
        self.adj[u].append(v)
        if not self.directed:
            self.adj[v].append(u)

    def neighbors(self, u: int) -> list[int]:
        return self.adj[u]

    def reverse(self) -> "Graph":
        """Return the transpose graph (all edges reversed)."""
        gt = Graph(self.n, directed=True)
        for u in range(self.n):
            for v in self.adj[u]:
                gt.adj[v].append(u)
        return gt

    def __repr__(self) -> str:
        kind = "directed" if self.directed else "undirected"
        edges = sum(len(a) for a in self.adj) if self.directed else sum(len(a) for a in self.adj) // 2
        return f"Graph({self.n} vertices, {edges} edges, {kind})"


# ---------------------------------------------------------------------------
# BFS — parent map + distances
# ---------------------------------------------------------------------------

def bfs(graph: Graph, start: int) -> tuple[list[int], list[int]]:
    """BFS from start. Returns (parent, dist) arrays. Unreachable vertices have dist=-1."""
    dist = [-1] * graph.n
    parent = [-1] * graph.n
    dist[start] = 0
    q: deque[int] = deque([start])
    while q:
        u = q.popleft()
        for v in graph.adj[u]:
            if dist[v] == -1:
                dist[v] = dist[u] + 1
                parent[v] = u
                q.append(v)
    return parent, dist


# ---------------------------------------------------------------------------
# DFS — discovery / finish times (iterative)
# ---------------------------------------------------------------------------

def dfs(graph: Graph, start: int) -> tuple[list[int], list[int], list[int]]:
    """DFS from start. Returns (parent, disc, fin) arrays. Unreachable: disc=-1."""
    WHITE, GRAY, BLACK = 0, 1, 2
    color = [WHITE] * graph.n
    parent = [-1] * graph.n
    disc = [-1] * graph.n
    fin = [-1] * graph.n
    time = 0
    stack: list[tuple[int, bool]] = [(start, False)]  # (vertex, finished_flag)
    color[start] = GRAY
    disc[start] = time
    time += 1
    while stack:
        u, finished = stack.pop()
        if finished:
            fin[u] = time
            time += 1
            color[u] = BLACK
            continue
        # re-push with finished marker
        stack.append((u, True))
        for v in graph.adj[u]:
            if color[v] == WHITE:
                color[v] = GRAY
                parent[v] = u
                disc[v] = time
                time += 1
                stack.append((v, False))
    return parent, disc, fin


# ---------------------------------------------------------------------------
# Connected components (undirected)
# ---------------------------------------------------------------------------

def connected_components(graph: Graph) -> list[list[int]]:
    """Return list of connected components (vertex lists) for an undirected graph."""
    visited = [False] * graph.n
    components: list[list[int]] = []
    for s in range(graph.n):
        if visited[s]:
            continue
        comp: list[int] = []
        q: deque[int] = deque([s])
        visited[s] = True
        while q:
            u = q.popleft()
            comp.append(u)
            for v in graph.adj[u]:
                if not visited[v]:
                    visited[v] = True
                    q.append(v)
        components.append(comp)
    return components


# ---------------------------------------------------------------------------
# Bipartite check — BFS 2-coloring
# ---------------------------------------------------------------------------

def is_bipartite(graph: Graph) -> tuple[bool, list[int] | None]:
    """Check if graph is bipartite using BFS 2-coloring.

    Returns (True, color_array) where color[v] is 0 or 1,
    or (False, None) if not bipartite.
    """
    color = [-1] * graph.n
    for s in range(graph.n):
        if color[s] != -1:
            continue
        color[s] = 0
        q: deque[int] = deque([s])
        while q:
            u = q.popleft()
            for v in graph.adj[u]:
                if color[v] == -1:
                    color[v] = 1 - color[u]
                    q.append(v)
                elif color[v] == color[u]:
                    return False, None
    return True, color


# ---------------------------------------------------------------------------
# Cycle detection (directed) — DFS-based
# ---------------------------------------------------------------------------

def has_cycle_directed(graph: Graph) -> bool:
    """Return True if the directed graph contains a cycle (back edge in DFS)."""
    WHITE, GRAY, BLACK = 0, 1, 2
    color = [WHITE] * graph.n

    def dfs_visit(u: int) -> bool:
        color[u] = GRAY
        for v in graph.adj[u]:
            if color[v] == GRAY:
                return True
            if color[v] == WHITE and dfs_visit(v):
                return True
        color[u] = BLACK
        return False

    for u in range(graph.n):
        if color[u] == WHITE:
            if dfs_visit(u):
                return True
    return False


# ---------------------------------------------------------------------------
# Topological sort — Kahn's (BFS)
# ---------------------------------------------------------------------------

def topological_sort_kahn(graph: Graph) -> list[int]:
    """Kahn's BFS-based topological sort. Raises ValueError if graph has a cycle."""
    in_deg = [0] * graph.n
    for u in range(graph.n):
        for v in graph.adj[u]:
            in_deg[v] += 1
    q: deque[int] = deque(u for u in range(graph.n) if in_deg[u] == 0)
    order: list[int] = []
    while q:
        u = q.popleft()
        order.append(u)
        for v in graph.adj[u]:
            in_deg[v] -= 1
            if in_deg[v] == 0:
                q.append(v)
    if len(order) != graph.n:
        raise ValueError("Graph has a cycle — topological sort impossible")
    return order


# ---------------------------------------------------------------------------
# Topological sort — DFS reverse-postorder
# ---------------------------------------------------------------------------

def topological_sort_dfs(graph: Graph) -> list[int]:
    """DFS-based topological sort (reverse postorder). Raises ValueError if cycle."""
    WHITE, GRAY, BLACK = 0, 1, 2
    color = [WHITE] * graph.n
    order: list[int] = []

    def dfs_visit(u: int) -> None:
        color[u] = GRAY
        for v in graph.adj[u]:
            if color[v] == GRAY:
                raise ValueError("Graph has a cycle — topological sort impossible")
            if color[v] == WHITE:
                dfs_visit(v)
        color[u] = BLACK
        order.append(u)

    for u in range(graph.n):
        if color[u] == WHITE:
            dfs_visit(u)
    order.reverse()
    return order


# ---------------------------------------------------------------------------
# Tarjan's SCC — single DFS pass
# ---------------------------------------------------------------------------

def tarjan_scc(graph: Graph) -> list[list[int]]:
    """Find all SCCs using Tarjan's algorithm. Returns list of component lists."""
    index_counter = [0]
    index = [-1] * graph.n
    low = [0] * graph.n
    on_stack = [False] * graph.n
    stack: list[int] = []
    sccs: list[list[int]] = []

    def strongconnect(v: int) -> None:
        index[v] = index_counter[0]
        low[v] = index_counter[0]
        index_counter[0] += 1
        stack.append(v)
        on_stack[v] = True

        for w in graph.adj[v]:
            if index[w] == -1:
                strongconnect(w)
                low[v] = min(low[v], low[w])
            elif on_stack[w]:
                low[v] = min(low[v], index[w])

        if low[v] == index[v]:
            scc: list[int] = []
            while True:
                w = stack.pop()
                on_stack[w] = False
                scc.append(w)
                if w == v:
                    break
            sccs.append(scc)

    for v in range(graph.n):
        if index[v] == -1:
            strongconnect(v)
    return sccs


# ---------------------------------------------------------------------------
# Kosaraju's SCC — two DFS passes
# ---------------------------------------------------------------------------

def kosaraju_scc(graph: Graph) -> list[list[int]]:
    """Find all SCCs using Kosaraju's algorithm (two DFS passes on transpose)."""
    visited = [False] * graph.n
    finish_order: list[int] = []

    def dfs1(u: int) -> None:
        visited[u] = True
        for v in graph.adj[u]:
            if not visited[v]:
                dfs1(v)
        finish_order.append(u)

    for u in range(graph.n):
        if not visited[u]:
            dfs1(u)

    gt = graph.reverse()
    visited = [False] * graph.n
    sccs: list[list[int]] = []

    for u in reversed(finish_order):
        if not visited[u]:
            scc: list[int] = []
            stack = [u]
            visited[u] = True
            while stack:
                v = stack.pop()
                scc.append(v)
                for w in gt.adj[v]:
                    if not visited[w]:
                        visited[w] = True
                        stack.append(w)
            sccs.append(scc)
    return sccs


# ---------------------------------------------------------------------------
# Demos
# ---------------------------------------------------------------------------

def main() -> None:
    print("=== Graph Algorithms I — BFS, DFS, Topo, SCC ===\n")

    # --- Build sample graphs ---
    # Undirected graph for BFS/DFS/bipartite
    ug = Graph(7, directed=False)
    for u, v in [(0, 1), (0, 2), (1, 3), (1, 4), (2, 5), (2, 6)]:
        ug.add_edge(u, v)

    # --- BFS ---
    print("--- BFS from node 0 ---")
    parent, dist = bfs(ug, 0)
    print(f"  Distances: {dist}")
    print(f"  Parents:   {parent}")
    # Reconstruct path to node 4
    path = []
    v = 4
    while v != -1:
        path.append(v)
        v = parent[v]
    path.reverse()
    print(f"  Shortest path 0→4: {path}")
    print()

    # --- DFS ---
    print("--- DFS from node 0 ---")
    dg = Graph(6, directed=True)
    for u, v in [(0, 1), (0, 2), (1, 3), (2, 3), (3, 4), (4, 5)]:
        dg.add_edge(u, v)
    _, disc, fin = dfs(dg, 0)
    print(f"  Discovery: {disc}")
    print(f"  Finish:    {fin}")
    print()

    # --- Connected components ---
    print("--- Connected components ---")
    cg = Graph(8, directed=False)
    for u, v in [(0, 1), (1, 2), (3, 4), (4, 5), (6, 7)]:
        cg.add_edge(u, v)
    comps = connected_components(cg)
    for i, c in enumerate(comps):
        print(f"  Component {i}: {sorted(c)}")
    print()

    # --- Bipartite check ---
    print("--- Bipartite check ---")
    ok, colors = is_bipartite(ug)
    print(f"  Tree (bipartite): {ok}, colors={colors}")
    odd_cycle = Graph(3, directed=False)
    odd_cycle.add_edge(0, 1)
    odd_cycle.add_edge(1, 2)
    odd_cycle.add_edge(2, 0)
    ok2, _ = is_bipartite(odd_cycle)
    print(f"  Triangle (not bipartite): {ok2}")
    print()

    # --- Cycle detection ---
    print("--- Cycle detection (directed) ---")
    dag = Graph(4, directed=True)
    dag.add_edge(0, 1)
    dag.add_edge(1, 2)
    dag.add_edge(2, 3)
    print(f"  DAG has cycle: {has_cycle_directed(dag)}")
    cyclic = Graph(4, directed=True)
    cyclic.add_edge(0, 1)
    cyclic.add_edge(1, 2)
    cyclic.add_edge(2, 0)
    cyclic.add_edge(2, 3)
    print(f"  Cyclic graph has cycle: {has_cycle_directed(cyclic)}")
    print()

    # --- Topological sort ---
    print("--- Topological sort ---")
    ts = Graph(6, directed=True)
    for u, v in [(5, 2), (5, 0), (4, 0), (4, 1), (2, 3), (3, 1)]:
        ts.add_edge(u, v)
    order_kahn = topological_sort_kahn(ts)
    order_dfs = topological_sort_dfs(ts)
    print(f"  Kahn's:     {order_kahn}")
    print(f"  DFS-based:  {order_dfs}")
    print()

    # --- SCC ---
    print("--- Strongly Connected Components ---")
    sg = Graph(8, directed=True)
    for u, v in [(0, 1), (1, 2), (2, 0), (2, 3), (3, 4), (4, 5), (5, 3), (6, 5), (6, 7), (7, 6)]:
        sg.add_edge(u, v)
    tarjan_result = tarjan_scc(sg)
    kosaraju_result = kosaraju_scc(sg)
    print(f"  Tarjan:   {[sorted(s) for s in tarjan_result]}")
    print(f"  Kosaraju: {[sorted(s) for s in kosaraju_result]}")
    print()

    # --- Larger example: dependency graph ---
    print("--- Build order (topo sort on module deps) ---")
    modules = Graph(5, directed=True)
    modules.add_edge(0, 2)  # core → utils
    modules.add_edge(1, 2)  # api → utils
    modules.add_edge(2, 3)  # utils → db
    modules.add_edge(2, 4)  # utils → cache
    modules.add_edge(3, 4)  # db → cache
    names = ["core", "api", "utils", "db", "cache"]
    order = topological_sort_kahn(modules)
    print(f"  Build order: {[names[i] for i in order]}")


if __name__ == "__main__":
    main()
