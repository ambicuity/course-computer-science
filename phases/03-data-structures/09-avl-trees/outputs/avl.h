/*
 * avl.h — single-header AVL tree (int keys).
 *
 *   AvlNode *t = NULL;
 *   t = avl_insert(t, 42);
 *   if (avl_contains(t, 42)) ...
 *   t = avl_delete(t, 42);
 *   avl_free(t);
 */
#ifndef AVL_H
#define AVL_H

#include <stdlib.h>

typedef struct AvlNode {
    int                 key;
    int                 height;
    struct AvlNode     *left, *right;
} AvlNode;

static inline int avl__h(AvlNode *n) { return n ? n->height : 0; }
static inline int avl__bf(AvlNode *n) { return avl__h(n->left) - avl__h(n->right); }
static inline int avl__max(int a, int b) { return a > b ? a : b; }
static inline void avl__update(AvlNode *n) { n->height = 1 + avl__max(avl__h(n->left), avl__h(n->right)); }

static inline AvlNode *avl__rl(AvlNode *n) {
    AvlNode *r = n->right; n->right = r->left; r->left = n;
    avl__update(n); avl__update(r); return r;
}
static inline AvlNode *avl__rr(AvlNode *n) {
    AvlNode *l = n->left; n->left = l->right; l->right = n;
    avl__update(n); avl__update(l); return l;
}

static inline AvlNode *avl__rebalance(AvlNode *n) {
    avl__update(n);
    int b = avl__bf(n);
    if (b > 1)  { if (avl__bf(n->left) < 0)  n->left  = avl__rl(n->left);  return avl__rr(n); }
    if (b < -1) { if (avl__bf(n->right) > 0) n->right = avl__rr(n->right); return avl__rl(n); }
    return n;
}

static inline AvlNode *avl_new(int k) {
    AvlNode *n = (AvlNode *)malloc(sizeof(*n));
    n->key = k; n->height = 1; n->left = n->right = NULL;
    return n;
}

static inline AvlNode *avl_insert(AvlNode *n, int k) {
    if (!n) return avl_new(k);
    if      (k < n->key) n->left  = avl_insert(n->left,  k);
    else if (k > n->key) n->right = avl_insert(n->right, k);
    else return n;
    return avl__rebalance(n);
}

static inline AvlNode *avl__min(AvlNode *n) { while (n->left) n = n->left; return n; }

static inline AvlNode *avl_delete(AvlNode *n, int k) {
    if (!n) return NULL;
    if      (k < n->key) n->left  = avl_delete(n->left,  k);
    else if (k > n->key) n->right = avl_delete(n->right, k);
    else {
        if (!n->left || !n->right) {
            AvlNode *c = n->left ? n->left : n->right;
            free(n); return c;
        }
        AvlNode *s = avl__min(n->right);
        n->key = s->key;
        n->right = avl_delete(n->right, s->key);
    }
    return avl__rebalance(n);
}

static inline int avl_contains(const AvlNode *n, int k) {
    while (n) {
        if      (k < n->key) n = n->left;
        else if (k > n->key) n = n->right;
        else return 1;
    }
    return 0;
}

static inline void avl_free(AvlNode *n) {
    if (!n) return;
    avl_free(n->left); avl_free(n->right); free(n);
}

#endif /* AVL_H */
