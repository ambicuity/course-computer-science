"""
Indexing — Hash, Bitmap, GiST
Phase 10 — Databases & Storage Systems

Implements:
- ExtendibleHashIndex: directory-based extendible hashing with global/local depth
- BitmapIndex: roaring-bitmap-like index supporting AND/OR queries
- Performance comparison demo
"""

import sys
import time
from typing import Any, Optional


class Bucket:
    def __init__(self, depth: int):
        self.depth = depth
        self.keys: list[int] = []
        self.values: list[int] = []

    @property
    def capacity(self) -> int:
        return 2

    @property
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
        if self.is_full:
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

    def items(self):
        return zip(self.keys, self.values)


class ExtendibleHashIndex:
    def __init__(self):
        self._global_depth = 1
        self._directory: list[Bucket] = [Bucket(self._global_depth)
                                         for _ in range(1 << self._global_depth)]

    def _hash_int(self, key: int) -> int:
        return hash(key) & 0xFFFFFFFF

    def _mask(self, depth: int) -> int:
        return (1 << depth) - 1

    def _dir_index(self, key: int, depth: int) -> int:
        return self._hash_int(key) & self._mask(depth)

    def search(self, key: int) -> Optional[int]:
        idx = self._dir_index(key, self._global_depth)
        return self._directory[idx].search(key)

    def insert(self, key: int, value: int) -> None:
        idx = self._dir_index(key, self._global_depth)
        bucket = self._directory[idx]
        if not bucket.is_full or bucket.search(key) is not None:
            bucket.insert(key, value)
            return
        self._split(idx, key, value)

    def _split(self, idx: int, key: int, value: int) -> None:
        bucket = self._directory[idx]
        old_depth = bucket.depth
        new_depth = old_depth + 1

        if new_depth > self._global_depth:
            self._double_directory()

        b0 = Bucket(new_depth)
        b1 = Bucket(new_depth)

        mask_bit = 1 << old_depth

        for k, v in bucket.items():
            target = b0 if (self._dir_index(k, new_depth) & mask_bit) == 0 else b1
            target.keys.append(k)
            target.values.append(v)

        target = b0 if (self._dir_index(key, new_depth) & mask_bit) == 0 else b1
        target.keys.append(key)
        target.values.append(value)

        for i, bp in enumerate(self._directory):
            if bp is bucket:
                if (i & mask_bit) == 0:
                    self._directory[i] = b0
                else:
                    self._directory[i] = b1

    def _double_directory(self) -> None:
        old_size = len(self._directory)
        self._directory.extend(self._directory[i % old_size]
                               for i in range(old_size))
        self._global_depth += 1

    def remove(self, key: int) -> bool:
        idx = self._dir_index(key, self._global_depth)
        return self._directory[idx].remove(key)


class Bitmap:
    def __init__(self, size: int = 0):
        self.size = size
        nwords = (size + 63) // 64 if size else 0
        self.words: list[int] = [0] * nwords

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

    def __and__(self, other: "Bitmap") -> "Bitmap":
        n = min(len(self.words), len(other.words))
        r = Bitmap()
        r.words = [self.words[i] & other.words[i] for i in range(n)]
        r.size = min(self.size, other.size)
        return r

    def __or__(self, other: "Bitmap") -> "Bitmap":
        n = max(len(self.words), len(other.words))
        r = Bitmap()
        r.words = [0] * n
        for i in range(n):
            a = self.words[i] if i < len(self.words) else 0
            b = other.words[i] if i < len(other.words) else 0
            r.words[i] = a | b
        r.size = max(self.size, other.size)
        return r

    def __invert__(self) -> "Bitmap":
        r = Bitmap()
        r.size = self.size
        mask = (1 << 64) - 1
        r.words = [~w & mask for w in self.words]
        return r

    def count(self) -> int:
        return sum(w.bit_count() for w in self.words)


class RoaringBitmap:
    CHUNK_SHIFT = 16
    CHUNK_SIZE = 1 << CHUNK_SHIFT
    SPARSE_THRESHOLD = 4096

    class Chunk:
        def __init__(self):
            self.dense: Optional[Bitmap] = None
            self.sparse: list[int] = []

    def __init__(self):
        self.chunks: dict[int, "RoaringBitmap.Chunk"] = {}

    def _chunk(self, key: int) -> "RoaringBitmap.Chunk":
        hi = key >> self.CHUNK_SHIFT
        if hi not in self.chunks:
            self.chunks[hi] = RoaringBitmap.Chunk()
        return self.chunks[hi]

    def add(self, key: int) -> None:
        lo = key & (self.CHUNK_SIZE - 1)
        c = self._chunk(key)
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

    def __and__(self, other: "RoaringBitmap") -> "RoaringBitmap":
        r = RoaringBitmap()
        for hi, c in self.chunks.items():
            if hi not in other.chunks:
                continue
            oc = other.chunks[hi]
            if c.dense is not None and oc.dense is not None:
                rc = RoaringBitmap.Chunk()
                rc.dense = c.dense & oc.dense
                r.chunks[hi] = rc
            elif c.dense is None and oc.dense is None:
                inter = sorted(set(c.sparse) & set(oc.sparse))
                if inter:
                    rc = RoaringBitmap.Chunk()
                    rc.sparse = inter
                    r.chunks[hi] = rc
            else:
                dense = c.dense if c.dense is not None else oc.dense
                sparse = oc.sparse if c.dense is not None else c.sparse
                rc = RoaringBitmap.Chunk()
                rc.sparse = [v for v in sparse if dense.get(v)]
                r.chunks[hi] = rc
        return r

    def __or__(self, other: "RoaringBitmap") -> "RoaringBitmap":
        r = RoaringBitmap()
        for hi, c in self.chunks.items():
            r.chunks[hi] = c
        for hi, c in other.chunks.items():
            if hi in r.chunks:
                ex = r.chunks[hi]
                if c.dense is not None and ex.dense is not None:
                    ex.dense = ex.dense | c.dense
                elif c.dense is None and ex.dense is None:
                    ex.sparse = sorted(set(ex.sparse) | set(c.sparse))
                    if len(ex.sparse) > self.SPARSE_THRESHOLD:
                        bm = Bitmap(self.CHUNK_SIZE)
                        for v in ex.sparse:
                            bm.set(v)
                        ex.dense = bm
                        ex.sparse = []
                elif c.dense is not None:
                    for v in ex.sparse:
                        c.dense.set(v)
                    ex.dense = c.dense
                    ex.sparse = []
                else:
                    for v in c.sparse:
                        if ex.dense.get(v) == 0:
                            ex.sparse.append(v)
                    ex.sparse = sorted(set(ex.sparse))
                    if len(ex.sparse) > self.SPARSE_THRESHOLD:
                        bm = Bitmap(self.CHUNK_SIZE)
                        for v in ex.sparse:
                            bm.set(v)
                        ex.dense = bm
                        ex.sparse = []
            else:
                r.chunks[hi] = c
        return r

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

    def insert(self, row_id: int, value: Any) -> None:
        if value not in self.bitmaps:
            self.bitmaps[value] = RoaringBitmap()
        self.bitmaps[value].add(row_id)

    def query_eq(self, value: Any) -> RoaringBitmap:
        return self.bitmaps.get(value, RoaringBitmap())

    def query_in(self, values: list[Any]) -> RoaringBitmap:
        r = RoaringBitmap()
        for v in values:
            if v in self.bitmaps:
                r = r | self.bitmaps[v]
        return r

    def query_and(self, v1: Any, v2: Any) -> RoaringBitmap:
        b1 = self.bitmaps.get(v1, RoaringBitmap())
        b2 = self.bitmaps.get(v2, RoaringBitmap())
        return b1 & b2


def demo_extendible_hash():
    print("=== Extendible Hash Index Demo ===")
    idx = ExtendibleHashIndex()
    for k, v in [(10, 100), (22, 200), (1, 10), (7, 70),
                 (15, 150), (3, 30), (31, 310), (9, 90)]:
        idx.insert(k, v)
    print(f"global_depth={idx._global_depth}, dir_size={len(idx._directory)}")
    for k in [10, 22, 1, 7, 15, 3, 31, 9, 99]:
        v = idx.search(k)
        print(f"  search({k:3d}) -> {v}")
    print()


def demo_bitmap_index():
    print("=== Bitmap Index Demo ===")
    idx = BitmapIndex("status")
    statuses = ["pending", "shipped", "cancelled", "shipped", "pending",
                "shipped", "pending", "cancelled", "shipped", "shipped"]
    for row_id, s in enumerate(statuses):
        idx.insert(row_id, s)
    shipped = idx.query_eq("shipped")
    cancelled = idx.query_eq("cancelled")
    print(f"  shipped:   {[i for i in range(10) if shipped.contains(i)]}")
    print(f"  cancelled: {[i for i in range(10) if cancelled.contains(i)]}")
    combined = shipped | cancelled
    print(f"  shipped|cancelled: {[i for i in range(10) if combined.contains(i)]}")

    idx2 = BitmapIndex("region")
    for row_id, r in enumerate([1, 2, 1, 3, 2, 1, 3, 2, 1, 2]):
        idx2.insert(row_id, r)
    region1 = idx2.query_eq(1)
    region2 = idx2.query_eq(2)
    shipped_region1 = shipped & region1
    print(f"  shipped & region=1: {[i for i in range(10) if shipped_region1.contains(i)]}")
    shipped_or_cancelled_r2 = (shipped | cancelled) & region2
    print(f"  (shipped|cancelled) & region=2: {[i for i in range(10) if shipped_or_cancelled_r2.contains(i)]}")
    print()


def demo_roaring():
    print("=== Roaring Bitmap Compression Demo ===")
    rb1 = RoaringBitmap()
    for i in range(0, 100000, 3):
        rb1.add(i)
    rb2 = RoaringBitmap()
    for i in range(0, 100000, 5):
        rb2.add(i)
    r_and = rb1 & rb2
    r_or = rb1 | rb2
    print(f"  multiples of 3: {rb1.cardinality()}")
    print(f"  multiples of 5: {rb2.cardinality()}")
    print(f"  multiples of 15: {r_and.cardinality()}")
    print(f"  multiples of 3 or 5: {r_or.cardinality()}")
    print()


def demo_performance():
    print("=== Performance Comparison ===")
    n = 50000

    t0 = time.perf_counter()
    eh = ExtendibleHashIndex()
    for i in range(n):
        eh.insert(i, i * 10)
    for i in range(n):
        eh.search(i)
    t_eh = time.perf_counter() - t0

    t0 = time.perf_counter()
    bm = BitmapIndex("even")
    for i in range(n):
        bm.insert(i, i % 2)
    for i in range(n):
        bm.query_eq(i % 2)
    t_bm = time.perf_counter() - t0

    print(f"  ExtendibleHash: {n} inserts + {n} searches: {t_eh:.4f}s")
    print(f"  BitmapIndex:    {n} inserts + {n} queries:    {t_bm:.4f}s")
    print()


def main():
    demo_extendible_hash()
    demo_bitmap_index()
    demo_roaring()
    demo_performance()


if __name__ == "__main__":
    main()
