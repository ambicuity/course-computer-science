"""main.py — binary tree traversal patterns in Python.

Same tree, same traversals, cleaner. Includes the "tuple recursion" for
diameter+height in one pass.
"""
from __future__ import annotations
from collections import deque
from dataclasses import dataclass


@dataclass
class Node:
    data: int
    left: "Node | None" = None
    right: "Node | None" = None


def preorder(n: Node | None, out: list[int]) -> None:
    if n is None: return
    out.append(n.data); preorder(n.left, out); preorder(n.right, out)

def inorder(n: Node | None, out: list[int]) -> None:
    if n is None: return
    inorder(n.left, out); out.append(n.data); inorder(n.right, out)

def postorder(n: Node | None, out: list[int]) -> None:
    if n is None: return
    postorder(n.left, out); postorder(n.right, out); out.append(n.data)


def inorder_iter(root: Node | None) -> list[int]:
    out: list[int] = []
    stack: list[Node] = []
    cur = root
    while cur or stack:
        while cur:
            stack.append(cur)
            cur = cur.left
        cur = stack.pop()
        out.append(cur.data)
        cur = cur.right
    return out


def bfs(root: Node | None) -> list[int]:
    out: list[int] = []
    if root is None: return out
    q: deque[Node] = deque([root])
    while q:
        n = q.popleft()
        out.append(n.data)
        if n.left:  q.append(n.left)
        if n.right: q.append(n.right)
    return out


def stats(n: Node | None) -> tuple[int, int, bool]:
    """Return (height, diameter, balanced) in one pass."""
    if n is None: return (0, 0, True)
    lh, ld, lb = stats(n.left)
    rh, rd, rb = stats(n.right)
    h = 1 + max(lh, rh)
    d = max(ld, rd, lh + rh)
    bal = lb and rb and abs(lh - rh) <= 1
    return (h, d, bal)


def main() -> None:
    t = Node(1,
             Node(2, Node(4), Node(5, None, Node(7))),
             Node(3, None, Node(6)))

    pre, ino, post = [], [], []
    preorder(t, pre); inorder(t, ino); postorder(t, post)
    print("preorder :", pre)
    print("inorder  :", ino)
    print("postorder:", post)
    print("inorder_iter :", inorder_iter(t))
    print("BFS          :", bfs(t))

    h, d, bal = stats(t)
    print(f"\nheight={h}  diameter={d}  balanced={bal}")


if __name__ == "__main__":
    main()
