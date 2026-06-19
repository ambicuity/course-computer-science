"""
Dynamic Programming II — 2D and Beyond
Phase 04 — Algorithms & Complexity Analysis

Five classic 2D DP problems built from scratch with traceback/reconstruction.
Includes space-optimized variants for LCS and edit distance.
"""


def lcs(a: str, b: str) -> str:
    m, n = len(a), len(b)
    dp = [[0] * (n + 1) for _ in range(m + 1)]
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if a[i - 1] == b[j - 1]:
                dp[i][j] = dp[i - 1][j - 1] + 1
            else:
                dp[i][j] = max(dp[i - 1][j], dp[i][j - 1])
    result = []
    i, j = m, n
    while i > 0 and j > 0:
        if a[i - 1] == b[j - 1]:
            result.append(a[i - 1])
            i -= 1
            j -= 1
        elif dp[i - 1][j] >= dp[i][j - 1]:
            i -= 1
        else:
            j -= 1
    return "".join(reversed(result))


def lcs_space_opt(a: str, b: str) -> int:
    if len(a) < len(b):
        a, b = b, a
    n = len(b)
    prev = [0] * (n + 1)
    for ch_a in a:
        curr = [0] * (n + 1)
        for j in range(1, n + 1):
            if ch_a == b[j - 1]:
                curr[j] = prev[j - 1] + 1
            else:
                curr[j] = max(prev[j], curr[j - 1])
        prev = curr
    return prev[n]


def edit_distance(a: str, b: str) -> tuple[int, list[str]]:
    m, n = len(a), len(b)
    dp = [[0] * (n + 1) for _ in range(m + 1)]
    for i in range(m + 1):
        dp[i][0] = i
    for j in range(n + 1):
        dp[0][j] = j
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if a[i - 1] == b[j - 1]:
                dp[i][j] = dp[i - 1][j - 1]
            else:
                dp[i][j] = 1 + min(dp[i - 1][j], dp[i][j - 1], dp[i - 1][j - 1])
    ops = []
    i, j = m, n
    while i > 0 or j > 0:
        if i > 0 and j > 0 and a[i - 1] == b[j - 1]:
            ops.append(f"  {a[i - 1]}")
            i -= 1
            j -= 1
        elif i > 0 and j > 0 and dp[i][j] == dp[i - 1][j - 1] + 1:
            ops.append(f"R {a[i - 1]}>{b[j - 1]}")
            i -= 1
            j -= 1
        elif i > 0 and dp[i][j] == dp[i - 1][j] + 1:
            ops.append(f"D {a[i - 1]}")
            i -= 1
        else:
            ops.append(f"I {b[j - 1]}")
            j -= 1
    return dp[m][n], list(reversed(ops))


def edit_distance_space_opt(a: str, b: str) -> int:
    if len(a) < len(b):
        a, b = b, a
    n = len(b)
    prev = list(range(n + 1))
    for ch_a in a:
        curr = [0] * (n + 1)
        curr[0] = prev[0] + 1
        for j in range(1, n + 1):
            if ch_a == b[j - 1]:
                curr[j] = prev[j - 1]
            else:
                curr[j] = 1 + min(prev[j], curr[j - 1], prev[j - 1])
        prev = curr
    return prev[n]


def knapsack_01(items: list[tuple[int, int]], W: int) -> tuple[int, list[int]]:
    n = len(items)
    dp = [[0] * (W + 1) for _ in range(n + 1)]
    for i in range(1, n + 1):
        wt, val = items[i - 1]
        for w in range(W + 1):
            dp[i][w] = dp[i - 1][w]
            if w >= wt:
                dp[i][w] = max(dp[i][w], dp[i - 1][w - wt] + val)
    selected = []
    w = W
    for i in range(n, 0, -1):
        if dp[i][w] != dp[i - 1][w]:
            selected.append(i - 1)
            w -= items[i - 1][0]
    return dp[n][W], list(reversed(selected))


def knapsack_unbounded(items: list[tuple[int, int]], W: int) -> int:
    dp = [0] * (W + 1)
    for w in range(1, W + 1):
        for wt, val in items:
            if w >= wt:
                dp[w] = max(dp[w], dp[w - wt] + val)
    return dp[W]


def matrix_chain(dims: list[int]) -> tuple[int, str]:
    n = len(dims) - 1
    dp = [[0] * n for _ in range(n)]
    split = [[0] * n for _ in range(n)]
    for length in range(2, n + 1):
        for i in range(n - length + 1):
            j = i + length - 1
            dp[i][j] = float("inf")
            for k in range(i, j):
                cost = dp[i][k] + dp[k + 1][j] + dims[i] * dims[k + 1] * dims[j + 1]
                if cost < dp[i][j]:
                    dp[i][j] = cost
                    split[i][j] = k

    def build(i: int, j: int) -> str:
        if i == j:
            return f"A{i + 1}"
        k = split[i][j]
        left = build(i, k)
        right = build(k + 1, j)
        return f"({left} x {right})"

    return dp[0][n - 1], build(0, n - 1)


def main() -> None:
    # --- LCS ---
    a, b = "ABCBDAB", "BDCAB"
    print("=" * 60)
    print("Longest Common Subsequence")
    print("=" * 60)
    print(f"  a = {a}")
    print(f"  b = {b}")
    seq = lcs(a, b)
    print(f"  LCS = \"{seq}\" (length {len(seq)})")
    print(f"  LCS length (space-opt) = {lcs_space_opt(a, b)}")
    print()

    # --- Edit Distance ---
    a2, b2 = "kitten", "sitting"
    print("=" * 60)
    print("Edit Distance (Levenshtein)")
    print("=" * 60)
    print(f"  a = {a2}")
    print(f"  b = {b2}")
    dist, alignment = edit_distance(a2, b2)
    print(f"  Distance = {dist}")
    print("  Alignment:")
    for op in alignment:
        print(f"    {op}")
    print(f"  Distance (space-opt) = {edit_distance_space_opt(a2, b2)}")
    print()

    # --- 0/1 Knapsack ---
    items = [(2, 6), (2, 10), (3, 12), (7, 5), (1, 4)]
    capacity = 15
    print("=" * 60)
    print("0/1 Knapsack")
    print("=" * 60)
    print(f"  Items (weight, value): {items}")
    print(f"  Capacity: {capacity}")
    best, chosen = knapsack_01(items, capacity)
    print(f"  Max value = {best}")
    print(f"  Selected items: {[items[i] for i in chosen]}")
    print()

    # --- Unbounded Knapsack (coin change) ---
    coins = [(1, 1), (3, 1), (4, 1)]
    target = 6
    print("=" * 60)
    print("Unbounded Knapsack (min coins for target)")
    print("=" * 60)
    print(f"  Coins: {[c[0] for c in coins]}")
    print(f"  Target amount: {target}")
    result = knapsack_unbounded(coins, target)
    print(f"  Max value (= min coins with value=1) = {result}")
    print()

    # --- Matrix Chain Multiplication ---
    dims = [30, 35, 15, 5, 10, 20, 25]
    print("=" * 60)
    print("Matrix Chain Multiplication")
    print("=" * 60)
    print(f"  Dimensions: {dims}")
    cost, paren = matrix_chain(dims)
    print(f"  Minimum scalar multiplications = {cost}")
    print(f"  Optimal parenthesization: {paren}")


if __name__ == "__main__":
    main()
