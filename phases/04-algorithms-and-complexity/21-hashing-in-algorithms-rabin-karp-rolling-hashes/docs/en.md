# Hashing in Algorithms — Rabin-Karp, Rolling Hashes

> Turn a substring search into a number comparison — slide the hash window, verify only on collision.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 04 lessons 01–20
**Time:** ~60 minutes

## Learning Objectives

- Understand polynomial rolling hashes and why O(1) per-window updates make substring search fast
- Implement Rabin-Karp from scratch and compare against naive O(nm) search
- Analyse collision probability via the birthday paradox and apply double hashing for safety
- Apply rolling hashes to longest common substring, distinct-substring counting, and plagiarism detection

## The Problem

Lesson 19 covered deterministic string matching (KMP, Z, Boyer-Moore). Those find a **single** pattern in O(n). But many real tasks need more: multiple patterns, approximate duplicates, or substring queries. A rolling hash turns every substring into an O(1)-comparable fingerprint.

## The Concept

### Polynomial Rolling Hash

Treat `s[0..m-1]` as a polynomial evaluated at base `p`:

```
H(s) = (s[0]·p^0 + s[1]·p^1 + … + s[m-1]·p^(m-1)) mod M
```

`p` is a prime (e.g. 131). `M` is a large prime (e.g. 10^9 + 7).

**Worked example — `"cab"`, p = 31, M = 10^9+7:**

| i | char | ascii | ascii · 31^i |
|---|------|-------|--------------|
| 0 | c    | 99    | 99 |
| 1 | a    | 97    | 3007 |
| 2 | b    | 98    | 93862 |

`H("cab") = (99 + 3007 + 93862) mod M = 96968`

### Rolling Update — O(1) per Shift

When the window slides right, the first char leaves and a new one enters:

```
H_new = ((H_old - s[old_start] · p^(m-1)) · p + s[new_end]) mod M
```

Precompute `p^(m-1) mod M` once. Each shift is a few integer ops — **O(1)**.

### Rabin-Karp Algorithm

1. Hash the pattern: `H_p`.
2. Hash the first window of text: `H_w`.
3. Slide `n − m + 1` times. If `H_w == H_p`, verify character-by-character. Roll forward.

| Case    | Time    | When |
|---------|---------|------|
| Average | O(n+m)  | Collisions rare — few verifications |
| Worst   | O(nm)   | Every window collides (adversarial) |

### Birthday Paradox and Double Hashing

With `q` strings hashed into `M` buckets, collision probability ≈ `q²/(2M)`. For `M = 10^9+7` and one million strings: `P ≈ 0.0005`.

**Double hashing** uses two independent `(p, M)` pairs. Collision requires both to match: `P ≈ (q²/(2M))²` — effectively zero for any practical input.

**Applications:** plagiarism detection (hash overlapping n-grams), longest common substring (binary search + hash set), longest duplicate substring, content addressing (git SHA-1).

## Build It

### Step 1: Polynomial Hash and Rabin-Karp

```python
def polynomial_hash(s, base=131, mod=10**9 + 7):
    h = 0
    for ch in s:
        h = (h * base + ord(ch)) % mod
    return h

def rolling_hash_search(text, pattern):
    n, m = len(text), len(pattern)
    if m > n or m == 0:
        return []
    base, mod = 131, 10**9 + 7
    h_pattern = polynomial_hash(pattern, base, mod)
    h_window = polynomial_hash(text[:m], base, mod)
    power = pow(base, m - 1, mod)
    matches = []
    for i in range(n - m + 1):
        if h_window == h_pattern:
            if text[i:i+m] == pattern:
                matches.append(i)
        if i < n - m:
            h_window = (h_window - ord(text[i]) * power) * base + ord(text[i + m])
            h_window %= mod
    return matches
```

### Step 2: Longest Common Substring via Binary Search + Hashing

Binary search on answer length L. For each L, hash all length-L substrings of s1 into a set, then check s2's length-L substrings against it. O(n log n) total.

```python
def longest_common_substring(s1, s2):
    base, mod = 131, 10**9 + 7

    def has_common(L):
        if L == 0:
            return True
        hashes = set()
        power = pow(base, L - 1, mod)
        h = polynomial_hash(s1[:L], base, mod)
        hashes.add(h)
        for i in range(1, len(s1) - L + 1):
            h = (h - ord(s1[i - 1]) * power) * base + ord(s1[i + L - 1])
            h %= mod
            hashes.add(h)
        h = polynomial_hash(s2[:L], base, mod)
        if h in hashes:
            return True
        for i in range(1, len(s2) - L + 1):
            h = (h - ord(s2[i - 1]) * power) * base + ord(s2[i + L - 1])
            h %= mod
            if h in hashes:
                return True
        return False

    lo, hi, best = 0, min(len(s1), len(s2)), 0
    while lo <= hi:
        mid = (lo + hi) // 2
        if has_common(mid):
            best = mid
            lo = mid + 1
        else:
            hi = mid - 1
    return best
```

### Step 3: Double Hashing (see `code/main.py` for full version)

Use two `(base, mod)` pairs — `(131, 10^9+7)` and `(137, 10^9+9)`. A match requires both hashes to agree. The collision probability drops from `P` to `P²`.

## Use It

- **`git`** uses SHA-1 for content addressing — every object identified by its hash. Same principle as polynomial rolling hash, just cryptographic.
- **Plagiarism detectors** (Turnitin, Moss) hash overlapping n-grams and flag shared fingerprints.
- **rsync** uses a rolling Adler-32 checksum to identify changed blocks, transferring only deltas.
- Production codebase use 64-bit moduli (wrapping arithmetic gives free modulo) and precomputed modular inverses. Your version is functionally equivalent — production adds speed, not new ideas.

## Read the Source

- **Git:** `builtin/hash-object.c` — SHA-1 content addressing; **rsync:** `checksum.c` — Adler-32 rolling hash.

## Ship It

The artifact is a **rolling-hash module** in `outputs/` bundling `polynomial_hash`, `rolling_hash_search`, `multi_pattern_search`, `longest_common_substring`, and double-hashing variants.

## Exercises

1. **Easy.** Implement `count_distinct_substrings(s)` using rolling hash: hash every substring of every length, collect into a set, return size. Verify against brute force on small strings.
2. **Medium.** Implement `longest_common_substring(s1, s2)` via binary search + double hashing. Test on strings of length 10 000+ and confirm O(log n) binary search rounds.
3. **Hard.** Extend to **longest duplicate substring** in a single string: binary search on length, rolling hash + suffix index set. Return any one duplicate. Prove O(n log n) expected.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Rolling hash | "Sliding window hash" | Polynomial hash updated in O(1) per shift |
| Polynomial hash | "Base-p hash" | H = Σ s[i]·p^i mod M — string as number in base p |
| Rabin-Karp | "Hash-based string search" | Slide rolling hash; verify on match. O(n+m) avg, O(nm) worst |
| Birthday paradox | "Collisions happen sooner than you think" | With √M samples in space M, collision probability ≈ 50% |
| Double hashing | "Two hashes are safer" | Two independent (p, M) pairs; collision drops to P² |

## Further Reading

- Cormen et al., *Introduction to Algorithms*, Ch. 32; Karp & Rabin, "Efficient Randomized Pattern-Matching Algorithms," 1987.
- [LeetCode 1044 — Longest Duplicate Substring](https://leetcode.com/problems/longest-duplicate-substring/) — rolling hash + binary search.
