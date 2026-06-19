/*
 * skiplist.h — single-header skip list for int keys.
 *   SkipList s; sl_init(&s);
 *   sl_insert(&s, 42);
 *   if (sl_search(&s, 42)) ...
 *   sl_delete(&s, 42);
 *   sl_free(&s);
 */
#ifndef SKIPLIST_H
#define SKIPLIST_H

#include <stdlib.h>
#include <stdbool.h>

#ifndef SKIPLIST_MAX_LEVEL
#  define SKIPLIST_MAX_LEVEL 16
#endif

typedef struct SkipListNode {
    int                   key;
    int                   level;
    struct SkipListNode **next;
} SkipListNode;

typedef struct {
    SkipListNode *head;
    int           max_level;
    int           n;
    unsigned long rng;
} SkipList;

static inline SkipListNode *sl__new(int key, int lvl) {
    SkipListNode *n = (SkipListNode *)malloc(sizeof(*n));
    n->key = key; n->level = lvl;
    n->next = (SkipListNode **)calloc(lvl, sizeof(SkipListNode *));
    return n;
}

static inline int sl__rand(SkipList *s) {
    s->rng = s->rng * 6364136223846793005ULL + 1442695040888963407ULL;
    int lvl = 1;
    unsigned long t = s->rng >> 32;
    while ((t & 1) && lvl < SKIPLIST_MAX_LEVEL) { lvl++; t >>= 1; }
    return lvl;
}

static inline void sl_init(SkipList *s) {
    s->head = sl__new(-1, SKIPLIST_MAX_LEVEL);
    s->max_level = 1; s->n = 0;
    s->rng = 0x9e3779b9;
}

static inline bool sl_search(const SkipList *s, int key) {
    const SkipListNode *cur = s->head;
    for (int lvl = s->max_level - 1; lvl >= 0; --lvl)
        while (cur->next[lvl] && cur->next[lvl]->key < key) cur = cur->next[lvl];
    cur = cur->next[0];
    return cur && cur->key == key;
}

static inline void sl_insert(SkipList *s, int key) {
    SkipListNode *update[SKIPLIST_MAX_LEVEL] = {0};
    SkipListNode *cur = s->head;
    for (int lvl = s->max_level - 1; lvl >= 0; --lvl) {
        while (cur->next[lvl] && cur->next[lvl]->key < key) cur = cur->next[lvl];
        update[lvl] = cur;
    }
    if (cur->next[0] && cur->next[0]->key == key) return;
    int lvl = sl__rand(s);
    if (lvl > s->max_level) {
        for (int i = s->max_level; i < lvl; ++i) update[i] = s->head;
        s->max_level = lvl;
    }
    SkipListNode *n = sl__new(key, lvl);
    for (int i = 0; i < lvl; ++i) {
        n->next[i] = update[i]->next[i];
        update[i]->next[i] = n;
    }
    s->n++;
}

static inline bool sl_delete(SkipList *s, int key) {
    SkipListNode *update[SKIPLIST_MAX_LEVEL] = {0};
    SkipListNode *cur = s->head;
    for (int lvl = s->max_level - 1; lvl >= 0; --lvl) {
        while (cur->next[lvl] && cur->next[lvl]->key < key) cur = cur->next[lvl];
        update[lvl] = cur;
    }
    cur = cur->next[0];
    if (!cur || cur->key != key) return false;
    for (int i = 0; i < cur->level; ++i)
        if (update[i]->next[i] == cur) update[i]->next[i] = cur->next[i];
    while (s->max_level > 1 && !s->head->next[s->max_level - 1]) s->max_level--;
    free(cur->next); free(cur);
    s->n--;
    return true;
}

static inline void sl_free(SkipList *s) {
    SkipListNode *cur = s->head;
    while (cur) {
        SkipListNode *next = cur->next[0];
        free(cur->next); free(cur);
        cur = next;
    }
}

#endif /* SKIPLIST_H */
