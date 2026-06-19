"""main.py — naive suffix array + Kasai LCP in Python.

Concise reference implementation. For large inputs use pydivsufsort (SA-IS bindings).
"""
from __future__ import annotations


def build_sa(s: str) -> list[int]:
    """O(n^2 log n) naive — fine for small s."""
    return sorted(range(len(s)), key=lambda i: s[i:])


def build_lcp(s: str, sa: list[int]) -> list[int]:
    """Kasai's algorithm — O(n)."""
    n = len(s)
    isa = [0] * n
    for i, p in enumerate(sa): isa[p] = i
    lcp = [0] * n
    h = 0
    for i in range(n):
        if isa[i] > 0:
            j = sa[isa[i] - 1]
            while i + h < n and j + h < n and s[i + h] == s[j + h]:
                h += 1
            lcp[isa[i]] = h
            if h > 0: h -= 1
    return lcp


def sa_search(s: str, sa: list[int], p: str) -> int:
    """Binary-search for p; -1 if not present."""
    lo, hi = 0, len(sa)
    m = len(p)
    while lo < hi:
        mid = (lo + hi) // 2
        suf = s[sa[mid]:sa[mid] + m]
        if suf < p: lo = mid + 1
        else: hi = mid
    if lo < len(sa) and s[sa[lo]:sa[lo] + m] == p:
        return sa[lo]
    return -1


def longest_repeated(s: str) -> str:
    sa = build_sa(s)
    lcp = build_lcp(s, sa)
    best = 0; idx = 0
    for i, h in enumerate(lcp):
        if h > best: best, idx = h, sa[i]
    return s[idx:idx + best]


def main() -> None:
    text = "the quick brown fox jumps over the lazy dog. the quick fox is quick."
    sa = build_sa(text)
    print("first 5 sorted suffixes:")
    for i in range(5):
        print(f"  SA[{i}]={sa[i]:3d}: {text[sa[i]:sa[i]+30]!r}")
    print(f"\nlongest repeated substring: {longest_repeated(text)!r}")
    print(f"search 'quick' → offset {sa_search(text, sa, 'quick')}")
    print(f"search 'zebra' → offset {sa_search(text, sa, 'zebra')}")


if __name__ == "__main__":
    main()
