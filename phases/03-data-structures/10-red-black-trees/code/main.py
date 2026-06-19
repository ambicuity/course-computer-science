"""main.py — Left-Leaning Red-Black tree in Python (Sedgewick 2008).

Concise reference implementation. Insert is 3 fixup actions; delete is omitted
(use the C version for full delete; Python's SortedList in `sortedcontainers`
covers production needs).
"""
from __future__ import annotations
from dataclasses import dataclass
import random

RED, BLACK = True, False


@dataclass
class Node:
    key: int
    color: bool = RED
    left: "Node | None" = None
    right: "Node | None" = None


def is_red(n: Node | None) -> bool:
    return n is not None and n.color is RED


def rotate_left(n: Node) -> Node:
    r = n.right; assert r is not None
    n.right = r.left
    r.left = n
    r.color = n.color
    n.color = RED
    return r


def rotate_right(n: Node) -> Node:
    l = n.left; assert l is not None
    n.left = l.right
    l.right = n
    l.color = n.color
    n.color = RED
    return l


def flip_colors(n: Node) -> None:
    n.color = not n.color
    n.left.color  = not n.left.color
    n.right.color = not n.right.color


def insert(n: Node | None, k: int) -> Node:
    if n is None:
        return Node(k, color=RED)
    if k < n.key: n.left = insert(n.left, k)
    elif k > n.key: n.right = insert(n.right, k)
    else: return n

    if is_red(n.right) and not is_red(n.left): n = rotate_left(n)
    if is_red(n.left) and is_red(n.left.left): n = rotate_right(n)
    if is_red(n.left) and is_red(n.right):     flip_colors(n)
    return n


def rb_insert(root: Node | None, k: int) -> Node:
    root = insert(root, k)
    root.color = BLACK
    return root


def verify(n: Node | None) -> tuple[int, int]:
    """Return (height, black_height)."""
    if n is None: return (0, 0)
    if is_red(n) and (is_red(n.left) or is_red(n.right)):
        raise AssertionError(f"red-red at key={n.key}")
    lh, lbh = verify(n.left)
    rh, rbh = verify(n.right)
    if lbh != rbh:
        raise AssertionError(f"BH mismatch at key={n.key}: {lbh} vs {rbh}")
    return (1 + max(lh, rh), lbh + (1 if n.color is BLACK else 0))


def main() -> None:
    t = None
    for i in range(1, 1001): t = rb_insert(t, i)
    h, bh = verify(t)
    print(f"sequential 1..1000: height={h}, black-height={bh}, invariants OK")

    random.seed(42)
    t = None
    for _ in range(10000):
        t = rb_insert(t, random.randrange(100000))
    h, bh = verify(t)
    print(f"random insert n=10000: height={h}, black-height={bh}, invariants OK")


if __name__ == "__main__":
    main()
