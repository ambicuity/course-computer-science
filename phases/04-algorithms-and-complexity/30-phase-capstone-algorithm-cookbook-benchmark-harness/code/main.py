"""
Phase Capstone — Algorithm Cookbook + Benchmark Harness
Phase 04 — Algorithms & Complexity Analysis

Python benchmark harness with algorithm implementations,
input generators, CSV output, and comparison table printing.
"""

import csv
import heapq
import random
import time
from typing import Callable


# ---------------------------------------------------------------------------
# Benchmark Harness
# ---------------------------------------------------------------------------

class BenchmarkHarness:
    """Register algorithms and input generators, run benchmarks, produce reports."""

    def __init__(self):
        self.results: list[dict] = []
        self._generators: dict[str, Callable] = {}
        self._algorithms: dict[str, Callable] = {}

    def register_generator(self, name: str, fn: Callable) -> None:
        self._generators[name] = fn

    def register_algorithm(self, name: str, fn: Callable) -> None:
        self._algorithms[name] = fn

    def run(self, n_values: list[int], generators: list[str] | None = None,
            algorithms: list[str] | None = None, repeats: int = 5) -> None:
        gens = generators or list(self._generators.keys())
        algs = algorithms or list(self._algorithms.keys())
        for n in n_values:
            for gen_name in gens:
                base_data = self._generators[gen_name](n)
                for alg_name in algs:
                    times = []
                    for _ in range(repeats):
                        arr = list(base_data)
                        t0 = time.perf_counter()
                        self._algorithms[alg_name](arr)
                        elapsed = (time.perf_counter() - t0) * 1000
                        times.append(elapsed)
                    avg = sum(times) / len(times)
                    self.results.append({
                        "algorithm": alg_name,
                        "input": gen_name,
                        "n": n,
                        "time_ms": round(avg, 3),
                    })

    def to_csv(self, path: str) -> None:
        with open(path, "w", newline="") as f:
            writer = csv.DictWriter(
                f, fieldnames=["algorithm", "input", "n", "time_ms"]
            )
            writer.writeheader()
            writer.writerows(self.results)

    def print_table(self) -> None:
        header = f"{'Algorithm':<16} {'Input':<14} {'N':>8} {'Time (ms)':>10}"
        print(header)
        print("-" * len(header))
        for r in self.results:
            print(
                f"{r['algorithm']:<16} {r['input']:<14} "
                f"{r['n']:>8} {r['time_ms']:>10.3f}"
            )

    def summary(self) -> dict:
        """Return a dict grouping results by (algorithm, input)."""
        summary = {}
        for r in self.results:
            key = (r["algorithm"], r["input"])
            summary.setdefault(key, []).append((r["n"], r["time_ms"]))
        return summary


# ---------------------------------------------------------------------------
# Sorting Algorithms
# ---------------------------------------------------------------------------

def insertion_sort(arr: list[int]) -> list[int]:
    for i in range(1, len(arr)):
        key = arr[i]
        j = i - 1
        while j >= 0 and arr[j] > key:
            arr[j + 1] = arr[j]
            j -= 1
        arr[j + 1] = key
    return arr


def selection_sort(arr: list[int]) -> list[int]:
    n = len(arr)
    for i in range(n):
        min_idx = i
        for j in range(i + 1, n):
            if arr[j] < arr[min_idx]:
                min_idx = j
        arr[i], arr[min_idx] = arr[min_idx], arr[i]
    return arr


def merge_sort(arr: list[int]) -> list[int]:
    if len(arr) <= 1:
        return arr
    mid = len(arr) // 2
    left = merge_sort(arr[:mid])
    right = merge_sort(arr[mid:])
    i = j = 0
    result = []
    while i < len(left) and j < len(right):
        if left[i] <= right[j]:
            result.append(left[i])
            i += 1
        else:
            result.append(right[j])
            j += 1
    result.extend(left[i:])
    result.extend(right[j:])
    for k, v in enumerate(result):
        arr[k] = v
    return arr


def quick_sort(arr: list[int]) -> list[int]:
    _qs(arr, 0, len(arr) - 1)
    return arr


def _qs(arr: list[int], lo: int, hi: int) -> None:
    if lo >= hi:
        return
    mid = (lo + hi) // 2
    piv = sorted([(arr[lo], lo), (arr[mid], mid), (arr[hi], hi)])[1][1]
    arr[lo], arr[piv] = arr[piv], arr[lo]
    pivot = arr[lo]
    i = lo + 1
    for j in range(lo + 1, hi + 1):
        if arr[j] < pivot:
            arr[i], arr[j] = arr[j], arr[i]
            i += 1
    arr[lo], arr[i - 1] = arr[i - 1], arr[lo]
    _qs(arr, lo, i - 2)
    _qs(arr, i, hi)


def heap_sort(arr: list[int]) -> list[int]:
    n = len(arr)

    def sift_down(start, end):
        root = start
        while 2 * root + 1 <= end:
            child = 2 * root + 1
            if child + 1 <= end and arr[child] < arr[child + 1]:
                child += 1
            if arr[root] < arr[child]:
                arr[root], arr[child] = arr[child], arr[root]
                root = child
            else:
                return

    for i in range(n // 2 - 1, -1, -1):
        sift_down(i, n - 1)
    for i in range(n - 1, 0, -1):
        arr[0], arr[i] = arr[i], arr[0]
        sift_down(0, i - 1)
    return arr


# ---------------------------------------------------------------------------
# Searching Algorithms
# ---------------------------------------------------------------------------

def binary_search(arr: list[int], target: int) -> int:
    lo, hi = 0, len(arr) - 1
    while lo <= hi:
        mid = (lo + hi) // 2
        if arr[mid] == target:
            return mid
        elif arr[mid] < target:
            lo = mid + 1
        else:
            hi = mid - 1
    return -1


def exponential_search(arr: list[int], target: int) -> int:
    if arr[0] == target:
        return 0
    n = len(arr)
    bound = 1
    while bound < n and arr[bound] <= target:
        bound *= 2
    lo = bound // 2
    hi = min(bound, n - 1)
    while lo <= hi:
        mid = (lo + hi) // 2
        if arr[mid] == target:
            return mid
        elif arr[mid] < target:
            lo = mid + 1
        else:
            hi = mid - 1
    return -1


def linear_search(arr: list[int], target: int) -> int:
    for i, v in enumerate(arr):
        if v == target:
            return i
    return -1


# ---------------------------------------------------------------------------
# Graph Algorithms
# ---------------------------------------------------------------------------

def bfs(graph: dict[int, list[int]], src: int) -> dict[int, int]:
    dist = {src: 0}
    queue = [src]
    head = 0
    while head < len(queue):
        u = queue[head]
        head += 1
        for v in graph.get(u, []):
            if v not in dist:
                dist[v] = dist[u] + 1
                queue.append(v)
    return dist


def dfs(graph: dict[int, list[int]], src: int) -> list[int]:
    visited = []
    stack = [src]
    seen = set()
    while stack:
        u = stack.pop()
        if u in seen:
            continue
        seen.add(u)
        visited.append(u)
        for v in reversed(graph.get(u, [])):
            if v not in seen:
                stack.append(v)
    return visited


def dijkstra(graph: dict[int, list[tuple[int, float]]], src: int) -> dict[int, float]:
    dist = {v: float("inf") for v in graph}
    dist[src] = 0
    pq = [(0, src)]
    while pq:
        d, u = heapq.heappop(pq)
        if d > dist[u]:
            continue
        for v, w in graph[u]:
            nd = d + w
            if nd < dist[v]:
                dist[v] = nd
                heapq.heappush(pq, (nd, v))
    return dist


# ---------------------------------------------------------------------------
# Input Generators
# ---------------------------------------------------------------------------

def gen_random(n: int) -> list[int]:
    return [random.randint(0, n) for _ in range(n)]


def gen_sorted(n: int) -> list[int]:
    return list(range(n))


def gen_reversed(n: int) -> list[int]:
    return list(range(n, 0, -1))


def gen_nearly_sorted(n: int, swaps: int = 10) -> list[int]:
    arr = list(range(n))
    for _ in range(swaps):
        i = random.randint(0, n - 1)
        j = random.randint(0, n - 1)
        arr[i], arr[j] = arr[j], arr[i]
    return arr


def gen_adversarial_quick(n: int) -> list[int]:
    arr = list(range(n))
    mid = n // 2
    arr[0], arr[mid] = arr[mid], arr[0]
    return arr


def gen_many_duplicates(n: int) -> list[int]:
    return [random.randint(0, 10) for _ in range(n)]


def gen_graph(V: int, E: int) -> dict[int, list[tuple[int, float]]]:
    graph = {i: [] for i in range(V)}
    for _ in range(E):
        u = random.randint(0, V - 1)
        v = random.randint(0, V - 1)
        w = random.uniform(1, 100)
        graph[u].append((v, w))
    return graph


def gen_unweighted_graph(V: int, E: int) -> dict[int, list[int]]:
    graph = {i: [] for i in range(V)}
    for _ in range(E):
        u = random.randint(0, V - 1)
        v = random.randint(0, V - 1)
        graph[u].append(v)
    return graph


# ---------------------------------------------------------------------------
# Decision Tree (Cookbook CLI)
# ---------------------------------------------------------------------------

def cookbook_recommend() -> str:
    """Interactive decision tree: answer questions, get algorithm recommendation."""
    print("\n=== Algorithm Cookbook ===\n")
    q1 = input("Problem type? [sort/search/graph/optimize/string]: ").strip().lower()

    if q1 == "sort":
        bounded = input("Input bounded small range? [y/n]: ").strip().lower() == "y"
        if bounded:
            return "Counting Sort O(n+k) or Radix Sort O(n·d)"
        nearly = input("Nearly sorted / small n? [y/n]: ").strip().lower() == "y"
        if nearly:
            return "Insertion Sort — O(n) on nearly-sorted input"
        stable = input("Need stability? [y/n]: ").strip().lower() == "y"
        if stable:
            return "Merge Sort O(n log n) — stable, guaranteed"
        mem = input("Memory constrained? [y/n]: ").strip().lower() == "y"
        if mem:
            return "Heapsort O(1) space or Quicksort O(log n) space"
        return "Quicksort (median-of-3) — best general-purpose sort"

    elif q1 == "search":
        weighted = input("Data sorted? [y/n]: ").strip().lower() == "y"
        if not weighted:
            return "Linear Search O(n) — or hash table O(1) avg"
        stream = input("Unbounded / infinite stream? [y/n]: ").strip().lower() == "y"
        if stream:
            return "Exponential Search O(log i) then Binary Search"
        return "Binary Search O(log n)"

    elif q1 == "graph":
        print("  [1] Shortest path, unweighted")
        print("  [2] Shortest path, non-negative weights")
        print("  [3] Shortest path, negative weights")
        print("  [4] All-pairs shortest path")
        print("  [5] Cycle detection / topological sort")
        print("  [6] Minimum spanning tree")
        print("  [7] Maximum flow")
        choice = input("  Choice [1-7]: ").strip()
        mapping = {
            "1": "BFS O(V+E)",
            "2": "Dijkstra O(E log V)",
            "3": "Bellman-Ford O(VE) — also detects negative cycles",
            "4": "Floyd-Warshall O(V³) or Johnson O(VE + V² log V)",
            "5": "DFS — back edge = cycle, finish-time reverse = topo order",
            "6": "Kruskal (sparse) or Prim (dense) — both O(E log V)",
            "7": "Dinic O(V²E) or Edmonds-Karp O(VE²)",
        }
        return mapping.get(choice, "Invalid choice")

    elif q1 == "optimize":
        greedy = input("Greedy choice property holds? [y/n]: ").strip().lower() == "y"
        if greedy:
            return "Greedy — prove with exchange argument or matroid structure"
        return "Dynamic Programming — overlapping subproblems + optimal substructure"

    elif q1 == "string":
        print("  [1] Single pattern matching")
        print("  [2] Multiple patterns")
        print("  [3] Approximate / fuzzy matching")
        choice = input("  Choice [1-3]: ").strip()
        mapping = {
            "1": "KMP O(n+m) or Boyer-Moore (sublinear in practice)",
            "2": "Aho-Corasick O(n+m+z)",
            "3": "Edit distance DP O(nm)",
        }
        return mapping.get(choice, "Invalid choice")

    return "Unknown problem type"


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    harness = BenchmarkHarness()

    # Register generators
    harness.register_generator("random", gen_random)
    harness.register_generator("sorted", gen_sorted)
    harness.register_generator("reversed", gen_reversed)
    harness.register_generator("nearly_sorted", gen_nearly_sorted)
    harness.register_generator("adversarial", gen_adversarial_quick)

    # Register sorting algorithms
    harness.register_algorithm("insertion", insertion_sort)
    harness.register_algorithm("selection", selection_sort)
    harness.register_algorithm("merge", merge_sort)
    harness.register_algorithm("quick", quick_sort)
    harness.register_algorithm("heap", heap_sort)

    # Run sorting benchmarks
    print("=" * 60)
    print("SORTING BENCHMARKS")
    print("=" * 60)
    harness.run(
        n_values=[500, 1000, 2000],
        repeats=3,
    )
    harness.print_table()
    harness.to_csv("outputs/benchmark_results.csv")
    print(f"\nCSV written to outputs/benchmark_results.csv")

    # Searching benchmarks
    print("\n" + "=" * 60)
    print("SEARCHING BENCHMARKS")
    print("=" * 60)
    search_harness = BenchmarkHarness()
    search_harness.register_generator("sorted", gen_sorted)

    def bench_binary(arr):
        binary_search(arr, len(arr) // 2)

    def bench_exponential(arr):
        exponential_search(arr, len(arr) // 2)

    def bench_linear(arr):
        linear_search(arr, len(arr) // 2)

    search_harness.register_algorithm("binary", bench_binary)
    search_harness.register_algorithm("exponential", bench_exponential)
    search_harness.register_algorithm("linear", bench_linear)
    search_harness.run(n_values=[10000, 100000, 1000000], repeats=3)
    search_harness.print_table()

    # Graph benchmarks
    print("\n" + "=" * 60)
    print("GRAPH BENCHMARKS")
    print("=" * 60)
    print(f"{'Algorithm':<16} {'V':>6} {'E':>8} {'Time (ms)':>10}")
    print("-" * 44)

    for V in [100, 500, 1000]:
        E = V * 4
        g_unw = gen_unweighted_graph(V, E)
        g_w = gen_graph(V, E)

        t0 = time.perf_counter()
        for _ in range(5):
            bfs(g_unw, 0)
        bfs_t = (time.perf_counter() - t0) / 5 * 1000

        t0 = time.perf_counter()
        for _ in range(5):
            dfs(g_unw, 0)
        dfs_t = (time.perf_counter() - t0) / 5 * 1000

        t0 = time.perf_counter()
        for _ in range(5):
            dijkstra(g_w, 0)
        dij_t = (time.perf_counter() - t0) / 5 * 1000

        print(f"{'BFS':<16} {V:>6} {E:>8} {bfs_t:>10.3f}")
        print(f"{'DFS':<16} {V:>6} {E:>8} {dfs_t:>10.3f}")
        print(f"{'Dijkstra':<16} {V:>6} {E:>8} {dij_t:>10.3f}")
        print()

    # Decision tree demo
    print("=" * 60)
    print("ALGORITHM COOKBOOK — Quick Reference")
    print("=" * 60)
    print("Sorting:   Bounded range? → Counting/Radix. Nearly sorted? → Insertion.")
    print("           Need stability? → Merge. General? → Quicksort (median-3).")
    print("Searching: Sorted data? → Binary O(log n). Unbounded? → Exponential.")
    print("           Unsorted? → Linear O(n) or hash table O(1).")
    print("Graph:     Unweighted? → BFS. Non-negative? → Dijkstra.")
    print("           Negative? → Bellman-Ford. MST? → Kruskal/Prim.")
    print("Optimize:  Greedy property? → Greedy. Otherwise → DP.")
    print("Strings:   Single pattern? → KMP/Boyer-Moore. Multiple? → Aho-Corasick.")

    print("\nAll benchmarks complete.")


if __name__ == "__main__":
    main()
