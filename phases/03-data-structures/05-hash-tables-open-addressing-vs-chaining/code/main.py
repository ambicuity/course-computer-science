"""main.py — chaining and linear-probing hash tables in Python.

Compares a hand-rolled chaining dict and a hand-rolled open-addressing dict
against CPython's built-in `dict`. The point is to see how the cost shape
changes with load factor.
"""
from __future__ import annotations
import time, random


class ChainDict:
    def __init__(self, cap: int = 16):
        self.cap = cap
        self.len = 0
        self.buckets: list[list[tuple]] = [[] for _ in range(cap)]

    def _idx(self, key) -> int:
        return hash(key) & (self.cap - 1)

    def _resize(self) -> None:
        old = self.buckets
        self.cap *= 2
        self.buckets = [[] for _ in range(self.cap)]
        self.len = 0
        for bucket in old:
            for k, v in bucket:
                self[k] = v

    def __setitem__(self, key, val) -> None:
        if 4 * self.len > 3 * self.cap:
            self._resize()
        i = self._idx(key)
        for j, (k, _) in enumerate(self.buckets[i]):
            if k == key:
                self.buckets[i][j] = (key, val)
                return
        self.buckets[i].append((key, val))
        self.len += 1

    def __getitem__(self, key):
        for k, v in self.buckets[self._idx(key)]:
            if k == key:
                return v
        raise KeyError(key)

    def __contains__(self, key) -> bool:
        try: self[key]; return True
        except KeyError: return False


class LinearProbeDict:
    EMPTY = object()
    def __init__(self, cap: int = 16):
        self.cap = cap
        self.len = 0
        self.slots: list = [(self.EMPTY, None)] * cap

    def _idx(self, key) -> int:
        return hash(key) & (self.cap - 1)

    def _resize(self) -> None:
        old = self.slots
        self.cap *= 2
        self.slots = [(self.EMPTY, None)] * self.cap
        self.len = 0
        for k, v in old:
            if k is not self.EMPTY: self[k] = v

    def __setitem__(self, key, val) -> None:
        if 2 * self.len > self.cap:
            self._resize()
        i = self._idx(key)
        while True:
            k, _ = self.slots[i]
            if k is self.EMPTY:
                self.slots[i] = (key, val); self.len += 1; return
            if k == key:
                self.slots[i] = (key, val); return
            i = (i + 1) & (self.cap - 1)

    def __getitem__(self, key):
        i = self._idx(key)
        while True:
            k, v = self.slots[i]
            if k is self.EMPTY: raise KeyError(key)
            if k == key: return v
            i = (i + 1) & (self.cap - 1)


def bench(label: str, fn) -> float:
    t0 = time.perf_counter()
    fn()
    t = time.perf_counter() - t0
    print(f"  {label:<28} {t * 1000:>8.2f} ms")
    return t


def main() -> None:
    N = 50_000
    random.seed(42)
    keys = [random.getrandbits(48) for _ in range(N)]

    def py_dict():
        d: dict = {}
        for k in keys: d[k] = 1
        for k in keys: _ = d[k]

    def chain():
        d = ChainDict()
        for k in keys: d[k] = 1
        for k in keys: _ = d[k]

    def linear():
        d = LinearProbeDict()
        for k in keys: d[k] = 1
        for k in keys: _ = d[k]

    print(f"== {N} inserts + {N} lookups ==")
    bench("CPython dict (production)", py_dict)
    bench("Chaining (hand-rolled)", chain)
    bench("Linear probing (hand-rolled)", linear)


if __name__ == "__main__":
    main()
