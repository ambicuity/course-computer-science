/* main.c — AVL tree with insert, delete, height-balance verification. */
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
#include <time.h>

typedef struct Node {
    int key;
    int height;             /* height = max child height + 1; leaf = 1 */
    struct Node *left, *right;
} Node;

static int h(Node *n) { return n ? n->height : 0; }
static int bf(Node *n) { return h(n->left) - h(n->right); }
static int max(int a, int b) { return a > b ? a : b; }

static Node *new_node(int k) {
    Node *n = malloc(sizeof(*n));
    n->key = k; n->height = 1; n->left = n->right = NULL;
    return n;
}

static void update_height(Node *n) {
    n->height = 1 + max(h(n->left), h(n->right));
}

static Node *rotate_left(Node *n) {
    Node *r = n->right;
    n->right = r->left;
    r->left = n;
    update_height(n);
    update_height(r);
    return r;
}

static Node *rotate_right(Node *n) {
    Node *l = n->left;
    n->left = l->right;
    l->right = n;
    update_height(n);
    update_height(l);
    return l;
}

static Node *rebalance(Node *n) {
    update_height(n);
    int b = bf(n);
    if (b > 1) {
        if (bf(n->left) < 0) n->left = rotate_left(n->left);     /* LR */
        return rotate_right(n);                                   /* LL */
    }
    if (b < -1) {
        if (bf(n->right) > 0) n->right = rotate_right(n->right);  /* RL */
        return rotate_left(n);                                    /* RR */
    }
    return n;
}

static Node *insert(Node *n, int k) {
    if (!n) return new_node(k);
    if (k < n->key)      n->left  = insert(n->left,  k);
    else if (k > n->key) n->right = insert(n->right, k);
    else return n;                                                /* no dup */
    return rebalance(n);
}

static Node *min_node(Node *n) {
    while (n->left) n = n->left;
    return n;
}

static Node *delete(Node *n, int k) {
    if (!n) return NULL;
    if (k < n->key)      n->left  = delete(n->left,  k);
    else if (k > n->key) n->right = delete(n->right, k);
    else {
        if (!n->left || !n->right) {
            Node *child = n->left ? n->left : n->right;
            free(n);
            return child;
        }
        Node *s = min_node(n->right);
        n->key = s->key;
        n->right = delete(n->right, s->key);
    }
    return rebalance(n);
}

static int contains(Node *n, int k) {
    while (n) {
        if      (k < n->key) n = n->left;
        else if (k > n->key) n = n->right;
        else return 1;
    }
    return 0;
}

static int verify_avl(Node *n) {       /* returns height or -1 on violation */
    if (!n) return 0;
    int l = verify_avl(n->left);
    int r = verify_avl(n->right);
    if (l < 0 || r < 0) return -1;
    if (abs(l - r) > 1) return -1;
    return 1 + max(l, r);
}

static void tree_free(Node *n) { if (!n) return; tree_free(n->left); tree_free(n->right); free(n); }

int main(void) {
    /* 1. Sequential — AVL should keep height tiny */
    printf("== Sequential insert 1..1000 ==\n");
    Node *t = NULL;
    for (int i = 1; i <= 1000; ++i) t = insert(t, i);
    int hh = verify_avl(t);
    printf("  height = %d  (max ≤ 1.44·log2(1002) ≈ 14)  invariant=%s\n",
           hh, hh >= 0 ? "OK" : "VIOLATED");
    tree_free(t);

    /* 2. Random — same balance regardless of order */
    printf("\n== Random insert n=10000 ==\n");
    srand(42);
    t = NULL;
    for (int i = 0; i < 10000; ++i) t = insert(t, rand() % 100000);
    hh = verify_avl(t);
    printf("  height = %d  (max ≤ 1.44·log2(10002) ≈ 19)  invariant=%s\n",
           hh, hh >= 0 ? "OK" : "VIOLATED");

    /* 3. Adversarial delete */
    for (int i = 50000; i < 60000; ++i) t = delete(t, i);
    hh = verify_avl(t);
    printf("  after 10K deletes: height=%d  invariant=%s\n",
           hh, hh >= 0 ? "OK" : "VIOLATED");
    tree_free(t);

    /* 4. Show the 4 rebalance cases via specific insertion sequences */
    printf("\n== Rebalance case demos ==\n");

    int ll_seq[] = {3, 2, 1};
    int rr_seq[] = {1, 2, 3};
    int lr_seq[] = {3, 1, 2};
    int rl_seq[] = {1, 3, 2};

    /* LL: insert 3, 2, 1 → root was 3 with bf=+2, child=2 with bf=+1 → rotate_right */
    Node *ll = NULL;
    for (int i = 0; i < 3; ++i) ll = insert(ll, ll_seq[i]);
    printf("  LL insert 3,2,1: root=%d (expect 2)\n", ll->key);
    tree_free(ll);

    /* RR: insert 1, 2, 3 */
    Node *rr = NULL;
    for (int i = 0; i < 3; ++i) rr = insert(rr, rr_seq[i]);
    printf("  RR insert 1,2,3: root=%d (expect 2)\n", rr->key);
    tree_free(rr);

    /* LR: insert 3, 1, 2 */
    Node *lr = NULL;
    for (int i = 0; i < 3; ++i) lr = insert(lr, lr_seq[i]);
    printf("  LR insert 3,1,2: root=%d (expect 2)\n", lr->key);
    tree_free(lr);

    /* RL: insert 1, 3, 2 */
    Node *rl = NULL;
    for (int i = 0; i < 3; ++i) rl = insert(rl, rl_seq[i]);
    printf("  RL insert 1,3,2: root=%d (expect 2)\n", rl->key);
    tree_free(rl);

    return 0;
}
