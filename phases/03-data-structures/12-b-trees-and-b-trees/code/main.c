/* main.c — B-tree of order m=4 with insert and search. Uses CLRS algorithm.
 *
 * Invariant: every node has between ⌈m/2⌉−1=1 and m−1=3 keys
 * (root may have fewer). All leaves at same depth.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

#define M 4                           /* order */
#define MAX_KEYS (M - 1)              /* 3 */
#define MIN_KEYS ((M + 1) / 2 - 1)    /* 1 */

typedef struct Node {
    int          n;                   /* # keys present */
    int          keys[MAX_KEYS + 1];  /* +1 for transient overflow */
    struct Node *children[MAX_KEYS + 2];
    int          leaf;
} Node;

static Node *new_node(int leaf) {
    Node *n = calloc(1, sizeof(*n));
    n->leaf = leaf; return n;
}

/* Split child y of x at index i. y is full (has MAX_KEYS keys). */
static void split_child(Node *x, int i) {
    Node *y = x->children[i];
    Node *z = new_node(y->leaf);
    int mid = MAX_KEYS / 2;            /* mid index = 1 for M=4: keys[0,1,2] → mid=1 */
    int median = y->keys[mid];

    /* z gets the keys after the median. */
    z->n = MAX_KEYS - mid - 1;          /* 3 - 1 - 1 = 1 */
    for (int j = 0; j < z->n; ++j) z->keys[j] = y->keys[mid + 1 + j];
    if (!y->leaf) {
        for (int j = 0; j <= z->n; ++j) z->children[j] = y->children[mid + 1 + j];
    }
    y->n = mid;                          /* y keeps keys before median */

    /* Shift x's children to make room for z. */
    for (int j = x->n; j >= i + 1; --j) x->children[j + 1] = x->children[j];
    x->children[i + 1] = z;
    for (int j = x->n - 1; j >= i; --j) x->keys[j + 1] = x->keys[j];
    x->keys[i] = median;
    x->n++;
}

static void insert_nonfull(Node *x, int k) {
    int i = x->n - 1;
    if (x->leaf) {
        while (i >= 0 && k < x->keys[i]) { x->keys[i + 1] = x->keys[i]; --i; }
        x->keys[i + 1] = k;
        x->n++;
    } else {
        while (i >= 0 && k < x->keys[i]) --i;
        ++i;
        if (x->children[i]->n == MAX_KEYS) {
            split_child(x, i);
            if (k > x->keys[i]) ++i;
        }
        insert_nonfull(x->children[i], k);
    }
}

static Node *bt_insert(Node *root, int k) {
    if (!root) { Node *r = new_node(1); r->keys[0] = k; r->n = 1; return r; }
    if (root->n == MAX_KEYS) {
        Node *s = new_node(0);
        s->children[0] = root;
        split_child(s, 0);
        insert_nonfull(s, k);
        return s;
    }
    insert_nonfull(root, k);
    return root;
}

static int bt_contains(const Node *n, int k) {
    while (n) {
        int i = 0;
        while (i < n->n && k > n->keys[i]) ++i;
        if (i < n->n && k == n->keys[i]) return 1;
        if (n->leaf) return 0;
        n = n->children[i];
    }
    return 0;
}

static void print_tree(const Node *n, int depth) {
    if (!n) return;
    for (int i = 0; i < depth; ++i) printf("  ");
    printf("[");
    for (int i = 0; i < n->n; ++i) printf("%d%s", n->keys[i], i + 1 < n->n ? "," : "");
    printf("]%s\n", n->leaf ? " (leaf)" : "");
    if (!n->leaf)
        for (int i = 0; i <= n->n; ++i) print_tree(n->children[i], depth + 1);
}

/* All leaves at same depth check */
static int leaf_depth(const Node *n, int d, int *first_leaf_depth) {
    if (!n) return 1;
    if (n->leaf) {
        if (*first_leaf_depth == -1) *first_leaf_depth = d;
        else if (*first_leaf_depth != d) return 0;
        return 1;
    }
    for (int i = 0; i <= n->n; ++i)
        if (!leaf_depth(n->children[i], d + 1, first_leaf_depth)) return 0;
    return 1;
}

static void tree_free(Node *n) {
    if (!n) return;
    if (!n->leaf) for (int i = 0; i <= n->n; ++i) tree_free(n->children[i]);
    free(n);
}

int main(void) {
    /* Insert a sequence designed to cause multiple splits. */
    int seq[] = {10, 20, 30, 5, 15, 25, 35, 1, 7, 12, 17, 22, 27, 32, 37, 40, 45, 50};
    int n = sizeof(seq) / sizeof(seq[0]);

    Node *t = NULL;
    for (int i = 0; i < n; ++i) t = bt_insert(t, seq[i]);

    printf("== B-tree (order M=%d) after inserting %d keys ==\n\n", M, n);
    print_tree(t, 0);

    int first = -1;
    int balanced = leaf_depth(t, 0, &first);
    printf("\nall leaves at same depth (%d): %s\n", first, balanced ? "YES" : "NO");

    /* Contains check */
    printf("contains(17) = %d (expect 1)\n", bt_contains(t, 17));
    printf("contains(99) = %d (expect 0)\n", bt_contains(t, 99));

    tree_free(t);
    return 0;
}
