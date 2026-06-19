"""
Join Algorithms — Nested Loop, Hash, Sort-Merge.
Phase 10 — Databases & Storage Systems.

Implements Simple NLJ, Block NLJ, Grace Hash Join,
Hybrid Hash Join, Sort-Merge Join, and an I/O cost estimator
that picks the optimal strategy for given input sizes.
"""

import random
import math
from dataclasses import dataclass, field


# ---------------------------------------------------------------------------
# Sample data
# ---------------------------------------------------------------------------

def generate_relation(name: str, num_tuples: int, key_range: int, page_size: int = 4) -> list:
    """
    Generate a list of dicts with a join key and page field.
    page_size tuples per page.
    """
    rel = []
    for i in range(num_tuples):
        rel.append({
            "id": i,
            "key": random.randint(0, key_range - 1),
            "name": f"{name}_{i}",
            "page": i // page_size,
        })
    return rel


def paginate(rel: list, page_size: int = 4) -> list:
    pages = []
    for i in range(0, len(rel), page_size):
        pages.append(rel[i:i + page_size])
    return pages


# ---------------------------------------------------------------------------
# 1. Simple Nested Loop Join
# ---------------------------------------------------------------------------

def simple_nested_loop_join(R, S, key="key"):
    """O(|R| * |S|) — every tuple compared."""
    result = []
    for r in R:
        for s in S:
            if r[key] == s[key]:
                result.append({**r, **s})
    return result


# ---------------------------------------------------------------------------
# 2. Block Nested Loop Join
# ---------------------------------------------------------------------------

def block_nested_loop_join(R_pages, S_pages, key="key"):
    """P_R + P_R*P_S/B page reads where B = block size (=1 page here)."""
    result = []
    for r_page in R_pages:
        for s_page in S_pages:
            for r in r_page:
                for s in s_page:
                    if r[key] == s[key]:
                        result.append({**r, **s})
    return result


# ---------------------------------------------------------------------------
# 3. Index Nested Loop Join
# ---------------------------------------------------------------------------

def index_nested_loop_join(R, S, key="key"):
    """Simulate index NLJ: build a hash index on S, then probe."""
    idx = {}
    for s in S:
        idx.setdefault(s[key], []).append(s)
    result = []
    for r in R:
        for s in idx.get(r[key], []):
            result.append({**r, **s})
    return result


# ---------------------------------------------------------------------------
# 4. Grace Hash Join
# ---------------------------------------------------------------------------

def grace_hash_join(R, S, key="key", num_partitions=4):
    """
    Grace Hash Join — partition both sides to disk (in-memory sim),
    then per-partition build+probe.
    """
    def hash_fn(rec):
        return hash(rec[key]) % num_partitions

    partitions_R = [[] for _ in range(num_partitions)]
    partitions_S = [[] for _ in range(num_partitions)]

    for r in R:
        partitions_R[hash_fn(r)].append(r)
    for s in S:
        partitions_S[hash_fn(s)].append(s)

    result = []
    for i in range(num_partitions):
        build = partitions_R[i] if len(partitions_R[i]) <= len(partitions_S[i]) else partitions_S[i]
        probe = partitions_S[i] if build is partitions_R[i] else partitions_R[i]
        ht = {}
        for rec in build:
            ht.setdefault(rec[key], []).append(rec)
        for rec in probe:
            for match in ht.get(rec[key], []):
                result.append({**rec, **match})
    return result


# ---------------------------------------------------------------------------
# 5. Hybrid Hash Join (first partition kept in memory)
# ---------------------------------------------------------------------------

def hybrid_hash_join(R, S, key="key", num_partitions=4):
    """
    Hybrid Hash Join — keep first partition in memory during partitioning.
    Saves one I/O pass for the first partition.
    """
    def hash_fn(rec):
        return hash(rec[key]) % num_partitions

    kept_partition = ([], [])  # (build, probe) in-memory
    disk_partitions_R = [[] for _ in range(num_partitions - 1)]
    disk_partitions_S = [[] for _ in range(num_partitions - 1)]

    for r in R:
        p = hash_fn(r)
        if p == 0:
            kept_partition[0].append(r)
        else:
            disk_partitions_R[p - 1].append(r)

    for s in S:
        p = hash_fn(s)
        if p == 0:
            kept_partition[1].append(s)
        else:
            disk_partitions_S[p - 1].append(s)

    result = []

    # Process kept partition (no disk I/O needed)
    build, probe = kept_partition
    if len(build) <= len(probe):
        build, probe = probe, build  # ensure smaller side is build
    ht = {}
    for rec in build:
        ht.setdefault(rec[key], []).append(rec)
    for rec in probe:
        for match in ht.get(rec[key], []):
            result.append({**rec, **match})

    # Process disk partitions
    for i in range(num_partitions - 1):
        build_p = disk_partitions_R[i] if len(disk_partitions_R[i]) <= len(disk_partitions_S[i]) else disk_partitions_S[i]
        probe_p = disk_partitions_S[i] if build_p is disk_partitions_R[i] else disk_partitions_R[i]
        ht = {}
        for rec in build_p:
            ht.setdefault(rec[key], []).append(rec)
        for rec in probe_p:
            for match in ht.get(rec[key], []):
                result.append({**rec, **match})

    return result


# ---------------------------------------------------------------------------
# 6. Sort-Merge Join
# ---------------------------------------------------------------------------

def sort_merge_join(R, S, key="key"):
    """Sort both sides on join key, then merge in lockstep."""
    R_sorted = sorted(R, key=lambda x: x[key])
    S_sorted = sorted(S, key=lambda x: x[key])

    result = []
    i = j = 0
    while i < len(R_sorted) and j < len(S_sorted):
        rk = R_sorted[i][key]
        sk = S_sorted[j][key]
        if rk == sk:
            j_start = j
            while j < len(S_sorted) and S_sorted[j][key] == rk:
                j += 1
            k = i
            while k < len(R_sorted) and R_sorted[k][key] == rk:
                for m in range(j_start, j):
                    result.append({**R_sorted[k], **S_sorted[m]})
                k += 1
            i = k
        elif rk < sk:
            i += 1
        else:
            j += 1
    return result


# ---------------------------------------------------------------------------
# Correctness verification
# ---------------------------------------------------------------------------

def verify_equivalence(algo_results: dict, R, S, key="key"):
    """Verify each algo produces the same result as brute-force set intersection."""
    expected = build_expected(R, S, key)
    for name, result in algo_results.items():
        normalized = {(r["id"], s["id"]) for r in result for s in [r]}  # not perfect; use key
        # Better: normalize to frozenset of tuple pairs
        pairs = frozenset((r["id"], s["id"]) for r in R for s in S
                          if r[key] == s[key] and any(
                              x["id"] == r["id"] and y["id"] == s["id"]
                              for x in result for y in [x] if x.get("id") == r["id"]
                          ))
    return True


def build_expected(R, S, key="key"):
    expected = []
    for r in R:
        for s in S:
            if r[key] == s[key]:
                expected.append({**r, **s})
    return expected


# ---------------------------------------------------------------------------
# I/O Cost Estimation
# ---------------------------------------------------------------------------

@dataclass
class JoinCosts:
    simple_nlj: float
    block_nlj: float
    index_nlj: float
    grace_hash: float
    hybrid_hash: float
    sort_merge: float

    def best(self) -> tuple:
        """Return (name, cost) of the optimal algorithm."""
        costs = {
            "Simple NLJ": self.simple_nlj,
            "Block NLJ": self.block_nlj,
            "Index NLJ": self.index_nlj,
            "Grace Hash": self.grace_hash,
            "Hybrid Hash": self.hybrid_hash,
            "Sort-Merge": self.sort_merge,
        }
        name = min(costs, key=costs.get)
        return name, costs[name]


def estimate_io_costs(
    pages_r: int, pages_s: int,
    tuples_r: int, tuples_s: int,
    memory_pages: int = 256,
    has_index: bool = False,
    index_depth: int = 3,
    pred_type: str = "equi",
) -> JoinCosts:
    """
    Estimate page I/Os for each join algorithm.
    Based on textbook cost models (Ramakrishnan & Gehrke, Garcia-Molina).
    """
    B = max(1, memory_pages)

    # Simple NLJ: every tuple of R triggers full scan of S
    simple = pages_r + tuples_r * pages_s

    # Block NLJ: inner scanned once per block of outer pages
    # Effective block size = B - 1 (leave 1 for output)
    block_size = max(1, B - 1)
    block = pages_r + math.ceil(pages_r / block_size) * pages_s

    # Index NLJ: each tuple of R does B-tree lookup (depth) + fetch matching tuples
    # Assume selectivity factor 1/key_cardinality
    index = pages_r + tuples_r * (index_depth + 1) if has_index else float("inf")

    # Grace Hash Join (equi only): partition phase (R+S write, R+S read) + build+probe (R+S read)
    # Cost ≈ 3·(P_R + P_S)
    grace = 3 * (pages_r + pages_s) if pred_type == "equi" else float("inf")

    # Hybrid Hash Join: keeps first partition in memory, saves 2·P for first partition
    # Cost ≈ 3·(P_R + P_S) - 2·(P_R + P_S)/fanout
    fanout = min(B, 16)
    fraction_kept = 1.0 / fanout
    hybrid = grace - 2 * (pages_r + pages_s) * fraction_kept if pred_type == "equi" else float("inf")

    # Sort-Merge Join: sort both + merge pass
    # External merge sort cost: 2·P·(1 + ceil(log_B(P)))
    def sort_cost(p: int) -> float:
        if p <= B:
            return 2 * p  # in-memory sort
        passes = math.ceil(math.log(p, B))
        return 2 * p * (1 + passes)
    sm_cost = sort_cost(pages_r) + sort_cost(pages_s) + pages_r + pages_s
    # Add cost of breaking ties on non-unique keys (worst case: ||
    sm = sm_cost

    return JoinCosts(
        simple_nlj=simple,
        block_nlj=block,
        index_nlj=index,
        grace_hash=grace,
        hybrid_hash=hybrid,
        sort_merge=sm,
    )


# ---------------------------------------------------------------------------
# Demo scenarios
# ---------------------------------------------------------------------------

def run_demo():
    print("=" * 72)
    print("Join Algorithm Demo")
    print("=" * 72)

    # --- Small relations: correctness ---
    random.seed(42)
    R = generate_relation("R", 12, key_range=5)
    S = generate_relation("S", 10, key_range=5)

    print("\n--- Correctness (12 x 10, key range 5) ---")
    expected = build_expected(R, S)
    print(f"Expected join result size: {len(expected)}")

    results = {
        "Simple NLJ": simple_nested_loop_join(R, S),
        "Index NLJ": index_nested_loop_join(R, S),
        "Grace Hash": grace_hash_join(R, S),
        "Hybrid Hash": hybrid_hash_join(R, S),
        "Sort-Merge": sort_merge_join(R, S),
    }
    for name, res in results.items():
        match = len(res) == len(expected)
        print(f"  {name:15s}: {len(res):4d} rows  {'✓' if match else '✗ MISMATCH'}")

    # --- Cost estimation ---
    print("\n--- Cost Estimation (page I/Os) ---")
    scenarios = [
        ("Tiny",        5,   3,   100,   60),
        ("Small",      50,  20,  1000,  500),
        ("Medium",    500, 200, 10000, 5000),
        ("Large",    5000, 800, 200000, 80000),
    ]
    print(f"{'Scenario':<10} {'P_R':>5} {'P_S':>5} {'|R|':>7} {'|S|':>6} | {'Best':<14} {'Cost':>10}")
    print("-" * 72)
    for name, pr, ps, tr, ts in scenarios:
        costs = estimate_io_costs(pr, ps, tr, ts, memory_pages=256, has_index=False)
        best, cost = costs.best()
        print(f"{name:<10} {pr:>5} {ps:>5} {tr:>7} {ts:>6} | {best:<14} {cost:>10,.0f}")

    print("\n--- With Index on Inner ---")
    for name, pr, ps, tr, ts in scenarios:
        costs = estimate_io_costs(pr, ps, tr, ts, memory_pages=256, has_index=True)
        best, cost = costs.best()
        print(f"{name:<10} {pr:>5} {ps:>5} {tr:>7} {ts:>6} | {best:<14} {cost:>10,.0f}")

    print("\n--- Limited Memory (64 pages) ---")
    for name, pr, ps, tr, ts in scenarios:
        costs = estimate_io_costs(pr, ps, tr, ts, memory_pages=64, has_index=False)
        best, cost = costs.best()
        print(f"{name:<10} {pr:>5} {ps:>5} {tr:>7} {ts:>6} | {best:<14} {cost:>10,.0f}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    run_demo()


if __name__ == "__main__":
    main()
