"""
Hashing in Algorithms — Rabin-Karp, Rolling Hashes
Phase 04 — Algorithms & Complexity Analysis
"""

import random


def polynomial_hash(s: str, base: int = 131, mod: int = 10**9 + 7) -> int:
    h = 0
    for ch in s:
        h = (h * base + ord(ch)) % mod
    return h


def rolling_hash_search(text: str, pattern: str) -> list[int]:
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
            if text[i : i + m] == pattern:
                matches.append(i)
        if i < n - m:
            h_window = (h_window - ord(text[i]) * power) * base + ord(text[i + m])
            h_window %= mod
    return matches


def multi_pattern_search(text: str, patterns: list[str]) -> dict[int, list[int]]:
    base, mod = 131, 10**9 + 7
    by_len: dict[int, set[int]] = {}
    for p in patterns:
        by_len.setdefault(len(p), set()).add(polynomial_hash(p, base, mod))
    results: dict[int, list[int]] = {}
    for length, hash_set in by_len.items():
        if length > len(text):
            continue
        power = pow(base, length - 1, mod)
        h = polynomial_hash(text[:length], base, mod)
        for i in range(len(text) - length + 1):
            if h in hash_set:
                results.setdefault(length, []).append(i)
            if i < len(text) - length:
                h = (h - ord(text[i]) * power) * base + ord(text[i + length])
                h %= mod
    return results


def longest_common_substring(s1: str, s2: str) -> int:
    base, mod = 131, 10**9 + 7

    def has_common(L: int) -> bool:
        if L == 0:
            return True
        hashes: set[int] = set()
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


def double_hash(
    s: str,
    p1: int = 131,
    m1: int = 10**9 + 7,
    p2: int = 137,
    m2: int = 10**9 + 9,
) -> tuple[int, int]:
    return (polynomial_hash(s, p1, m1), polynomial_hash(s, p2, m2))


def rabin_karp_double(text: str, pattern: str) -> list[int]:
    n, m = len(text), len(pattern)
    if m > n or m == 0:
        return []
    p1, m1, p2, m2 = 131, 10**9 + 7, 137, 10**9 + 9
    hp = double_hash(pattern, p1, m1, p2, m2)
    hw1 = polynomial_hash(text[:m], p1, m1)
    hw2 = polynomial_hash(text[:m], p2, m2)
    pw1, pw2 = pow(p1, m - 1, m1), pow(p2, m - 1, m2)
    matches = []
    for i in range(n - m + 1):
        if (hw1, hw2) == hp:
            if text[i : i + m] == pattern:
                matches.append(i)
        if i < n - m:
            hw1 = ((hw1 - ord(text[i]) * pw1) * p1 + ord(text[i + m])) % m1
            hw2 = ((hw2 - ord(text[i]) * pw2) * p2 + ord(text[i + m])) % m2
    return matches


def birthday_paradox_simulation(modulus: int, trials: int = 10000) -> dict[int, float]:
    results: dict[int, float] = {}
    for q in [100, 1000, 10000, 100000]:
        collisions = 0
        for _ in range(trials):
            seen: set[int] = set()
            hit = False
            for _ in range(q):
                v = random.randint(0, modulus - 1)
                if v in seen:
                    hit = True
                    break
                seen.add(v)
            if hit:
                collisions += 1
        results[q] = collisions / trials
    return results


def main() -> None:
    text = "abcabcabc"
    pattern = "abc"
    print(f"Rabin-Karp search for '{pattern}' in '{text}':")
    print(f"  Single pattern: {rolling_hash_search(text, pattern)}")
    print(f"  Double hash:    {rabin_karp_double(text, pattern)}")

    patterns = ["abc", "bca", "xyz"]
    print(f"\nMulti-pattern search {patterns}:")
    print(f"  {multi_pattern_search(text, patterns)}")

    s1 = "banana"
    s2 = "canaan"
    print(f"\nLongest common substring of '{s1}' and '{s2}': {longest_common_substring(s1, s2)}")

    # Brute-force verification
    def brute_lcs(a: str, b: str) -> int:
        best = 0
        for i in range(len(a)):
            for j in range(i + 1, len(a) + 1):
                if a[i:j] in b:
                    best = max(best, j - i)
        return best

    assert longest_common_substring(s1, s2) == brute_lcs(s1, s2)

    # Distinct substring count via rolling hash
    def count_distinct_substrings(s: str) -> int:
        base, mod = 131, 10**9 + 7
        seen: set[int] = set()
        for length in range(1, len(s) + 1):
            power = pow(base, length - 1, mod)
            h = polynomial_hash(s[:length], base, mod)
            seen.add(h)
            for i in range(1, len(s) - length + 1):
                h = (h - ord(s[i - 1]) * power) * base + ord(s[i + length - 1])
                h %= mod
                seen.add(h)
        return len(seen)

    print(f"\nDistinct substrings of 'aba': {count_distinct_substrings('aba')} (brute: 5)")

    # Birthday paradox
    print("\nBirthday paradox (M = 10^9+7, 10k trials):")
    bp = birthday_paradox_simulation(10**9 + 7, trials=10000)
    for q, rate in bp.items():
        print(f"  q={q:>6}: collision rate = {rate:.4f}")


if __name__ == "__main__":
    main()
