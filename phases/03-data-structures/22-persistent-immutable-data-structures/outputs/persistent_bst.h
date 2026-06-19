/*
 * persistent_bst.h — single-header path-copying persistent BST.
 *
 *   const BstNode *v1 = NULL;
 *   const BstNode *v2 = pbst_insert(v1, 42);
 *   if (pbst_contains(v1, 42)) ...           // false
 *   if (pbst_contains(v2, 42)) ...           // true
 *
 * Memory: this header doesn't free nodes (each insert allocates O(log n) new ones).
 * For production: track via reference counts or arena allocation.
 */
#ifndef PERSISTENT_BST_H
#define PERSISTENT_BST_H

#include <stdlib.h>
#include <stdbool.h>

typedef struct BstNode {
    int                  key;
    const struct BstNode *left, *right;
} BstNode;

static inline const BstNode *pbst__new(int k, const BstNode *l, const BstNode *r) {
    BstNode *n = (BstNode *)malloc(sizeof(*n));
    n->key = k; n->left = l; n->right = r;
    return n;
}

static inline const BstNode *pbst_insert(const BstNode *t, int k) {
    if (!t) return pbst__new(k, NULL, NULL);
    if (k < t->key) return pbst__new(t->key, pbst_insert(t->left,  k), t->right);
    if (k > t->key) return pbst__new(t->key, t->left, pbst_insert(t->right, k));
    return t;
}

static inline bool pbst_contains(const BstNode *t, int k) {
    while (t) {
        if      (k < t->key) t = t->left;
        else if (k > t->key) t = t->right;
        else return true;
    }
    return false;
}

#endif /* PERSISTENT_BST_H */
