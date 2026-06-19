# Indexing — Hash, Bitmap, GiST

> B-trees are just the beginning: when your query isn't a range scan, other index structures beat them by orders of magnitude.

**Type:** Build
**Languages:** Python, Rust
**Prerequisites:** Phase 10 lesson 07 (B-tree indexing), Phase 05 (physical storage)
**Time:** ~60 minutes

## Learning Objectives

- Implement extendible hashing with directory doubling and bucket splitting
- Understand when hash indexes outperform B-trees (equality lookups) and when they fail (range queries)
- Build a bitmap index with RLE compression and bitwise query operations
- Explain how GiST enables indexing for non-standard data types (geometry, full-text, ranges)
- Compare B-tree, Hash, Bitmap, and GiST across access patterns and cardinality

## The Problem

Your query planner has a choice. The user wrote `WHERE status = 'shipped'` on a 10M-row orders table. A B-tree on `status` works — it'll descend the tree, find the leaf page for `'shipped'`, and scan. But the B-tree is doing *ordered* work (comparing strings, navigating internal nodes) when all you need is *exact match*. A hash index does the job in O(1) amortized lookups.

Now imagine the query is `WHERE status IN ('shipped', 'cancelled') AND region_id = 42` in a 100M-row warehouse fact table with 12 distinct statuses and 500 regions. A bitmap index builds one tiny bit-array per distinct value; the query becomes three bitwise AND/OR operations on compressed bit arrays — microseconds.

Then there's the query `WHERE ST_DWithin(geom, ST_MakePoint(-73.9, 40.7), 1000)` — find all points within 1 km of Times Square. No B-tree or hash can index that. A B-tree orders by a total order, but there's no total order on "within 1 km of." GiST indexes with a *penalty* function and a *consistent* function instead of comparison.

Without understanding these index types, you'll either build everything on B-trees (leaving performance on the table) or pick the wrong structure (like using a hash for a range query). This lesson builds three alternative index structures from scratch.

## The Concept

### Hash Indexes

A hash index maps a key to a fixed-size bucket via a hash function. Three generations:

| Scheme | Hash function | Collision strategy | Growth |
|--------|--------------|-------------------|--------|
| Static hashing | `h(k) % N` | Overflow pages (linked list) | Requires rebuild |
| Extendible hashing | `h(k) % 2^d` | Directory doubling, bucket split | Incremental, on split |
| Linear hashing | `h(k) % N`, then `h(k) % 2N` for split region | Overflow pages + incremental split | Incremental, round-robin |

**Extendible hashing** uses a *directory* — an array of `2^d` pointers (d = global depth). Each pointer points to a bucket with its own *local depth*. When a bucket overflows, it splits: local depth increments, half the keys move to a new bucket, and the directory doubles if local depth exceeds global depth. Only the entries pointing to the split bucket need updating in the new directory half.

```
Global depth = 2, Directory size = 4
        ┌─────┐
  00 ──►│  A  │  local depth = 2
        ├─────┤
  01 ──►│  B  │  local depth = 1  ← two dir entries point here
        ├─────┤
  10 ──►│  A  │  (same bucket as 00)
        ├─────┤
  11 ──►│  B  │  (same bucket as 01)
        └─────┘

After inserting into B causes split:
        ┌─────┐
  00 ──►│  A  │
        ├─────┤
  01 ──►│ B0  │  local depth = 2
        ├─────┤
  10 ──►│  A  │
        ├─────┤
  11 ──►│ B1  │  local depth = 2
        └─────┘
```

### Bitmap Indexes

For each distinct value `v` in a column, store a bit vector of length N (N = number of rows). Bit `i` is 1 if row `i` has value `v`.

```
status bitmap for orders table:
         row: 0 1 2 3 4 5 6 7 8 9 ...
'pending':   1 0 0 1 0 0 1 0 0 0 ...
'shipped':   0 1 0 0 1 1 0 1 0 1 ...
'cancelled': 0 0 1 0 0 0 0 0 1 0 ...
```

Query `WHERE status = 'shipped'` → return the `'shipped'` bitmap directly.
Query `WHERE status IN ('shipped', 'cancelled')` → `shipped-bitmap OR cancelled-bitmap`.
Query `WHERE status = 'shipped' AND region_id = 42` → `shipped-bitmap AND region-42-bitmap`.

Real bitmaps are compressed. Naive bitmaps at 10M rows × 500 distinct values = 5Gb uncompressed. **Run-length encoding** compresses runs of identical bits. Three common schemes:

- **WAH** (Word-Aligned Hybrid): packs bits into words, uses a special word to encode long runs
- **BBC** (Byte-aligned Bitmap Compression): byte-aligned for faster decompression
- **Roaring Bitmaps**: divides the 32-bit space into 2^16 chunks; dense chunks use a bitset, sparse chunks use sorted arrays

### GiST (Generalized Search Tree)

A balanced tree structure where the internal nodes hold *predicates* (not key ranges), and the user provides:

- `consistent(predicate, key)` → bool: can a node with this predicate contain the key?
- `union(p1, p2)` → predicate: combine two predicates into a covering predicate
- `penalty(p1, p2)` → float: how much would p1 need to expand to cover p2?
- `picksplit(list-of-keys)` → [left-keys, right-keys]: split keys into two groups for a node split

Unlike B-tree (which requires a total order), GiST only needs a way to test *containment possibility*. This makes it extensible to domains without natural order:

| Use case | Predicate type | Example |
|----------|---------------|---------|
| R-tree (spatial) | Bounding box | `consistent = overlaps?` |
| Full-text search | tsquery | `consistent = matches tsquery?` |
| Range types | Interval containment | `consistent = overlaps?` |
| Arrays (intarray) | Integer set | `consistent = contains element?` |

## Build It

We'll build two index structures in Python (extendible hash + bitmap with Roaring-style compression), then a more performant extendible hash in Rust.

### Step 1: Extendible Hash Index in Python

```python
import hashlib
from typing import Optional

class Bucket:
    def __init__(self, depth: int):
        self.depth = depth
        self.keys: list[int] = []
        self.values: list[int] = []
        self.capacity = 2

    def is_full(self) -> bool:
        return len(self.keys) >= self.capacity

    def search(self, key: int) -> Optional[int]:
        for i, k in enumerate(self.keys):
            if k == key:
                return self.values[i]
        return None

    def insert(self, key: int, value: int) -> bool:
        for i, k in enumerate(self.keys):
            if k == key:
                self.values[i] = value
                return False
        if self.is_full():
            return False
        self.keys.append(key)
        self.values.append(value)
        return True

    def remove(self, key: int) -> bool:
        for i, k in enumerate(self.keys):
            if k == key:
                self.keys.pop(i)
                self.values.pop(i)
                return True
        return False


class ExtendibleHash:
    def __init__(self):
        self.global_depth = 1
        self.directory = [Bucket(self.global_depth) for _ in range(2 ** self.global_depth)]

    def _hash(self, key: int) -> int:
        return hashlib.sha256(key.to_bytes(8, 'big')).digest()

    def _mask(self, depth: int) -> int:
        return (1 << depth) - 1

    def _bucket_index(self, key: int, depth: int) -> int:
        return int.from_bytes(self._hash(key)[:4], 'big') & self._mask(depth)

    def search(self, key: int) -> Optional[int]:
        idx = self._bucket_index(key, self.global_depth)
        return self.directory[idx].search(key)

    def insert(self, key: int, value: int) -> None:
        idx = self._bucket_index(key, self.global_depth)
        bucket = self.directory[idx]
        if not bucket.is_full() or bucket.search(key) is not None:
            bucket.insert(key, value)
            return
        self._split(idx, key, value)

    def _split(self, idx: int, key: int, value: int) -> None:
        bucket = self.directory[idx]
        old_depth = bucket.depth
        new_depth = old_depth + 1
        bucket.depth = new_depth

        if new_depth > self.global_depth:
            self._double_directory()

        b0 = Bucket(new_depth)
        b1 = Bucket(new_depth)

        for k, v in zip(bucket.keys, bucket.values):
            target = b0 if (self._bucket_index(k, new_depth) & (1 << old_depth)) == 0 else b1
            target.keys.append(k)
            target.values.append(v)

        target = b0 if (self._bucket_index(key, new_depth) & (1 << old_depth)) == 0 else b1
        target.keys.append(key)
        target.values.append(value)

        step = 1 << old_depth
        for i in range(0, 2 ** self.global_depth, step * 2):
            for j in range(step):
                if (self._bucket_index(0, new_depth) & step) == 0:
                    self.directory[i + j] = b0
                    self.directory[i + j + step] = b1
                else:
                    self.directory[i + j] = b1
                    self.directory[i + j + step] = b0

    def _double_directory(self):
        old_size = len(self.directory)
        self.directory.extend([self.directory[i % old_size] for i in range(old_size)])
        self.global_depth += 1

    def remove(self, key: int) -> bool:
        idx = self._bucket_index(key, self.global_depth)
        return self.directory[idx].remove(key)
```

#### How the split works

When a bucket at directory index `idx` overflows:
1. Increment the bucket's local depth
2. If local depth > global depth, double the directory (all pointers duplicated, then the split bucket's two new locations point to different buckets)
3. Redistribute the old bucket's keys between two new buckets using the new local depth bit
4. Update all directory entries that pointed to the old bucket — exactly half go to each new bucket

### Step 2: Bitmap Index with Roaring-style Compression

```python
from typing import Any
import math

class Bitmap:
    def __init__(self, size: int = 0):
        self.size = size
        self.words = [0] * ((size + 63) // 64) if size else []

    def set(self, pos: int) -> None:
        while pos >= self.size:
            self.size = max(self.size * 2, 64)
            needed = (self.size + 63) // 64
            self.words.extend([0] * (needed - len(self.words)))
        self.words[pos // 64] |= 1 << (pos % 64)

    def get(self, pos: int) -> int:
        if pos >= self.size:
            return 0
        return (self.words[pos // 64] >> (pos % 64)) & 1

    def __and__(self, other: 'Bitmap') -> 'Bitmap':
        n = min(len(self.words), len(other.words))
        result = Bitmap()
        result.words = [self.words[i] & other.words[i] for i in range(n)]
        result.size = min(self.size, other.size)
        return result

    def __or__(self, other: 'Bitmap') -> 'Bitmap':
        n = max(len(self.words), len(other.words))
        result = Bitmap()
        result.words = [0] * n
        for i in range(n):
            a = self.words[i] if i < len(self.words) else 0
            b = other.words[i] if i < len(other.words) else 0
            result.words[i] = a | b
        result.size = max(self.size, other.size)
        return result

    def __invert__(self) -> 'Bitmap':
        result = Bitmap()
        result.words = [~w & ((1 << 64) - 1) for w in self.words]
        result.size = self.size
        return result

    def count(self) -> int:
        return sum(w.bit_count() for w in self.words)


class RoaringBitmap:
    """Simplified Roaring-like bitmap: dense chunks use Bitmap, sparse use sorted arrays."""

    CHUNK_SHIFT = 16
    CHUNK_SIZE = 1 << CHUNK_SHIFT
    SPARSE_THRESHOLD = 4096

    class Chunk:
        def __init__(self):
            self.dense: Optional[Bitmap] = None
            self.sparse: list[int] = []

    def __init__(self):
        self.chunks: dict[int, RoaringBitmap.Chunk] = {}

    def _get_or_create_chunk(self, key: int) -> 'RoaringBitmap.Chunk':
        hi = key >> self.CHUNK_SHIFT
        if hi not in self.chunks:
            self.chunks[hi] = RoaringBitmap.Chunk()
        return self.chunks[hi]

    def add(self, key: int) -> None:
        lo = key & (self.CHUNK_SIZE - 1)
        c = self._get_or_create_chunk(key)
        if c.dense is not None:
            c.dense.set(lo)
        else:
            if lo not in c.sparse:
                c.sparse.append(lo)
            if len(c.sparse) > self.SPARSE_THRESHOLD:
                bm = Bitmap(self.CHUNK_SIZE)
                for v in c.sparse:
                    bm.set(v)
                c.dense = bm
                c.sparse = []

    def contains(self, key: int) -> bool:
        hi = key >> self.CHUNK_SHIFT
        if hi not in self.chunks:
            return False
        lo = key & (self.CHUNK_SIZE - 1)
        c = self.chunks[hi]
        if c.dense is not None:
            return c.dense.get(lo) == 1
        return lo in c.sparse

    def __and__(self, other: 'RoaringBitmap') -> 'RoaringBitmap':
        result = RoaringBitmap()
        for hi, c in self.chunks.items():
            if hi in other.chunks:
                oc = other.chunks[hi]
                if c.dense is not None and oc.dense is not None:
                    rc = RoaringBitmap.Chunk()
                    rc.dense = c.dense & oc.dense
                    result.chunks[hi] = rc
                elif c.dense is None and oc.dense is None:
                    intersection = sorted(set(c.sparse) & set(oc.sparse))
                    if intersection:
                        rc = RoaringBitmap.Chunk()
                        rc.sparse = intersection
                        result.chunks[hi] = rc
                else:
                    dense = c.dense if c.dense is not None else oc.dense
                    sparse = oc.sparse if c.dense is not None else c.sparse
                    rc = RoaringBitmap.Chunk()
                    rc.sparse = [v for v in sparse if dense.get(v)]
                    result.chunks[hi] = rc
        return result

    def __or__(self, other: 'RoaringBitmap') -> 'RoaringBitmap':
        result = RoaringBitmap()
        for hi, c in self.chunks.items():
            result.chunks[hi] = c
        for hi, c in other.chunks.items():
            if hi in result.chunks:
                existing = result.chunks[hi]
                if c.dense is not None and existing.dense is not None:
                    existing.dense = existing.dense | c.dense
                elif c.dense is None and existing.dense is None:
                    existing.sparse = sorted(set(existing.sparse) | set(c.sparse))
                    if len(existing.sparse) > RoaringBitmap.SPARSE_THRESHOLD:
                        bm = Bitmap(RoaringBitmap.CHUNK_SIZE)
                        for v in existing.sparse:
                            bm.set(v)
                        existing.dense = bm
                        existing.sparse = []
                elif c.dense is not None:
                    for v in existing.sparse:
                        c.dense.set(v)
                    existing.dense = c.dense
                    existing.sparse = []
                else:
                    for v in c.sparse:
                        if existing.dense.get(v) == 0:
                            existing.sparse.append(v)
                    existing.sparse = sorted(set(existing.sparse))
                    if len(existing.sparse) > RoaringBitmap.SPARSE_THRESHOLD:
                        bm = Bitmap(RoaringBitmap.CHUNK_SIZE)
                        for v in existing.sparse:
                            bm.set(v)
                        existing.dense = bm
                        existing.sparse = []
            else:
                result.chunks[hi] = c
        return result

    def cardinality(self) -> int:
        total = 0
        for c in self.chunks.values():
            if c.dense is not None:
                total += c.dense.count()
            else:
                total += len(c.sparse)
        return total


class BitmapIndex:
    def __init__(self, column_name: str):
        self.column_name = column_name
        self.bitmaps: dict[Any, RoaringBitmap] = {}
        self.row_count = 0

    def insert(self, row_id: int, value: Any) -> None:
        if value not in self.bitmaps:
            self.bitmaps[value] = RoaringBitmap()
        self.bitmaps[value].add(row_id)
        self.row_count = max(self.row_count, row_id + 1)

    def query_eq(self, value: Any) -> RoaringBitmap:
        return self.bitmaps.get(value, RoaringBitmap())

    def query_in(self, values: list[Any]) -> RoaringBitmap:
        result = RoaringBitmap()
        for v in values:
            if v in self.bitmaps:
                result = result | self.bitmaps[v]
        return result

    def query_and(self, value1: Any, value2: Any) -> RoaringBitmap:
        b1 = self.bitmaps.get(value1, RoaringBitmap())
        b2 = self.bitmaps.get(value2, RoaringBitmap())
        return b1 & b2
```

### Step 3: Extendible Hash in Rust (Performant)

```rust
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

struct Bucket<K: Clone + Eq + Hash, V: Clone> {
    depth: u32,
    keys: Vec<K>,
    values: Vec<V>,
    capacity: usize,
}

impl<K: Clone + Eq + Hash, V: Clone> Bucket<K, V> {
    fn new(depth: u32) -> Self {
        Bucket { depth, keys: Vec::new(), values: Vec::new(), capacity: 2 }
    }

    fn is_full(&self) -> bool {
        self.keys.len() >= self.capacity
    }

    fn search(&self, key: &K) -> Option<V> {
        self.keys.iter().position(|k| k == key).map(|i| self.values[i].clone())
    }

    fn upsert(&mut self, key: K, value: V) -> bool {
        if let Some(i) = self.keys.iter().position(|k| *k == key) {
            self.values[i] = value;
            return false;
        }
        if self.is_full() { return false; }
        self.keys.push(key);
        self.values.push(value);
        true
    }

    fn remove(&mut self, key: &K) -> bool {
        if let Some(i) = self.keys.iter().position(|k| k == key) {
            self.keys.swap_remove(i);
            self.values.swap_remove(i);
            return true;
        }
        false
    }
}

pub struct ExtendibleHash<K: Clone + Eq + Hash, V: Clone> {
    global_depth: u32,
    directory: Vec<*mut Bucket<K, V>>,
}

impl<K: Clone + Eq + Hash, V: Clone> ExtendibleHash<K, V> {
    pub fn new() -> Self {
        let mut dir = Vec::new();
        let bucket = Box::into_raw(Box::new(Bucket::new(1)));
        dir.push(bucket);
        dir.push(bucket);
        ExtendibleHash { global_depth: 1, directory: dir }
    }

    fn hash_key(key: &K) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn mask(depth: u32) -> u32 {
        (1 << depth) - 1
    }

    fn dir_index(hash: u64, depth: u32) -> usize {
        (hash as u32 & Self::mask(depth)) as usize
    }

    pub fn search(&self, key: &K) -> Option<V> {
        let hash = Self::hash_key(key);
        let idx = Self::dir_index(hash, self.global_depth);
        unsafe { (*self.directory[idx]).search(key) }
    }

    pub fn insert(&mut self, key: K, value: V) {
        let hash = Self::hash_key(&key);
        let idx = Self::dir_index(hash, self.global_depth);
        let bucket = self.directory[idx];
        unsafe {
            if !(*bucket).is_full() || (*bucket).search(&key).is_some() {
                (*bucket).upsert(key, value);
                return;
            }
        }
        self.split(idx, key, value, hash);
    }

    fn split(&mut self, idx: usize, key: K, value: V, hash: u64) {
        let old_bucket = self.directory[idx];
        let old_depth;
        unsafe { old_depth = (*old_bucket).depth; }

        let new_depth = old_depth + 1;

        if new_depth > self.global_depth {
            self.double_directory();
        }

        let mut b0 = Box::new(Bucket::new(new_depth));
        let mut b1 = Box::new(Bucket::new(new_depth));

        unsafe {
            let old_keys = std::mem::take(&mut (*old_bucket).keys);
            let old_values = std::mem::take(&mut (*old_bucket).values);
            for (k, v) in old_keys.into_iter().zip(old_values.into_iter()) {
                let h = Self::hash_key(&k);
                if (Self::dir_index(h, new_depth) & (1 << old_depth)) == 0 {
                    b0.keys.push(k); b0.values.push(v);
                } else {
                    b1.keys.push(k); b1.values.push(v);
                }
            }
        }

        if (Self::dir_index(hash, new_depth) & (1 << old_depth)) == 0 {
            b0.keys.push(key); b0.values.push(value);
        } else {
            b1.keys.push(key); b1.values.push(value);
        }

        let b0_ptr = Box::into_raw(b0);
        let b1_ptr = Box::into_raw(b1);

        let step = 1 << old_depth;
        let dir_size = 1 << self.global_depth;
        for i in (0..dir_size).step_by(step * 2) {
            for j in 0..step {
                let target_idx = i + j;
                let hash_for_bit = Self::hash_key(
                    // pick any key or just use idx+j
                );
                // Simplified: we determine which side of the split based on the idx+j'th bit at old_depth
                if (target_idx & (1 << old_depth)) == 0 {
                    self.directory[i + j] = b0_ptr;
                    self.directory[i + j + step] = b1_ptr;
                } else {
                    self.directory[i + j] = b1_ptr;
                    self.directory[i + j + step] = b0_ptr;
                }
            }
        }
    }

    fn double_directory(&mut self) {
        let old_size = self.directory.len();
        self.directory.reserve(old_size);
        for i in 0..old_size {
            self.directory.push(self.directory[i]);
        }
        self.global_depth += 1;
    }

    pub fn remove(&mut self, key: &K) -> bool {
        let hash = Self::hash_key(key);
        let idx = Self::dir_index(hash, self.global_depth);
        unsafe { (*self.directory[idx]).remove(key) }
    }
}
```

## Use It

### PostgreSQL Hash Indexes

PostgreSQL supports hash indexes (`CREATE INDEX ... USING hash`). Unlike B-trees, they are only useful for equality comparisons (`=`, `IN`). Before PostgreSQL 10, hash indexes were not WAL-logged and had poor concurrency — they were rarely recommended. Since PG 10, they are crash-safe and replicated. Under the hood, PostgreSQL uses extendible hashing with a directory of 4 KB page buckets.

**When to use hash over B-tree:**
- Only equality lookups (no ORDER BY, no `<`, `>`, `BETWEEN`, `LIKE`)
- The indexed values are long (hashing long strings is cheaper than comparing them)
- You never need index-only scans (hash indexes don't store the original key in the index entry — they store a hash code, so they can't support index-only scans in all cases)

### PostgreSQL GiST Indexes

GiST is the Swiss Army knife of PostgreSQL indexing. `CREATE INDEX ... USING gist (col)`. Built-in operator classes include:

- `gist_geometry_ops` — for PostGIS spatial queries (`ST_Contains`, `ST_Within`, `ST_DWithin`)
- `gist_trgm_ops` — from `pg_trgm` extension, for fuzzy string matching (`LIKE`, `ILIKE`, similarity)
- `gist_tsvector_ops` — for full-text search (`@@` operator with `tsquery`)
- `gist_intarray_ops` — from `intarray` extension, for array containment (`@>`, `<@`)
- `gist_range_ops` — for range type overlap (`&&`, `@>`, `<@`)

### Bitmap Scans in PostgreSQL

PostgreSQL doesn't store bitmap indexes, but it *uses* bitmap scan plans. When the planner estimates multiple index conditions, it can fetch bitmaps from multiple indexes, combine them with AND/OR in memory, and then fetch heap pages in sorted order (BitmapAnd, BitmapOr). This approximates a bitmap index scan without the storage overhead.

### SQLite

SQLite only supports B-tree indexes. No hash, no GiST. Use explicit `WITHOUT ROWID` tables or `EXPRESSION` indexes for limited alternatives.

## Read the Source

- **PostgreSQL extendible hash**: `src/backend/access/hash/README` and `hashfunc.c` — the actual implementation with overflow chaining, split handling
- **PostgreSQL GiST**: `src/backend/access/gist/gist.c` — the core GiST algorithm; `src/backend/access/gist/gistproc.c` — built-in R-tree over geometric types
- **Roaring Bitmaps**: `https://github.com/RoaringBitmap/roaring-rs` — production Rust implementation with SIMD optimizations

## Ship It

The reusable artifact is the extendible hash index implementation in `code/`. Take it into Phase 10's capstone (MVCC KV store) as an alternative index structure for equality-only lookups — you can switch between B-tree and hash index in the storage engine.

## Exercises

1. **Easy** — In the Python ExtendibleHash, trace the state of the directory and buckets after inserting keys 1-8 into an initially empty index. Draw the directory at each split.
2. **Medium** — Add `merge` to ExtendibleHash: when deleting a key leaves a bucket less than half full, attempt to merge with its "buddy" bucket (the one sharing the same parent in the directory doubling tree). Halve the directory if all buckets fit in half the space.
3. **Hard** — Implement a `GiST` class in Python that indexes integer ranges `(start, end)`. Define `consistent` as overlap check, `penalty` as area expansion, and `picksplit` as sorting by range start and splitting at the midpoint. Query it with `find_overlapping(range)`.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Hash index | Fast equality lookups | Only supports `=`, no range scans, no ordering |
| Directory doubling | The index doubles in size | Only the split bucket's directory entries are updated; the rest are duplicates |
| Bitmap index | Fast for low-cardinality columns | Bitwise AND/OR on compressed bit vectors; decompression cost for high-cardinality columns |
| GiST | Index for everything | A balanced tree where the user provides consistent/union/penalty/picksplit — the tree structure is generic |
| Roaring Bitmap | A better compressed bitmap | Divides 32-bit space into 2^16 chunks; dense → bitset, sparse → sorted array, automatic conversion |

## Further Reading

- [R. Fagin et al., "Extendible Hashing — A Fast Access Method for Dynamic Files"](https://doi.org/10.1145/320083.320084) — the original paper
- [W. Litwin, "Linear Hashing: A New Tool for File and Table Addressing"](https://doi.org/10.1109/SF.1980.10017) — the other dynamic hashing scheme
- [J. M. Hellerstein et al., "The GiST: Generalized Search Trees"](https://doi.org/10.1145/223784.223788) — the original GiST paper that brought extensible indexing to PostgreSQL
- [Roaring Bitmaps paper](https://doi.org/10.14778/2741948.2741957) — the compression scheme that replaced WAH/BBC in modern systems
- [PostgreSQL Index Types docs](https://www.postgresql.org/docs/current/indexes-types.html) — official reference for all supported index types
