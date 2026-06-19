"""main.py — Bloom filter + Count-Min sketch in Python."""
from __future__ import annotations
import math


def mix64(x: int) -> int:
    x = (x + 0x9e3779b97f4a7c15) & 0xFFFFFFFFFFFFFFFF
    x = ((x ^ (x >> 30)) * 0xbf58476d1ce4e5b9) & 0xFFFFFFFFFFFFFFFF
    x = ((x ^ (x >> 27)) * 0x94d049bb133111eb) & 0xFFFFFFFFFFFFFFFF
    return x ^ (x >> 31)


class Bloom:
    def __init__(self, m: int, k: int) -> None:
        self.m, self.k = m, k
        self.bits = bytearray((m + 7) // 8)

    def _hashes(self, x: int) -> list[int]:
        return [mix64(x + i * 0xdeadbeef) % self.m for i in range(self.k)]

    def add(self, x: int) -> None:
        for h in self._hashes(x):
            self.bits[h // 8] |= 1 << (h & 7)

    def contains(self, x: int) -> bool:
        for h in self._hashes(x):
            if not (self.bits[h // 8] >> (h & 7)) & 1: return False
        return True


class CountMin:
    def __init__(self, w: int, d: int) -> None:
        self.w, self.d = w, d
        self.t = [[0] * w for _ in range(d)]

    def add(self, x: int, c: int = 1) -> None:
        for i in range(self.d):
            self.t[i][mix64(x + i * 0xcafef00d) % self.w] += c

    def estimate(self, x: int) -> int:
        return min(self.t[i][mix64(x + i * 0xcafef00d) % self.w] for i in range(self.d))


def main() -> None:
    b = Bloom(96000, 7)
    n_in, n_out = 10_000, 100_000
    for i in range(n_in): b.add(i)
    fp = sum(1 for i in range(n_in, n_in + n_out) if b.contains(i))
    fn = sum(1 for i in range(n_in) if not b.contains(i))
    theoretical = (1 - math.exp(-7 * n_in / 96000)) ** 7
    print(f"Bloom (m=96000, k=7, n={n_in}):")
    print(f"  false negatives: {fn} (must be 0)")
    print(f"  false positives: {fp} / {n_out} = {fp/n_out:.4f}  (theoretical {theoretical:.4f})")

    cm = CountMin(256, 4)
    for _ in range(1000): cm.add(42)
    for i in range(100):
        for _ in range(10): cm.add(i)
    print(f"\nCount-Min (w=256, d=4):")
    print(f"  estimate(42)  = {cm.estimate(42)}  (true 1010)")
    print(f"  estimate(7)   = {cm.estimate(7)}  (true 10)")
    print(f"  estimate(999) = {cm.estimate(999)}  (true 0)")


if __name__ == "__main__":
    main()
