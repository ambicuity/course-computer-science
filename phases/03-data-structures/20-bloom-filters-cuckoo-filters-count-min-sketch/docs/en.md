# Bloom Filters, Cuckoo Filters, Count-Min Sketch

> Probabilistic data structures: give up exactness for tiny memory. The 10-bit-per-element Bloom filter, the deletable Cuckoo filter, the streaming Count-Min sketch.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L05–L06 (hash tables, hash functions)
**Time:** ~75 minutes

## Learning Objectives

- Implement a **Bloom filter**: m bits, k hash functions; ε-false-positive set membership in O(k) per op.
- Implement a **Cuckoo filter**: similar accuracy with the ability to delete and slightly better space/locality.
- Implement a **Count-Min sketch**: estimate frequencies in O(k) memory per item, with controllable error.
- Compute the optimal parameters: given n items and target false-positive rate ε, find optimal m and k.

## The Problem

Exact set membership and frequency counting cost Θ(n) memory. But for many uses, an *approximate* answer is enough:

- **CDN cache front**: "is this URL likely in the cache?" — false positives mean a redundant cache check; false negatives are forbidden.
- **Database join hint**: "do these two tables likely have overlapping keys?" — false positives cause wasted joins; false negatives miss matches.
- **Stream heavy-hitters**: "what are the top URLs by request count, approximately?"

Probabilistic structures use 1-5 bytes per item with controllable error. The Bloom filter is the canonical example; Cuckoo improves on it; Count-Min sketch handles frequencies.

## The Concept

### Bloom filter

m-bit array B; k independent hash functions h₁..h_k.

**Insert(x)**: set bits B[h_i(x) mod m] = 1 for all i.

**Contains(x)**: return true iff all B[h_i(x) mod m] = 1.

False positives: possible (bits set by other inserts coincide).
False negatives: NONE — if x was inserted, all its bits are 1.

Optimal parameters: given n items and target false-positive rate ε:

- m / n = -log₂(ε) / ln(2) ≈ 1.44 log₂(1/ε) bits per element.
- k = (m / n) ln(2) hash functions.

So for ε = 1%: ~9.6 bits/element, k = 7 hashes.

For ε = 0.1%: 14.4 bits/element, k = 10 hashes.

Compare with a hash set: 32+ bytes per element. Bloom uses ~1.2 bytes for 1% error — a 25× reduction.

### Cuckoo filter

A more recent (2014) variant. Stores small "fingerprints" (8-12 bits each) in two candidate buckets per item. When inserting, if both buckets are full, evict a random fingerprint and reinsert (recursive "cuckoo").

| | Bloom | Cuckoo |
|---|-------|--------|
| Memory at 1% FPR | ~10 bits/element | ~8 bits/element |
| Delete | No | YES |
| Cache hits per query | ~7 | ~2 |
| Variable error | Easy | Harder |

Cuckoo filter is the modern choice for most applications — deletion matters.

### Count-Min sketch

For frequency counting (not just membership). w × d 2D array; d independent hash functions.

**Add(x, c)**: for each row i in 0..d-1: counts[i][h_i(x) mod w] += c.

**Estimate(x)**: return min over i of counts[i][h_i(x) mod w].

The min trick: by pigeonhole, at least one row has no hash collisions for popular items → the min is an accurate estimate. Always overestimates (never under).

Parameters: width w = e/ε, depth d = ln(1/δ). For ε = 0.01, δ = 0.001: w = 272, d = 7. Memory: ~2K counters total — vs hashing millions of distinct items exactly.

Used in: stream processing systems (Heavy Hitters), distributed counters (HyperLogLog's cousin), distributed databases.

### Tradeoff summary

| Goal | Tool |
|------|------|
| Set membership, low memory, no delete | Bloom |
| Set membership, low memory, with delete | Cuckoo |
| Frequency estimation, no enumeration | Count-Min |
| Cardinality (distinct count) | HyperLogLog (Phase 4 L27) |

## Build It

`code/main.c`:

1. Bloom filter with m bits, k hashes (mix64 with different seeds).
2. Cuckoo filter (8-bit fingerprints, 4-entry buckets).
3. Count-Min sketch with width=256, depth=4.
4. Empirically measure false-positive rate vs theoretical prediction.

`code/main.py` mirrors with cleaner code.

`code/main.rs` standard idiomatic Rust.

### Run

```sh
clang -O2 main.c -o pds && ./pds
```

## Use It

- **CDN / web caches**: "is this URL in the cache?" → Bloom test before a real lookup.
- **Bitcoin / Ethereum**: light clients use Bloom filters for transaction membership.
- **Google BigTable, Cassandra, HBase**: per-SSTable Bloom filter says whether a key MIGHT be in this file → skip 99% of disk reads.
- **Stream heavy-hitter detection**: Count-Min over an event stream → top-K trending items.
- **Spell checkers (historical)**: a Bloom filter of the dictionary.

## Read the Source

- *Bloom 1970* — the original paper.
- *Fan et al. Cuckoo Filter (2014)* — the modern improvement.
- [Cassandra's BloomFilter](https://github.com/apache/cassandra/blob/trunk/src/java/org/apache/cassandra/utils/BloomFilter.java).
- [Apache DataSketches](https://datasketches.apache.org/) — Yahoo's production stream-sketch library.

## Ship It

This lesson ships **`outputs/bloom.h`** — single-header Bloom filter.

## Exercises

1. **Easy.** Insert 10K random ints into a Bloom filter (m=100K bits, k=7). Test 10K NEW random ints; measure FPR; verify ≈ 1%.
2. **Medium.** Implement a Cuckoo filter and demonstrate `delete(x)`. Verify that delete-then-check returns false (and all other elements still present).
3. **Hard.** Use a Count-Min sketch to find the top-K most-common words in a large text file. Compare with an exact `Counter` for accuracy.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Bloom filter | "Probabilistic set" | m-bit array + k hash funcs; O(k) ops; ε FPR |
| False positive | "Says yes wrongly" | Hash collisions with other inserted elements |
| Fingerprint | "Mini-hash" | 8-12 bit hash of an item; stored instead of the full key |
| Cuckoo filter | "Delete-capable Bloom" | Two-bucket fingerprint storage with cuckoo-eviction |
| Count-Min sketch | "Streaming counter" | w×d table; estimate count via min over rows |
| HyperLogLog | "Cardinality estimator" | Different sketch for #distinct items; Phase 4 L27 |

## Further Reading

- *Mining of Massive Datasets* by Leskovec — chapter on stream-sketch algorithms.
- *Probabilistic Data Structures* by Andrii Gakhov — book covering all four major families.
- [HyperLogLog: from Counting to Approximate Counting](https://www.youtube.com/watch?v=lJYufx0bfpw) — Cliff Click talk.
