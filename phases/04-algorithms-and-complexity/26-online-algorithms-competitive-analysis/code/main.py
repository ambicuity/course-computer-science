"""
Online Algorithms & Competitive Analysis
Phase 04 — Algorithms & Complexity Analysis

Implements LRU/FIFO paging simulators, Belady's optimal offline algorithm,
ski rental (deterministic + randomized), and online bipartite matching
(ranking algorithm). Each produces a competitive ratio against the offline optimum.
"""

import random
from collections import OrderedDict


# ---------------------------------------------------------------------------
# Paging: LRU simulator
# ---------------------------------------------------------------------------


def lru_simulator(pages: list[int], cache_size: int) -> int:
    """Simulate LRU paging. Return the number of page faults."""
    cache: OrderedDict[int, None] = OrderedDict()
    faults = 0
    for page in pages:
        if page in cache:
            cache.move_to_end(page)
        else:
            faults += 1
            if len(cache) >= cache_size:
                cache.popitem(last=False)
            cache[page] = None
    return faults


# ---------------------------------------------------------------------------
# Paging: FIFO simulator
# ---------------------------------------------------------------------------


def fifo_simulator(pages: list[int], cache_size: int) -> int:
    """Simulate FIFO paging. Return the number of page faults."""
    cache: OrderedDict[int, None] = OrderedDict()
    faults = 0
    for page in pages:
        if page in cache:
            continue
        faults += 1
        if len(cache) >= cache_size:
            cache.popitem(last=False)
        cache[page] = None
    return faults


# ---------------------------------------------------------------------------
# Paging: Belady's optimal offline algorithm
# ---------------------------------------------------------------------------


def belady_optimal(pages: list[int], cache_size: int) -> int:
    """Optimal offline paging via Belady's algorithm (evict farthest-future)."""
    cache: set[int] = set()
    faults = 0
    n = len(pages)
    for i, page in enumerate(pages):
        if page in cache:
            continue
        faults += 1
        if len(cache) < cache_size:
            cache.add(page)
        else:
            farthest = -1
            victim = None
            for p in cache:
                try:
                    next_use = pages.index(p, i + 1)
                except ValueError:
                    victim = p
                    break
                if next_use > farthest:
                    farthest = next_use
                    victim = p
            cache.remove(victim)
            cache.add(page)
    return faults


def paging_competitive_ratio(pages: list[int], cache_size: int) -> dict:
    """Compute observed competitive ratios for LRU and FIFO vs Belady's optimum."""
    opt_faults = belady_optimal(pages, cache_size)
    lru_faults = lru_simulator(pages, cache_size)
    fifo_faults = fifo_simulator(pages, cache_size)
    return {
        "opt_faults": opt_faults,
        "lru_faults": lru_faults,
        "fifo_faults": fifo_faults,
        "lru_ratio": lru_faults / opt_faults if opt_faults > 0 else (float("inf") if lru_faults > 0 else 1.0),
        "fifo_ratio": fifo_faults / opt_faults if opt_faults > 0 else (float("inf") if fifo_faults > 0 else 1.0),
    }


# ---------------------------------------------------------------------------
# Ski rental: deterministic strategy
# ---------------------------------------------------------------------------


def ski_rental_deterministic(days: int, buy_cost: int) -> dict:
    """Rent for buy_cost days, then buy. Returns cost breakdown."""
    rent_days = min(days, buy_cost)
    bought = days >= buy_cost
    total_cost = rent_days + (buy_cost if bought else 0)
    opt_cost = min(days, buy_cost)
    return {
        "strategy": f"rent {buy_cost} days then buy",
        "days": days,
        "total_cost": total_cost,
        "opt_cost": opt_cost,
        "ratio": total_cost / opt_cost if opt_cost > 0 else float("inf"),
    }


# ---------------------------------------------------------------------------
# Ski rental: randomized strategy
# ---------------------------------------------------------------------------


def ski_rental_randomized(days: int, buy_cost: int, trials: int = 10_000) -> dict:
    """Randomized ski rental: each day independently buy with prob 1/buy_cost."""
    costs: list[int] = []
    for _ in range(trials):
        bought = False
        total = 0
        for day in range(days):
            if bought:
                break
            if random.random() < 1.0 / buy_cost:
                total += buy_cost
                bought = True
            else:
                total += 1
        costs.append(total)
    avg_cost = sum(costs) / len(costs)
    opt_cost = min(days, buy_cost)
    return {
        "strategy": f"randomized (p=1/{buy_cost} per day)",
        "days": days,
        "avg_cost": round(avg_cost, 2),
        "opt_cost": opt_cost,
        "avg_ratio": round(avg_cost / opt_cost, 4) if opt_cost > 0 else float("inf"),
    }


# ---------------------------------------------------------------------------
# Online bipartite matching: ranking algorithm
# ---------------------------------------------------------------------------


def _max_bipartite_matching(
    graph: dict[str, list[str]],
    left_vertices: list[str],
    right_vertices: list[str],
) -> set[tuple[str, str]]:
    """Maximum bipartite matching via augmenting paths (offline)."""
    match_r: dict[str, str] = {}

    def dfs(left: str, visited: set[str]) -> bool:
        for right in graph.get(left, []):
            if right in visited:
                continue
            visited.add(right)
            if right not in match_r or dfs(match_r[right], visited):
                match_r[right] = left
                return True
        return False

    for left in left_vertices:
        dfs(left, set())

    return {(match_r[r], r) for r in match_r}


def online_matching(
    graph: dict[str, list[str]],
    arrival_order: list[str],
) -> tuple[set[tuple[str, str]], float]:
    """
    Ranking algorithm for online bipartite matching.

    graph: {left_vertex: [right_neighbors]}
    arrival_order: sequence of left vertices arriving one at a time.
    Returns (matching set, competitive ratio vs OPT).
    """
    all_right: set[str] = set()
    for neighbors in graph.values():
        all_right.update(neighbors)

    right_list = list(all_right)
    random.shuffle(right_list)
    priority = {v: i for i, v in enumerate(right_list)}

    matched_right: set[str] = set()
    matching: set[tuple[str, str]] = set()

    for left in arrival_order:
        neighbors = graph.get(left, [])
        best = None
        best_pri = float("inf")
        for n in neighbors:
            if n not in matched_right and priority[n] < best_pri:
                best = n
                best_pri = priority[n]
        if best is not None:
            matched_right.add(best)
            matching.add((left, best))

    opt = _max_bipartite_matching(graph, arrival_order, list(all_right))
    ratio = len(matching) / len(opt) if opt else 1.0
    return matching, ratio


# ---------------------------------------------------------------------------
# Main: demonstrations
# ---------------------------------------------------------------------------


def main() -> None:
    print("=" * 60)
    print("Online Algorithms & Competitive Analysis")
    print("=" * 60)

    # --- Paging ---
    print("\n--- Paging (k=3) ---")
    pages = [1, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5]
    cache_size = 3
    print(f"Request sequence: {pages}")
    print(f"Cache size: {cache_size}")

    result = paging_competitive_ratio(pages, cache_size)
    print(f"  Belady's (OPT): {result['opt_faults']} faults")
    print(f"  LRU:            {result['lru_faults']} faults  (ratio: {result['lru_ratio']:.2f})")
    print(f"  FIFO:           {result['fifo_faults']} faults  (ratio: {result['fifo_ratio']:.2f})")

    # Adversarial sequence: round-robin on k+1 pages
    print("\n  Adversarial sequence (round-robin k+1 pages):")
    adv_pages = [i % (cache_size + 1) for i in range(30)]
    result2 = paging_competitive_ratio(adv_pages, cache_size)
    print(f"  Sequence: {adv_pages[:12]}...")
    print(f"  Belady's (OPT): {result2['opt_faults']} faults")
    print(f"  LRU:            {result2['lru_faults']} faults  (ratio: {result2['lru_ratio']:.2f})")
    print(f"  FIFO:           {result2['fifo_faults']} faults  (ratio: {result2['fifo_ratio']:.2f})")
    print(f"  (Theoretical worst-case ratio: {cache_size})")

    # --- Ski rental ---
    print("\n--- Ski Rental (buy_cost=7) ---")
    for days in [3, 7, 10, 20]:
        det = ski_rental_deterministic(days, buy_cost=7)
        rnd = ski_rental_randomized(days, buy_cost=7, trials=50_000)
        print(f"  {days} days:")
        print(f"    Deterministic: cost={det['total_cost']}, OPT={det['opt_cost']}, ratio={det['ratio']:.2f}")
        print(f"    Randomized:    avg_cost={rnd['avg_cost']}, OPT={rnd['opt_cost']}, avg_ratio={rnd['avg_ratio']:.4f}")
    print(f"  Theoretical randomized bound: e/(e-1) ≈ {2.71828 / 1.71828:.4f}")

    # --- Online matching ---
    print("\n--- Online Bipartite Matching (Ranking) ---")
    graph = {
        "a": ["1", "2"],
        "b": ["1", "3"],
        "c": ["2", "3"],
        "d": ["3", "4"],
    }
    arrival = ["a", "b", "c", "d"]
    trials = 100
    ratios = []
    for _ in range(trials):
        _, ratio = online_matching(graph, arrival)
        ratios.append(ratio)
    avg_ratio = sum(ratios) / len(ratios)
    print(f"  Graph: {graph}")
    print(f"  Arrival order: {arrival}")
    matching, single_ratio = online_matching(graph, arrival)
    print(f"  Sample matching: {matching} (size={len(matching)})")
    print(f"  Single-trial ratio: {single_ratio:.2f}")
    print(f"  Average ratio over {trials} trials: {avg_ratio:.4f}")
    print(f"  Theoretical bound: 1-1/e ≈ {1 - 1/2.71828:.4f}")


if __name__ == "__main__":
    main()
