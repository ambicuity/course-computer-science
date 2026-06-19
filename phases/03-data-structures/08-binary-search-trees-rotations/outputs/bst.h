/*
 * bst.h — single-header binary search tree with rotation primitives.
 *
 *   BstNode *t = NULL;
 *   t = bst_insert(t, 42);
 *   bst_contains(t, 42);
 *   t = bst_delete(t, 42);
 *   bst_free(t);
 */
#ifndef BST_H
#define BST_H

#include <stdlib.h>

typedef struct BstNode {
    int key;
    struct BstNode *left, *right;
} BstNode;

static inline BstNode *bst_new(int k) {
    BstNode *n = (BstNode *)calloc(1, sizeof(*n));
    n->key = k; return n;
}

static inline BstNode *bst_insert(BstNode *n, int k) {
    if (!n) return bst_new(k);
    if      (k < n->key) n->left  = bst_insert(n->left,  k);
    else if (k > n->key) n->right = bst_insert(n->right, k);
    return n;
}

static inline int bst_contains(const BstNode *n, int k) {
    while (n) {
        if      (k < n->key) n = n->left;
        else if (k > n->key) n = n->right;
        else return 1;
    }
    return 0;
}

static inline BstNode *bst__min(BstNode *n) { while (n && n->left) n = n->left; return n; }

static inline BstNode *bst_delete(BstNode *n, int k) {
    if (!n) return NULL;
    if      (k < n->key) n->left  = bst_delete(n->left,  k);
    else if (k > n->key) n->right = bst_delete(n->right, k);
    else {
        if (!n->left)  { BstNode *r = n->right; free(n); return r; }
        if (!n->right) { BstNode *l = n->left;  free(n); return l; }
        BstNode *s = bst__min(n->right);
        n->key = s->key;
        n->right = bst_delete(n->right, s->key);
    }
    return n;
}

static inline BstNode *bst_rotate_left(BstNode *n) {
    BstNode *r = n->right;
    n->right = r->left;
    r->left = n;
    return r;
}

static inline BstNode *bst_rotate_right(BstNode *n) {
    BstNode *l = n->left;
    n->left = l->right;
    l->right = n;
    return l;
}

static inline void bst_free(BstNode *n) {
    if (!n) return;
    bst_free(n->left); bst_free(n->right); free(n);
}

#endif /* BST_H */
