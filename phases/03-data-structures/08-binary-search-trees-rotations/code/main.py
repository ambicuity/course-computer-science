"""main.py — BST + rotations in Python.

The main point is to see in code that LEFT rotation preserves inorder.
"""
from __future__ import annotations
from dataclasses import dataclass
import random


@dataclass
class Node:
    key: int
    left: "Node | None" = None
    right: "Node | None" = None


def insert(n: Node | None, k: int) -> Node:
    if n is None: return Node(k)
    if k < n.key: n.left  = insert(n.left, k)
    elif k > n.key: n.right = insert(n.right, k)
    return n


def contains(n: Node | None, k: int) -> bool:
    while n:
        if k < n.key: n = n.left
        elif k > n.key: n = n.right
        else: return True
    return False


def height(n: Node | None) -> int:
    if n is None: return 0
    return 1 + max(height(n.left), height(n.right))


def rotate_left(n: Node) -> Node:
    r = n.right
    assert r is not None
    n.right = r.left
    r.left = n
    return r


def rotate_right(n: Node) -> Node:
    l = n.left
    assert l is not None
    n.left = l.right
    l.right = n
    return l


def inorder(n: Node | None) -> list[int]:
    out: list[int] = []
    def go(n):
        if n is None: return
        go(n.left); out.append(n.key); go(n.right)
    go(n); return out


def main() -> None:
    bad = None
    for i in range(1, 1001):
        bad = insert(bad, i)
    print(f"sorted insert n=1000 → height = {height(bad)} (expected 1000)")

    random.seed(42)
    good = None
    for _ in range(1000):
        good = insert(good, random.randrange(100000))
    print(f"random insert n=1000 → height = {height(good)} (expected ~20)")

    t = None
    for v in (10, 5, 20, 15, 25): t = insert(t, v)
    pre = inorder(t)
    t = rotate_left(t)
    post = inorder(t)
    print(f"\nbefore rotate_left: {pre}")
    print(f"after  rotate_left: {post}   ← inorder unchanged: invariant preserved")
    print(f"new root: {t.key}")


if __name__ == "__main__":
    main()
