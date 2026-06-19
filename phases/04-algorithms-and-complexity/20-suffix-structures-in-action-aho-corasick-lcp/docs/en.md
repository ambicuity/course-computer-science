# Suffix Structures in Action — Aho-Corasick, LCP

> One pattern is boring. Search for thousands simultaneously — suffix arrays, LCP arrays, and the Aho-Corasick automaton make it fast.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 04 lessons 01–19 (especially lesson 19 — KMP and Z-algorithm)
**Time:** ~75 minutes

## Learning Objectives

- Construct a suffix array in O(n log n) using the doubling technique.
- Build an LCP array in O(n) using Kasai's algorithm.
- Build and query an Aho-Corasick automaton for multi-pattern matching in O(n + m + z).
- Apply these structures to bioinformatics, spam filtering, and text indexing.

## The Problem

Lesson 19 solved single-pattern search. Now suppose you have 10,000 keywords and a 1 GB text. Running KMP 10,000 times gives O(10,000 × n) — too slow. We need data structures that handle **multiple patterns simultaneously**, or that precompute information about the text to answer many queries fast.

Two families of solutions exist:
1. **Suffix structures** (suffix arrays, LCP arrays) — preprocess the *text*, then answer any pattern query.
2. **Automata** (Aho-Corasick) — preprocess the *patterns*, then scan the text once.

## The Concept

### Suffix Arrays — O(n log n) Construction

A **suffix array** `sa` is a permutation of indices sorted so that `text[sa[0]..]` < `text[sa[1]..]` < ... lexicographically.

```
text = "banana"
sa = [5, 3, 1, 0, 4, 2]   (suffixes: a, ana, anana, banana, na, nana)
```

**Doubling algorithm:** sort by first character, then first 2, then 4, 8, ... Each round uses the previous ranking as a radix-sort key. After ceil(log2(n)) rounds, all suffixes are sorted. Each round is O(n), total O(n log n).

For each position *i*, maintain rank tuple `(rank[i], rank[i + k])` where *k* doubles each round. When all ranks are unique, sorting is complete.

```
Round 0 (k=1):  sort by first char
  "banana" → ranks: b=1, a=0, n=2, a=0, n=2, a=0
Round 1 (k=2):  sort by (rank[i], rank[i+1])
  "ba"→(1,0), "an"→(0,2), "na"→(2,0), "an"→(0,2), "na"→(2,0), "a"→(0,-1)
Round 2 (k=4):  sort by (rank[i], rank[i+2]) — all ranks now unique
```

### LCP Array via Kasai's Algorithm — O(n)

The **LCP array** stores `lcp[i] = LCP(sa[i-1], sa[i])` — the longest common prefix between consecutive sorted suffixes.

```
sa  = [5, 3, 1, 0, 4, 2]
lcp = [ _, 1, 3, 0, 0, 2 ]
```

`lcp[2] = 3` because `"ana..."` and `"anana..."` share prefix `"ana"`.

The LCP array unlocks several queries: the maximum LCP value gives the **longest repeated substring**; `sum(lcp[1:])` counts duplicate prefixes, so `n*(n+1)/2 - sum(lcp[1:])` gives the **count of distinct substrings**.

**Kasai's algorithm** processes suffixes in *text order* (not sorted order), maintaining running LCP *h*. When moving from suffix *i* to *i-1* in text order, the LCP is at least *h - 1*. We only compare beyond that — total work is O(n).

### Aho-Corasick — O(n + m + z)

Aho-Corasick extends KMP from one pattern to many. It builds a **trie** of all patterns, adds **failure links** (KMP's failure function generalized to a trie), and **output links**.

**Construction:**
1. Insert all patterns into a trie — each node represents a prefix.
2. BFS from root to build failure links: for node *u* with edge char *c*, failure link goes to the state the automaton would reach if *c* had mismatched.
3. Propagate outputs: if a failure target is a pattern endpoint, that pattern is output from the current node too.

**Search:** walk the trie character by character. On mismatch, follow failure links. Report patterns ending at each node. Time: O(n + m + z) where *z* is the number of matches.

### When to Use What

| Structure | Preprocesses | Best for |
|-----------|-------------|----------|
| Suffix array + LCP | Text | Many queries on one text |
| Aho-Corasick | Patterns | Many patterns, one text scan |
| KMP (Lesson 19) | One pattern | Single pattern repeated queries |

## Build It

Full implementations live in `code/main.py` and `code/main.rs`. Key structures:

**Suffix array (doubling)** — `build_suffix_array(s)`: sorts suffixes by `(rank[i], rank[i+k])` keys; each round refines until all ranks unique. See `code/main.py:build_suffix_array`.

**LCP (Kasai)** — `build_lcp(s, sa)`: processes suffixes in text order, maintains running *h*, drops by at most 1 per step. See `code/main.py:build_lcp`.

**Aho-Corasick** — `AhoCorasick(patterns)`: builds trie, then BFS failure links. `search(text)` walks the automaton, reports all matches. See `code/main.py:AhoCorasick`.

The code also includes helpers: `longest_repeated_substring(s)` via SA + LCP, and `count_distinct_substrings(s)` using the formula `n*(n+1)/2 - sum(lcp[1:])`.

## Use It

- **Bioinformatics:** Genome databases (BLAST, Bowtie) use suffix arrays and LCP arrays for fast sequence alignment — searching a 3-billion-base genome for a 25-base probe takes milliseconds.
- **Spam filters:** Email scanners build an Aho-Corasick automaton from thousands of spam keywords and scan each message body in a single pass.
- **`grep -f` (multi-pattern):** GNU grep uses Aho-Corasick (or Commentz-Walter) when given a file of patterns.
- **Practical suffix arrays:** The `SAIS` algorithm (induced sorting) constructs suffix arrays in O(n) and is used in `divsufsort`, the fastest known practical suffix array builder.
- **Search engines:** Elasticsearch and Lucene use inverted indexes (conceptually similar to suffix arrays) for full-text search over billions of documents.

## Read the Source

- `divsufsort` library — O(n) suffix array via induced sorting; the gold standard for practical suffix array construction.
- GNU grep `src/kwset.c` — Aho-Corasick variant (Commentz-Walter) with a trie of reversed patterns.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A multi-pattern search engine combining suffix array + LCP indexing with Aho-Corasick automaton** — preprocess the text once, answer any pattern query fast.

## Exercises

1. **Easy** — Build the suffix array and LCP array by hand for `"aababc"`. Identify the longest repeated substring from the LCP array.
2. **Medium** — Count distinct substrings of `"abcab"` using the formula `n*(n+1)/2 - sum(lcp[1:])`. Implement and verify against brute force.
3. **Hard** — Extend the Aho-Corasick automaton with explicit output links (pointer to nearest pattern-endpoint ancestor via failure links) instead of storing full output lists. Compare memory usage.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Suffix array | "sorted suffixes" | Permutation `sa` where `text[sa[0]..]` < `text[sa[1]..]` < ... lexicographically |
| LCP array | "longest common prefix array" | `lcp[i]` = LCP between `sa[i-1]` and `sa[i]` |
| Doubling technique | "prefix-doubling" | Sort suffixes by 1-char, 2-char, 4-char, ... prefixes until all ranks unique; O(n log n) |
| Kasai's algorithm | "linear LCP" | Computes LCP array in O(n) by processing suffixes in text order |
| Failure link | "where to fall back" | Longest proper suffix of current trie path that is also a trie path |
| Output link | "which patterns end here" | Patterns ending at a node, propagated via failure links |

## Further Reading

- Manber & Myers, "Suffix Arrays: A New Method for On-Line String Searches" (1993)
- Kasai et al., "Linear-Time Longest-Common-Prefix Computation in Suffix Arrays and Its Applications" (2001)
- Aho & Corasick, "Efficient String Matching: An Aid to Bibliographic Search" (1975)
- Puglisi, Smyth, Turpin, "A Taxonomy of Suffix Array Construction Algorithms" (2007)
