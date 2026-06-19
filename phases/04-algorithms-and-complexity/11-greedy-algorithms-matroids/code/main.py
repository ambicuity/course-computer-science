"""
Greedy Algorithms & Matroids
Phase 04 — Algorithms & Complexity Analysis

From-scratch implementations of activity selection, Huffman coding,
fractional knapsack, job sequencing, and matroid verification.
"""

import heapq
from collections import Counter
from itertools import combinations


# ---------------------------------------------------------------------------
# 1. Activity Selection
# ---------------------------------------------------------------------------

def activity_selection(activities: list[tuple[int, int]]) -> list[tuple[int, int]]:
    """Return a maximum-size set of non-overlapping activities.

    Each activity is (start, finish).  Greedy strategy: sort by finish time,
    then always pick the next activity whose start >= last selected finish.

    >>> activity_selection([(1, 4), (3, 5), (0, 6), (5, 7), (3, 9), (5, 9),
    ...                    (6, 10), (8, 11), (8, 12), (2, 14), (12, 16)])
    [(1, 4), (5, 7), (8, 11), (12, 16)]
    """
    if not activities:
        return []

    sorted_acts = sorted(activities, key=lambda a: a[1])
    selected = [sorted_acts[0]]

    for start, finish in sorted_acts[1:]:
        if start >= selected[-1][1]:
            selected.append((start, finish))

    return selected


# ---------------------------------------------------------------------------
# 2. Huffman Coding
# ---------------------------------------------------------------------------

class HuffmanNode:
    """A node in the Huffman tree."""

    __slots__ = ("char", "freq", "left", "right")

    def __init__(self, char: str | None = None, freq: int = 0,
                 left: "HuffmanNode | None" = None,
                 right: "HuffmanNode | None" = None):
        self.char = char
        self.freq = freq
        self.left = left
        self.right = right

    def __lt__(self, other: "HuffmanNode") -> bool:
        return self.freq < other.freq

    def is_leaf(self) -> bool:
        return self.char is not None


def build_huffman_tree(text: str) -> HuffmanNode:
    """Build a Huffman tree from character frequencies in *text*."""
    freq = Counter(text)
    heap = [HuffmanNode(char=c, freq=f) for c, f in freq.items()]
    heapq.heapify(heap)

    # Edge case: single unique character
    if len(heap) == 1:
        return HuffmanNode(freq=heap[0].freq, left=heap[0])

    while len(heap) > 1:
        left = heapq.heappop(heap)
        right = heapq.heappop(heap)
        merged = HuffmanNode(freq=left.freq + right.freq, left=left, right=right)
        heapq.heappush(heap, merged)

    return heap[0]


def _build_codes(node: HuffmanNode, prefix: str = "") -> dict[str, str]:
    """Recursively extract character -> bit-string mappings."""
    if node.is_leaf():
        return {node.char: prefix or "0"}
    codes: dict[str, str] = {}
    codes.update(_build_codes(node.left, prefix + "0"))
    codes.update(_build_codes(node.right, prefix + "1"))
    return codes


def huffman_encode(text: str) -> tuple[str, dict[str, str], HuffmanNode]:
    """Encode *text* using Huffman coding.

    Returns (encoded_bitstring, code_table, tree).
    The tree is needed for decoding.

    >>> encoded, codes, tree = huffman_encode("aabbc")
    >>> huffman_decode(encoded, tree)
    'aabbc'
    """
    if not text:
        return "", {}, HuffmanNode()

    tree = build_huffman_tree(text)
    codes = _build_codes(tree)
    encoded = "".join(codes[ch] for ch in text)

    original_bits = len(text) * 8
    compressed_bits = len(encoded)
    ratio = compressed_bits / original_bits if original_bits else 0.0

    print(f"  Original: {original_bits} bits ({len(text)} chars x 8)")
    print(f"  Encoded:  {compressed_bits} bits")
    print(f"  Ratio:    {ratio:.2%}")
    print(f"  Codes:    {codes}")

    return encoded, codes, tree


def huffman_decode(encoded: str, tree: HuffmanNode) -> str:
    """Decode a Huffman-encoded bitstring using the original tree.

    >>> encoded, codes, tree = huffman_encode("aabbc")
    >>> huffman_decode(encoded, tree)
    'aabbc'
    """
    if not encoded or tree is None:
        return ""

    decoded: list[str] = []
    node = tree

    for bit in encoded:
        node = node.left if bit == "0" else node.right
        if node.is_leaf():
            decoded.append(node.char)
            node = tree

    return "".join(decoded)


# ---------------------------------------------------------------------------
# 3. Fractional Knapsack
# ---------------------------------------------------------------------------

def fractional_knapsack(items: list[tuple[float, float]], W: float) -> tuple[float, list[tuple[int, float]]]:
    """Maximise value within capacity W (items can be split).

    items = [(value, weight), ...]
    Returns (total_value, [(item_index, fraction_taken), ...])

    >>> fractional_knapsack([(60, 10), (100, 20), (120, 30)], 50)
    (240.0, ...)
    """
    if W <= 0 or not items:
        return 0.0, []

    indexed = sorted(enumerate(items), key=lambda x: x[1][0] / x[1][1], reverse=True)

    total_value = 0.0
    remaining = W
    taken: list[tuple[int, float]] = []

    for idx, (value, weight) in indexed:
        if remaining <= 0:
            break
        take = min(weight, remaining)
        fraction = take / weight
        total_value += value * fraction
        remaining -= take
        taken.append((idx, fraction))

    return total_value, taken


# ---------------------------------------------------------------------------
# 4. Job Sequencing
# ---------------------------------------------------------------------------

def job_sequencing(jobs: list[tuple[int, int]]) -> tuple[int, list[int]]:
    """Maximise profit by scheduling jobs with deadlines.

    jobs = [(deadline, profit), ...].  Each job takes 1 time unit.
    Returns (total_profit, slot_assignments) where slot_assignments[t] is the
    job index (by original order) scheduled at time t, or -1 if empty.

    >>> job_sequencing([(4, 20), (1, 10), (1, 40), (1, 30)])
    (60, ...)
    """
    if not jobs:
        return 0, []

    # Sort by profit descending, keep original index
    indexed = sorted(enumerate(jobs), key=lambda x: x[1][1], reverse=True)
    max_deadline = max(d for d, _ in jobs)

    slots = [-1] * (max_deadline + 1)  # slots[0] unused
    total_profit = 0
    scheduled: list[int] = []

    for orig_idx, (deadline, profit) in indexed:
        for t in range(deadline, 0, -1):
            if slots[t] == -1:
                slots[t] = orig_idx
                total_profit += profit
                scheduled.append(orig_idx)
                break

    return total_profit, slots


# ---------------------------------------------------------------------------
# 5. Matroid Verification
# ---------------------------------------------------------------------------

def is_matroid(elements: list, independent_fn) -> bool:
    """Verify that (elements, independent_fn) forms a matroid.

    Checks:
      1. Hereditary: every subset of an independent set is independent.
      2. Exchange: |A| < |B| both independent => ∃ x ∈ B\\A with A∪{x} independent.

    For large ground sets this is exponential — intended for pedagogical use.

    >>> is_matroid([1, 2, 3], lambda s: len(s) <= 2)
    True
    >>> is_matroid([1, 2, 3], lambda s: len(s) <= 1 or set(s) == {1, 2})
    False
    """
    n = len(elements)

    # Enumerate all independent sets
    all_indep: list[set] = []
    for r in range(n + 1):
        for subset in combinations(elements, r):
            if independent_fn(list(subset)):
                all_indep.append(set(subset))

    all_indep_set = frozenset(frozenset(s) for s in all_indep)

    # 1. Hereditary: every subset of an independent set is independent
    for s in all_indep:
        for elem in s:
            subset = s - {elem}
            if frozenset(subset) not in all_indep_set:
                print(f"  Hereditary violated: {s} is independent but {subset} is not")
                return False

    # 2. Exchange property
    for a in all_indep:
        for b in all_indep:
            if len(a) < len(b):
                diff = b - a
                can_extend = any(frozenset(a | {x}) in all_indep_set for x in diff)
                if not can_extend:
                    print(f"  Exchange violated: |A|={len(a)} < |B|={len(b)}, no element extends A={a} from B\\A={diff}")
                    return False

    return True


# ---------------------------------------------------------------------------
# 6. Optimal Merge Pattern (Exercise 2)
# ---------------------------------------------------------------------------

def optimal_merge(files: list[int]) -> tuple[int, list[tuple[int, int, int]]]:
    """Merge files minimising total cost using the greedy (Huffman) strategy.

    Returns (total_cost, merge_log) where each merge log entry is
    (file_a_size, file_b_size, merged_size).

    >>> optimal_merge([8, 4, 6, 12])
    (58, ...)
    """
    if len(files) <= 1:
        return 0, []

    heap = list(files)
    heapq.heapify(heap)
    total_cost = 0
    log: list[tuple[int, int, int]] = []

    while len(heap) > 1:
        a = heapq.heappop(heap)
        b = heapq.heappop(heap)
        merged = a + b
        total_cost += merged
        log.append((a, b, merged))
        heapq.heappush(heap, merged)

    return total_cost, log


# ---------------------------------------------------------------------------
# Demo
# ---------------------------------------------------------------------------

def main() -> None:
    print("=" * 60)
    print("GREEDY ALGORITHMS & MATROIDS")
    print("=" * 60)

    # --- Activity Selection ---
    print("\n--- Activity Selection ---")
    activities = [(1, 4), (3, 5), (0, 6), (5, 7), (3, 9),
                  (5, 9), (6, 10), (8, 11), (8, 12), (2, 14), (12, 16)]
    result = activity_selection(activities)
    print(f"  Activities: {activities}")
    print(f"  Selected:   {result}  ({len(result)} activities)")

    # --- Huffman Coding ---
    print("\n--- Huffman Coding ---")
    text = "greedy algorithms are greedy because they make greedy choices"
    print(f"  Text: \"{text}\"")
    encoded, codes, tree = huffman_encode(text)
    decoded = huffman_decode(encoded, tree)
    print(f"  Decoded matches original: {decoded == text}")

    # --- Fractional Knapsack ---
    print("\n--- Fractional Knapsack ---")
    items = [(60, 10), (100, 20), (120, 30)]
    capacity = 50
    value, taken = fractional_knapsack(items, capacity)
    print(f"  Items (value, weight): {items}")
    print(f"  Capacity: {capacity}")
    print(f"  Max value: {value}")
    print(f"  Taken: {[(items[i], f'{frac:.0%}') for i, frac in taken]}")

    # --- Job Sequencing ---
    print("\n--- Job Sequencing ---")
    jobs = [(4, 20), (1, 10), (1, 40), (1, 30), (3, 50)]
    profit, slots = job_sequencing(jobs)
    print(f"  Jobs (deadline, profit): {jobs}")
    print(f"  Max profit: {profit}")
    print(f"  Slot assignments: {slots}")

    # --- Greedy vs DP: Coin Change ---
    print("\n--- Coin Change: Greedy vs Optimal ---")
    def greedy_change(coins, target):
        coins_sorted = sorted(coins, reverse=True)
        result = []
        for c in coins_sorted:
            while target >= c:
                target -= c
                result.append(c)
        return result

    print(f"  Coins [1,5,10], target 12:")
    g1 = greedy_change([1, 5, 10], 12)
    print(f"    Greedy: {g1} = {len(g1)} coins")

    print(f"  Coins [1,3,4], target 6:")
    g2 = greedy_change([1, 3, 4], 6)
    print(f"    Greedy:  {g2} = {len(g2)} coins")
    print(f"    Optimal: [3, 3] = 2 coins")
    print(f"    Greedy FAILS — canonical property does not hold for [1,3,4]")

    # --- Matroid Verification ---
    print("\n--- Matroid Verification ---")

    # Uniform matroid U(2,4): independent if |S| <= 2
    print("  U(2,4) — subsets of size <= 2:")
    u24 = is_matroid([1, 2, 3, 4], lambda s: len(s) <= 2)
    print(f"    Is matroid: {u24}")

    # Graphic matroid on K3 (triangle): edges {a,b,c}, independent if acyclic
    print("  Graphic matroid on K3 (triangle):")
    edges = ["ab", "bc", "ac"]
    def is_forest(subset):
        # A subset of edges of K3 is acyclic iff it has < 3 edges
        # (the only cycle is the full triangle)
        return len(subset) < 3
    gm = is_matroid(edges, is_forest)
    print(f"    Is matroid: {gm}")

    # Non-matroid: hereditary fails
    print("  Non-matroid (hereditary fails):")
    nm = is_matroid([1, 2, 3], lambda s: len(s) <= 1 or set(s) == {1, 2})
    print(f"    Is matroid: {nm}")

    # --- Optimal Merge Pattern ---
    print("\n--- Optimal Merge Pattern ---")
    files = [8, 4, 6, 12]
    cost, log = optimal_merge(files)
    print(f"  File sizes: {files}")
    print(f"  Merge log: {log}")
    print(f"  Total cost: {cost}")

    print("\n" + "=" * 60)
    print("Done.")


if __name__ == "__main__":
    main()
