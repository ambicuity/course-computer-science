# Binary Trees — Traversal and Recursion Patterns

> Trees recurse so cleanly that they're the data structure that teaches you to think recursively. Then they teach you why recursion has a stack — and how to do without one.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L01-04
**Time:** ~60 minutes

## Learning Objectives

- Implement preorder, inorder, postorder, and level-order traversal recursively and iteratively.
- Use the explicit-stack pattern that converts any recursive traversal to iterative.
- Implement **Morris traversal**: O(n) time, O(1) space, no stack, no recursion.
- Recognize the canonical tree-recursion shapes (diameter, max-depth, LCA, serialize) and the "return tuple" trick that solves them in one pass.

## The Problem

A binary tree is a recursive data structure: each node has 0, 1, or 2 children. The natural cost: a stack frame per node = O(h) extra space. For balanced h = O(log n); for degenerate h = n.

Real systems can't afford O(n) extra stack — a 100K-node tree on a 1MB-stack system blows up. So you must know how to:

1. **Traverse iteratively** with an explicit stack/queue.
2. **Traverse with O(1) space** (Morris's threaded-tree trick).
3. **Compute properties in one pass**: avoid quadratic-recursive patterns.

This lesson is the foundation for every later tree (BST, AVL, RB, B-tree, suffix tree).

## The Concept

### Tree node

```c
typedef struct Node {
    int data;
    struct Node *left, *right;
} Node;
```

### Three depth-first orders

| Order | Visit | Use |
|-------|-------|-----|
| **Preorder** | self → left → right | Serialize, clone |
| **Inorder** | left → self → right | BST → sorted |
| **Postorder** | left → right → self | Delete (free children first), expression eval |

Recursive (canonical):

```c
void inorder(Node *n) {
    if (!n) return;
    inorder(n->left);
    visit(n);
    inorder(n->right);
}
```

Iterative (with explicit stack) — push the "rest of the work" instead of recursing:

```c
void inorder_iter(Node *root) {
    Node *stack[H]; int top = 0;
    Node *cur = root;
    while (cur || top > 0) {
        while (cur) { stack[top++] = cur; cur = cur->left; }
        cur = stack[--top];
        visit(cur);
        cur = cur->right;
    }
}
```

### Level-order (BFS)

```c
queue.push(root);
while (!queue.empty()) {
    Node *n = queue.pop_front();
    visit(n);
    if (n->left) queue.push(n->left);
    if (n->right) queue.push(n->right);
}
```

O(n) time, O(width) space — width up to n/2 for a complete tree.

### Morris traversal (inorder, O(1) space)

Each leaf temporarily "threads" itself to its inorder successor through its right child. Two passes per node → O(n) time, O(1) space.

```c
void morris_inorder(Node *root) {
    Node *cur = root;
    while (cur) {
        if (!cur->left) { visit(cur); cur = cur->right; }
        else {
            Node *pred = cur->left;
            while (pred->right && pred->right != cur) pred = pred->right;
            if (!pred->right) { pred->right = cur; cur = cur->left; }
            else { pred->right = NULL; visit(cur); cur = cur->right; }
        }
    }
}
```

Use cases: heap-constrained embedded code; in-place tree-to-list conversions.

### Recursion patterns for tree properties

Almost every tree problem decomposes via this template:

```c
Result solve(Node *n) {
    if (!n) return base_case;
    Result l = solve(n->left);
    Result r = solve(n->right);
    return combine(n->data, l, r);
}
```

| Problem | base_case | combine |
|---------|-----------|---------|
| Count nodes | 0 | 1 + l + r |
| Max depth | 0 | 1 + max(l, r) |
| Sum | 0 | n.data + l + r |
| **Diameter** | (h=0, d=0) | h = 1+max(lh, rh); d = max(ld, rd, lh+rh) |
| **Balanced?** | (h=0, bal=T) | bal = lb ∧ rb ∧ |lh-rh| ≤ 1 |
| **LCA(a, b)** | NULL | if l ∧ r → self; else l ?? r |

The diameter/balance patterns are the famous "return a tuple" trick: solve two properties in one pass to avoid recomputing height O(n) times (naïve = O(n²)).

## Build It

`code/main.c` builds a binary tree, then runs all four traversals recursively, all three DFS iteratively, Morris inorder, and computes max-depth + diameter + balance in one pass each.

`code/main.py` mirrors with cleaner code.

`code/main.rs` uses `Option<Box<Node>>`.

### Run

```sh
clang -O2 main.c -o tree && ./tree
python3 main.py
```

## Use It

- **JSON/YAML/AST traversal**: every parser produces a tree; you walk it inorder or preorder.
- **HTML DOM**: CSS selector match = DFS; layout = visit-then-recurse.
- **Filesystem walks**: `find` is DFS preorder.
- **Compiler IR**: postorder evaluation of expression trees → emit one instruction per node.

## Read the Source

- [CPython `Python/ast.c`](https://github.com/python/cpython/blob/main/Python/ast.c) — AST visitor patterns.
- [LLVM `RecursiveASTVisitor`](https://clang.llvm.org/doxygen/classclang_1_1RecursiveASTVisitor.html).
- [Linux `lib/rbtree.c`](https://github.com/torvalds/linux/blob/master/lib/rbtree.c) — iterative walk in `rb_next`/`rb_prev`.

## Ship It

This lesson ships **`outputs/tree.h`** — a generic binary-tree header with all four traversal modes (callback API).

## Exercises

1. **Easy.** Implement `tree_height` and `tree_count` recursively; verify against iterative.
2. **Medium.** Implement `is_symmetric(Node*)` — does the tree mirror itself? Recursion: tree symmetric iff left and right are mirrors. ≤ 10 lines.
3. **Hard.** Serialize a tree (preorder + `#` for null) and deserialize. Both O(n).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Preorder / Inorder / Postorder | "DFS variants" | When you visit self relative to children |
| Level-order | "BFS" | Visit nodes by depth using a queue |
| Morris traversal | "Threaded" | O(1)-space inorder via temporary parent threads |
| Diameter | "Longest path" | Number of edges in the longest path between two nodes |
| LCA | "Lowest common ancestor" | Deepest node that is ancestor of both targets |

## Further Reading

- [Morris's 1979 paper](https://www.sciencedirect.com/science/article/pii/0020019079900683) — threaded binary trees.
- *Introduction to Algorithms* (CLRS) Ch. 12.
- *Pearls of Functional Algorithm Design* by Richard Bird.
