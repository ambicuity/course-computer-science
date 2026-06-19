/* main.c — Binary tree traversals + recursion patterns. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

typedef struct Node {
    int data;
    struct Node *left, *right;
} Node;

static Node *make(int v, Node *l, Node *r) {
    Node *n = malloc(sizeof(*n));
    n->data = v; n->left = l; n->right = r;
    return n;
}

/* ===================== Recursive ===================== */
static void preorder (Node *n) { if (!n) return; printf("%d ", n->data); preorder(n->left); preorder(n->right); }
static void inorder  (Node *n) { if (!n) return; inorder(n->left); printf("%d ", n->data); inorder(n->right); }
static void postorder(Node *n) { if (!n) return; postorder(n->left); postorder(n->right); printf("%d ", n->data); }

/* ===================== Iterative ===================== */
static void inorder_iter(Node *root) {
    Node *stack[64]; int top = 0;
    Node *cur = root;
    while (cur || top > 0) {
        while (cur) { stack[top++] = cur; cur = cur->left; }
        cur = stack[--top];
        printf("%d ", cur->data);
        cur = cur->right;
    }
}

static void bfs(Node *root) {
    Node *queue[256]; int head = 0, tail = 0;
    if (root) queue[tail++] = root;
    while (head < tail) {
        Node *n = queue[head++];
        printf("%d ", n->data);
        if (n->left)  queue[tail++] = n->left;
        if (n->right) queue[tail++] = n->right;
    }
}

/* ===================== Morris inorder (O(1) space) ===================== */
static void morris_inorder(Node *root) {
    Node *cur = root;
    while (cur) {
        if (!cur->left) { printf("%d ", cur->data); cur = cur->right; }
        else {
            Node *pred = cur->left;
            while (pred->right && pred->right != cur) pred = pred->right;
            if (!pred->right) { pred->right = cur; cur = cur->left; }
            else              { pred->right = NULL; printf("%d ", cur->data); cur = cur->right; }
        }
    }
}

/* ===================== Tuple recursion: diameter + height + balanced ===================== */
typedef struct { int height; int diameter; int balanced; } Stats;

static Stats stats(Node *n) {
    if (!n) return (Stats){0, 0, 1};
    Stats L = stats(n->left), R = stats(n->right);
    int h = 1 + (L.height > R.height ? L.height : R.height);
    int through = L.height + R.height;
    int d = L.diameter > R.diameter ? L.diameter : R.diameter;
    if (through > d) d = through;
    int bal = L.balanced && R.balanced && abs(L.height - R.height) <= 1;
    return (Stats){h, d, bal};
}

/* ===================== Cleanup ===================== */
static void tree_free(Node *n) { if (!n) return; tree_free(n->left); tree_free(n->right); free(n); }

int main(void) {
    /*       1
            / \
           2   3
          / \   \
         4   5   6
              \
               7
    */
    Node *t = make(1,
                   make(2,
                        make(4, NULL, NULL),
                        make(5, NULL,
                             make(7, NULL, NULL))),
                   make(3,
                        NULL,
                        make(6, NULL, NULL)));

    printf("== Recursive traversals ==\n");
    printf("preorder : "); preorder(t);  printf("\n");
    printf("inorder  : "); inorder(t);   printf("\n");
    printf("postorder: "); postorder(t); printf("\n");

    printf("\n== Iterative ==\n");
    printf("inorder  : "); inorder_iter(t); printf("\n");
    printf("BFS      : "); bfs(t);          printf("\n");

    printf("\n== Morris inorder (O(1) space) ==\n");
    printf("inorder  : "); morris_inorder(t); printf("\n");

    Stats s = stats(t);
    printf("\n== One-pass properties ==\n");
    printf("height=%d  diameter=%d  balanced=%s\n",
           s.height, s.diameter, s.balanced ? "yes" : "no");

    tree_free(t);
    return 0;
}
