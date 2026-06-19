"""
Dynamic Programming I — 1D, Memoization, Tabulation
Phase 04 — Algorithms & Complexity Analysis

Five canonical 1D DP problems with memoization, tabulation,
state-transition printing, and correctness checks.
"""

from __future__ import annotations

import bisect
import functools
from typing import List, Tuple


# ---------------------------------------------------------------------------
# 1. Fibonacci — memoization vs tabulation with call counting
# ---------------------------------------------------------------------------

class _CallCounter:
    """Global counter to track how many fib calls are made."""
    count = 0


def fib_naive(n: int) -> int:
    """Naive recursive Fibonacci — O(2^n)."""
    _CallCounter.count += 1
    if n <= 1:
        return n
    return fib_naive(n - 1) + fib_naive(n - 2)


def fib_memo(n: int, cache: dict[int, int] | None = None) -> int:
    """Memoized Fibonacci — O(n) time, O(n) space."""
    if cache is None:
        cache = {}
    _CallCounter.count += 1
    if n <= 1:
        return n
    if n not in cache:
        cache[n] = fib_memo(n - 1, cache) + fib_memo(n - 2, cache)
    return cache[n]


def fib_tab(n: int) -> int:
    """Tabulated Fibonacci — O(n) time, O(n) space."""
    if n <= 1:
        return n
    dp = [0] * (n + 1)
    dp[1] = 1
    for i in range(2, n + 1):
        dp[i] = dp[i - 1] + dp[i - 2]
    return dp[n]


def fib_tab_fibonacci_states(n: int) -> None:
    """Print the tabulation states for Fibonacci."""
    if n <= 1:
        print(f"  dp[{n}] = {n}")
        return
    dp = [0] * (n + 1)
    dp[1] = 1
    print(f"  dp[0] = {dp[0]}")
    print(f"  dp[1] = {dp[1]}")
    for i in range(2, n + 1):
        dp[i] = dp[i - 1] + dp[i - 2]
        print(f"  dp[{i}] = dp[{i-1}] + dp[{i-2}] = {dp[i-1]} + {dp[i-2]} = {dp[i]}")


# ---------------------------------------------------------------------------
# 2. Coin Change — minimum coins with solution reconstruction
# ---------------------------------------------------------------------------

def coin_change(coins: List[int], amount: int) -> Tuple[int, List[int]]:
    """
    Return (min_coins, chosen_coins) to make `amount` with given denominations.
    Returns (-1, []) if impossible.

    State:  dp[a] = minimum coins to make amount a
    Base:   dp[0] = 0
    Trans:  dp[a] = 1 + min(dp[a - c]) for each coin c <= a
    """
    INF = amount + 1
    dp = [INF] * (amount + 1)
    parent = [-1] * (amount + 1)  # which coin was chosen at each amount
    dp[0] = 0

    for a in range(1, amount + 1):
        for c in coins:
            if c <= a and dp[a - c] + 1 < dp[a]:
                dp[a] = dp[a - c] + 1
                parent[a] = c

    if dp[amount] >= INF:
        return -1, []

    # Reconstruct solution
    result = []
    a = amount
    while a > 0:
        result.append(parent[a])
        a -= parent[a]
    return dp[amount], result


def coin_change_states(coins: List[int], amount: int) -> None:
    """Print the tabulation states for coin change."""
    INF = amount + 1
    dp = [INF] * (amount + 1)
    parent = [-1] * (amount + 1)
    dp[0] = 0
    print(f"  dp[0] = 0")

    for a in range(1, amount + 1):
        for c in coins:
            if c <= a and dp[a - c] + 1 < dp[a]:
                dp[a] = dp[a - c] + 1
                parent[a] = c
        if dp[a] < INF:
            print(f"  dp[{a}] = {dp[a]}  (used coin {parent[a]})")
        else:
            print(f"  dp[{a}] = inf  (impossible)")


# ---------------------------------------------------------------------------
# 3. Longest Increasing Subsequence — O(n^2) DP + O(n log n) patience
# ---------------------------------------------------------------------------

def lis_dp(arr: List[int]) -> Tuple[int, List[int]]:
    """
    O(n^2) DP. Return (length, actual_subsequence).

    State:  dp[i] = length of LIS ending at index i
    Base:   dp[i] = 1 for all i
    Trans:  dp[i] = 1 + max(dp[j]) for j < i and arr[j] < arr[i]
    """
    n = len(arr)
    if n == 0:
        return 0, []
    dp = [1] * n
    parent = [-1] * n

    for i in range(1, n):
        for j in range(i):
            if arr[j] < arr[i] and dp[j] + 1 > dp[i]:
                dp[i] = dp[j] + 1
                parent[i] = j

    # Find endpoint of the best subsequence
    best_len = max(dp)
    best_end = dp.index(best_len)

    # Reconstruct
    subseq = []
    idx = best_end
    while idx != -1:
        subseq.append(arr[idx])
        idx = parent[idx]
    subseq.reverse()
    return best_len, subseq


def lis_patience(arr: List[int]) -> int:
    """
    O(n log n) patience sorting variant.
    Returns the LENGTH of the LIS (not the actual subsequence).
    """
    tails: list[int] = []
    for x in arr:
        pos = bisect.bisect_left(tails, x)
        if pos == len(tails):
            tails.append(x)
        else:
            tails[pos] = x
    return len(tails)


def lis_dp_states(arr: List[int]) -> None:
    """Print the tabulation states for LIS."""
    n = len(arr)
    if n == 0:
        print("  (empty array)")
        return
    dp = [1] * n
    parent = [-1] * n
    print(f"  dp[0] = 1  (arr[0]={arr[0]})")

    for i in range(1, n):
        for j in range(i):
            if arr[j] < arr[i] and dp[j] + 1 > dp[i]:
                dp[i] = dp[j] + 1
                parent[i] = j
        src = f", extend from j={parent[i]}" if parent[i] != -1 else ""
        print(f"  dp[{i}] = {dp[i]}  (arr[{i}]={arr[i]}{src})")


# ---------------------------------------------------------------------------
# 4. House Robber — non-adjacent max sum
# ---------------------------------------------------------------------------

def house_robber(nums: List[int]) -> Tuple[int, List[int]]:
    """
    Return (max_money, houses_robbed).

    State:  dp[i] = max money from houses 0..i
    Base:   dp[0] = nums[0],  dp[1] = max(nums[0], nums[1])
    Trans:  dp[i] = max(dp[i-1], dp[i-2] + nums[i])
    """
    n = len(nums)
    if n == 0:
        return 0, []
    if n == 1:
        return nums[0], [0]

    dp = [0] * n
    dp[0] = nums[0]
    dp[1] = max(nums[0], nums[1])

    for i in range(2, n):
        dp[i] = max(dp[i - 1], dp[i - 2] + nums[i])

    # Reconstruct: walk backwards
    robbed = []
    i = n - 1
    while i >= 0:
        if i == 0:
            robbed.append(i)
            break
        if i == 1:
            if nums[1] >= nums[0]:
                robbed.append(i)
            else:
                robbed.append(0)
            break
        if dp[i - 2] + nums[i] >= dp[i - 1]:
            robbed.append(i)
            i -= 2
        else:
            i -= 1
    robbed.reverse()
    return dp[n - 1], robbed


def house_robber_states(nums: List[int]) -> None:
    """Print the tabulation states for house robber."""
    n = len(nums)
    if n == 0:
        print("  (no houses)")
        return
    dp = [0] * n
    dp[0] = nums[0]
    print(f"  dp[0] = {dp[0]}  (nums[0]={nums[0]})")
    if n >= 2:
        dp[1] = max(nums[0], nums[1])
        print(f"  dp[1] = max({nums[0]}, {nums[1]}) = {dp[1]}")
    for i in range(2, n):
        dp[i] = max(dp[i - 1], dp[i - 2] + nums[i])
        pick = f"dp[{i-2}]+nums[{i}]={dp[i-2]}+{nums[i]}={dp[i-2]+nums[i]}"
        skip = f"dp[{i-1}]={dp[i-1]}"
        winner = "pick" if dp[i - 2] + nums[i] >= dp[i - 1] else "skip"
        print(f"  dp[{i}] = max({skip}, {pick}) = {dp[i]}  ({winner})")


# ---------------------------------------------------------------------------
# 5. Climbing Stairs — counting paths (Fibonacci variant)
# ---------------------------------------------------------------------------

def climb_stairs(n: int) -> int:
    """
    Number of distinct ways to climb n stairs (1 or 2 steps at a time).

    State:  dp[i] = ways to reach step i
    Base:   dp[0] = 1,  dp[1] = 1
    Trans:  dp[i] = dp[i-1] + dp[i-2]
    """
    if n <= 1:
        return 1
    dp = [0] * (n + 1)
    dp[0] = 1
    dp[1] = 1
    for i in range(2, n + 1):
        dp[i] = dp[i - 1] + dp[i - 2]
    return dp[n]


def climb_stairs_states(n: int) -> None:
    """Print the tabulation states for climbing stairs."""
    if n <= 1:
        print(f"  dp[{n}] = 1")
        return
    dp = [0] * (n + 1)
    dp[0] = 1
    dp[1] = 1
    print(f"  dp[0] = 1")
    print(f"  dp[1] = 1")
    for i in range(2, n + 1):
        dp[i] = dp[i - 1] + dp[i - 2]
        print(f"  dp[{i}] = dp[{i-1}] + dp[{i-2}] = {dp[i-1]} + {dp[i-2]} = {dp[i]}")


# ---------------------------------------------------------------------------
# Main — run all with correctness checks
# ---------------------------------------------------------------------------

def main() -> None:
    passed = 0
    failed = 0

    def check(label: str, got, expected):
        nonlocal passed, failed
        if got == expected:
            print(f"  PASS  {label}")
            passed += 1
        else:
            print(f"  FAIL  {label}: got {got}, expected {expected}")
            failed += 1

    # -- Fibonacci -----------------------------------------------------------
    print("=" * 60)
    print("1. FIBONACCI")
    print("=" * 60)

    fib_expected = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55]
    for i, exp in enumerate(fib_expected):
        check(f"fib_memo({i})", fib_memo(i), exp)
        check(f"fib_tab({i})", fib_tab(i), exp)

    # Call count comparison
    print("\n  Call count comparison (fib(20)):")
    _CallCounter.count = 0
    fib_naive(20)
    naive_calls = _CallCounter.count
    _CallCounter.count = 0
    fib_memo(20)
    memo_calls = _CallCounter.count
    _CallCounter.count = 0
    fib_tab(20)
    tab_calls = _CallCounter.count
    print(f"    Naive:  {naive_calls:,} recursive calls")
    print(f"    Memo:   {memo_calls:,} calls (with cache)")
    print(f"    Tab:    {tab_calls} (loop iterations, no recursion)")

    print("\n  State transition for fib(8):")
    fib_tab_fibonacci_states(8)

    # -- Coin Change ---------------------------------------------------------
    print("\n" + "=" * 60)
    print("2. COIN CHANGE")
    print("=" * 60)

    n_coins, coins_used = coin_change([1, 3, 4], 6)
    check("coin_change([1,3,4], 6) min coins", n_coins, 2)
    check("coin_change([1,3,4], 6) sum", sum(coins_used), 6)

    n_coins2, _ = coin_change([2], 3)
    check("coin_change([2], 3) impossible", n_coins2, -1)

    n_coins3, coins3 = coin_change([1, 5, 10, 25], 30)
    check("coin_change([1,5,10,25], 30) min coins", n_coins3, 2)

    print("\n  State transition for coin_change([1,3,4], 6):")
    coin_change_states([1, 3, 4], 6)

    # -- LIS -----------------------------------------------------------------
    print("\n" + "=" * 60)
    print("3. LONGEST INCREASING SUBSEQUENCE")
    print("=" * 60)

    test_arr = [10, 9, 2, 5, 3, 7, 101, 18]
    lis_len, lis_subseq = lis_dp(test_arr)
    check("lis_dp length", lis_len, 4)
    check("lis_patience length", lis_patience(test_arr), 4)
    # Verify the returned subsequence is valid
    check("lis_dp subsequence length matches", len(lis_subseq), 4)
    is_increasing = all(lis_subseq[i] < lis_subseq[i + 1] for i in range(len(lis_subseq) - 1))
    check("lis_dp subsequence is increasing", is_increasing, True)

    check("lis_dp([])", lis_dp([])[0], 0)
    check("lis_dp([1])", lis_dp([1])[0], 1)
    check("lis_dp([5,4,3,2,1])", lis_dp([5, 4, 3, 2, 1])[0], 1)
    check("lis_patience([1,2,3,4,5])", lis_patience([1, 2, 3, 4, 5]), 5)

    print("\n  State transition for lis_dp([10,9,2,5,3,7,101,18]):")
    lis_dp_states(test_arr)

    # -- House Robber --------------------------------------------------------
    print("\n" + "=" * 60)
    print("4. HOUSE ROBBER")
    print("=" * 60)

    money, houses = house_robber([2, 7, 9, 3, 1])
    check("house_robber([2,7,9,3,1]) max", money, 12)
    check("house_robber([2,7,9,3,1]) no adjacent",
          all(abs(houses[i] - houses[i + 1]) > 1 for i in range(len(houses) - 1)) if len(houses) > 1 else True,
          True)

    money2, _ = house_robber([1, 2, 3, 1])
    check("house_robber([1,2,3,1])", money2, 4)

    check("house_robber([])", house_robber([])[0], 0)
    check("house_robber([5])", house_robber([5])[0], 5)

    print("\n  State transition for house_robber([2,7,9,3,1]):")
    house_robber_states([2, 7, 9, 3, 1])

    # -- Climbing Stairs -----------------------------------------------------
    print("\n" + "=" * 60)
    print("5. CLIMBING STAIRS")
    print("=" * 60)

    climb_expected = [1, 1, 2, 3, 5, 8, 13, 21, 34, 55]
    for i, exp in enumerate(climb_expected):
        check(f"climb_stairs({i})", climb_stairs(i), exp)

    print("\n  State transition for climb_stairs(8):")
    climb_stairs_states(8)

    # -- Summary -------------------------------------------------------------
    print("\n" + "=" * 60)
    print(f"RESULTS: {passed} passed, {failed} failed")
    print("=" * 60)


if __name__ == "__main__":
    main()
