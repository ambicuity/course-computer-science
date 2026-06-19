"""main.py — sparse table for RMQ in Python."""
from __future__ import annotations
import math


class RMQ:
    def __init__(self, a: list[int]) -> None:
        n = len(a)
        self.n = n
        self.log2 = [0] * (n + 1)
        for i in range(2, n + 1):
            self.log2[i] = self.log2[i // 2] + 1
        max_k = self.log2[n] + 1
        self.t: list[list[int]] = [list(a)]
        for k in range(1, max_k):
            prev = self.t[-1]
            half = 1 << (k - 1)
            row = [min(prev[i], prev[i + half]) for i in range(n - (1 << k) + 1)]
            self.t.append(row)

    def query(self, l: int, r: int) -> int:                # inclusive
        k = self.log2[r - l + 1]
        return min(self.t[k][l], self.t[k][r - (1 << k) + 1])


def main() -> None:
    import random
    random.seed(42)
    a = [random.randrange(1000) for _ in range(500)]
    rmq = RMQ(a)
    ok = all(rmq.query(l, r) == min(a[l:r+1])
             for l, r in [(random.randrange(500), 0) for _ in range(200)]
             if (r := random.randrange(l, 500)) or True)
    # Simpler check:
    ok = True
    for _ in range(200):
        l = random.randrange(500)
        r = random.randrange(l, 500)
        if rmq.query(l, r) != min(a[l:r+1]): ok = False; break
    print(f"RMQ verified on 200 random queries: {ok}")
    print(f"query(10, 50) = {rmq.query(10, 50)}, brute = {min(a[10:51])}")


if __name__ == "__main__":
    main()
