"""matching.py — bipartite max-matching via Kuhn's augmenting-path algorithm.

Used in Phase 04 L18 (bipartite matching family) and Phase 16 (scheduling).
"""
from __future__ import annotations

from typing import Dict, List


def bipartite_matching(adj: Dict, left: List) -> Dict:
    """Return right→left max matching. adj[u] (for u in left) lists u's right-neighbors."""
    match: Dict = {}

    def try_kuhn(u, seen) -> bool:
        for v in adj.get(u, []):
            if v in seen: continue
            seen.add(v)
            if v not in match or try_kuhn(match[v], seen):
                match[v] = u
                return True
        return False

    for u in left:
        try_kuhn(u, set())
    return match


if __name__ == "__main__":
    prefs = {
        "Alice":  ["P1", "P2"],
        "Bob":    ["P1"],
        "Carol":  ["P2", "P3"],
        "Dan":    ["P3", "P4"],
    }
    m = bipartite_matching(prefs, list(prefs))
    assert len(m) == 4
    print(f"matching library smoke-test OK; match = {m}")
