"""markov.py — discrete Markov chains: stationary distribution + PageRank.

Pure Python. Used downstream by Phase 11 (PageRank, distributed gossip),
Phase 17 (MCMC).
"""
from __future__ import annotations

from typing import Dict, List


def vec_mul(pi: List[float], P: List[List[float]]) -> List[float]:
    n = len(pi)
    out = [0.0] * len(P[0])
    for j in range(len(P[0])):
        s = 0.0
        for i in range(n):
            s += pi[i] * P[i][j]
        out[j] = s
    return out


def stationary(P: List[List[float]], tol: float = 1e-12, max_iter: int = 10000) -> List[float]:
    n = len(P)
    pi = [1.0 / n] * n
    for _ in range(max_iter):
        new = vec_mul(pi, P)
        if max(abs(new[i] - pi[i]) for i in range(n)) < tol:
            return new
        pi = new
    return pi


def pagerank(adj: Dict[int, List[int]], damping: float = 0.85, tol: float = 1e-10) -> List[float]:
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
    for i in range(n):
        for j in range(n):
            P[i][j] = damping * P[i][j] + (1 - damping) / n
    return stationary(P, tol=tol)


if __name__ == "__main__":
    P = [[0.9, 0.1], [0.5, 0.5]]
    pi = stationary(P)
    assert abs(pi[0] - 5/6) < 1e-4
    assert abs(pi[1] - 1/6) < 1e-4
    adj = {0: [1, 2], 1: [2], 2: [0], 3: [2]}
    pr = pagerank(adj)
    assert pr[2] > pr[0]   # C has highest rank
    print(f"markov library smoke-test OK; π_weather = ({pi[0]:.4f}, {pi[1]:.4f})")
