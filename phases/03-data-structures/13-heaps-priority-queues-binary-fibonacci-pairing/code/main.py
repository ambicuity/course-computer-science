"""main.py — binary heap from scratch, compared with stdlib heapq."""
from __future__ import annotations
import heapq
import random
import time


def sift_up(a: list[int], i: int) -> None:
    while i > 0:
        p = (i - 1) // 2
        if a[p] <= a[i]: return
        a[p], a[i] = a[i], a[p]
        i = p


def sift_down(a: list[int], n: int, i: int) -> None:
    while True:
        l, r = 2 * i + 1, 2 * i + 2
        smallest = i
        if l < n and a[l] < a[smallest]: smallest = l
        if r < n and a[r] < a[smallest]: smallest = r
        if smallest == i: return
        a[i], a[smallest] = a[smallest], a[i]
        i = smallest


def push(a: list[int], x: int) -> None:
    a.append(x)
    sift_up(a, len(a) - 1)


def pop(a: list[int]) -> int:
    top = a[0]
    last = a.pop()
    if a:
        a[0] = last
        sift_down(a, len(a), 0)
    return top


def build_heap(a: list[int]) -> None:
    """Floyd's O(n) heapify in place."""
    n = len(a)
    for i in range(n // 2 - 1, -1, -1):
        sift_down(a, n, i)


def verify(a: list[int]) -> bool:
    for i in range(1, len(a)):
        if a[(i - 1) // 2] > a[i]: return False
    return True


def main() -> None:
    a: list[int] = []
    for x in [5, 1, 9, 3, 7, 2, 8, 4, 6]:
        push(a, x)
    print("After pushes, peek =", a[0], "(expect 1)")
    print("invariant:", verify(a))
    out = []
    while a: out.append(pop(a))
    print("pops:", out)

    # Bench against stdlib
    random.seed(42)
    N = 200_000
    data = [random.randrange(1 << 20) for _ in range(N)]

    a = data.copy()
    t0 = time.perf_counter()
    build_heap(a)
    t_mine = time.perf_counter() - t0
    assert verify(a)

    a = data.copy()
    t0 = time.perf_counter()
    heapq.heapify(a)
    t_lib = time.perf_counter() - t0

    print(f"\nbuild_heap (N={N}):")
    print(f"  ours:        {t_mine*1000:.1f} ms")
    print(f"  heapq.heapify: {t_lib*1000:.1f} ms")


if __name__ == "__main__":
    main()
