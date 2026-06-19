"""main.py — dynamic array in Python with growth-factor comparison.

Mirrors main.c. Makes the amortized analysis VISIBLE: see how each policy fares
on the SAME workload.
"""
from __future__ import annotations
from dataclasses import dataclass, field
import time


@dataclass
class Vec:
    data: list = field(default_factory=lambda: [None] * 4)
    len_: int = 0
    cap: int = 4
    resizes: int = 0
    total_copies: int = 0

    def push_factor(self, x, factor: float) -> None:
        if self.len_ == self.cap:
            new_cap = max(self.cap + 1, int(self.cap * factor))
            self.total_copies += self.len_
            self.resizes += 1
            new_data = [None] * new_cap
            for i in range(self.len_):
                new_data[i] = self.data[i]
            self.data = new_data
            self.cap = new_cap
        self.data[self.len_] = x
        self.len_ += 1

    def push_plus_k(self, x, k: int) -> None:
        if self.len_ == self.cap:
            new_cap = self.cap + k
            self.total_copies += self.len_
            self.resizes += 1
            new_data = [None] * new_cap
            for i in range(self.len_):
                new_data[i] = self.data[i]
            self.data = new_data
            self.cap = new_cap
        self.data[self.len_] = x
        self.len_ += 1


def report(label: str, v: Vec, n: int) -> None:
    print(
        f"{label:>14}: cap={v.cap:>8}  resizes={v.resizes:>6}  "
        f"copies={v.total_copies:>14}  amortized={(n + v.total_copies) / n:.2f} writes/push"
    )


def main() -> None:
    N = 200_000
    print(f"== Dynamic-array growth: {N} pushes ==\n")

    v2 = Vec()
    for i in range(N):
        v2.push_factor(i, 2.0)
    report("2.0× growth", v2, N)

    v15 = Vec()
    for i in range(N):
        v15.push_factor(i, 1.5)
    report("1.5× growth", v15, N)

    vk = Vec()
    for i in range(N):
        vk.push_plus_k(i, 8)
    report("+8   growth", vk, N)

    print()
    print("== CPython's list.append uses ~1.125× growth (Objects/listobject.c) ==")
    py: list[int] = []
    t0 = time.perf_counter()
    for i in range(N):
        py.append(i)
    t = time.perf_counter() - t0
    print(f"  list.append × {N}: {t * 1e6:.1f} us  ({t / N * 1e9:.1f} ns/op)")


if __name__ == "__main__":
    main()
