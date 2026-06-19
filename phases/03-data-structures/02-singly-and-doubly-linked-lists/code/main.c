/* main.c — SLL, DLL, and Linux-style intrusive list demo. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stddef.h>
#include <assert.h>

/* ============================================================ */
/* Singly Linked List (SLL)                                     */
/* ============================================================ */

typedef struct SNode { int data; struct SNode *next; } SNode;
typedef struct { SNode *head; SNode *tail; size_t len; } SLL;

static void sll_init(SLL *l) { memset(l, 0, sizeof(*l)); }

static void sll_push_front(SLL *l, int x) {
    SNode *n = malloc(sizeof(*n));
    n->data = x; n->next = l->head;
    l->head = n;
    if (!l->tail) l->tail = n;
    l->len++;
}

static void sll_push_back(SLL *l, int x) {
    SNode *n = malloc(sizeof(*n));
    n->data = x; n->next = NULL;
    if (l->tail) l->tail->next = n; else l->head = n;
    l->tail = n;
    l->len++;
}

static int sll_pop_front(SLL *l) {
    assert(l->head);
    SNode *n = l->head;
    int x = n->data;
    l->head = n->next;
    if (!l->head) l->tail = NULL;
    free(n);
    l->len--;
    return x;
}

static void sll_reverse(SLL *l) {
    SNode *prev = NULL, *cur = l->head;
    l->tail = l->head;
    while (cur) {
        SNode *next = cur->next;
        cur->next = prev;
        prev = cur;
        cur = next;
    }
    l->head = prev;
}

static void sll_print(const char *label, const SLL *l) {
    printf("%s: [", label);
    for (SNode *n = l->head; n; n = n->next) printf("%d%s", n->data, n->next ? ", " : "");
    printf("]  len=%zu\n", l->len);
}

static void sll_free(SLL *l) {
    SNode *n = l->head;
    while (n) { SNode *next = n->next; free(n); n = next; }
    sll_init(l);
}

/* ============================================================ */
/* Doubly Linked List (DLL)                                     */
/* ============================================================ */

typedef struct DNode { int data; struct DNode *prev, *next; } DNode;
typedef struct { DNode *head; DNode *tail; size_t len; } DLL;

static void dll_init(DLL *l) { memset(l, 0, sizeof(*l)); }

static void dll_push_back(DLL *l, int x) {
    DNode *n = malloc(sizeof(*n));
    n->data = x; n->prev = l->tail; n->next = NULL;
    if (l->tail) l->tail->next = n; else l->head = n;
    l->tail = n;
    l->len++;
}

static void dll_remove(DLL *l, DNode *n) {
    if (n->prev) n->prev->next = n->next; else l->head = n->next;
    if (n->next) n->next->prev = n->prev; else l->tail = n->prev;
    free(n);
    l->len--;
}

static void dll_print(const char *label, const DLL *l) {
    printf("%s fwd: [", label);
    for (DNode *n = l->head; n; n = n->next) printf("%d%s", n->data, n->next ? ", " : "");
    printf("]\n%s rev: [", label);
    for (DNode *n = l->tail; n; n = n->prev) printf("%d%s", n->data, n->prev ? ", " : "");
    printf("]\n");
}

static void dll_free(DLL *l) {
    DNode *n = l->head;
    while (n) { DNode *next = n->next; free(n); n = next; }
    dll_init(l);
}

/* ============================================================ */
/* Linux-style intrusive list                                    */
/* ============================================================ */

struct list_head { struct list_head *prev, *next; };

#define LIST_HEAD_INIT(name) { &(name), &(name) }
#define list_for_each(pos, head) for (pos = (head)->next; pos != (head); pos = pos->next)
#define container_of(ptr, type, member) \
    ((type *)((char *)(ptr) - offsetof(type, member)))
#define list_entry(ptr, type, member) container_of(ptr, type, member)

static void list_add_tail(struct list_head *node, struct list_head *head) {
    node->prev = head->prev;
    node->next = head;
    head->prev->next = node;
    head->prev = node;
}

struct task {
    int pid;
    char name[16];
    struct list_head tasks;
};

int main(void) {
    /* SLL */
    printf("== SLL ==\n");
    SLL s; sll_init(&s);
    for (int i = 1; i <= 5; ++i) sll_push_back(&s, i);
    sll_print("after push_back 1..5", &s);
    sll_push_front(&s, 0);
    sll_print("after push_front 0  ", &s);
    sll_reverse(&s);
    sll_print("after reverse        ", &s);
    while (s.len) sll_pop_front(&s);
    sll_free(&s);

    /* DLL */
    printf("\n== DLL ==\n");
    DLL d; dll_init(&d);
    for (int i = 10; i <= 50; i += 10) dll_push_back(&d, i);
    dll_print("DLL after push_back ×5", &d);
    DNode *mid = d.head->next->next;
    printf("removing node with data=%d\n", mid->data);
    dll_remove(&d, mid);
    dll_print("after remove(30)        ", &d);
    dll_free(&d);

    /* Intrusive list */
    printf("\n== Intrusive list (Linux-style) ==\n");
    struct list_head all_tasks = LIST_HEAD_INIT(all_tasks);
    struct task tasks[] = {
        { 100, "init",    {NULL, NULL} },
        { 101, "kthread", {NULL, NULL} },
        { 102, "shell",   {NULL, NULL} },
    };
    for (size_t i = 0; i < sizeof(tasks)/sizeof(tasks[0]); ++i)
        list_add_tail(&tasks[i].tasks, &all_tasks);

    struct list_head *pos;
    list_for_each(pos, &all_tasks) {
        struct task *t = list_entry(pos, struct task, tasks);
        printf("  pid=%d name=%s   (recovered via container_of)\n", t->pid, t->name);
    }

    printf("\n== done ==\n");
    return 0;
}
