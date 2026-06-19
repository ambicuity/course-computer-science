# Dynamic Programming II — 2D and Beyond

> Dynamic Programming II — 2D and Beyond — the part of CS you can't skip.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 04 lessons 01–08
**Time:** ~75 minutes

## Learning Objectives

- Understand the core concept introduced in this lesson and why it matters.
- Implement the lesson's "Build It" artifact from scratch in one of: Python.
- Compare your from-scratch implementation against the production tool used in industry.
- Ship the reusable artifact (see "Ship It") and add it to your toolbox.

## The Problem

Lesson 08 covered 1D DP — one index, one dimension. But most real-world DP problems have **two or more dimensions**. Comparing two DNA sequences, filling a knapsack, multiplying a chain of matrices — each requires a 2D table of subproblems. Without 2D DP you cannot solve LCS, edit distance, knapsack, or matrix chain — the problems that dominate interviews and underpin production tools like `diff`, genomic aligners, and compilers.

## The Concept

### 2D state spaces: two indices instead of one

In 1D DP the state is `dp[i]` — one index. In 2D DP the state is `dp[i][j]` — two indices, typically one per input sequence (or row/column of a grid, or start/end of an interval).

Every 2D DP problem reduces to three questions:
1. What does `dp[i][j]` represent? (Define the state.)
2. How do I express `dp[i][j]` using smaller subproblems? (Write the recurrence.)
3. In what order do I fill the table? (Determine iteration order.)

### Problem 1 — Longest Common Subsequence (LCS)

**State:** `dp[i][j]` = LCS length of `a[:i]` and `b[:j]`. **Recurrence:** match → `dp[i-1][j-1]+1`, else `max(dp[i-1][j], dp[i][j-1])`. **Time:** O(mn).

**Traceback:** Walk backwards from `dp[m][n]`. Follow diagonal on match, follow the direction of the max otherwise. Reconstructs the actual LCS string.

### Problem 2 — Edit Distance (Levenshtein)

**State:** `dp[i][j]` = min edits to transform `a[:i]` into `b[:j]`. **Recurrence:** match → `dp[i-1][j-1]`, else `1 + min(delete, insert, replace)`. **Base:** `dp[i][0]=i`, `dp[0][j]=j`. **Time:** O(mn). This is the foundation of every `diff` tool.

### Problem 3 — 0/1 Knapsack

**State:** `dp[i][w]` = max value using first `i` items with capacity `w`. **Recurrence:** `max(dp[i-1][w], dp[i-1][w-wt]+val)`. **Time:** O(nW), pseudo-polynomial.

**Traceback:** If `dp[i][w] != dp[i-1][w]`, item `i` was included.

### Problem 4 — Unbounded Knapsack

Same as 0/1 but items can be reused. Include term uses `dp[i][w-wt]` (same row, not `i-1`). Coin change is a special case.

### Problem 5 — Matrix Chain Multiplication

**State:** `dp[i][j]` = min scalar multiplications for `A_i × ... × A_j`. **Recurrence:** try every split `k`: `dp[i][k] + dp[k+1][j] + dims[i-1]*dims[k]*dims[j]`. **Time:** O(n³). Interval DP — subproblems are contiguous sub-arrays. Store optimal `k` per `[i][j]` for parenthesization traceback.

### Space optimization: rolling arrays

When `dp[i][j]` depends only on row `i-1`, collapse to 1–2 rows. LCS: O(mn) → O(min(m,n)). Trade-off: lose traceback without re-computation.

## Build It

All implementations with full traceback live in `code/main.py`. Here are the core recurrences:

### Step 1: LCS with traceback

```python
def lcs(a: str, b: str) -> str:
    m, n = len(a), len(b)
    dp = [[0] * (n + 1) for _ in range(m + 1)]
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if a[i - 1] == b[j - 1]:
                dp[i][j] = dp[i - 1][j - 1] + 1
            else:
                dp[i][j] = max(dp[i - 1][j], dp[i][j - 1])
    # traceback: walk backwards following diagonal on match, max direction otherwise
    result, i, j = [], m, n
    while i > 0 and j > 0:
        if a[i - 1] == b[j - 1]:
            result.append(a[i - 1]); i -= 1; j -= 1
        elif dp[i - 1][j] >= dp[i][j - 1]:
            i -= 1
        else:
            j -= 1
    return "".join(reversed(result))
```

### Step 2: Edit distance with alignment

```python
def edit_distance(a: str, b: str) -> tuple[int, list[str]]:
    m, n = len(a), len(b)
    dp = [[0] * (n + 1) for _ in range(m + 1)]
    for i in range(m + 1): dp[i][0] = i
    for j in range(n + 1): dp[0][j] = j
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if a[i - 1] == b[j - 1]:
                dp[i][j] = dp[i - 1][j - 1]
            else:
                dp[i][j] = 1 + min(dp[i - 1][j], dp[i][j - 1], dp[i - 1][j - 1])
    # traceback recovers insert/delete/replace operations
    ...
```

### Step 3: 0/1 Knapsack — traceback to find selected items

```python
selected, w = [], W
for i in range(n, 0, -1):
    if dp[i][w] != dp[i - 1][w]:
        selected.append(i - 1); w -= items[i - 1][0]
```

### Step 4: Space-optimized LCS — O(min(m,n)) instead of O(mn)

```python
prev = [0] * (len(b) + 1)
for ch_a in a:
    curr = [0] * (len(b) + 1)
    for j in range(1, len(b) + 1):
        if ch_a == b[j - 1]: curr[j] = prev[j - 1] + 1
        else: curr[j] = max(prev[j], curr[j - 1])
    prev = curr
return prev[len(b)]
```

## Use It

Production tools that rely on 2D DP:

- **`git diff` / `diff`** — Myers diff, an optimized edit distance variant (O(n·d) where d is edit count).
- **`vimdiff`**, **VS Code merge editor** — Same foundation with hunk/context heuristics.
- **Biopython's `pairwise2`** — Needleman-Wunsch with scoring matrices (BLOSUM, PAM).
- **Compilers** — GCC/LLVM use matrix-chain-style optimal parenthesization.

Our implementations return the core answer and traceback. Production adds formatting, scoring matrices, heuristic pruning, and incremental updates.

## Read the Source

- [Python `difflib`](https://github.com/python/cpython/blob/main/Lib/difflib.py) — SequenceMatcher uses a modified LCS with quick-ratio optimizations.
- [Biopython `pairwise2`](https://github.com/biopython/biopython/blob/master/Bio/pairwise2.py) — Needleman-Wunsch with affine gap penalties.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained reference snippet you can reuse in later phases.**

## Exercises

1. **Easy** — Longest palindromic subsequence. Hint: LCS of `s` and `s[::-1]`, or interval DP: `dp[i][j]` = longest palindromic subsequence of `s[i:j+1]`.

2. **Medium** — Minimum path sum in a grid. `m × n` grid, top-left to bottom-right (right/down only), minimize sum. State: `dp[i][j] = grid[i][j] + min(dp[i-1][j], dp[i][j-1])`.

3. **Hard** — Egg drop. `k` eggs, `n` floors. `dp[k][n] = 1 + min_{x} max(dp[k-1][x-1], dp[k][n-x])`. Optimize from O(kn²) to O(kn log n) via binary search on `x`.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| 2D DP | "two-dimensional DP" | DP table indexed by two variables, solving problems over pairs of inputs or grids |
| Traceback | "reconstruct the solution" | Walk backwards through the DP table to recover the optimal solution |
| Rolling array | "space optimization" | Reuse 1–2 rows instead of the full table, reducing space to O(min(m,n)) |
| Interval DP | "range DP" | State `dp[i][j]` for a contiguous sub-problem, solved by trying split points |
| Pseudo-polynomial | "polynomial in value" | Runtime depends on numeric magnitude (O(nW)), not bit-length |

## Further Reading

- Cormen et al., *Introduction to Algorithms*, 4th ed., Chapters 14–15.
- Skiena, *The Algorithm Design Manual*, Chapter 8.
- [LeetCode DP tag](https://leetcode.com/tag/dynamic-programming/).
