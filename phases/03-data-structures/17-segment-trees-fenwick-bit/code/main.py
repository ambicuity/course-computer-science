"""main.py — iterative segment tree + Fenwick (BIT) in Python."""
from __future__ import annotations


class SegTree:
    def __init__(self, a: list[int]) -> None:
        self.n = len(a)
        self.t = [0] * (2 * self.n)
        for i, x in enumerate(a): self.t[i + self.n] = x
        for i in range(self.n - 1, 0, -1):
            self.t[i] = self.t[2*i] + self.t[2*i+1]

    def update(self, i: int, x: int) -> None:
        i += self.n
        self.t[i] = x
        i >>= 1
        while i:
            self.t[i] = self.t[2*i] + self.t[2*i+1]
            i >>= 1

    def query(self, l: int, r: int) -> int:           # [l, r)
        res = 0
        l += self.n; r += self.n
        while l < r:
            if l & 1: res += self.t[l]; l += 1
            if r & 1: r -= 1; res += self.t[r]
            l >>= 1; r >>= 1
        return res


class Fenwick:
    def __init__(self, n: int) -> None:
        self.n = n
        self.b = [0] * (n + 1)

    def add(self, i: int, x: int) -> None:
        i += 1
        while i <= self.n:
            self.b[i] += x
            i += i & -i

    def prefix(self, i: int) -> int:
        s = 0
        while i > 0:
            s += self.b[i]
            i -= i & -i
        return s

    def range(self, l: int, r: int) -> int:           # [l, r)
        return self.prefix(r) - self.prefix(l)


def main() -> None:
    A = [1, 3, 5, 7, 9, 11, 13, 15]
    st = SegTree(A)
    print(f"SegTree.query(0, 8) = {st.query(0, 8)}  (expect 64)")
    print(f"SegTree.query(2, 5) = {st.query(2, 5)}  (expect 21)")
    st.update(2, 100)
    print(f"after update(2, 100): query(2, 5) = {st.query(2, 5)}  (expect 116)")

    f = Fenwick(len(A))
    for i, x in enumerate(A): f.add(i, x)
    print(f"\nFenwick.range(0, 8) = {f.range(0, 8)}  (expect 64)")
    print(f"Fenwick.range(2, 5) = {f.range(2, 5)}  (expect 21)")
    f.add(2, 95)
    print(f"after add(2, +95): range(2, 5) = {f.range(2, 5)}  (expect 116)")


if __name__ == "__main__":
    main()
