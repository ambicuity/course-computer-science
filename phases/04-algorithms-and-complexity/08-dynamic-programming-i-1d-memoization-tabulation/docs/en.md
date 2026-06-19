# Dynamic Programming I — 1D, Memoization, Tabulation

> Dynamic Programming I — 1D, Memoization, Tabulation — the part of CS you can't skip.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 04 lessons 01–07
**Time:** ~75 minutes

## Learning Objectives

- Identify problems with **optimal substructure** and **overlapping subproblems** — the two pillars of DP.
- Implement both **memoization** (top-down) and **tabulation** (bottom-up) and know when each wins.
- Build five canonical 1D DP solutions from scratch: Fibonacci, Coin Change, LIS, House Robber, Climbing Stairs.
- Recognise the DP pattern in new problems and reconstruct actual solutions, not just values.

## The Problem

This lesson sits in **Phase 04 — Algorithms & Complexity Analysis**. Without the concept it teaches, you cannot
build the phase's capstone (An algorithms cookbook plus a benchmark harness.). Concretely, *not* knowing this means you get stuck the
moment you try to master the canon — sorting, dp, graphs, strings, geometry, randomization — and the analysis tools that bound them.

The next few sections walk through the smallest concrete scenario where this gap hurts, then build
the mental model, then the code, then the production equivalent.

## The Concept

### Two ingredients that make a problem a DP problem

1. **Optimal substructure** — the optimal solution to the problem contains optimal solutions to its sub-problems.
2. **Overlapping subproblems** — the same sub-problems are solved multiple times.

When both hold, you can solve each sub-problem **once**, store the answer, and reuse it. That is dynamic programming.

### Memoization vs Tabulation

```
Memoization (top-down)          Tabulation (bottom-up)
─────────────────────────       ─────────────────────────
Start from the original         Start from the smallest
problem, recurse, cache         sub-problem, fill a table
answers as you return.          row-by-row until you
                                reach the original problem.

  fib(5)                         dp[0]=0, dp[1]=1
  ├── fib(4)                     dp[2]=1, dp[3]=2
  │   ├── fib(3)                 dp[4]=3, dp[5]=5
  │   │   ├── fib(2)
  │   │   └── fib(1) ← cached
  │   └── fib(2) ← cached
  └── fib(3) ← cached
```

| Aspect | Memoization | Tabulation |
|--------|-------------|------------|
| Direction | Top-down (recursion) | Bottom-up (iteration) |
| Only computes needed states? | Yes | No (fills entire table) |
| Stack overflow risk? | Yes, for deep recursion | No |
| Overhead | Function call + dict lookup | Array index |
| Best when | Few sub-problems actually needed | All sub-problems needed |

### The DP toolkit: state, transition, base case

Every DP solution has three parts:

- **State** — what parameters identify a sub-problem? (e.g., `dp[i]` = answer for prefix `[0..i]`)
- **Transition** — how to compute `dp[i]` from previously computed states?
- **Base case** — what are the answers for the smallest sub-problems?

### Problem 1: Fibonacci — the gateway drug

**Naive** recursion: `fib(n) = fib(n-1) + fib(n-2)`. Time: O(2^n). Space: O(n) call stack.

```
                    fib(5)
                  /        \
              fib(4)       fib(3)      ← fib(3) computed twice
             /      \      /    \
         fib(3)   fib(2) fib(2) fib(1) ← fib(2) computed three times
        /    \
    fib(2)  fib(1)
```

The recursion tree has exponential overlap. Fix: store answers.

**Memoized:**

```python
def fib_memo(n, cache={}):
    if n <= 1:
        return n
    if n not in cache:
        cache[n] = fib_memo(n-1) + fib_memo(n-2)
    return cache[n]
```

**Tabulated:**

```python
def fib_tab(n):
    if n <= 1:
        return n
    dp = [0] * (n + 1)
    dp[1] = 1
    for i in range(2, n + 1):
        dp[i] = dp[i-1] + dp[i-2]
    return dp[n]
```

Both: **O(n) time, O(n) space** (tabulation can be O(1) space with two variables).

### Problem 2: Coin Change — minimum coins with reconstruction

Given coin denominations `coins` and a target `amount`, find the minimum number of coins needed and which coins to use.

**State:** `dp[a]` = minimum coins to make amount `a`.

**Transition:** `dp[a] = 1 + min(dp[a - c] for c in coins if a - c >= 0)`

**Base case:** `dp[0] = 0` (zero coins to make amount 0).

```
coins = [1, 3, 4], amount = 6

dp[0] = 0
dp[1] = 1 + dp[0] = 1  (use coin 1)
dp[2] = 1 + dp[1] = 2  (use coin 1)
dp[3] = 1 + dp[0] = 1  (use coin 3)  ← better than 1+dp[2]=3
dp[4] = 1 + dp[0] = 1  (use coin 4)  ← better than 1+dp[3]=2
dp[5] = 1 + dp[2] = 3  (use coin 3)
dp[6] = 1 + dp[3] = 2  (use coin 3)  ← answer: 2 coins (3+3)
```

**Reconstruction:** track which coin was chosen at each amount, then walk backwards from `amount` to `0`.

### Problem 3: Longest Increasing Subsequence (LIS)

Given `arr`, find the length of the longest strictly increasing subsequence.

**O(n²) DP:**

**State:** `dp[i]` = length of LIS ending at index `i`.

**Transition:** `dp[i] = 1 + max(dp[j] for j < i if arr[j] < arr[i])`, or `1` if no such `j`.

**Base case:** `dp[0] = 1`.

```
arr = [10, 9, 2, 5, 3, 7, 101, 18]

dp[0]=1   (10)
dp[1]=1   (9)
dp[2]=1   (2)
dp[3]=2   (2,5)
dp[4]=2   (2,3)
dp[5]=3   (2,3,7)
dp[6]=4   (2,3,7,101)
dp[7]=4   (2,3,7,18)

Answer: 4
```

**O(n log n) — Patience Sorting:**

Maintain a list `tails` where `tails[i]` is the smallest possible tail value for an increasing subsequence of length `i+1`. For each element, binary search for its insertion position.

```python
import bisect

def lis_patience(arr):
    tails = []
    for x in arr:
        pos = bisect.bisect_left(tails, x)
        if pos == len(tails):
            tails.append(x)
        else:
            tails[pos] = x
    return len(tails)
```

`tails` is NOT the actual LIS — it just has the same length.

### Problem 4: House Robber

Given `nums` (money in each house along a street), maximise theft without robbing two adjacent houses.

**State:** `dp[i]` = max money from houses `0..i`.

**Transition:** `dp[i] = max(dp[i-1], dp[i-2] + nums[i])`

Either skip house `i` (keep `dp[i-1]`), or rob it (add `nums[i]` to `dp[i-2]`).

**Base case:** `dp[0] = nums[0]`, `dp[1] = max(nums[0], nums[1])`.

```
nums = [2, 7, 9, 3, 1]

dp[0] = 2
dp[1] = max(2, 7) = 7
dp[2] = max(7, 2+9)  = 11
dp[3] = max(11, 7+3) = 11
dp[4] = max(11, 11+1) = 12

Answer: 12  (rob houses 0, 2, 4 → 2+9+1)
```

### Problem 5: Climbing Stairs

You can climb 1 or 2 steps at a time. How many distinct ways to reach step `n`?

**State:** `dp[i]` = number of ways to reach step `i`.

**Transition:** `dp[i] = dp[i-1] + dp[i-2]`

**Base case:** `dp[0] = 1`, `dp[1] = 1`.

This is Fibonacci in disguise.

```
n = 5

dp[0]=1, dp[1]=1, dp[2]=2, dp[3]=3, dp[4]=5, dp[5]=8

Answer: 8
```

### How to identify a DP problem

Ask yourself:

1. Does the problem ask for **min / max / count**?
2. Are there **constraints** (capacity, adjacency, ordering)?
3. Can you break the problem into **overlapping sub-problems**?

If yes to all three — it is probably DP.

Quick heuristic: if you are considering **exhaustive search with pruning**, DP is likely the optimisation you need.

## Build It

### Step 1: Memoization template

```python
from functools import lru_cache

def solve_memo(args):
    @lru_cache(maxsize=None)
    def dp(state):
        # base case
        if is_base(state):
            return base_value
        # transition
        return combine(dp(next_state_1), dp(next_state_2), ...)
    return dp(initial_state)
```

### Step 2: Tabulation template

```python
def solve_tab(args):
    n = size(args)
    dp = [initial_value] * (n + 1)
    dp[base] = base_value
    for i in range(base + 1, n + 1):
        dp[i] = combine(dp[prev_1], dp[prev_2], ...)
    return dp[n]
```

### Step 3: Space-optimised tabulation

When `dp[i]` depends only on a constant number of previous states:

```python
def solve_optimised(args):
    prev2, prev1 = base_values
    for i in range(2, n + 1):
        curr = combine(prev1, prev2)
        prev2, prev1 = prev1, curr
    return prev1
```

## Use It

DP is not in Python's standard library as a standalone tool, but the patterns appear everywhere:

- **`functools.lru_cache`** — the built-in memoization decorator. Use it for top-down DP without writing your own cache dict.
- **NumPy / Pandas** — vectorised operations over tabulated data are essentially tabulation on arrays.
- **Competitive programming** — Codeforces, LeetCode, AtCoder all have dedicated DP categories. The 1D problems in this lesson are the foundation for 2D, bitmask, and tree DP (Lesson 10).
- **Bioinformatics** — sequence alignment (Needleman-Wunsch, Smith-Waterman) is 2D DP on strings. The core state-transition logic is identical to what you build here.
- **Operations research** — shortest paths (Dijkstra is greedy DP), inventory management, scheduling.

### Memo vs Tab: practical advice

- **Start with memoization.** It is closer to the recursive definition and easier to get right.
- **Convert to tabulation** when you need to avoid recursion-depth limits or want space optimisation.
- **Use `lru_cache`** for quick prototyping; switch to explicit arrays for competition code (faster constant factor).

## Read the Source

- CPython `functools.py` — the `lru_cache` implementation. Look at the C-accelerated `_lru_cache_wrapper` for how production memoization avoids per-call dict overhead.
- Python `bisect` module — used in the O(n log n) LIS patience-sorting variant.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained DP reference module (`code/main.py`) with memoized + tabulated solutions for five canonical problems, state-transition printers, and correctness checks.**

## Exercises

1. **Decode Ways** (Medium) — A string of digits `'1'..'9'` can be decoded as A=1, B=2, ..., Z=26. Count the number of valid decodings. (Hint: `dp[i]` = ways to decode `s[:i]`. Check one-digit and two-digit endings.)
2. **Maximum Product Subarray** (Hard) — Given an integer array, find the contiguous subarray with the largest product. (Hint: track both `max_prod` and `min_prod` at each position — a negative number flips them.)
3. **Word Break** (Medium) — Given a string `s` and a dictionary `wordDict`, determine if `s` can be segmented into dictionary words. (Hint: `dp[i]` = whether `s[:i]` is breakable. Try every word ending at position `i`.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Optimal substructure | "The optimal answer contains optimal answers to sub-problems" | A problem where combining optimal solutions to sub-problems gives the optimal solution to the whole |
| Overlapping subproblems | "Same work done many times" | The recursive decomposition revisits the same sub-problems repeatedly |
| Memoization | "Top-down DP" or "cache the recursion" | Store results of expensive function calls and return the cached result on repeat calls |
| Tabulation | "Bottom-up DP" | Build a table iteratively from the smallest sub-problem to the target problem |
| State | "What does `dp[i]` mean?" | The parameters that uniquely identify a sub-problem |
| Transition | "The recurrence" | The formula that computes `dp[i]` from previously computed states |
| Base case | "The trivial sub-problem" | The known answer for the smallest input, used to bootstrap the recurrence |

## Further Reading

- Cormen et al., *Introduction to Algorithms* (CLRS), Chapter 15 — Dynamic Programming
- Skiena, *The Algorithm Design Manual*, Chapter 8 — DP worked examples
- Competitive Programmer's Handbook (Laaksonen), Chapter 7 — free PDF, concise DP treatment
- LeetCode Dynamic Programming study plan — curated problem set from easy to hard
