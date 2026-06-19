# String Matching — KMP, Z, Boyer-Moore

> Finding a needle in a haystack fast — the algorithms that make `grep`, text editors, and bioinformatics pipelines work at scale.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 04 lessons 01–18
**Time:** ~75 minutes

## Learning Objectives

- Understand why naive string-search is O(nm) and when it breaks down.
- Implement KMP with the failure function for guaranteed O(n + m) matching.
- Implement the Z-algorithm for pattern matching via Z-array construction.
- Implement Boyer-Moore with the bad character and good suffix heuristics.
- Compare all algorithms with step counts on realistic inputs.

## The Problem

You have a text of length *n* and a pattern of length *m*. Find every occurrence of the pattern in the text. The naive approach slides the pattern one character at a time, checking all *m* characters at each position — O(nm). For a 1 GB genome and a 20-base probe, that's billions of redundant comparisons. We need algorithms that skip characters intelligently.

## The Concept

### Naive Search — O(nm) Baseline

For each position *i* in the text, check if `text[i..i+m]` equals the pattern. Worst case: text = `"aaaaaaaaab"`, pattern = `"aaab"` — we compare 36 characters instead of the 13 truly necessary.

```
text:    a a a a a a a a a b
pattern: a a a b               ✗ (shift 1)
           a a a b             ✗ (shift 1)
             a a a b           ✗ (shift 1)
...
                   a a a b     ✓
```

Every mismatch at the last pattern character forces re-examination of already-compared characters. The key insight: **preprocess the pattern** to know how far to shift.

### KMP (Knuth-Morris-Pratt) — O(n + m)

KMP builds a **failure function** (LPS array). For each position *j*, `lps[j]` stores the length of the longest proper prefix of `pattern[0..j]` that is also a suffix.

**Failure function for `"ababc"`:** lps = [0, 0, 1, 2, 0]. When a mismatch occurs at position *j*, shift so that `lps[j-1]` characters of the prefix align with the already-matched suffix — we never re-examine text characters.

Building the failure function takes O(m) (same two-pointer idea as the search, applied to the pattern against itself). The search takes O(n): each text character is examined at most once.

### Z-Algorithm — O(n + m)

The **Z-array** of string *S*: `Z[i]` = length of longest substring at position *i* matching a prefix of *S*.

```
S =  a b a b a b c
Z =  - 0 4 0 2 0 0
```

**Construction** uses a Z-box `[l, r]` tracking the rightmost segment known to match a prefix. For each position *i*: if *i ≤ r*, use the mirror value `Z[i - l]` as a starting point; otherwise brute-force expand. Total work is O(n).

**Pattern matching:** concatenate `pattern + "$" + text`, build the Z-array, report positions where `Z[i] == m`.

### Boyer-Moore — Sublinear in Practice

Boyer-Moore scans the pattern **right to left**. On mismatch, it shifts by the max of two heuristics:

- **Bad character:** shift so the rightmost occurrence of the mismatched text character in the pattern aligns with the mismatch position. If the character doesn't appear in the pattern, skip by the full pattern length.
- **Good suffix:** if a suffix matched before the mismatch, shift to the next occurrence of that suffix in the pattern.

For English text with a 256-character alphabet, Boyer-Moore typically examines only n/m characters — **sublinear** in text length. Worst case is still O(nm), but a galloping variant avoids this.

### Step Counts

| Algorithm | Text: 1000 a's + b, Pattern: "aaab" (21 chars) |
|-----------|------|
| Naive | ~209,000 comparisons |
| KMP | ~10,000 comparisons |
| Z | ~10,023 comparisons |
| Boyer-Moore | ~10,000 comparisons |

| Algorithm | English text (30K chars), Pattern: "pattern" |
|-----------|------|
| Naive | ~32,000 comparisons |
| KMP | ~30,000 comparisons |
| Boyer-Moore | ~7,400 comparisons |

## Build It

Full implementations live in `code/main.py` and `code/main.rs`. Key data structures:

**KMP failure function** — two-pointer scan of pattern against itself: on match, extend; on mismatch, fall back via `lps[length-1]`. See `code/main.py:_build_lps`.

**Z-array construction** — maintain a Z-box `[l, r]`; for each *i*, reuse mirror value if inside box, then expand. See `code/main.py:_build_z`.

**Boyer-Moore good suffix table** — two-phase construction: (1) case where matching suffix reappears elsewhere in pattern, (2) case where a prefix of the pattern matches a suffix of the match. See `code/main.py:_good_suffix_table`.

Each algorithm returns `(matches, comparison_count)` so you can benchmark them side-by-side.

## Use It

- **`grep`** uses Boyer-Moore variants (and Aho-Corasick for `-f` multi-pattern mode) to scan files at near disk-bandwidth speed.
- **Text editors** (VS Code find, Vim `/`) use KMP or similar linear-time algorithms — the failure function lets them resume from the previous match without rescanning.
- **`strstr()` in glibc** uses a two-way string matching algorithm combining ideas from KMP and Boyer-Moore for guaranteed linear time with small constants.
- **Bioinformatics:** tools like Bowtie2 use FM-index (a compressed suffix array variant) that inherits ideas from both KMP and Boyer-Moore for genome alignment.

## Read the Source

- `grep/src/kwset.c` in GNU grep — Boyer-Moore variant (Commentz-Walter) for multi-pattern with reversed-pattern trie.
- `libc/string/strstr.c` in musl libc — the two-way algorithm, a practical hybrid of KMP and Boyer-Moore ideas.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A string search library with all four algorithms and a benchmark harness** — drop-in for any project needing fast substring search.

## Exercises

1. **Easy** — Build the KMP failure function by hand for `"aabaaab"`. Verify your answer matches the code output.
2. **Medium** — Find all occurrences of `"ana"` in `"bananananana"` using the Z-algorithm. Show the full Z-array for the concatenated string.
3. **Hard** — Compare Boyer-Moore step counts on English prose vs. random DNA sequences (A/C/G/T). Explain why Boyer-Moore shines on English text but performs worse on small alphabets.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Failure function (LPS) | "KMP's skip table" | `lps[j]` = longest proper prefix of `pattern[0..j]` that is also a suffix; determines safe shift on mismatch |
| Z-array | "Z-values" | `Z[i]` = length of longest substring at position *i* matching a prefix of *S*; built in O(n) via Z-box |
| Bad character heuristic | "skip to the mismatched char" | On mismatch at `text[i]`, shift so the rightmost occurrence of `text[i]` in the pattern aligns with position *i* |
| Good suffix heuristic | "shift by matched suffix" | If the last *k* pattern characters matched, shift to the next occurrence of that suffix in the pattern |

## Further Reading

- Knuth, Morris, Pratt, "Fast Pattern Matching in Strings" (1977)
- Boyer, Moore, "A Fast String Searching Algorithm" (1977)
- Gusfield, *Algorithms on Strings, Trees, and Sequences*, Chapters 1–2
- Smyth, *Computing Patterns in Strings*, Chapter 5 (Z-algorithm and variants)
