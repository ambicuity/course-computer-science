/*
 * tree.h — generic binary-tree header with all four traversals via callbacks.
 *
 * The tree stores int payloads (extend as needed). The "visitor" callback type
 * lets you do arbitrary work per node without hand-coding the recursion each
 * time.
 *
 * Usage:
 *   #include "tree.h"
 *   Node *t = tree_new(...);
 *   tree_inorder(t, my_visitor, my_ctx);
 *   tree_free(t);
 */
#ifndef TREE_H
#define TREE_H

#include <stddef.h>
#include <stdlib.h>

typedef struct TreeNode {
    int                 data;
    struct TreeNode    *left, *right;
} TreeNode;

typedef void (*TreeVisitor)(int data, void *ctx);

static inline TreeNode *tree_new(int v, TreeNode *l, TreeNode *r) {
    TreeNode *n = (TreeNode *)malloc(sizeof(*n));
    n->data = v; n->left = l; n->right = r;
    return n;
}

static inline void tree_preorder(TreeNode *n, TreeVisitor f, void *ctx) {
    if (!n) return; f(n->data, ctx);
    tree_preorder(n->left, f, ctx); tree_preorder(n->right, f, ctx);
}

static inline void tree_inorder(TreeNode *n, TreeVisitor f, void *ctx) {
    if (!n) return;
    tree_inorder(n->left, f, ctx); f(n->data, ctx); tree_inorder(n->right, f, ctx);
}

static inline void tree_postorder(TreeNode *n, TreeVisitor f, void *ctx) {
    if (!n) return;
    tree_postorder(n->left, f, ctx); tree_postorder(n->right, f, ctx); f(n->data, ctx);
}

static inline void tree_bfs(TreeNode *root, TreeVisitor f, void *ctx) {
    /* Caller must size the queue if very deep; simple bounded version: */
    TreeNode *queue[8192]; int head = 0, tail = 0;
    if (root) queue[tail++] = root;
    while (head < tail) {
        TreeNode *n = queue[head++];
        f(n->data, ctx);
        if (n->left)  queue[tail++] = n->left;
        if (n->right) queue[tail++] = n->right;
    }
}

static inline void tree_free(TreeNode *n) {
    if (!n) return;
    tree_free(n->left); tree_free(n->right); free(n);
}

#endif /* TREE_H */
