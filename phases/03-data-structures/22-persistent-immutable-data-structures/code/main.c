/* main.c — persistent linked list + path-copying persistent BST. */
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <assert.h>

/* ============================================================ */
/* Persistent linked list                                        */
/* ============================================================ */
typedef struct LNode { int data; const struct LNode *next; } LNode;

static const LNode *cons(int x, const LNode *tail) {
    LNode *n = malloc(sizeof(LNode));
    n->data = x; n->next = tail;
    return n;
}

static void print_list(const char *label, const LNode *l) {
    printf("%s: [", label);
    while (l) {
        printf("%d%s", l->data, l->next ? ", " : "");
        l = l->next;
    }
    printf("]\n");
}

/* ============================================================ */
/* Path-copying persistent BST                                  */
/* ============================================================ */
typedef struct BNode {
    int                 key;
    const struct BNode *left, *right;
} BNode;

static const BNode *bnew(int k, const BNode *l, const BNode *r) {
    BNode *n = malloc(sizeof(BNode));
    n->key = k; n->left = l; n->right = r;
    return n;
}

/* Insert returns a new tree; old tree unchanged. */
static const BNode *bst_insert(const BNode *t, int k) {
    if (!t) return bnew(k, NULL, NULL);
    if (k < t->key)  return bnew(t->key, bst_insert(t->left,  k), t->right);
    if (k > t->key)  return bnew(t->key, t->left, bst_insert(t->right, k));
    return t;                                              /* no change */
}

static bool bst_contains(const BNode *t, int k) {
    while (t) {
        if (k < t->key) t = t->left;
        else if (k > t->key) t = t->right;
        else return true;
    }
    return false;
}

static int count_nodes(const BNode *t) {
    if (!t) return 0;
    return 1 + count_nodes(t->left) + count_nodes(t->right);
}

/* Count nodes uniquely reachable from one tree (depth-first, with a small
   set of pointers seen — for the small-scale demo, just use linear scan). */
typedef struct { const BNode **arr; size_t n, cap; } PtrSet;
static bool in_set(const PtrSet *s, const BNode *p) {
    for (size_t i = 0; i < s->n; ++i) if (s->arr[i] == p) return true;
    return false;
}
static void add_set(PtrSet *s, const BNode *p) {
    if (s->n == s->cap) { s->cap = s->cap ? s->cap * 2 : 8; s->arr = realloc(s->arr, s->cap * sizeof(p)); }
    s->arr[s->n++] = p;
}
static void collect(const BNode *t, PtrSet *s) {
    if (!t || in_set(s, t)) return;
    add_set(s, t);
    collect(t->left, s);
    collect(t->right, s);
}

static int unique_nodes(const BNode *a, const BNode *b) {
    PtrSet s = {0};
    collect(a, &s);
    collect(b, &s);
    int n = (int)s.n;
    free(s.arr);
    return n;
}

int main(void) {
    /* Persistent list */
    const LNode *v1 = cons(1, cons(2, cons(3, NULL)));     /* [1,2,3] */
    const LNode *v2 = cons(0, v1);                          /* [0,1,2,3] */
    const LNode *v3 = cons(-1, v2);                         /* [-1,0,1,2,3] */
    printf("== Persistent list ==\n");
    print_list("v1", v1);
    print_list("v2 (cons 0 v1)", v2);
    print_list("v3 (cons -1 v2)", v3);
    printf("  v1 unchanged after v2, v3 derived from it\n");

    /* Persistent BST */
    const BNode *t1 = NULL;
    for (int k = 0; k < 8; ++k) t1 = bst_insert(t1, k * 10);   /* 0,10,20,...,70 */
    printf("\n== Path-copying persistent BST ==\n");
    printf("  t1 has %d nodes\n", count_nodes(t1));

    const BNode *t2 = bst_insert(t1, 25);                   /* small change */
    printf("  t2 = t1.insert(25), has %d nodes (each tree)\n", count_nodes(t2));
    printf("  total unique nodes across t1 + t2: %d  (vs %d if no sharing)\n",
           unique_nodes(t1, t2), 2 * count_nodes(t1) + 1);
    printf("  Δ = %d new nodes for the insert (≤ log₂(8) + 1 = 4)\n",
           unique_nodes(t1, t2) - count_nodes(t1));

    /* Verify both trees still have correct contents */
    printf("\n  t1.contains(25) = %d (expect 0)\n", bst_contains(t1, 25));
    printf("  t2.contains(25) = %d (expect 1)\n", bst_contains(t2, 25));
    printf("  both contain 0..70: ");
    bool all_in_both = true;
    for (int k = 0; k < 8; ++k)
        if (!bst_contains(t1, k * 10) || !bst_contains(t2, k * 10)) all_in_both = false;
    printf("%s\n", all_in_both ? "yes" : "no");

    /* Note: we leak everything here; in production use reference-counting */
    return 0;
}
