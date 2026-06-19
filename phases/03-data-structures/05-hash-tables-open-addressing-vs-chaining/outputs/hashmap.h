/*
 * hashmap.h — single-header Robin Hood open-addressing hash map for uint64 → int.
 *
 * Usage:
 *     #define HASHMAP_IMPLEMENTATION
 *     #include "hashmap.h"
 *
 *     HashMap m;
 *     hm_init(&m);
 *     hm_put(&m, 42, 100);
 *     int v;
 *     if (hm_get(&m, 42, &v)) printf("got %d\n", v);
 *     hm_remove(&m, 42);
 *     hm_free(&m);
 *
 * License: MIT.
 */
#ifndef HASHMAP_H
#define HASHMAP_H

#include <stddef.h>
#include <stdint.h>

typedef struct {
    uint64_t key;
    int      val;
    uint8_t  occupied;
    uint32_t dist;
} HMEntry;

typedef struct {
    HMEntry *slots;
    size_t   cap, len;
} HashMap;

void hm_init(HashMap *m);
void hm_put(HashMap *m, uint64_t k, int v);
int  hm_get(const HashMap *m, uint64_t k, int *out);
int  hm_remove(HashMap *m, uint64_t k);
size_t hm_size(const HashMap *m);
void hm_free(HashMap *m);

#ifdef HASHMAP_IMPLEMENTATION
#include <stdlib.h>
#include <string.h>
#include <assert.h>

static uint64_t hm__mix(uint64_t x) {
    x += 0x9e3779b97f4a7c15ULL;
    x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
    x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
    return x ^ (x >> 31);
}

void hm_init(HashMap *m) {
    m->cap = 16;
    m->slots = (HMEntry *)calloc(m->cap, sizeof(HMEntry));
    m->len = 0;
}

static void hm__resize(HashMap *m) {
    HMEntry *old = m->slots;
    size_t old_cap = m->cap;
    m->cap *= 2;
    m->slots = (HMEntry *)calloc(m->cap, sizeof(HMEntry));
    m->len = 0;
    for (size_t i = 0; i < old_cap; ++i)
        if (old[i].occupied) hm_put(m, old[i].key, old[i].val);
    free(old);
}

void hm_put(HashMap *m, uint64_t k, int v) {
    if (m->len * 10 > m->cap * 9) hm__resize(m);
    size_t mask = m->cap - 1;
    size_t i = hm__mix(k) & mask;
    HMEntry in = { k, v, 1, 0 };
    while (1) {
        if (!m->slots[i].occupied) { m->slots[i] = in; m->len++; return; }
        if (m->slots[i].key == in.key) { m->slots[i].val = in.val; return; }
        if (m->slots[i].dist < in.dist) {
            HMEntry tmp = m->slots[i]; m->slots[i] = in; in = tmp;
        }
        in.dist++;
        i = (i + 1) & mask;
    }
}

int hm_get(const HashMap *m, uint64_t k, int *out) {
    size_t mask = m->cap - 1;
    size_t i = hm__mix(k) & mask;
    uint32_t dist = 0;
    while (1) {
        if (!m->slots[i].occupied || m->slots[i].dist < dist) return 0;
        if (m->slots[i].key == k) { *out = m->slots[i].val; return 1; }
        dist++;
        i = (i + 1) & mask;
    }
}

int hm_remove(HashMap *m, uint64_t k) {
    size_t mask = m->cap - 1;
    size_t i = hm__mix(k) & mask;
    uint32_t dist = 0;
    while (m->slots[i].occupied && m->slots[i].dist >= dist) {
        if (m->slots[i].key == k) {
            /* back-shift delete */
            size_t j = (i + 1) & mask;
            while (m->slots[j].occupied && m->slots[j].dist > 0) {
                m->slots[i] = m->slots[j];
                m->slots[i].dist--;
                i = j;
                j = (j + 1) & mask;
            }
            m->slots[i] = (HMEntry){0};
            m->len--;
            return 1;
        }
        dist++;
        i = (i + 1) & mask;
    }
    return 0;
}

size_t hm_size(const HashMap *m) { return m->len; }
void hm_free(HashMap *m) { free(m->slots); memset(m, 0, sizeof(*m)); }

#endif /* HASHMAP_IMPLEMENTATION */
#endif /* HASHMAP_H */
