# Hash Function Design — Universal, Tabulation, SipHash

> A hash table is only as good as the hash function feeding it. This lesson is about what makes a hash function actually good — for speed, for distribution, and against adversarial inputs.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L05 (hash tables), P01 L14 (modular arithmetic)
**Time:** ~75 minutes

## Learning Objectives

- Distinguish *cryptographic* hashes (SHA-2, BLAKE) from *non-cryptographic* hashes (FNV, mix64, FxHash, Wyhash, SipHash) — different goals.
- Implement and test FNV-1a, splitmix64, and a tabulation hash; measure their **avalanche** (one input bit flips → 50% output bits flip).
- Implement SipHash-1-3 from scratch (~30 lines); explain why it's the default for DOS-resistant hash maps.
- Understand **universal hashing** theory: a family H is universal if Pr[h(x)=h(y)] ≤ 1/m for x≠y. Used in tabulation hashing.

## The Problem

A hash function maps an input of any size to a fixed-size integer. The hash *table* uses this to pick a slot. The properties you need depend on what you're protecting against:

| Goal | Examples |
|------|----------|
| Speed for HashMap keys | FxHash, Wyhash, mix64 |
| DOS resistance | SipHash, Wyhash (keyed) |
| Integrity (collision-free in practice) | SHA-256, BLAKE3 |
| Universal distribution proven | Tabulation hashing |

Mix them up — e.g., use FNV-1a for a DOS-facing HTTP server — and you have a vulnerability. Use SHA-256 inside HashMap keys and you've burned 10× the CPU for no benefit.

## The Concept

### What makes a hash "good"

Three properties matter:

1. **Distribution / avalanche**: flipping one input bit should flip ~50% of output bits, independent of the input.
2. **Speed**: hashing a u64 key in a HashMap should cost ≤ 10 cycles; hashing a 64-byte string ≤ 50.
3. **Adversarial resistance**: an attacker who knows the algorithm shouldn't be able to construct colliding inputs without huge work.

Properties 1+2 require a non-cryptographic hash with strong mixing. Property 3 requires a keyed cryptographic-style construction (SipHash) or strong randomization.

### Hash function families

#### 1. Multiplicative hashing (Knuth)

```c
hash = (key * 2654435761u) >> 16
```

Multiply by a "good" constant near φ × 2^32, take the top bits. One multiplication, one shift. Fast, but predictable — adversary can construct collisions.

#### 2. FNV-1a (Fowler-Noll-Vo)

```c
hash = 14695981039346656037ULL;            /* FNV offset basis */
for each byte b in input:
    hash ^= b;
    hash *= 1099511628211ULL;              /* FNV prime */
```

Two ops per byte. Cache-friendly. Reasonable distribution. **Predictable**, so vulnerable to DOS.

#### 3. SplitMix64 / Murmur / mix64

Pure-integer "avalanche" hashes designed to map any 64-bit input to a well-mixed 64-bit output:

```c
uint64_t mix64(uint64_t x) {
    x += 0x9e3779b97f4a7c15ULL;
    x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
    x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
    return x ^ (x >> 31);
}
```

This is what most "integer key" HashMaps use internally. ~3 ns per call.

#### 4. Tabulation hashing

Lookup-table based. Split key into bytes; XOR a precomputed random table:

```c
uint64_t T[8][256];                          /* initialize at startup with random */

uint64_t tab_hash(uint64_t key) {
    uint64_t h = 0;
    for (int i = 0; i < 8; ++i) {
        h ^= T[i][(key >> (i * 8)) & 0xff];
    }
    return h;
}
```

**Theoretically universal**: probability of collision = 1/2^64 over random table choices. Used in academic algorithms requiring proven guarantees (e.g., perfect hashing).

#### 5. SipHash

A pseudo-random function: keyed (k0, k1 secret), produces 64-bit MAC of input. Designed to be O(n) for input length n with small constants. Used by:

- Python (since 3.4)
- Rust `std::collections::HashMap` default
- OpenBSD `getrandom`, Postgres' hash partitioning
- Linux skb_hash

The compression function ("SipRound") is:

```c
v0 += v1; v1 = ROTL(v1, 13); v1 ^= v0; v0 = ROTL(v0, 32);
v2 += v3; v3 = ROTL(v3, 16); v3 ^= v2;
v0 += v3; v3 = ROTL(v3, 21); v3 ^= v0;
v2 += v1; v1 = ROTL(v1, 17); v1 ^= v2; v2 = ROTL(v2, 32);
```

SipHash-1-3 means: 1 round per 8-byte message block, 3 finalization rounds. Fast (~1 ns per byte) and provably secure under standard cryptographic assumptions.

#### 6. Modern: Wyhash, FxHash, xxhash

- **xxhash** (XXH64, XXH3) — extremely fast, popular for hashing large data.
- **Wyhash** — current speed champion (~0.5 ns/byte). Used in Zig, mold linker.
- **FxHash** — Firefox's per-thread hash. Used by `rustc` internally (not DOS-resistant).

### Avalanche test

A hash function's quality can be tested. For random inputs and output bit b:

```
flip_count[b] = number of (input, bit-flip) pairs where output bit b also flipped
```

Good hash: every output bit flips with probability ≈ 0.5 for any single-bit input flip. Print as a 64×64 heatmap; check that all entries are 0.45 - 0.55.

FNV-1a fails on input bit 0 (input enters byte-by-byte; flipping bit 0 affects only the low byte first). Modern hashes pass uniformly.

### Universality, formally

A family H = {h: U → [m]} is **2-universal** if for all distinct x, y ∈ U: Pr_{h∈H}[h(x)=h(y)] ≤ 1/m.

Why we care: under universality, the expected chain length in chaining = 1 + α regardless of input distribution. Adversary can't construct colliders without knowing the random choice of h.

Tabulation hashing is provably 3-universal — strong enough for almost everything.

## Build It

`code/main.c`:

1. FNV-1a on integer and string inputs.
2. splitmix64 (mix64) — the canonical integer mixer.
3. Tabulation hashing with `T[8][256]` initialized from `/dev/urandom`.
4. SipHash-1-3 (full implementation, ~50 lines).
5. **Avalanche test**: for each hash, measure bit-flip propagation over 100K random inputs.

`code/main.py`: shows Python's `hash()` and the per-process `PYTHONHASHSEED` randomization.

`code/main.rs`: hands-on with Rust's `Hasher` trait, comparing default SipHash with FxHash via `hashbrown`.

### Run

```sh
clang -O2 main.c -o hash && ./hash
python3 main.py
```

## Use It

- **HashMap keys**: pick a fast non-crypto hash + a per-table random seed. SipHash if you face untrusted input; FxHash/Wyhash if you don't.
- **Content-addressable storage (Git, IPFS)**: cryptographic hash (SHA-1, SHA-256, BLAKE3). Collisions = data loss.
- **Bloom filters / Count-Min**: needs independent hashes — tabulation or double-hashed splitmix.
- **Probabilistic algorithms (HyperLogLog)**: a good-quality non-crypto hash suffices.

## Read the Source

- [Reference SipHash C implementation](https://github.com/veorq/SipHash/blob/master/siphash.c) — 100 lines, lovely to read.
- [Rust `hashbrown`'s default hasher](https://github.com/rust-lang/hashbrown/blob/master/src/raw/mod.rs) — uses FxHash + SwissTable.
- [Python `_Py_HashBytes` in `Python/pyhash.c`](https://github.com/python/cpython/blob/main/Python/pyhash.c) — SipHash-2-4 with per-process seed.

## Ship It

This lesson ships **`outputs/siphash.h`** — a clean, single-header SipHash-1-3 / SipHash-2-4 implementation, MIT licensed.

## Exercises

1. **Easy.** Implement FNV-1a for a string; verify against a published reference value (`"foobar"` → `0x85944171f73967e8`).
2. **Medium.** Build a collision-counting test: hash 1M random strings with FNV-1a, mix64, and SipHash. Count buckets with > 5 collisions when reduced mod 2^20. Compare.
3. **Hard.** Implement an avalanche-quality test that scores each hash function on a 64×64 grid (input bit × output bit), prints heat colors, and computes the worst-case bit. Use it to find the bug in this broken hash: `h(x) = x * 31`.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Avalanche | "Bit propagation" | Flipping one input bit should flip ~50% output bits |
| Universal hashing | "Provably good distribution" | A family where Pr[h(x)=h(y)] ≤ 1/m |
| Keyed hash | "PRF with secret key" | h_k(x): security relies on k being unknown to adversary |
| Hash flooding | "DOS via collisions" | Adversary submits N keys all → one bucket; O(N²) work |
| FNV / xxhash / SipHash | (algorithms) | FNV: 1991 byte-by-byte; xxhash: 2012 SIMD-friendly; SipHash: 2012 keyed PRF |

## Further Reading

- *Cryptographic Hash Functions* by Preneel (2010) — survey, includes SipHash.
- [SMHasher](https://github.com/aappleby/smhasher) — the canonical hash-function test suite.
- [Aumasson & Bernstein, SipHash paper (2012)](https://www.aumasson.jp/siphash/siphash.pdf) — the design rationale.
- *Hash Tables Are Easy, Right?* by Malte Skarupke — practical hash-table design talk.
