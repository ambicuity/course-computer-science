/* main.c — Left-Leaning Red-Black tree (Sedgewick 2008).
 *
 * Same asymptotic guarantees as a classic RB tree (h ≤ 2 log n) with a much
 * simpler set of cases. Insert needs just 3 helpers (rotate_left, rotate_right,
 * flip_colors); delete needs 2 more (move_red_left/right) but is still ~30 lines
 * vs full RB's ~300. The Linux kernel's lib/rbtree.c uses the classic CLRS form
 * — read it after you understand LLRB to compare.
 */
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <assert.h>
#include <time.h>

#define RED   true
#define BLACK false

typedef struct Node {
    int key;
    bool color;             /* color of the INCOMING link from parent */
    struct Node *left, *right;
} Node;

static bool is_red(Node *n) { return n && n->color == RED; }

static Node *new_node(int k) {
    Node *n = malloc(sizeof(*n));
    n->key = k; n->color = RED; n->left = n->right = NULL;
    return n;
}

static Node *rotate_left(Node *n) {
    Node *r = n->right;
    n->right = r->left;
    r->left = n;
    r->color = n->color;
    n->color = RED;
    return r;
}
static Node *rotate_right(Node *n) {
    Node *l = n->left;
    n->left = l->right;
    l->right = n;
    l->color = n->color;
    n->color = RED;
    return l;
}
static void flip_colors(Node *n) {
    n->color = !n->color;
    n->left->color = !n->left->color;
    n->right->color = !n->right->color;
}

/* ============================================================ */
/* INSERT                                                       */
/* ============================================================ */

static Node *insert(Node *n, int k) {
    if (!n) return new_node(k);

    if      (k < n->key) n->left  = insert(n->left,  k);
    else if (k > n->key) n->right = insert(n->right, k);
    else return n;

    /* Three fixup actions: */
    if (is_red(n->right) && !is_red(n->left))            n = rotate_left(n);
    if (is_red(n->left)  && is_red(n->left->left))       n = rotate_right(n);
    if (is_red(n->left)  && is_red(n->right))            flip_colors(n);

    return n;
}

/* ============================================================ */
/* DELETE (delete-min and delete-by-key)                        */
/* ============================================================ */

static Node *fix_up(Node *n) {
    if (is_red(n->right) && !is_red(n->left))            n = rotate_left(n);
    if (n->left && is_red(n->left) && is_red(n->left->left)) n = rotate_right(n);
    if (is_red(n->left) && is_red(n->right))             flip_colors(n);
    return n;
}

static Node *move_red_left(Node *n) {
    flip_colors(n);
    if (is_red(n->right->left)) {
        n->right = rotate_right(n->right);
        n = rotate_left(n);
        flip_colors(n);
    }
    return n;
}

static Node *move_red_right(Node *n) {
    flip_colors(n);
    if (is_red(n->left->left)) {
        n = rotate_right(n);
        flip_colors(n);
    }
    return n;
}

static Node *delete_min(Node *n) {
    if (!n->left) { free(n); return NULL; }
    if (!is_red(n->left) && !is_red(n->left->left)) n = move_red_left(n);
    n->left = delete_min(n->left);
    return fix_up(n);
}

static Node *min_node(Node *n) {
    while (n->left) n = n->left;
    return n;
}

static Node *delete_key(Node *n, int k) {
    if (k < n->key) {
        if (!is_red(n->left) && (n->left && !is_red(n->left->left)))
            n = move_red_left(n);
        n->left = delete_key(n->left, k);
    } else {
        if (is_red(n->left)) n = rotate_right(n);
        if (k == n->key && !n->right) { free(n); return NULL; }
        if (n->right && !is_red(n->right) && !is_red(n->right->left))
            n = move_red_right(n);
        if (k == n->key) {
            Node *s = min_node(n->right);
            n->key = s->key;
            n->right = delete_min(n->right);
        } else {
            n->right = delete_key(n->right, k);
        }
    }
    return fix_up(n);
}

/* ============================================================ */
/* Public API                                                   */
/* ============================================================ */

static Node *rb_insert(Node *root, int k) {
    root = insert(root, k);
    root->color = BLACK;
    return root;
}

static Node *rb_delete(Node *root, int k) {
    if (!root) return NULL;
    if (!is_red(root->left) && !is_red(root->right))
        root->color = RED;
    root = delete_key(root, k);
    if (root) root->color = BLACK;
    return root;
}

static bool contains(Node *n, int k) {
    while (n) {
        if      (k < n->key) n = n->left;
        else if (k > n->key) n = n->right;
        else return true;
    }
    return false;
}

/* ============================================================ */
/* Invariants                                                    */
/* ============================================================ */

static int verify(Node *n, int black_count, int *path_count) {
    /* Returns 1 if OK; 0 on violation. Sets *path_count to the BH of each path. */
    if (!n) {
        if (*path_count == -1) *path_count = black_count;
        else if (*path_count != black_count) {
            printf("    !! black-height mismatch: expected %d, got %d\n", *path_count, black_count);
            return 0;
        }
        return 1;
    }
    if (is_red(n) && (is_red(n->left) || is_red(n->right))) {
        printf("    !! consecutive reds at key=%d\n", n->key);
        return 0;
    }
    int bc = black_count + (n->color == BLACK ? 1 : 0);
    return verify(n->left, bc, path_count) && verify(n->right, bc, path_count);
}

static int height(Node *n) {
    if (!n) return 0;
    int l = height(n->left), r = height(n->right);
    return 1 + (l > r ? l : r);
}

static int count(Node *n) {
    if (!n) return 0;
    return 1 + count(n->left) + count(n->right);
}

static void tree_free(Node *n) { if (!n) return; tree_free(n->left); tree_free(n->right); free(n); }

int main(void) {
    /* Sequential */
    printf("== Sequential insert 1..1000 ==\n");
    Node *t = NULL;
    for (int i = 1; i <= 1000; ++i) t = rb_insert(t, i);
    int pc = -1;
    int ok = verify(t, 0, &pc);
    printf("  count=%d  height=%d  black-height=%d  invariants=%s  (h ≤ 2 log2(1001) ≈ 20)\n",
           count(t), height(t), pc, ok ? "OK" : "VIOLATED");
    tree_free(t);

    /* Random insert + delete stress */
    printf("\n== Random 10K insert + 5K delete ==\n");
    srand(42);
    t = NULL;
    int keys[20000]; int nk = 0;
    for (int i = 0; i < 10000; ++i) {
        int k = rand() % 100000;
        if (!contains(t, k)) {
            t = rb_insert(t, k);
            keys[nk++] = k;
        }
    }
    for (int i = 0; i < 5000 && i < nk; ++i) t = rb_delete(t, keys[i]);
    pc = -1;
    ok = verify(t, 0, &pc);
    printf("  count=%d  height=%d  black-height=%d  invariants=%s\n",
           count(t), height(t), pc, ok ? "OK" : "VIOLATED");
    tree_free(t);

    return 0;
}
