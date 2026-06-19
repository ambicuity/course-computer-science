"""
Minimum Spanning Trees — Prim, Kruskal, Borůvka
Phase 04 — Algorithms & Complexity Analysis

Three MST algorithms built from scratch, sharing a common Union-Find core.
All return (edge_list, total_weight) for uniform comparison.
"""

import heapq


# ---------------------------------------------------------------------------
# Union-Find with path compression + union by rank
# ---------------------------------------------------------------------------

class UnionFind:
    def __init__(self, n: int):
        self.parent = list(range(n))
        self.rank = [0] * n

    def find(self, x: int) -> int:
        while self.parent[x] != x:
            self.parent[x] = self.parent[self.parent[x]]  # path splitting
            x = self.parent[x]
        return x

    def union(self, x: int, y: int) -> bool:
        rx, ry = self.find(x), self.find(y)
        if rx == ry:
            return False
        if self.rank[rx] < self.rank[ry]:
            rx, ry = ry, rx
        self.parent[ry] = rx
        if self.rank[rx] == self.rank[ry]:
            self.rank[rx] += 1
        return True


# ---------------------------------------------------------------------------
# Kruskal's — sort edges, greedily add lightest non-cycle edge
# ---------------------------------------------------------------------------

def kruskal(edges: list[tuple[int, int, int]], v: int) -> tuple[list[tuple[int, int, int]], int]:
    """Return MST edges and total weight via Kruskal's algorithm.

    Args:
        edges: list of (u, v, weight) undirected edges.
        v: number of vertices (0-indexed).
    Returns:
        (mst_edges, total_weight)
    """
    sorted_edges = sorted(edges, key=lambda e: e[2])
    uf = UnionFind(v)
    mst: list[tuple[int, int, int]] = []
    total = 0
    for u, vtx, w in sorted_edges:
        if uf.union(u, vtx):
            mst.append((u, vtx, w))
            total += w
            if len(mst) == v - 1:
                break
    return mst, total


# ---------------------------------------------------------------------------
# Prim's — grow MST from vertex 0 using a min-heap
# ---------------------------------------------------------------------------

def prim(graph: list[list[tuple[int, int]]], v: int) -> tuple[list[tuple[int, int, int]], int]:
    """Return MST edges and total weight via Prim's algorithm.

    Args:
        graph: adjacency list where graph[u] = [(v, weight), ...].
        v: number of vertices.
    Returns:
        (mst_edges, total_weight)
    """
    visited = [False] * v
    heap: list[tuple[int, int, int]] = []
    mst: list[tuple[int, int, int]] = []
    total = 0

    visited[0] = True
    for neighbor, w in graph[0]:
        heapq.heappush(heap, (w, 0, neighbor))

    while heap and len(mst) < v - 1:
        w, u, vtx = heapq.heappop(heap)
        if visited[vtx]:
            continue
        visited[vtx] = True
        mst.append((u, vtx, w))
        total += w
        for nb, nw in graph[vtx]:
            if not visited[nb]:
                heapq.heappush(heap, (nw, vtx, nb))

    return mst, total


# ---------------------------------------------------------------------------
# Borůvka's — each component finds its lightest outgoing edge per phase
# ---------------------------------------------------------------------------

def boruvka(edges: list[tuple[int, int, int]], v: int) -> tuple[list[tuple[int, int, int]], int]:
    """Return MST edges and total weight via Borůvka's algorithm.

    Args:
        edges: list of (u, v, weight) undirected edges.
        v: number of vertices.
    Returns:
        (mst_edges, total_weight)
    """
    uf = UnionFind(v)
    mst: list[tuple[int, int, int]] = []
    total = 0
    num_components = v

    while num_components > 1:
        cheapest = [-1] * v  # cheapest[comp_root] = index of lightest outgoing edge
        for i, (u, vtx, w) in enumerate(edges):
            ru, rv = uf.find(u), uf.find(vtx)
            if ru == rv:
                continue
            if cheapest[ru] == -1 or edges[cheapest[ru]][2] > w:
                cheapest[ru] = i
            if cheapest[rv] == -1 or edges[cheapest[rv]][2] > w:
                cheapest[rv] = i

        merged_any = False
        for i in range(v):
            ci = cheapest[i]
            if ci == -1:
                continue
            u, vtx, w = edges[ci]
            if uf.union(u, vtx):
                mst.append((u, vtx, w))
                total += w
                num_components -= 1
                merged_any = True

        if not merged_any:
            break  # disconnected graph

    return mst, total


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def edges_to_adjacency(edges: list[tuple[int, int, int]], v: int) -> list[list[tuple[int, int]]]:
    """Convert edge list to adjacency list."""
    graph: list[list[tuple[int, int]]] = [[] for _ in range(v)]
    for u, vtx, w in edges:
        graph[u].append((vtx, w))
        graph[vtx].append((u, w))
    return graph


# ---------------------------------------------------------------------------
# Demo
# ---------------------------------------------------------------------------

def main() -> None:
    # 6-vertex example graph
    #       0
    #      / \
    #    (1) (3)
    #    /     \
    #   1---2   4
    #    \ / \ /
    #    (4)(2)
    #     |  |
    #     5--+
    #     (5)
    raw_edges = [
        (0, 1, 1), (0, 4, 3),
        (1, 2, 2), (1, 3, 4), (1, 5, 6),
        (2, 3, 5), (2, 5, 2),
        (3, 4, 7), (3, 5, 3),
        (4, 5, 5),
    ]
    V = 6

    print("=== Kruskal's ===")
    mst_k, w_k = kruskal(raw_edges, V)
    for u, v, wt in mst_k:
        print(f"  {u} -- {v}  weight {wt}")
    print(f"  Total weight: {w_k}\n")

    adj = edges_to_adjacency(raw_edges, V)

    print("=== Prim's ===")
    mst_p, w_p = prim(adj, V)
    for u, v, wt in mst_p:
        print(f"  {u} -- {v}  weight {wt}")
    print(f"  Total weight: {w_p}\n")

    print("=== Borůvka's ===")
    mst_b, w_b = boruvka(raw_edges, V)
    for u, v, wt in mst_b:
        print(f"  {u} -- {v}  weight {wt}")
    print(f"  Total weight: {w_b}\n")

    # Verification: all three must agree on total weight
    assert w_k == w_p == w_b, f"Weight mismatch: kruskal={w_k} prim={w_p} boruvka={w_b}"
    print(f"All three algorithms agree: MST weight = {w_k}")

    # Second-best MST (exercise 2)
    print("\n=== Second-Best MST ===")
    second_best = float("inf")
    mst_edges_set = set()
    for u, v, w in mst_k:
        mst_edges_set.add((min(u, v), max(u, v), w))

    for rem_u, rem_v, rem_w in mst_k:
        # Rebuild edge list excluding exactly this MST edge
        key = (min(rem_u, rem_v), max(rem_u, rem_v), rem_w)
        remaining = [e for e in raw_edges
                     if (min(e[0], e[1]), max(e[0], e[1]), e[2]) != key]
        mst2, w2 = kruskal(remaining, V)
        if len(mst2) == V - 1:
            second_best = min(second_best, w2)

    print(f"  Best MST weight:     {w_k}")
    print(f"  Second-best weight:  {second_best}")


if __name__ == "__main__":
    main()
