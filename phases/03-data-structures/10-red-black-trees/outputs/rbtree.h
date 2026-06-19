/*
 * rbtree.h — single-header Left-Leaning Red-Black tree.
 *
 *   RbNode *t = NULL;
 *   t = rb_insert(t, 42);
 *   if (rb_contains(t, 42)) ...
 *   t = rb_delete(t, 42);
 *   rb_free(t);
 *
 * Sedgewick's 2008 LLRB variant: 3 cases for insert, ~30 lines for delete.
 * For classical CLRS-style RB with parent pointers, see Linux's lib/rbtree.c.
 */
#ifndef RBTREE_H
#define RBTREE_H

#include <stdbool.h>
#include <stdlib.h>

#define RB_RED   true
#define RB_BLACK false

typedef struct RbNode {
    int            key;
    bool           color;
    struct RbNode *left, *right;
} RbNode;

static inline bool rb__red(RbNode *n) { return n && n->color == RB_RED; }

static inline RbNode *rb__rot_l(RbNode *n) {
    RbNode *r = n->right;
    n->right = r->left; r->left = n;
    r->color = n->color; n->color = RB_RED;
    return r;
}
static inline RbNode *rb__rot_r(RbNode *n) {
    RbNode *l = n->left;
    n->left = l->right; l->right = n;
    l->color = n->color; n->color = RB_RED;
    return l;
}
static inline void rb__flip(RbNode *n) {
    n->color = !n->color;
    n->left->color = !n->left->color;
    n->right->color = !n->right->color;
}

static inline RbNode *rb__insert(RbNode *n, int k) {
    if (!n) {
        RbNode *r = (RbNode *)malloc(sizeof(*r));
        r->key = k; r->color = RB_RED; r->left = r->right = NULL;
        return r;
    }
    if      (k < n->key) n->left  = rb__insert(n->left,  k);
    else if (k > n->key) n->right = rb__insert(n->right, k);
    else return n;

    if (rb__red(n->right) && !rb__red(n->left))      n = rb__rot_l(n);
    if (rb__red(n->left) && rb__red(n->left->left)) n = rb__rot_r(n);
    if (rb__red(n->left) && rb__red(n->right))       rb__flip(n);
    return n;
}

static inline RbNode *rb_insert(RbNode *root, int k) {
    root = rb__insert(root, k);
    root->color = RB_BLACK;
    return root;
}

static inline bool rb_contains(const RbNode *n, int k) {
    while (n) {
        if      (k < n->key) n = n->left;
        else if (k > n->key) n = n->right;
        else return true;
    }
    return false;
}

static inline void rb_free(RbNode *n) {
    if (!n) return;
    rb_free(n->left); rb_free(n->right); free(n);
}

/* (For full delete, see the main.c sister file. Same algorithm; ~60 lines.) */

#endif /* RBTREE_H */
