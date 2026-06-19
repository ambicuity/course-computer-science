/* main.c — Skip list with insert, search, delete. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

#define MAX_LEVEL 16
#define P 0.5

typedef struct Node {
    int           key;
    int           level;
    struct Node **next;
} Node;

typedef struct {
    Node *head;
    int   max_level;
    int   n;
} SkipList;

static unsigned long rng_state = 0xdeadbeef;
static int random_level(void) {
    rng_state = rng_state * 6364136223846793005ULL + 1442695040888963407ULL;
    int lvl = 1;
    unsigned long s = rng_state >> 32;
    while ((s & 1) && lvl < MAX_LEVEL) { lvl++; s >>= 1; }
    return lvl;
}

static Node *new_node(int key, int level) {
    Node *n = malloc(sizeof(Node));
    n->key = key; n->level = level;
    n->next = calloc(level, sizeof(Node *));
    return n;
}

static void sl_init(SkipList *s) {
    s->head = new_node(-1, MAX_LEVEL);
    s->max_level = 1; s->n = 0;
}

static int sl_search(const SkipList *s, int key) {
    const Node *cur = s->head;
    for (int lvl = s->max_level - 1; lvl >= 0; --lvl) {
        while (cur->next[lvl] && cur->next[lvl]->key < key) cur = cur->next[lvl];
    }
    cur = cur->next[0];
    return cur && cur->key == key;
}

static void sl_insert(SkipList *s, int key) {
    Node *update[MAX_LEVEL] = {0};
    Node *cur = s->head;
    for (int lvl = s->max_level - 1; lvl >= 0; --lvl) {
        while (cur->next[lvl] && cur->next[lvl]->key < key) cur = cur->next[lvl];
        update[lvl] = cur;
    }
    if (cur->next[0] && cur->next[0]->key == key) return;       /* duplicate */

    int lvl = random_level();
    if (lvl > s->max_level) {
        for (int i = s->max_level; i < lvl; ++i) update[i] = s->head;
        s->max_level = lvl;
    }
    Node *n = new_node(key, lvl);
    for (int i = 0; i < lvl; ++i) {
        n->next[i] = update[i]->next[i];
        update[i]->next[i] = n;
    }
    s->n++;
}

static int sl_delete(SkipList *s, int key) {
    Node *update[MAX_LEVEL] = {0};
    Node *cur = s->head;
    for (int lvl = s->max_level - 1; lvl >= 0; --lvl) {
        while (cur->next[lvl] && cur->next[lvl]->key < key) cur = cur->next[lvl];
        update[lvl] = cur;
    }
    cur = cur->next[0];
    if (!cur || cur->key != key) return 0;
    for (int i = 0; i < cur->level; ++i) {
        if (update[i]->next[i] == cur) update[i]->next[i] = cur->next[i];
    }
    while (s->max_level > 1 && !s->head->next[s->max_level - 1]) s->max_level--;
    free(cur->next); free(cur);
    s->n--;
    return 1;
}

static int sl_count_at_level(const SkipList *s, int lvl) {
    int c = 0;
    for (Node *cur = s->head->next[lvl]; cur; cur = cur->next[lvl]) ++c;
    return c;
}

static void sl_free(SkipList *s) {
    Node *cur = s->head;
    while (cur) {
        Node *next = cur->next[0];
        free(cur->next); free(cur);
        cur = next;
    }
}

int main(void) {
    SkipList s; sl_init(&s);

    /* Insert 0..999 in random-ish order */
    srand(42);
    int *keys = malloc(1000 * sizeof(int));
    for (int i = 0; i < 1000; ++i) keys[i] = i;
    for (int i = 999; i > 0; --i) {
        int j = rand() % (i + 1);
        int t = keys[i]; keys[i] = keys[j]; keys[j] = t;
    }
    for (int i = 0; i < 1000; ++i) sl_insert(&s, keys[i]);

    printf("== Skip list (n=1000) ==\n");
    printf("  max_level reached: %d  (expected ~log2(1000) = 10)\n", s.max_level);
    for (int lvl = 0; lvl < s.max_level; ++lvl) {
        printf("  level %d: %d nodes (expected ~%d)\n", lvl, sl_count_at_level(&s, lvl), 1000 >> lvl);
    }

    int found = 0;
    for (int i = 0; i < 1000; ++i) found += sl_search(&s, i);
    printf("  search all 0..999: found %d / 1000\n", found);
    printf("  search(1000): %d (expect 0)\n", sl_search(&s, 1000));

    /* Delete every other key */
    for (int i = 0; i < 1000; i += 2) sl_delete(&s, i);
    int found_after = 0;
    for (int i = 0; i < 1000; ++i) found_after += sl_search(&s, i);
    printf("  after deleting evens: found %d / 1000 (expect 500)\n", found_after);
    printf("  size: %d (expect 500)\n", s.n);

    sl_free(&s); free(keys);
    return 0;
}
