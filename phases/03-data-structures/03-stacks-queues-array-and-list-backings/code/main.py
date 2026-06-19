"""main.py — stacks & queues benchmark in Python.

Python doesn't expose pointers, so the "list backing" demo is illustrative.
The interesting comparison is `list.pop(0)` (O(n) shift) vs `deque.popleft()` (O(1) ring buffer).
"""
from __future__ import annotations
from collections import deque
import time


def bench(label: str, fn) -> None:
    t0 = time.perf_counter()
    fn()
    t = time.perf_counter() - t0
    print(f"  {label:<32}  {t * 1000:>7.2f} ms")


def main() -> None:
    N = 50_000

    # --- Stack ---
    print(f"== Stack-on-list ({N} push + pop) ==")
    def stack_list():
        s = []
        for i in range(N): s.append(i)
        while s: s.pop()
    bench("list.append/pop", stack_list)

    # --- Queue, naive (list.pop(0) is O(n)) ---
    print(f"\n== Queue: rolling-window, steady-state W=5000, R={N} cycles ==")
    W, R = 5000, N
    def queue_naive():
        q = list(range(W))
        for i in range(R):
            q.append(i)
            q.pop(0)        # O(n) shift!
    bench("list-as-queue (list.pop(0))", queue_naive)

    def queue_deque():
        q = deque(range(W))
        for i in range(R):
            q.append(i)
            q.popleft()
    bench("collections.deque", queue_deque)

    print(f"\nLesson: list.pop(0) is O(n). For FIFOs in Python, always use collections.deque.")


if __name__ == "__main__":
    main()
