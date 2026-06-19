/* main.c — Splay tree + Treap, side-by-side. */
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>

/* =========================================================
 * Splay Tree (top-down splay via recursive rotations)
 * ========================================================= */

typedef struct SNode {
    int key;
    struct SNode *left, *right;
} SNode;

static SNode *snew(int k) {
    SNode *n = calloc(1, sizeof(SNode));
    n->key = k; return n;
}

static SNode *srotate_left (SNode *n) { SNode *r=n->right; n->right=r->left; r->left =n; return r; }
static SNode *srotate_right(SNode *n) { SNode *l=n->left;  n->left =l->right; l->right=n; return l; }

/* Bottom-up recursive splay: splay key k (or its inorder neighbor) to root. */
static SNode *splay(SNode *root, int k) {
    if (!root || root->key == k) return root;
    if (k < root->key) {
        if (!root->left) return root;
        if (k < root->left->key) {                  /* zig-zig */
            root->left->left = splay(root->left->left, k);
            root = srotate_right(root);
        } else if (k > root->left->key) {           /* zig-zag */
            root->left->right = splay(root->left->right, k);
            if (root->left->right) root->left = srotate_left(root->left);
        }
        return root->left ? srotate_right(root) : root;
    } else {
        if (!root->right) return root;
        if (k > root->right->key) {                 /* zig-zig */
            root->right->right = splay(root->right->right, k);
            root = srotate_left(root);
        } else if (k < root->right->key) {          /* zig-zag */
            root->right->left = splay(root->right->left, k);
            if (root->right->left) root->right = srotate_right(root->right);
        }
        return root->right ? srotate_left(root) : root;
    }
}

static SNode *splay_insert(SNode *root, int k) {
    if (!root) return snew(k);
    root = splay(root, k);
    if (root->key == k) return root;
    SNode *n = snew(k);
    if (k < root->key) { n->left = root->left; n->right = root; root->left = NULL; }
    else               { n->right = root->right; n->left = root; root->right = NULL; }
    return n;
}

static int splay_contains(SNode **root_ptr, int k) {
    if (!*root_ptr) return 0;
    *root_ptr = splay(*root_ptr, k);
    return (*root_ptr)->key == k;
}

static int snode_height(const SNode *n) {
    if (!n) return 0;
    int l = snode_height(n->left), r = snode_height(n->right);
    return 1 + (l > r ? l : r);
}

static int verify_bst(const SNode *n, int lo, int hi) {
    if (!n) return 1;
    if (n->key <= lo || n->key >= hi) return 0;
    return verify_bst(n->left, lo, n->key) && verify_bst(n->right, n->key, hi);
}

static void sfree(SNode *n) { if (!n) return; sfree(n->left); sfree(n->right); free(n); }

/* =========================================================
 * Treap
 * ========================================================= */

typedef struct TNode {
    int key, prio;
    struct TNode *left, *right;
} TNode;

static unsigned long rand_seed = 0x9e3779b9;
static int rand_prio(void) {
    rand_seed = rand_seed * 6364136223846793005ULL + 1442695040888963407ULL;
    return (int)((rand_seed >> 32) & 0x7fffffff);
}

static TNode *tnew(int k) {
    TNode *n = calloc(1, sizeof(TNode));
    n->key = k; n->prio = rand_prio(); return n;
}

static TNode *trotate_left (TNode *n) { TNode *r=n->right; n->right=r->left; r->left =n; return r; }
static TNode *trotate_right(TNode *n) { TNode *l=n->left;  n->left =l->right; l->right=n; return l; }

static TNode *treap_insert(TNode *n, int k) {
    if (!n) return tnew(k);
    if (k < n->key) {
        n->left = treap_insert(n->left, k);
        if (n->left->prio > n->prio) n = trotate_right(n);
    } else if (k > n->key) {
        n->right = treap_insert(n->right, k);
        if (n->right->prio > n->prio) n = trotate_left(n);
    }
    return n;
}

static int treap_contains(const TNode *n, int k) {
    while (n) {
        if (k < n->key) n = n->left;
        else if (k > n->key) n = n->right;
        else return 1;
    }
    return 0;
}

static int tnode_height(const TNode *n) {
    if (!n) return 0;
    int l = tnode_height(n->left), r = tnode_height(n->right);
    return 1 + (l > r ? l : r);
}

static int verify_treap(const TNode *n, int lo, int hi, int parent_prio) {
    if (!n) return 1;
    if (n->key <= lo || n->key >= hi) return 0;
    if (n->prio > parent_prio) return 0;            /* max-heap on prio */
    return verify_treap(n->left, lo, n->key, n->prio)
        && verify_treap(n->right, n->key, hi, n->prio);
}

static void tfree(TNode *n) { if (!n) return; tfree(n->left); tfree(n->right); free(n); }

int main(void) {
    /* Splay tree: shape depends on access pattern, not strict invariants. */
    SNode *s = NULL;
    for (int i = 1; i <= 1000; ++i) s = splay_insert(s, i);
    printf("== Splay tree ==\n");
    printf("  height after sequential insert 1..1000: %d  BST=%s\n",
           snode_height(s),
           verify_bst(s, -2000000000, 2000000000) ? "OK" : "FAIL");

    /* Splay once for key 1: should bubble it to root and halve the depth. */
    int ok = splay_contains(&s, 1);
    printf("  found key=1: %d   root after splay(1) = %d   height = %d\n",
           ok, s->key, snode_height(s));

    /* Now find key 500 — should be O(log n) amortized. */
    splay_contains(&s, 500);
    printf("  root after splay(500) = %d   height = %d\n",
           s->key, snode_height(s));

    /* Repeated access to a small working set keeps height small there. */
    for (int t = 0; t < 50; ++t)
        for (int k = 1; k <= 10; ++k) splay_contains(&s, k);
    printf("  after working-set splaying (1..10 × 50): root = %d   height = %d\n",
           s->key, snode_height(s));
    sfree(s);

    /* Treap: sorted insert; height should be ~log n */
    rand_seed = 0xdeadbeef;
    TNode *t = NULL;
    for (int i = 1; i <= 10000; ++i) t = treap_insert(t, i);
    printf("\n== Treap ==\n");
    printf("  sorted insert 1..10000:  height = %d  (expected ~%d=2log2(10000))\n",
           tnode_height(t), 28);
    int treap_ok = verify_treap(t, -2000000000, 2000000000, 0x7fffffff);
    printf("  invariants (BST + heap): %s\n", treap_ok ? "OK" : "FAIL");
    printf("  contains(7777)=%d  contains(99999)=%d\n",
           treap_contains(t, 7777), treap_contains(t, 99999));
    tfree(t);

    return 0;
}
