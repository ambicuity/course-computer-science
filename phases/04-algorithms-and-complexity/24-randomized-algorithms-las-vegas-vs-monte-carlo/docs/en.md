# Randomized Algorithms — Las Vegas vs Monte Carlo

> Flip a coin inside the algorithm — get guaranteed correctness or guaranteed speed, never both for free.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 04 lessons 01–23
**Time:** ~75 minutes

## Learning Objectives

- Distinguish Las Vegas (always correct, randomized time) from Monte Carlo (bounded time, probabilistic correctness)
- Implement randomized quicksort, randomized select, Miller-Rabin, and Karger's min-cut from scratch
- Analyse expected running times using linearity of expectation
- Apply randomized algorithms to cryptographic primality testing and graph problems

## The Problem

Deterministic algorithms can be attacked. An adversary feeds sorted input to quicksort, forcing O(n²). A primality test checking every divisor up to √n is too slow for 2048-bit RSA keys. Randomization breaks adversarial patterns with guarantees that hold with overwhelming probability over the algorithm's coin flips.

## The Concept

### Two Families

| Property | Las Vegas | Monte Carlo |
|----------|-----------|-------------|
| Correctness | Always correct | May return wrong answer |
| Running time | Randomized (expected bound) | Deterministic bound |
| Examples | Randomized quicksort, select | Miller-Rabin, Karger's min-cut |

### Randomized Quicksort (Las Vegas)

Random pivot eliminates adversarial input. Expected comparisons:

```
E[comparisons] = 2n ln n ≈ 1.39n log₂ n
```

**Proof.** Xᵢⱼ = 1 if sorted elements i, j are compared. P(Xᵢⱼ = 1) = 2/(j−i+1). Linearity of expectation → E = Σ 2/(j−i+1) = 2n Hₙ. Worst case O(n²) with probability ≈ 1/n!.

### Randomized Select (Las Vegas)

Find kth smallest. Random pivot, recurse one side only. E[T(n)] ≤ 4n. Deterministic median-of-medians gives O(n) worst case but ~18n comparisons vs ~4n expected.

### Miller-Rabin Primality Test (Monte Carlo)

Write p−1 = 2^s·d (d odd). p passes base a if a^d ≡ 1 (mod p) or a^(2^r·d) ≡ −1 for some r < s. For composite n, ≤ 1/4 of bases are strong liars → P(error in k rounds) ≤ 4^(−k). For k=40: error ≤ 2^(−80). This is what `openssl genrsa` uses.

**Worked example.** 561 = 3×11×17 (Carmichael). Fermat: 2^560 mod 561 = 1 → wrongly says "probably prime." Miller-Rabin base 2: 560 = 2⁴×35, 2^35 mod 561 = 263 (not 1 or 560), squaring never hits 560 → correctly says "composite."

### Karger's Min-Cut (Monte Carlo)

Contract random edges until 2 vertices remain. P(finds min-cut) ≥ 2/(n(n−1)). Run n² ln n times → P(all failures) ≤ 1/n.

### Complexity Summary

| Algorithm | Type | Expected | Error |
|-----------|------|----------|-------|
| Randomized quicksort | Las Vegas | O(n log n) | 0 |
| Randomized select | Las Vegas | O(n) | 0 |
| Miller-Rabin | Monte Carlo | O(k log² n) | ≤ 4^(−k) |
| Karger's min-cut | Monte Carlo | O(n²m) per trial | ≤ 1/n² |

## Build It

### Step 1: Randomized Quicksort and Select

```python
import random

def randomized_quicksort(arr):
    comparisons = [0]
    def partition(lo, hi):
        ri = random.randint(lo, hi)
        arr[ri], arr[hi] = arr[hi], arr[ri]
        pivot = arr[hi]
        i = lo
        for j in range(lo, hi):
            comparisons[0] += 1
            if arr[j] <= pivot:
                arr[i], arr[j] = arr[j], arr[i]
                i += 1
        arr[i], arr[hi] = arr[hi], arr[i]
        return i
    def sort(lo, hi):
        if lo < hi:
            p = partition(lo, hi)
            sort(lo, p - 1)
            sort(p + 1, hi)
    sort(0, len(arr) - 1)
    return comparisons[0]

def randomized_select(arr, k):
    if len(arr) == 1:
        return arr[0]
    pivot = random.choice(arr)
    lows = [x for x in arr if x < pivot]
    highs = [x for x in arr if x > pivot]
    pivots = [x for x in arr if x == pivot]
    if k < len(lows):
        return randomized_select(lows, k)
    elif k < len(lows) + len(pivots):
        return pivot
    else:
        return randomized_select(highs, k - len(lows) - len(pivots))
```

### Step 2: Miller-Rabin Primality Test

```python
def miller_rabin(n, k=40):
    if n < 2: return False
    if n < 4: return True
    if n % 2 == 0: return False
    r, d = 0, n - 1
    while d % 2 == 0:
        r += 1; d //= 2
    for _ in range(k):
        a = random.randrange(2, n - 1)
        x = pow(a, d, n)
        if x == 1 or x == n - 1: continue
        for _ in range(r - 1):
            x = pow(x, 2, n)
            if x == n - 1: break
        else: return False
    return True
```

### Step 3: Karger's Min-Cut (see `code/main.py` for full version with repeated trials)

```python
def karger_min_cut(graph):
    import copy
    g = copy.deepcopy(graph)
    vertices = list(g.keys())
    while len(vertices) > 2:
        u = random.choice(vertices)
        v = random.choice(g[u])
        g[u].extend(g[v])
        for w in g[v]:
            g[w] = [x if x != v else u for x in g[w]]
        g[u] = [x for x in g[u] if x != u]
        vertices.remove(v)
    return len(g[vertices[0]])
```

## Use It

- **`openssl genrsa 2048`** generates RSA keys by running Miller-Rabin on random odd numbers until two large primes are found. Source: `crypto/bn/bn_prime.c`.
- **`random.shuffle`** uses Fisher-Yates — a Las Vegas permutation that is always valid.
- Production Miller-Rabin adds trial division by small primes to reject composites cheaply before expensive exponentiations.

## Ship It

The artifact is a **randomized algorithm toolkit** in `outputs/` bundling `randomized_quicksort`, `randomized_select`, `miller_rabin`, and `karger_min_cut`.

## Exercises

1. **Easy.** Prove expected comparisons in randomized quicksort is 2n Hₙ via the indicator variable argument. Verify numerically on arrays of size 100–10000 that comparisons/(2n ln n) → 1.
2. **Medium.** Implement the **Solovay-Strassen** primality test: n is probably prime if a^((n−1)/2) ≡ (a|n) (mod n) for random a, where (a|n) is the Jacobi symbol. Compare error bounds and speed against Miller-Rabin.
3. **Hard.** Implement Karger-Stein (recursive contraction with repeat until 2√n vertices remain, then branch). Achieve O(n² log³ n) expected time with P(failure) ≤ 1/n.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Las Vegas algorithm | "Randomized but always correct" | Random running time, zero error probability |
| Monte Carlo algorithm | "Probabilistic but fast" | Bounded time, bounded error probability |
| Strong pseudoprime | "Passes Miller-Rabin" | Composite n that fools base a — ≤ 1/4 of bases are liars |
| Carmichael number | "Fools Fermat" | Composite n where a^(n−1) ≡ 1 for all coprime a |
| Randomized contraction | "Merge random edges" | Karger's technique: contract random edge until 2 vertices |

## Further Reading

- Motwani & Raghavan, *Randomized Algorithms*, Cambridge, 1995.
- Cormen et al., *Introduction to Algorithms*, Ch. 5, 7, 31.
- [Miller-Rabin on Rosetta Code](https://rosettacode.org/wiki/Miller%E2%80%93Rabin_primality_test) — implementations in 40+ languages.
