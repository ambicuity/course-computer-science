"""Discrete Markov chains: power iteration, stationary distribution, PageRank.

Pure-Python (no numpy) so the demo runs in any environment.

Run:  python3 main.py
"""
from __future__ import annotations

from typing import Dict, List


# ── Matrix helpers (no numpy) ─────────────────────────────────────

def mat_mul(A: List[List[float]], B: List[List[float]]) -> List[List[float]]:
    n, k, m = len(A), len(A[0]), len(B[0])
    out = [[0.0] * m for _ in range(n)]
    for i in range(n):
        for r in range(k):
            air = A[i][r]
            if air == 0: continue
            for j in range(m):
                out[i][j] += air * B[r][j]
    return out


def mat_pow(P: List[List[float]], n: int) -> List[List[float]]:
    size = len(P)
    result = [[1.0 if i == j else 0.0 for j in range(size)] for i in range(size)]
    base = [row[:] for row in P]
    while n > 0:
        if n & 1:
            result = mat_mul(result, base)
        base = mat_mul(base, base)
        n >>= 1
    return result


def vec_mul(pi: List[float], P: List[List[float]]) -> List[float]:
    n = len(pi)
    out = [0.0] * len(P[0])
    for j in range(len(P[0])):
        s = 0.0
        for i in range(n):
            s += pi[i] * P[i][j]
        out[j] = s
    return out


# ── Stationary distribution by power iteration ────────────────────

def stationary(P: List[List[float]], tol: float = 1e-12, max_iter: int = 10000) -> List[float]:
    n = len(P)
    pi = [1.0 / n] * n
    for _ in range(max_iter):
        new = vec_mul(pi, P)
        if max(abs(new[i] - pi[i]) for i in range(n)) < tol:
            return new
        pi = new
    return pi


# ── PageRank ──────────────────────────────────────────────────────

def pagerank(adj: Dict[int, List[int]], damping: float = 0.85, tol: float = 1e-10) -> List[float]:
    """Compute PageRank for nodes 0..n-1. adj[u] = list of u's out-neighbors."""
    n = len(adj)
    P = [[0.0] * n for _ in range(n)]
    for u in adj:
        out = adj[u]
        if not out:
            for j in range(n):
                P[u][j] = 1.0 / n
        else:
            for v in out:
                P[u][v] += 1.0 / len(out)
    # Apply damping
    for i in range(n):
        for j in range(n):
            P[i][j] = damping * P[i][j] + (1 - damping) / n
    return stationary(P, tol=tol)


# ── Demo ──────────────────────────────────────────────────────────

def fmt(v): return "[" + ", ".join(f"{x:.4f}" for x in v) + "]"


def demo_weather():
    print("== Weather chain: P = [[0.9, 0.1], [0.5, 0.5]] ==")
    P = [[0.9, 0.1], [0.5, 0.5]]
    for n in [1, 2, 10, 100]:
        Pn = mat_pow(P, n)
        print(f"  P^{n:<3d}:  row 0 = {fmt(Pn[0])}    row 1 = {fmt(Pn[1])}")
    pi = stationary(P)
    print(f"  Stationary π (power iteration): {fmt(pi)}")
    print(f"  Expected:  π_sunny = 5/6 ≈ 0.8333,  π_rainy = 1/6 ≈ 0.1667")


def demo_walk_on_graph():
    print("\n== Random walk on a 4-cycle (lazy version to avoid period 2) ==")
    n = 4
    P = [[0.0] * n for _ in range(n)]
    for i in range(n):
        P[i][(i - 1) % n] = 0.5
        P[i][(i + 1) % n] = 0.5
    lazy = [[0.0] * n for _ in range(n)]
    for i in range(n):
        lazy[i][i] = 0.5
        for j in range(n):
            lazy[i][j] += 0.5 * P[i][j]
    pi = stationary(lazy)
    print(f"  Lazy-walk stationary π: {fmt(pi)}    (uniform, since all degrees = 2)")


def demo_pagerank():
    print("\n== PageRank on a tiny 4-page web ==")
    # A → B, C ;  B → C ;  C → A ;  D → C
    adj = {
        0: [1, 2],
        1: [2],
        2: [0],
        3: [2],
    }
    pr = pagerank(adj, damping=0.85)
    print(f"  PageRank: A={pr[0]:.4f}, B={pr[1]:.4f}, C={pr[2]:.4f}, D={pr[3]:.4f}")
    print(f"  C should be highest (3 inbound links).")
    assert pr[2] > pr[0] and pr[2] > pr[1] and pr[2] > pr[3]


def demo_mixing_comparison():
    print("\n== Mixing-time comparison: 6-cycle vs K_6 (lazy walks) ==")
    for name, build_adj in [
        ("6-cycle", lambda: [[(i - 1) % 6, (i + 1) % 6] for i in range(6)]),
        ("K_6",     lambda: [[j for j in range(6) if j != i] for i in range(6)]),
    ]:
        n = 6
        adj = build_adj()
        P = [[0.0] * n for _ in range(n)]
        for i in range(n):
            P[i][i] = 0.5
            for v in adj[i]:
                P[i][v] += 0.5 / len(adj[i])
        pi = stationary(P)

        pi_t = [0.0] * n; pi_t[0] = 1.0
        steps_to_eps = None
        for step in range(1, 2000):
            pi_t = vec_mul(pi_t, P)
            dist = sum(abs(pi_t[i] - pi[i]) for i in range(n))
            if dist < 0.01 and steps_to_eps is None:
                steps_to_eps = step
                break
        print(f"  {name:8s}: π = {fmt(pi)}, steps to ε=0.01: {steps_to_eps}")


def main():
    demo_weather()
    demo_walk_on_graph()
    demo_pagerank()
    demo_mixing_comparison()


if __name__ == "__main__":
    main()
