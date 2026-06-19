"""
String Matching — KMP, Z, Boyer-Moore
Phase 04 — Algorithms & Complexity Analysis, Lesson 19

Implements:
  1. naive_search    — O(nm) brute force with comparison count
  2. kmp_search      — O(n+m) via failure function (LPS array)
  3. z_search        — O(n+m) via Z-array construction
  4. boyer_moore     — O(nm) worst, sublinear in practice via bad-char + good-suffix
  5. benchmark       — side-by-side step counting on realistic inputs
"""

import time


# ---------------------------------------------------------------------------
# 1. Naive search  O(n * m)
# ---------------------------------------------------------------------------

def naive_search(text: str, pattern: str) -> tuple[list[int], int]:
    """Brute-force string search. Returns (matches, comparison_count)."""
    n, m = len(text), len(pattern)
    matches: list[int] = []
    comparisons = 0
    for i in range(n - m + 1):
        j = 0
        while j < m:
            comparisons += 1
            if text[i + j] != pattern[j]:
                break
            j += 1
        if j == m:
            matches.append(i)
    return matches, comparisons


# ---------------------------------------------------------------------------
# 2. KMP  O(n + m)
# ---------------------------------------------------------------------------

def _build_lps(pattern: str) -> list[int]:
    """Build the longest-proper-prefix-suffix (failure) array."""
    m = len(pattern)
    lps = [0] * m
    length = 0  # length of the previous longest prefix-suffix
    i = 1
    while i < m:
        if pattern[i] == pattern[length]:
            length += 1
            lps[i] = length
            i += 1
        elif length:
            length = lps[length - 1]
        else:
            lps[i] = 0
            i += 1
    return lps


def kmp_search(text: str, pattern: str) -> tuple[list[int], int]:
    """KMP string search. Returns (matches, comparison_count)."""
    n, m = len(text), len(pattern)
    if m == 0:
        return list(range(n + 1)), 0
    lps = _build_lps(pattern)
    matches: list[int] = []
    comparisons = 0
    i = 0  # text index
    j = 0  # pattern index
    while i < n:
        comparisons += 1
        if text[i] == pattern[j]:
            i += 1
            j += 1
        if j == m:
            matches.append(i - j)
            j = lps[j - 1]
        elif i < n and text[i] != pattern[j]:
            if j != 0:
                j = lps[j - 1]
            else:
                i += 1
    return matches, comparisons


# ---------------------------------------------------------------------------
# 3. Z-algorithm  O(n + m)
# ---------------------------------------------------------------------------

def _build_z(s: str) -> list[int]:
    """Build the Z-array for string s. Z[i] = length of longest
    substring starting at i that matches a prefix of s."""
    n = len(s)
    z = [0] * n
    l = r = 0
    for i in range(1, n):
        if i <= r:
            z[i] = min(r - i + 1, z[i - l])
        while i + z[i] < n and s[z[i]] == s[i + z[i]]:
            z[i] += 1
        if i + z[i] - 1 > r:
            l, r = i, i + z[i] - 1
    return z


def z_search(text: str, pattern: str) -> tuple[list[int], int]:
    """Z-algorithm string search. Returns (matches, comparison_count)."""
    m = len(pattern)
    if m == 0:
        return list(range(len(text) + 1)), 0
    combined = pattern + "\0" + text
    z = _build_z(combined)
    matches = [i - m - 1 for i in range(m + 1, len(combined)) if z[i] == m]
    return matches, len(combined)


# ---------------------------------------------------------------------------
# 4. Boyer-Moore  O(nm) worst, sublinear in practice
# ---------------------------------------------------------------------------

def _bad_char_table(pattern: str) -> dict[str, int]:
    """Map each character to its rightmost index in the pattern."""
    table: dict[str, int] = {}
    for i, ch in enumerate(pattern):
        table[ch] = i
    return table


def _good_suffix_table(pattern: str) -> list[int]:
    """Build the good-suffix shift table."""
    m = len(pattern)
    good_suffix = [0] * (m + 1)
    border = [0] * (m + 1)

    # Phase 1: case where the matching suffix also appears elsewhere in pattern
    i, j = m, m + 1
    border[i] = j
    while i > 0:
        while j <= m and pattern[i - 1] != pattern[j - 1]:
            if good_suffix[j] == 0:
                good_suffix[j] = j - i
            j = border[j]
        i -= 1
        j -= 1
        border[i] = j

    # Phase 2: case where a prefix of the pattern matches a suffix of the match
    j = border[0]
    for i in range(m + 1):
        if good_suffix[i] == 0:
            good_suffix[i] = j
        if i == j:
            j = border[j]
    return good_suffix


def boyer_moore(text: str, pattern: str) -> tuple[list[int], int]:
    """Boyer-Moore string search. Returns (matches, comparison_count)."""
    n, m = len(text), len(pattern)
    if m == 0:
        return list(range(n + 1)), 0
    if m > n:
        return [], 0

    bad_char = _bad_char_table(pattern)
    good_suffix = _good_suffix_table(pattern)

    matches: list[int] = []
    comparisons = 0
    i = 0
    while i <= n - m:
        j = m - 1
        while j >= 0:
            comparisons += 1
            if pattern[j] != text[i + j]:
                break
            j -= 1
        if j < 0:
            matches.append(i)
            i += good_suffix[0]
        else:
            bc_shift = j - bad_char.get(text[i + j], -1)
            gs_shift = good_suffix[j + 1]
            i += max(bc_shift, gs_shift)
    return matches, comparisons


# ---------------------------------------------------------------------------
# 5. Benchmark
# ---------------------------------------------------------------------------

def benchmark() -> None:
    """Run all four algorithms on representative inputs and print step counts."""
    import random
    import string

    print("=" * 72)
    print("String Matching — Benchmark")
    print("=" * 72)

    # --- Test 1: worst case for naive (many repeats, late mismatch) ---
    text1 = "a" * 10_000 + "b"
    pat1 = "a" * 20 + "b"
    print(f"\n[Test 1] Worst-case naive: text={len(text1)}, pattern={len(pat1)}")
    for name, fn in [("Naive", naive_search), ("KMP", kmp_search),
                     ("Z-alg", z_search), ("Boyer-Moore", boyer_moore)]:
        m, cmps = fn(text1, pat1)
        print(f"  {name:12s}: matches={len(m):3d}  comparisons={cmps:>8,d}")

    # --- Test 2: random English-like text ---
    random.seed(42)
    words = ["the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog",
             "algorithm", "search", "pattern", "string", "match", "text"]
    text2 = " ".join(random.choices(words, k=5000))
    pat2 = "pattern"
    print(f"\n[Test 2] Random English text: text={len(text2)}, pattern={len(pat2)}")
    for name, fn in [("Naive", naive_search), ("KMP", kmp_search),
                     ("Z-alg", z_search), ("Boyer-Moore", boyer_moore)]:
        m, cmps = fn(text2, pat2)
        print(f"  {name:12s}: matches={len(m):3d}  comparisons={cmps:>8,d}")

    # --- Test 3: DNA-like (small alphabet ACGT) ---
    text3 = "".join(random.choices("ACGT", k=50_000))
    pat3 = "ACGTACGT"
    print(f"\n[Test 3] DNA alphabet: text={len(text3)}, pattern={len(pat3)}")
    for name, fn in [("Naive", naive_search), ("KMP", kmp_search),
                     ("Z-alg", z_search), ("Boyer-Moore", boyer_moore)]:
        m, cmps = fn(text3, pat3)
        print(f"  {name:12s}: matches={len(m):3d}  comparisons={cmps:>8,d}")

    # --- Test 4: pattern not found ---
    text4 = "a" * 100_000
    pat4 = "b"
    print(f"\n[Test 4] Pattern absent: text={len(text4)}, pattern={len(pat4)}")
    for name, fn in [("Naive", naive_search), ("KMP", kmp_search),
                     ("Z-alg", z_search), ("Boyer-Moore", boyer_moore)]:
        m, cmps = fn(text4, pat4)
        print(f"  {name:12s}: matches={len(m):3d}  comparisons={cmps:>8,d}")

    # --- Verify correctness ---
    print("\n[Correctness check]")
    test_cases = [
        ("", "a"),
        ("a", ""),
        ("abc", "abc"),
        ("aaaaaa", "aa"),
        ("ababcababcabc", "abcab"),
        ("mississippi", "issi"),
    ]
    all_ok = True
    for t, p in test_cases:
        expected = naive_search(t, p)[0]
        for name, fn in [("KMP", kmp_search), ("Z-alg", z_search),
                         ("Boyer-Moore", boyer_moore)]:
            result = fn(t, p)[0]
            if result != expected:
                print(f"  FAIL: {name} on text={t!r} pattern={p!r}: "
                      f"got {result}, expected {expected}")
                all_ok = False
    if all_ok:
        print("  All algorithms agree on all test cases.")


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def main() -> None:
    # Quick demonstrations
    text = "ababcababcabc"
    pattern = "abcab"
    print(f"Text:    '{text}'")
    print(f"Pattern: '{pattern}'\n")

    for name, fn in [("Naive", naive_search), ("KMP", kmp_search),
                     ("Z-alg", z_search), ("Boyer-Moore", boyer_moore)]:
        matches, cmps = fn(text, pattern)
        print(f"  {name:12s}: matches at {matches}  (comparisons={cmps})")

    # KMP failure function demo
    print(f"\nKMP failure function for '{pattern}': {_build_lps(pattern)}")

    # Z-array demo
    demo = "ababab"
    print(f"Z-array for '{demo}': {_build_z(demo)}")

    # Full benchmark
    print()
    benchmark()


if __name__ == "__main__":
    main()
