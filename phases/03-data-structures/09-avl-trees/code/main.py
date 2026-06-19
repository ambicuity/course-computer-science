"""main.py — AVL tree in Python with all 4 rebalance cases."""
from __future__ import annotations
from dataclasses import dataclass
import random


@dataclass
class Node:
    key: int
    height: int = 1
    left: "Node | None" = None
    right: "Node | None" = None


def h(n: Node | None) -> int: return 0 if n is None else n.height
def bf(n: Node) -> int: return h(n.left) - h(n.right)


def update_height(n: Node) -> None:
    n.height = 1 + max(h(n.left), h(n.right))


def rotate_left(n: Node) -> Node:
    r = n.right; assert r is not None
    n.right = r.left
    r.left = n
    update_height(n); update_height(r)
    return r


def rotate_right(n: Node) -> Node:
    l = n.left; assert l is not None
    n.left = l.right
    l.right = n
    update_height(n); update_height(l)
    return l


def rebalance(n: Node) -> Node:
    update_height(n)
    b = bf(n)
    if b > 1:
        if bf(n.left) < 0: n.left = rotate_left(n.left)   # LR
        return rotate_right(n)                            # LL
    if b < -1:
        if bf(n.right) > 0: n.right = rotate_right(n.right)  # RL
        return rotate_left(n)                                # RR
    return n


def insert(n: Node | None, k: int) -> Node:
    if n is None: return Node(k)
    if k < n.key: n.left = insert(n.left, k)
    elif k > n.key: n.right = insert(n.right, k)
    else: return n
    return rebalance(n)


def verify(n: Node | None) -> int:
    """Return height; raise on AVL violation."""
    if n is None: return 0
    lh = verify(n.left); rh = verify(n.right)
    assert abs(lh - rh) <= 1, f"violation at key={n.key}, lh={lh}, rh={rh}"
    return 1 + max(lh, rh)


def main() -> None:
    t = None
    for i in range(1, 1001): t = insert(t, i)
    print(f"sorted insert n=1000 → height = {verify(t)} (max ≈ 14)")

    random.seed(42)
    t = None
    for _ in range(10000): t = insert(t, random.randrange(100000))
    print(f"random insert n=10000 → height = {verify(t)} (max ≈ 19)")

    for seq, label in [
        ([3, 2, 1], "LL"),
        ([1, 2, 3], "RR"),
        ([3, 1, 2], "LR"),
        ([1, 3, 2], "RL"),
    ]:
        t = None
        for k in seq: t = insert(t, k)
        print(f"  {label} insert {seq}: root = {t.key} (expect 2)")


if __name__ == "__main__":
    main()
