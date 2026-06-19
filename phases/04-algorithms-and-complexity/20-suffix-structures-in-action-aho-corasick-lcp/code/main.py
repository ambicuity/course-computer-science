"""
Suffix Structures in Action — Aho-Corasick, LCP
Phase 04 — Algorithms & Complexity Analysis, Lesson 20

Implements:
  1. build_suffix_array — O(n log n) doubling technique
  2. build_lcp          — O(n) Kasai's algorithm
  3. aho_corasick       — multi-pattern automaton (trie + failure links)
  4. search             — walk automaton over text, report all matches
"""

from collections import deque


# ---------------------------------------------------------------------------
# 1. Suffix array — O(n log n) doubling
# ---------------------------------------------------------------------------

def build_suffix_array(s: str) -> list[int]:
    """Return the suffix array of string s using the doubling technique.

    Each round sorts suffixes by their first 2^k characters using the
    previous ranking as a key for a stable sort (here: Python's Timsort
    on a composite key).
    """
    n = len(s)
    if n == 0:
        return []

    sa = list(range(n))
    rank = [ord(c) for c in s]
    tmp = [0] * n
    k = 1

    while k < n:
        def key(i: int) -> tuple[int, int]:
            return (rank[i], rank[i + k] if i + k < n else -1)

        sa.sort(key=key)

        tmp[sa[0]] = 0
        for i in range(1, n):
            tmp[sa[i]] = tmp[sa[i - 1]]
            if key(sa[i]) != key(sa[i - 1]):
                tmp[sa[i]] += 1

        rank = tmp[:]
        if rank[sa[-1]] == n - 1:
            break  # all ranks unique — fully sorted
        k *= 2

    return sa


# ---------------------------------------------------------------------------
# 2. LCP array — Kasai's algorithm O(n)
# ---------------------------------------------------------------------------

def build_lcp(s: str, sa: list[int]) -> list[int]:
    """Return the LCP array using Kasai's algorithm.

    lcp[i] = length of the longest common prefix between
             s[sa[i-1]..] and s[sa[i]..].
    lcp[0] is undefined (set to 0).
    """
    n = len(s)
    rank = [0] * n
    for i, pos in enumerate(sa):
        rank[pos] = i

    lcp = [0] * n
    h = 0
    for i in range(n):
        if rank[i] > 0:
            j = sa[rank[i] - 1]
            while i + h < n and j + h < n and s[i + h] == s[j + h]:
                h += 1
            lcp[rank[i]] = h
            if h:
                h -= 1
    return lcp


# ---------------------------------------------------------------------------
# 3. Aho-Corasick automaton
# ---------------------------------------------------------------------------

class AhoCorasick:
    """Multi-pattern string matching automaton.

    Attributes:
        trie:   list of dicts — trie[node] = {char: child}
        output: list of lists — output[node] = pattern indices ending here
        fail:   list of ints  — failure links
    """

    def __init__(self, patterns: list[str]) -> None:
        self.patterns = patterns
        self.trie: list[dict[str, int]] = [{}]
        self.output: list[list[int]] = [[]]
        self.fail: list[int] = [0]
        self._build_trie()
        self._build_fail()

    def _build_trie(self) -> None:
        """Insert all patterns into the trie."""
        for idx, pat in enumerate(self.patterns):
            node = 0
            for ch in pat:
                if ch not in self.trie[node]:
                    self.trie[node][ch] = len(self.trie)
                    self.trie.append({})
                    self.output.append([])
                    self.fail.append(0)
                node = self.trie[node][ch]
            self.output[node].append(idx)

    def _build_fail(self) -> None:
        """Build failure links via BFS (like KMP generalized to a trie)."""
        q: deque[int] = deque()
        # Initialize: children of root have fail = 0
        for ch, child in self.trie[0].items():
            self.fail[child] = 0
            q.append(child)

        while q:
            u = q.popleft()
            for ch, v in self.trie[u].items():
                q.append(v)
                # Follow failure links until we find a node with edge 'ch'
                f = self.fail[u]
                while f and ch not in self.trie[f]:
                    f = self.fail[f]
                self.fail[v] = self.trie[f].get(ch, 0)
                # Propagate outputs
                self.output[v].extend(self.output[self.fail[v]])

    def search(self, text: str) -> list[tuple[int, int, str]]:
        """Search text for all occurrences of all patterns.

        Returns list of (position, pattern_index, pattern).
        """
        matches: list[tuple[int, int, str]] = []
        node = 0
        for i, ch in enumerate(text):
            # Follow failure links until we can advance
            while node and ch not in self.trie[node]:
                node = self.fail[node]
            node = self.trie[node].get(ch, 0)

            for pat_idx in self.output[node]:
                pat = self.patterns[pat_idx]
                matches.append((i - len(pat) + 1, pat_idx, pat))
        return matches


# ---------------------------------------------------------------------------
# 4. Helpers — longest repeated substring, distinct substring count
# ---------------------------------------------------------------------------

def longest_repeated_substring(s: str) -> str:
    """Find the longest substring that appears at least twice using SA + LCP."""
    if len(s) < 2:
        return ""
    sa = build_suffix_array(s)
    lcp = build_lcp(s, sa)
    max_lcp = 0
    max_idx = 0
    for i in range(1, len(lcp)):
        if lcp[i] > max_lcp:
            max_lcp = lcp[i]
            max_idx = i
    if max_lcp == 0:
        return ""
    return s[sa[max_idx]:sa[max_idx] + max_lcp]


def count_distinct_substrings(s: str) -> int:
    """Count distinct substrings: n*(n+1)/2 - sum(lcp[1:])."""
    n = len(s)
    sa = build_suffix_array(s)
    lcp = build_lcp(s, sa)
    total = n * (n + 1) // 2
    return total - sum(lcp[1:])


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def main() -> None:
    print("=" * 72)
    print("Suffix Structures in Action — Aho-Corasick, LCP")
    print("=" * 72)

    # --- Suffix array demo ---
    text = "banana"
    sa = build_suffix_array(text)
    lcp = build_lcp(text, sa)
    print(f"\nText: '{text}'")
    print(f"Suffix array: {sa}")
    print(f"Sorted suffixes:")
    for i in sa:
        print(f"  sa[{sa.index(i):2d}] = {i:2d}  '{text[i:]}'")
    print(f"LCP array: {lcp}")

    # --- Longest repeated substring ---
    print(f"\nLongest repeated substring of '{text}': "
          f"'{longest_repeated_substring(text)}'")

    # --- Distinct substring count ---
    test_str = "abcab"
    print(f"\nDistinct substrings of '{test_str}': "
          f"{count_distinct_substrings(test_str)}")
    # Verify by brute force
    brute = len({test_str[i:j] for i in range(len(test_str))
                 for j in range(i + 1, len(test_str) + 1)})
    print(f"  (brute-force check: {brute})")

    # --- Aho-Corasick demo ---
    print(f"\n{'=' * 72}")
    print("Aho-Corasick Multi-Pattern Search")
    print("=" * 72)

    patterns = ["he", "she", "his", "hers"]
    ac = AhoCorasick(patterns)
    search_text = "ushers"
    print(f"\nPatterns: {patterns}")
    print(f"Text:     '{search_text}'")
    matches = ac.search(search_text)
    for pos, idx, pat in matches:
        print(f"  '{pat}' found at position {pos}")

    # --- Genome search demo ---
    print(f"\n{'=' * 72}")
    print("Genome Pattern Search (Aho-Corasick)")
    print("=" * 72)

    genome = "ACGTACGTACGTAGCTAGCTAGCTACGT"
    probes = ["ACGT", "TAGC", "GCTA", "TACG"]
    ac2 = AhoCorasick(probes)
    print(f"\nGenome:  '{genome}'")
    print(f"Probes:  {probes}")
    matches = ac2.search(genome)
    print(f"Matches ({len(matches)} total):")
    for pos, idx, pat in matches:
        print(f"  '{pat}' at position {pos}")

    # --- Suffix array for a genome ---
    print(f"\n{'=' * 72}")
    print("Suffix Array + LCP for Genome")
    print("=" * 72)

    small_genome = "ACGTACG"
    sa2 = build_suffix_array(small_genome)
    lcp2 = build_lcp(small_genome, sa2)
    print(f"\nText: '{small_genome}'")
    print(f"Suffix array: {sa2}")
    print(f"LCP array:    {lcp2}")
    print(f"Longest repeated substring: '{longest_repeated_substring(small_genome)}'")
    print(f"Distinct substrings: {count_distinct_substrings(small_genome)}")


if __name__ == "__main__":
    main()
