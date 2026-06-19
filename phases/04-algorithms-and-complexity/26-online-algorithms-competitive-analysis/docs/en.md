# Online Algorithms & Competitive Analysis

> Decisions under uncertainty — compete against an all-knowing adversary.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 04 lessons 01–25
**Time:** ~60 minutes

## Learning Objectives

- Understand the core concept introduced in this lesson and why it matters.
- Implement the lesson's "Build It" artifact from scratch in one of: Python.
- Compare your from-scratch implementation against the production tool used in industry.
- Ship the reusable artifact (see "Ship It") and add it to your toolbox.

## The Problem

This lesson sits in **Phase 04 — Algorithms & Complexity Analysis**. Without the concept it teaches, you cannot
build the phase's capstone (An algorithms cookbook plus a benchmark harness.). Concretely, *not* knowing this means you get stuck the
moment you try to master the canon — sorting, dp, graphs, strings, geometry, randomization — and the analysis tools that bound them.

In most algorithm design, we assume the entire input is available before we start computing. But many real systems cannot do this: a CPU cache manager must evict a page *now*, without knowing which pages will be requested next. An investor must decide *today* whether to rent or buy equipment, not knowing how many days they will actually need it. An ad server must match an arriving user to a campaign *immediately*, without seeing future users.

These are **online problems**: the input arrives piece by piece, and each piece must be answered before the next arrives. Classical worst-case analysis (which asks "how bad can the input be?") gives pessimistic answers here because an adaptive adversary can always feed the worst possible next item. **Competitive analysis** replaces absolute worst-case with a *relative* guarantee: the online algorithm's cost is never worse than a constant factor times the cost of an optimal offline algorithm that *does* see the future.

## The Concept

### Online vs Offline

An **offline algorithm** receives the entire input upfront and can plan optimally. An **online algorithm** processes a sequence of requests, making an irrevocable decision for each request before seeing subsequent ones.

```
Request stream:  r1  r2  r3  ...  rn
                 |   |   |        |
Offline:  sees [r1..rn]  ──────────►  optimal plan
Online:   sees r1 → decide → see r2 → decide → ...
```

### Competitive Ratio

An online algorithm ALG is **α-competitive** if for every request sequence σ:

    cost(ALG(σ))  ≤  α · cost(OPT(σ)) + β

where OPT is the optimal offline algorithm (one that knows the full sequence) and β is an additive constant independent of σ.

- **Strict competitive ratio**: the smallest α for which the inequality holds for all σ.
- **Amortized competitive analysis**: uses a potential function Φ to "bank" savings across operations, proving a tighter bound than analyzing each operation independently.

If α = 1, the online algorithm is as good as offline — possible only for trivial problems. The competitive ratio measures *how much worse* we are forced to be by ignorance of the future.

### The Paging / k-Cache Problem

A machine has a cache that holds **k** pages. The main memory holds **n > k** distinct pages. A request specifies a page: if it is in the cache, it is a *hit* (cost 0); otherwise, it is a *fault* (cost 1), and some cached page must be evicted.

**LRU (Least Recently Used)** evicts the page whose last access was furthest in the past.

**Theorem**: LRU is **k-competitive** for the paging problem.

**Proof sketch (potential function method)**:
Define the potential Φ = k · (# of pages in LRU's cache that are *not* in OPT's cache). Initially Φ₀ = 0.

On a request to page p:
- *Both fault*: OPT pays 1, LRU pays 1. OPT evicts q. If q was in LRU's cache, Φ decreases by k; otherwise Φ unchanged. ΔΦ ≤ 0. So LRU's amortized cost ≤ 1.
- *OPT hits, LRU faults*: LRU pays 1 + ΔΦ. The page LRU evicts was not requested recently, and OPT has it cached, so ΔΦ ≤ k − 1. Amortized cost ≤ k.
- *Both hit*: cost 0, ΔΦ = 0.

Summing over the sequence: Σ(actual cost for LRU) ≤ k · (# of faults by OPT) + Φ₀ − Φₙ ≤ k · cost(OPT). □

**FIFO** (first-in, first-out) is also k-competitive, proved similarly.

**Lower bound**: No deterministic online paging algorithm can achieve a competitive ratio better than k. This is proved via an adversary that requests k+1 distinct pages in a round-robin pattern — every deterministic algorithm faults on every request, while OPT faults only every k+1 requests.

### The Ski Rental Problem

You are at a ski resort. Each day you can either **rent** skis for cost 1, or **buy** skis for a one-time cost **d** (after which you pay nothing). You do not know in advance how many days you will ski.

- **Optimal offline**: if you ski t days and t ≥ d, buy immediately (cost d); if t < d, rent every day (cost t).
- **Deterministic strategy**: rent for some fixed number of days b, then buy. The worst case is t = b + 1 (you buy just after the last rental day): cost = b + d. OPT = min(b+1, d). The competitive ratio is maximized when b+1 = d, giving α = (b + d) / d. Setting b = d gives α = 2. No deterministic algorithm can do better than 2-competitive.
- **Randomized strategy**: each day, independently buy with probability 1/d. Expected cost for t days: E[cost] = Σᵢ₌₁ᵗ Prob(survive to day i) · 1 + Prob(buy on day i) · d = ... yields competitive ratio **e/(e−1) ≈ 1.58**. This is optimal for randomized ski rental.

### Online Bipartite Matching (Ranking Algorithm)

Vertices arrive one at a time on one side of a bipartite graph, and must be matched immediately or lost forever.

**Ranking algorithm** (Karp, Vazirani, Vazirani 1990): assign each right-side vertex a random priority. When a left vertex arrives, match it to the highest-priority unmatched neighbor.

**Theorem**: Ranking is **(1 − 1/e)-competitive** for online bipartite matching, i.e., it achieves at least (1 − 1/e) · OPT ≈ 63.2% of the maximum matching.

This bound is tight — no randomized algorithm can do better. The result is foundational for online ad allocation (matching advertisers to search queries).

### Worked Example: Paging

Cache size k = 3. Request sequence: 1, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5.

| Step | Page | LRU Cache (before) | LRU Action | OPT Cache (before) | OPT Action |
|------|------|--------------------|------------|--------------------|--------------------|
| 1 | 1 | {} | fault, load 1 | {} | fault, load 1 |
| 2 | 2 | {1} | fault, load 2 | {1} | fault, load 2 |
| 3 | 3 | {1,2} | fault, load 3 | {1,2} | fault, load 3 |
| 4 | 4 | {1,2,3} | fault, evict 1 | {1,2,3} | fault, evict 1 |
| 5 | 1 | {2,3,4} | fault, evict 2 | {2,3,4} | fault, evict 2 |
| 6 | 2 | {3,4,1} | fault, evict 3 | {3,4,1} | fault, evict 3 |
| 7 | 5 | {4,1,2} | fault, evict 4 | {4,1,2} | fault, evict 4 |
| 8 | 1 | {1,2,5} | hit | {1,2,5} | hit |
| 9 | 2 | {1,2,5} | hit | {1,2,5} | hit |
| 10 | 3 | {1,2,5} | fault, evict 5 | {1,2,5} | fault, evict 5 |
| 11 | 4 | {1,2,3} | fault, evict 1 | {1,2,3} | fault, evict 1 |
| 12 | 5 | {2,3,4} | fault, evict 2 | {2,3,4} | fault, evict 2 |

LRU faults: 10. OPT faults (Belady's): 10 on this sequence. Competitive ratio observed: 10/10 = 1. The worst-case ratio over all sequences is at most k = 3.

## Build It

### Step 1: Minimal LRU Simulator

A list-based LRU cache simulator — evict the page whose position in the list is furthest from the end (least recently used).

```python
from collections import OrderedDict


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
```

### Step 2: Belady's Optimal Offline Algorithm

To compute the competitive ratio, we need the optimal offline baseline. Belady's algorithm evicts the page whose next use is farthest in the future (or never used again).

```python
def belady_optimal(pages: list[int], cache_size: int) -> int:
    """Optimal offline paging via Belady's algorithm."""
    cache: set[int] = set()
    faults = 0
    for i, page in enumerate(pages):
        if page in cache:
            continue
        faults += 1
        if len(cache) < cache_size:
            cache.add(page)
        else:
            # Find the cached page used farthest in the future
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
```

### Step 3: Competitive Ratio Computation

```python
def competitive_ratio(pages: list[int], cache_size: int) -> float:
    """Compute observed competitive ratio: LRU faults / OPT faults."""
    opt_faults = belady_optimal(pages, cache_size)
    lru_faults = lru_simulator(pages, cache_size)
    if opt_faults == 0:
        return float("inf") if lru_faults > 0 else 1.0
    return lru_faults / opt_faults
```

### Step 4: Ski Rental — Deterministic and Randomized

```python
import random


def ski_rental_deterministic(days: int, buy_cost: int) -> dict:
    """Rent for buy_cost days, then buy. Returns cost breakdown."""
    rent_days = min(days, buy_cost)
    bought = days >= buy_cost
    total_cost = rent_days + (buy_cost if bought else 0)
    opt_cost = min(days, buy_cost)
    return {
        "strategy": f"rent {buy_cost} days then buy",
        "total_cost": total_cost,
        "opt_cost": opt_cost,
        "ratio": total_cost / opt_cost if opt_cost > 0 else float("inf"),
    }


def ski_rental_randomized(days: int, buy_cost: int, trials: int = 10_000) -> dict:
    """Randomized ski rental: each day buy with prob 1/buy_cost."""
    costs = []
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
        "avg_cost": round(avg_cost, 2),
        "opt_cost": opt_cost,
        "avg_ratio": round(avg_cost / opt_cost, 4) if opt_cost > 0 else float("inf"),
    }
```

### Step 5: Online Matching — Ranking Algorithm

```python
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
    import random as _random

    all_right: set[str] = set()
    for neighbors in graph.values():
        all_right.update(neighbors)

    # Assign random priorities to right-side vertices
    right_list = list(all_right)
    _random.shuffle(right_list)
    priority = {v: i for i, v in enumerate(right_list)}

    matched_right: set[str] = set()
    matching: set[tuple[str, str]] = set()

    for left in arrival_order:
        neighbors = graph.get(left, [])
        # Match to highest-priority unmatched neighbor
        best = None
        best_pri = float("inf")
        for n in neighbors:
            if n not in matched_right and priority[n] < best_pri:
                best = n
                best_pri = priority[n]
        if best is not None:
            matched_right.add(best)
            matching.add((left, best))

    # Compute offline optimum (maximum bipartite matching via greedy augmenting paths)
    opt = _max_bipartite_matching(graph, arrival_order, list(all_right))
    ratio = len(matching) / len(opt) if opt else 1.0
    return matching, ratio


def _max_bipartite_matching(
    graph: dict[str, list[str]],
    left_vertices: list[str],
    right_vertices: list[str],
) -> set[tuple[str, str]]:
    """Maximum bipartite matching via augmenting paths (offline)."""
    match_r: dict[str, str] = {}  # right -> left

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
```

## Use It

**Operating systems**: Modern OS kernels use approximations of LRU for page replacement. Linux uses a two-list variant (active/inactive) that approximates LRU without the overhead of tracking exact recency. The concept directly informs decisions about which memory pages to evict under memory pressure.

**Online ad allocation**: Search engines match arriving queries (left vertices) to advertisers (right vertices) using variants of the ranking algorithm. The (1 − 1/e) guarantee ensures revenue is at least 63% of the omniscient optimum — a provable worst-case floor even when query patterns are adversarial.

**Cloud resource management**: Auto-scaling decisions (buy more instances vs. rent on-demand) are real-world ski rental. The deterministic 2-competitive strategy maps to a rule like "if sustained demand exceeds d hours, switch to reserved instances."

**CDN caching**: Content delivery networks decide which objects to cache at edge nodes — this is paging with a non-uniform cost model. LRU and its variants (LRU-K, 2Q) dominate production deployments.

## Read the Source

- Linux kernel `mm/vmscan.c` — the LRU list management logic that drives page reclaim.
- `collections.OrderedDict` in CPython — the `move_to_end` and `popitem(last=False)` operations that make LRU O(1) per access.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained online-algorithms toolkit**: LRU/FIFO simulators, Belady's optimal, ski rental (deterministic + randomized), online matching (ranking), and competitive ratio computation — all in one module you can import in later phases.

## Exercises

1. **Easy** — Reproduce the LRU simulator from memory. Verify it matches `main.py` output on the worked-example sequence.

2. **Medium** — Implement randomized ski rental with `buy_cost = 10`. Run 100,000 trials for `days = 1..50` and plot the empirical competitive ratio vs. the theoretical bound e/(e-1) ≈ 1.58.

3. **Hard** — Prove LRU is k-competitive using the potential function Φ = k · (number of pages in LRU's cache not in OPT's cache). Show formally that on every request, the amortized cost Δ(actual + ΔΦ) ≤ k · (1 if OPT faults else 0). Extend the proof to show FIFO is also k-competitive with Φ = k · (# of pages in FIFO loaded more recently than OPT's oldest).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Online algorithm | "Algorithm that doesn't see the future" | Processes input one item at a time, committing to each answer before seeing the next |
| Competitive ratio | "How much worse than optimal" | The smallest α such that cost(ALG) ≤ α·cost(OPT) + β for all input sequences |
| k-competitive | "At most k times worse" | The online algorithm's cost is bounded by k times the optimal offline cost |
| Potential function | "Bank account for amortized analysis" | A mapping Φ from states to reals used to amortize costs across operations |
| Belady's algorithm | "Optimal page replacement" | Offline paging algorithm that evicts the page whose next use is farthest in the future |
| Ranking algorithm | "Random priorities for matching" | Online matching algorithm achieving (1-1/e)-competitive ratio via random vertex ordering |
| Ski rental | "Rent vs buy under uncertainty" | Canonical online problem: rent daily or buy once, unknown duration |

## Further Reading

- Borodin, A. and El-Yaniv, R. *Online Computation and Competitive Analysis*. Cambridge University Press, 1998.
- Karp, R., Vazirani, U., Vazirani, V. "An optimal algorithm for on-line bipartite matching." *STOC*, 1990.
- Sleator, D. and Tarjan, R. "Amortized efficiency of list update and paging rules." *Communications of the ACM*, 28(2), 1985.
