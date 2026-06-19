# Suffix Trees and Suffix Arrays

> Index every suffix of a string in O(n) space. Then ask any substring query in O(m). Genome assemblers, plagiarism detectors, bioinformatics — all live here.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L14 (tries), P04 L19 (string algorithms preview)
**Time:** ~90 minutes

## Learning Objectives

- Implement a **suffix array** in O(n log² n) by sorting suffixes; mention Skew/SA-IS for O(n).
- Implement the **LCP (longest common prefix) array** using Kasai's algorithm — O(n).
- Use suffix array + LCP for: substring search, longest repeated substring, longest common substring of two strings.
- Compare with **suffix tree** (Ukkonen's algorithm — O(n) but ~20× more memory).
- Understand why bioinformatics uses suffix arrays + FM-index (BWA, Bowtie aligners).

## The Problem

Given a fixed text T of length n, you'll ask many queries: "Is P a substring of T?" "How many times does P appear?" "What's the longest substring repeated in T?" "What's the longest common substring of T and S?"

Linear search is O(|P| × n) per query. KMP is O(|P| + n) — must scan T every time.

**Suffix tree / array**: O(n) build once. Each query: O(|P|).

The pre-processing makes the trade. For genome assembly (T is 3 billion bases; you query millions of reads), this is the only feasible approach.

## The Concept

### Suffix array

For string `T`, define n suffixes `T[0..]`, `T[1..]`, ..., `T[n-1..]`. Sort them lexicographically; record the original starting indices. That sorted list of indices IS the suffix array.

Example: T = "banana$":

| i | suffix | sorted (SA) |
|---|--------|-------------|
| 0 | banana$ | 6: $ |
| 1 | anana$  | 5: a$ |
| 2 | nana$   | 3: ana$ |
| 3 | ana$    | 1: anana$ |
| 4 | na$     | 0: banana$ |
| 5 | a$      | 4: na$ |
| 6 | $       | 2: nana$ |

`SA = [6, 5, 3, 1, 0, 4, 2]`

To search for pattern P: binary search in SA against T. O(|P| log n) per query.

### LCP array

`LCP[i]` = length of longest common prefix between SA[i] and SA[i-1] (adjacent sorted suffixes).

Kasai's algorithm computes LCP in O(n) given SA and ISA (inverse suffix array, where suffix starting at i has rank ISA[i]):

```c
for (int i = 0, h = 0; i < n; ++i) {
    if (ISA[i] > 0) {
        int j = SA[ISA[i] - 1];
        while (i + h < n && j + h < n && T[i + h] == T[j + h]) ++h;
        LCP[ISA[i]] = h;
        if (h > 0) --h;
    }
}
```

With SA + LCP:

- **Substring search**: O(|P| + log n) via binary search.
- **Longest repeated substring**: max(LCP).
- **Longest common substring of two strings**: concatenate with separator, compute SA + LCP, find max LCP between suffixes from different sources.
- **Number of distinct substrings of T**: n(n+1)/2 - Σ LCP.

### Suffix tree (Ukkonen)

The suffix tree of T is a trie containing every suffix of T, with single-child chains compressed (so it's actually a radix tree on suffixes). It has exactly n leaves and at most n internal nodes — O(n) space.

Ukkonen's algorithm builds it in O(n) by adding characters one at a time. It's beautiful, intricate, and ~20× more memory than suffix array. Almost nobody uses it in production anymore — suffix array + LCP is faster, smaller, and easier to engineer.

### FM-index

Combines suffix array with BWT (Burrows-Wheeler Transform) to give:
- O(|P|) substring count (no log n!).
- Compressed to 30-50% of the original text.

Used by BWA and Bowtie (genome aligners) to align ~100M short reads to a 3 GB genome in minutes.

## Build It

`code/main.c`:

1. Suffix array via doubling sort — O(n log² n).
2. Kasai's LCP — O(n).
3. Longest repeated substring on a Shakespeare passage.
4. Substring search via binary search.

`code/main.py` mirrors with cleaner code; uses sorted() for the SA.

`code/main.rs` uses simple naive sort.

### Run

```sh
clang -O2 main.c -o sa && ./sa
```

## Use It

- **Bioinformatics**: BWA, Bowtie, Bowtie2 — short-read aligners use FM-index.
- **Plagiarism detection (MOSS, Codequiry)**: longest-common-substring over thousands of submissions.
- **Linux `git blame`-equivalent indexing**: some tools precompute suffix arrays of large codebases.
- **Compression (LZ77 lookahead)**: longest-match search via suffix tree/array.
- **Bioperl** and ALL major genome tools.

## Read the Source

- [SA-IS reference implementation](https://github.com/yuta1984/sais) — Yuta Mori's O(n) construction.
- [BWA source](https://github.com/lh3/bwa) — Heng Li's aligner; uses FM-index, ~30k lines of C.
- *Algorithms on Strings, Trees, and Sequences* by Dan Gusfield — the bible of stringology.

## Ship It

This lesson ships **`outputs/suffix_array.h`** — single-header suffix array + LCP (Kasai).

## Exercises

1. **Easy.** Given SA + T, count the occurrences of substring P via two binary searches (lower_bound and upper_bound). O(|P| log n + occurrences).
2. **Medium.** Implement **longest common substring of two strings** using SA on the concatenation T1$T2#, where $ and # are separators not in either input. Look for max LCP between SA[i] and SA[i-1] where they belong to different sources.
3. **Hard.** Implement Sadakane's O(n) construction (SA-IS variant) and benchmark vs the O(n log² n) doubling-sort on a 10MB text file.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Suffix array | "SA" | Array of suffix start indices, sorted lexicographically |
| LCP array | "Adjacent common prefix" | LCP[i] = lcp(SA[i], SA[i-1]); enables many queries |
| Suffix tree | "Generalized trie" | Trie of all suffixes; compressed paths; O(n) space |
| FM-index | "Compressed index" | SA + BWT in compressed form; the bioinformatics standard |
| SA-IS | "Suffix array — Induced Sort" | Linear-time SA construction by Nong-Zhang-Chan 2009 |

## Further Reading

- *Algorithms on Strings, Trees, and Sequences* by Gusfield — the canonical text.
- Karkkainen & Sanders, *Simple Linear Work Suffix Array Construction* (2003) — the famous Skew algorithm.
- Nong-Zhang-Chan, *Linear Suffix Array Construction by Almost Pure Induced-Sorting* (SA-IS, 2009).
