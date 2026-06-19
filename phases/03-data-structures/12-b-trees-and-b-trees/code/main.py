"""main.py — B+-tree with linked leaves and range query.

This is the simplified shape used by Postgres / SQLite / MySQL InnoDB:
- Internal nodes hold KEYS only (routing keys).
- Leaf nodes hold (key, value) pairs.
- Leaves are doubly-linked → O(k) range scan.
"""
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Optional, Union


M = 4  # order; small for visibility


@dataclass
class Leaf:
    keys: list[int] = field(default_factory=list)
    values: list[str] = field(default_factory=list)
    next: "Optional[Leaf]" = None
    prev: "Optional[Leaf]" = None


@dataclass
class Internal:
    keys: list[int] = field(default_factory=list)
    children: list[Union[Leaf, "Internal"]] = field(default_factory=list)


Node = Union[Leaf, Internal]


def search(root: Node, key: int) -> str | None:
    while isinstance(root, Internal):
        i = 0
        while i < len(root.keys) and key >= root.keys[i]: i += 1
        root = root.children[i]
    for k, v in zip(root.keys, root.values):
        if k == key: return v
    return None


def range_scan(root: Node, lo: int, hi: int) -> list[tuple[int, str]]:
    # Find leaf containing lo
    node: Node = root
    while isinstance(node, Internal):
        i = 0
        while i < len(node.keys) and lo >= node.keys[i]: i += 1
        node = node.children[i]
    # Walk leaves
    out = []
    while node is not None:
        for k, v in zip(node.keys, node.values):
            if lo <= k <= hi: out.append((k, v))
            elif k > hi: return out
        node = node.next
    return out


def _split_leaf(leaf: Leaf) -> tuple[int, Leaf]:
    mid = len(leaf.keys) // 2
    new = Leaf(keys=leaf.keys[mid:], values=leaf.values[mid:])
    new.next = leaf.next
    if new.next: new.next.prev = new
    new.prev = leaf
    leaf.next = new
    leaf.keys = leaf.keys[:mid]
    leaf.values = leaf.values[:mid]
    return new.keys[0], new          # promote leftmost key of the right half


def _insert_into_leaf(leaf: Leaf, k: int, v: str) -> None:
    i = 0
    while i < len(leaf.keys) and leaf.keys[i] < k: i += 1
    if i < len(leaf.keys) and leaf.keys[i] == k:
        leaf.values[i] = v
        return
    leaf.keys.insert(i, k)
    leaf.values.insert(i, v)


def insert(root: Node, k: int, v: str) -> tuple[Node, Optional[tuple[int, Node]]]:
    if isinstance(root, Leaf):
        _insert_into_leaf(root, k, v)
        if len(root.keys) >= M:
            promote_key, new_leaf = _split_leaf(root)
            return root, (promote_key, new_leaf)
        return root, None

    # Internal
    i = 0
    while i < len(root.keys) and k >= root.keys[i]: i += 1
    new_child, promote = insert(root.children[i], k, v)
    root.children[i] = new_child
    if promote is None: return root, None

    pk, pn = promote
    root.keys.insert(i, pk)
    root.children.insert(i + 1, pn)
    if len(root.keys) >= M:
        mid = len(root.keys) // 2
        promote_k = root.keys[mid]
        right = Internal(
            keys=root.keys[mid + 1:],
            children=root.children[mid + 1:],
        )
        root.keys = root.keys[:mid]
        root.children = root.children[:mid + 1]
        return root, (promote_k, right)
    return root, None


def insert_root(root: Node, k: int, v: str) -> Node:
    new_root, promote = insert(root, k, v)
    if promote is None: return new_root
    pk, pn = promote
    return Internal(keys=[pk], children=[new_root, pn])


def main() -> None:
    root: Node = Leaf()
    for k in [10, 20, 30, 5, 15, 25, 35, 1, 7, 12, 17, 22, 27, 32, 37, 40, 45, 50]:
        root = insert_root(root, k, f"v{k}")

    print(f"Search 17 → {search(root, 17)}")
    print(f"Search 99 → {search(root, 99)}")
    print(f"Range [15, 35] → {range_scan(root, 15, 35)}")


if __name__ == "__main__":
    main()
