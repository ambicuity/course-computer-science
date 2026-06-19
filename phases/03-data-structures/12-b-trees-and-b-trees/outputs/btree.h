/*
 * btree.h — single-header B-tree (order configurable via BTREE_M).
 * Supports insert and search. Delete is sketched in the documentation;
 * a full delete is ~150 lines (follow CLRS Ch. 18).
 *
 *   BTreeNode *t = NULL;
 *   t = btree_insert(t, 42);
 *   if (btree_contains(t, 42)) ...
 *   btree_free(t);
 */
#ifndef BTREE_H
#define BTREE_H

#ifndef BTREE_M
#  define BTREE_M 4
#endif

#define BTREE_MAX_KEYS (BTREE_M - 1)
#define BTREE_MIN_KEYS ((BTREE_M + 1) / 2 - 1)

#include <stdlib.h>
#include <stdbool.h>

typedef struct BTreeNode {
    int               n;
    int               keys[BTREE_MAX_KEYS + 1];
    struct BTreeNode *children[BTREE_MAX_KEYS + 2];
    int               leaf;
} BTreeNode;

static inline BTreeNode *btree__new(int leaf) {
    BTreeNode *n = (BTreeNode *)calloc(1, sizeof(*n));
    n->leaf = leaf; return n;
}

static inline void btree__split(BTreeNode *x, int i) {
    BTreeNode *y = x->children[i];
    BTreeNode *z = btree__new(y->leaf);
    int mid = BTREE_MAX_KEYS / 2;
    int median = y->keys[mid];
    z->n = BTREE_MAX_KEYS - mid - 1;
    for (int j = 0; j < z->n; ++j) z->keys[j] = y->keys[mid + 1 + j];
    if (!y->leaf)
        for (int j = 0; j <= z->n; ++j) z->children[j] = y->children[mid + 1 + j];
    y->n = mid;
    for (int j = x->n; j >= i + 1; --j) x->children[j + 1] = x->children[j];
    x->children[i + 1] = z;
    for (int j = x->n - 1; j >= i; --j) x->keys[j + 1] = x->keys[j];
    x->keys[i] = median;
    x->n++;
}

static inline void btree__insert_nonfull(BTreeNode *x, int k) {
    int i = x->n - 1;
    if (x->leaf) {
        while (i >= 0 && k < x->keys[i]) { x->keys[i + 1] = x->keys[i]; --i; }
        x->keys[i + 1] = k;
        x->n++;
    } else {
        while (i >= 0 && k < x->keys[i]) --i;
        ++i;
        if (x->children[i]->n == BTREE_MAX_KEYS) {
            btree__split(x, i);
            if (k > x->keys[i]) ++i;
        }
        btree__insert_nonfull(x->children[i], k);
    }
}

static inline BTreeNode *btree_insert(BTreeNode *root, int k) {
    if (!root) { BTreeNode *r = btree__new(1); r->keys[0] = k; r->n = 1; return r; }
    if (root->n == BTREE_MAX_KEYS) {
        BTreeNode *s = btree__new(0);
        s->children[0] = root;
        btree__split(s, 0);
        btree__insert_nonfull(s, k);
        return s;
    }
    btree__insert_nonfull(root, k);
    return root;
}

static inline bool btree_contains(const BTreeNode *n, int k) {
    while (n) {
        int i = 0;
        while (i < n->n && k > n->keys[i]) ++i;
        if (i < n->n && k == n->keys[i]) return true;
        if (n->leaf) return false;
        n = n->children[i];
    }
    return false;
}

static inline void btree_free(BTreeNode *n) {
    if (!n) return;
    if (!n->leaf) for (int i = 0; i <= n->n; ++i) btree_free(n->children[i]);
    free(n);
}

#endif /* BTREE_H */
