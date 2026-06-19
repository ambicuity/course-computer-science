"""main.py — Splay tree + Treap in Python."""
from __future__ import annotations
from dataclasses import dataclass
import random


# ============================================================
# Splay tree (bottom-up recursive)
# ============================================================

@dataclass
class SNode:
    key: int
    left: "SNode | None" = None
    right: "SNode | None" = None


def srot_l(n: SNode) -> SNode:
    r = n.right; assert r is not None
    n.right = r.left; r.left = n; return r

def srot_r(n: SNode) -> SNode:
    l = n.left; assert l is not None
    n.left = l.right; l.right = n; return l


def splay(root: SNode | None, k: int) -> SNode | None:
    if root is None or root.key == k: return root
    if k < root.key:
        if root.left is None: return root
        if k < root.left.key:
            root.left.left = splay(root.left.left, k)
            root = srot_r(root)
        elif k > root.left.key:
            root.left.right = splay(root.left.right, k)
            if root.left.right is not None: root.left = srot_l(root.left)
        return srot_r(root) if root.left is not None else root
    else:
        if root.right is None: return root
        if k > root.right.key:
            root.right.right = splay(root.right.right, k)
            root = srot_l(root)
        elif k < root.right.key:
            root.right.left = splay(root.right.left, k)
            if root.right.left is not None: root.right = srot_r(root.right)
        return srot_l(root) if root.right is not None else root


def splay_insert(root: SNode | None, k: int) -> SNode:
    if root is None: return SNode(k)
    root = splay(root, k)
    if root.key == k: return root
    n = SNode(k)
    if k < root.key:
        n.left, n.right, root.left = root.left, root, None
    else:
        n.right, n.left, root.right = root.right, root, None
    return n


def sheight(n: SNode | None) -> int:
    if n is None: return 0
    return 1 + max(sheight(n.left), sheight(n.right))


# ============================================================
# Treap
# ============================================================

@dataclass
class TNode:
    key: int
    prio: int
    left: "TNode | None" = None
    right: "TNode | None" = None


def trot_l(n: TNode) -> TNode:
    r = n.right; assert r is not None
    n.right = r.left; r.left = n; return r

def trot_r(n: TNode) -> TNode:
    l = n.left; assert l is not None
    n.left = l.right; l.right = n; return l


def treap_insert(n: TNode | None, k: int) -> TNode:
    if n is None: return TNode(k, random.randrange(1 << 30))
    if k < n.key:
        n.left = treap_insert(n.left, k)
        if n.left.prio > n.prio: n = trot_r(n)
    elif k > n.key:
        n.right = treap_insert(n.right, k)
        if n.right.prio > n.prio: n = trot_l(n)
    return n


def theight(n: TNode | None) -> int:
    if n is None: return 0
    return 1 + max(theight(n.left), theight(n.right))


def main() -> None:
    print("== Splay ==")
    s = None
    for i in range(1, 1001): s = splay_insert(s, i)
    print(f"  chain height after sorted insert: {sheight(s)}")
    s = splay(s, 1)
    print(f"  after splay(1):   root={s.key}, height={sheight(s)}")
    s = splay(s, 500)
    print(f"  after splay(500): root={s.key}, height={sheight(s)}")

    print("\n== Treap ==")
    random.seed(42)
    t = None
    for i in range(1, 10001): t = treap_insert(t, i)
    print(f"  height after sorted insert 1..10000: {theight(t)}  (expected ~28)")


if __name__ == "__main__":
    main()
