# Binary Search Trees & Rotations

> A BST is an ordered tree where each subtree's keys lie in a strict range. Add rotations and you have the engine of every balanced tree.

**Type:** Build
**Languages:** C, Python, Rust
**Prerequisites:** P03 L07 (tree traversal)
**Time:** ~75 minutes

## Learning Objectives

- Implement BST insert, search, delete with the three deletion cases.
- Implement **left rotation** and **right rotation** — the universal primitive of every balanced BST.
- Understand why unbalanced BSTs degrade to O(n) and how rotations restore O(log n).
- Prove that rotations preserve the BST invariant.

## The Problem

A BST orders keys by position: for every node N, all keys in N's left subtree are < N.key, and all keys in N's right subtree are > N.key. This gives O(h) search/insert/delete and O(n) sorted iteration.

When inserts come in sorted order, h = n — the BST degenerates to a linked list. The fix is **self-balancing**: every operation locally restores a balance invariant via tree **rotations**.

## The Concept

### Invariant

```
∀ N: ∀ x ∈ left_subtree(N): x.key < N.key
     ∀ y ∈ right_subtree(N): y.key > N.key
```

### Search / Insert

```c
Node *search(Node *n, int k) {
    while (n) {
        if (k < n->key) n = n->left;
        else if (k > n->key) n = n->right;
        else return n;
    }
    return NULL;
}
```

Insert walks down like search; when you find NULL, hang a new leaf.

### Delete — three cases

1. **No children**: free node, set parent's pointer to NULL.
2. **One child**: set parent's pointer to the surviving child.
3. **Two children**: replace key with inorder successor's key; then delete the successor (which has ≤1 child).

### Rotations

**Left rotation** at N (when N has right child R):

```
    N                   R
   / \                 / \
  A   R       →       N   C
     / \             / \
    B   C           A   B
```

```c
Node *rotate_left(Node *n) {
    Node *r = n->right;
    n->right = r->left;
    r->left = n;
    return r;                /* new subtree root */
}

Node *rotate_right(Node *n) {
    Node *l = n->left;
    n->left = l->right;
    l->right = n;
    return l;
}
```

**Why rotations preserve BST order**: before, A < N < B < R < C. After left rotation: A < N < B (left subtree of R), B < R < C. Inorder is unchanged.

This is the entire mechanism. Self-balancing trees just decide *when* to rotate.

### Why unbalanced BSTs are bad

Insert 1, 2, 3, 4, 5 → right chain, h = 5. Sorted insert of n keys → O(n) height. Adversary controlling insert order can DOS a BST. Production systems use balanced BSTs (red-black, AVL, B-trees).

### Hibbard deletion's hidden tax

Standard inorder-successor delete biases the tree leftward over many ops. After O(n²) insert-delete pairs, an initially-balanced BST drifts to O(√n) height. Fix: randomize successor side. Balanced trees sidestep this entirely.

## Build It

`code/main.c`:

1. Plain BST with insert/search/delete.
2. Sequential insert 1..N → measure height (expect N: degenerate).
3. Random insert 1..N → measure height (expect ~2 log₂ N).
4. Rotation demo: rotate at root, verify inorder unchanged (proves invariant).

`code/main.py` mirrors with cleaner code.

`code/main.rs` uses `Option<Box<Node>>`.

### Run

```sh
clang -O2 -fsanitize=address main.c -o bst && ./bst
```

## Use It

- **C++ `std::map`/`std::set`**: red-black tree.
- **Java `TreeMap`/`TreeSet`**: red-black tree.
- **Linux kernel `lib/rbtree.c`**: red-black tree, used in scheduling, memory mgmt.

Use a balanced BST whenever you need *sorted-by-key* iteration AND O(log n) updates. For O(log n) without sorted iteration, use a hash map.

## Read the Source

- [Sedgewick's BST](https://algs4.cs.princeton.edu/32bst/) — clean Java reference.
- [Knuth TAOCP Vol. 3 §6.2.2](https://www-cs-faculty.stanford.edu/~knuth/taocp.html).

## Ship It

This lesson ships **`outputs/bst.h`** — a single-header BST with insert/search/delete and rotation primitives.

## Exercises

1. **Easy.** Implement `bst_inorder(Node*, int *out, int *n)`.
2. **Medium.** Implement `bst_validate(Node*)` — two approaches: pass down (min,max) range, or do inorder and check sortedness.
3. **Hard.** `bst_select(Node*, int k)` — k-th smallest in O(h) via subtree-size augmentation.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| BST invariant | "Sorted tree" | Left subtree < self < right subtree, for every node |
| Rotation | "Tree rebalancing" | Local 3-pointer rewire that preserves BST order |
| Inorder successor | "Next key" | Smallest key larger than self |
| Degenerate BST | "Linked list" | Height = n; happens with sorted input |
| Augmented tree | "Indexed BST" | Each node carries extra info (size, sum, min) |

## Further Reading

- *Introduction to Algorithms* (CLRS) Ch. 12.
- *Algorithms, 4th Edition* by Sedgewick.
