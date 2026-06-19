/*
 * treap.h — single-header treap (randomized BST).
 *
 *   Treap *t = NULL;
 *   t = treap_insert(t, 42);
 *   if (treap_contains(t, 42)) ...
 *   treap_free(t);
 *
 * Each node carries a uniformly-random priority; the tree is a BST on key
 * AND a max-heap on priority. Expected height O(log n) — adversarial keys
 * can't induce a degenerate tree because the structure depends only on
 * priorities (which the adversary cannot influence).
 */
#ifndef TREAP_H
#define TREAP_H

#include <stdbool.h>
#include <stdlib.h>

typedef struct Treap {
    int           key;
    int           prio;
    struct Treap *left, *right;
} Treap;

static unsigned long treap__seed = 0x9e3779b97f4a7c15ULL;
static inline int treap__rand_prio(void) {
    treap__seed = treap__seed * 6364136223846793005ULL + 1442695040888963407ULL;
    return (int)((treap__seed >> 32) & 0x7fffffff);
}

static inline Treap *treap__new(int k) {
    Treap *n = (Treap *)calloc(1, sizeof(*n));
    n->key = k; n->prio = treap__rand_prio();
    return n;
}

static inline Treap *treap__rot_l(Treap *n) {
    Treap *r = n->right; n->right = r->left; r->left = n; return r;
}
static inline Treap *treap__rot_r(Treap *n) {
    Treap *l = n->left;  n->left  = l->right; l->right = n; return l;
}

static inline Treap *treap_insert(Treap *n, int k) {
    if (!n) return treap__new(k);
    if (k < n->key) {
        n->left = treap_insert(n->left, k);
        if (n->left->prio > n->prio) n = treap__rot_r(n);
    } else if (k > n->key) {
        n->right = treap_insert(n->right, k);
        if (n->right->prio > n->prio) n = treap__rot_l(n);
    }
    return n;
}

static inline bool treap_contains(const Treap *n, int k) {
    while (n) {
        if      (k < n->key) n = n->left;
        else if (k > n->key) n = n->right;
        else return true;
    }
    return false;
}

static inline void treap_free(Treap *n) {
    if (!n) return;
    treap_free(n->left); treap_free(n->right); free(n);
}

#endif /* TREAP_H */
