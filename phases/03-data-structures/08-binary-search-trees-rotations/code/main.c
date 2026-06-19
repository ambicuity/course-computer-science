/* main.c — Binary Search Tree with rotations and balance demos. */
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
#include <string.h>

typedef struct Node {
    int key;
    struct Node *left, *right;
} Node;

static Node *new_node(int k) {
    Node *n = calloc(1, sizeof(Node));
    n->key = k; return n;
}

/* ===================== Insert / Search / Delete ===================== */
static Node *insert(Node *n, int k) {
    if (!n) return new_node(k);
    if      (k < n->key) n->left  = insert(n->left,  k);
    else if (k > n->key) n->right = insert(n->right, k);
    return n;
}

static int contains(const Node *n, int k) {
    while (n) {
        if      (k < n->key) n = n->left;
        else if (k > n->key) n = n->right;
        else return 1;
    }
    return 0;
}

static Node *min_node(Node *n) {
    while (n && n->left) n = n->left;
    return n;
}

static Node *delete(Node *n, int k) {
    if (!n) return NULL;
    if      (k < n->key) n->left  = delete(n->left,  k);
    else if (k > n->key) n->right = delete(n->right, k);
    else {
        /* found: handle 3 cases */
        if (!n->left)  { Node *r = n->right; free(n); return r; }
        if (!n->right) { Node *l = n->left;  free(n); return l; }
        Node *s = min_node(n->right);
        n->key = s->key;
        n->right = delete(n->right, s->key);
    }
    return n;
}

/* ===================== Rotations ===================== */
static Node *rotate_left(Node *n) {
    Node *r = n->right;
    n->right = r->left;
    r->left = n;
    return r;
}
static Node *rotate_right(Node *n) {
    Node *l = n->left;
    n->left = l->right;
    l->right = n;
    return l;
}

/* ===================== Diagnostics ===================== */
static int height(const Node *n) {
    if (!n) return 0;
    int l = height(n->left), r = height(n->right);
    return 1 + (l > r ? l : r);
}

static void inorder(const Node *n, int *out, int *count) {
    if (!n) return;
    inorder(n->left, out, count);
    out[(*count)++] = n->key;
    inorder(n->right, out, count);
}

static void tree_free(Node *n) { if (!n) return; tree_free(n->left); tree_free(n->right); free(n); }

int main(void) {
    /* 1. Demonstrate degenerate insertion */
    printf("== Sequential insert 1..1000 (sorted → linked-list shape) ==\n");
    Node *bad = NULL;
    for (int i = 1; i <= 1000; ++i) bad = insert(bad, i);
    printf("  height after sorted insert: %d  (n=1000, expected n)\n", height(bad));
    tree_free(bad);

    /* 2. Demonstrate balanced-on-average random insertion */
    printf("\n== Random insert n=1000 (uniform random keys) ==\n");
    srand(42);
    Node *good = NULL;
    for (int i = 0; i < 1000; ++i) good = insert(good, rand() % 100000);
    printf("  height after random insert: %d  (expected ~2 log2(1000) ≈ 20)\n", height(good));
    tree_free(good);

    /* 3. Rotation demo */
    printf("\n== Rotation demo ==\n");
    /*
            10
           /  \
          5    20
              /  \
             15   25
    */
    Node *t = insert(NULL, 10);
    t = insert(t, 5);
    t = insert(t, 20);
    t = insert(t, 15);
    t = insert(t, 25);
    int buf[16], cnt = 0;
    inorder(t, buf, &cnt);
    printf("  inorder before rotate: ");
    for (int i = 0; i < cnt; ++i) printf("%d ", buf[i]);
    printf("\n  rotating LEFT at root...\n");
    t = rotate_left(t);
    cnt = 0; inorder(t, buf, &cnt);
    printf("  inorder after rotate : ");
    for (int i = 0; i < cnt; ++i) printf("%d ", buf[i]);
    printf("\n  root is now: %d\n", t->key);
    printf("  Rotating RIGHT to restore...\n");
    t = rotate_right(t);
    printf("  root after restore: %d\n", t->key);

    /* 4. Delete demo */
    printf("\n== Delete demo ==\n");
    cnt = 0; inorder(t, buf, &cnt);
    printf("  before delete(20): ");
    for (int i = 0; i < cnt; ++i) printf("%d ", buf[i]);
    t = delete(t, 20);
    cnt = 0; inorder(t, buf, &cnt);
    printf("\n  after delete(20) : ");
    for (int i = 0; i < cnt; ++i) printf("%d ", buf[i]);
    printf("\n  contains(20)=%d  contains(15)=%d\n", contains(t, 20), contains(t, 15));

    tree_free(t);
    return 0;
}
