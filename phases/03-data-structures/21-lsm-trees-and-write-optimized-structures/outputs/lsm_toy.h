/*
 * lsm_toy.h — single-header toy LSM tree (memtable + sorted SSTables + Bloom).
 * For learning. Real LSMs (LevelDB, RocksDB) are tens of thousands of lines.
 *
 *   LSM l; lsm_init(&l);
 *   lsm_put(&l, key, value);
 *   int v; bool found = lsm_get(&l, key, &v);
 *   lsm_free(&l);
 */
#ifndef LSM_TOY_H
#define LSM_TOY_H

#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdbool.h>

#define LSM_MEMTABLE_LIMIT 1000
#define LSM_MAX_SSTABLES    16

typedef struct { uint8_t *bits; size_t m; int k; } LsmBloom;
typedef struct {
    uint64_t *keys;
    int      *values;
    size_t    n;
    LsmBloom  bloom;
} LsmSSTable;
typedef struct {
    uint64_t   mt_keys[LSM_MEMTABLE_LIMIT];
    int        mt_values[LSM_MEMTABLE_LIMIT];
    size_t     mt_n;
    LsmSSTable ssts[LSM_MAX_SSTABLES];
    int        n_ssts;
} LSM;

static uint64_t lsm__mix(uint64_t x) {
    x += 0x9e3779b97f4a7c15ULL;
    x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
    x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
    return x ^ (x >> 31);
}
static void lsm__bloom_init(LsmBloom *b, size_t m, int k) {
    b->m = m; b->k = k; b->bits = (uint8_t *)calloc((m + 7) / 8, 1);
}
static void lsm__bloom_add(LsmBloom *b, uint64_t x) {
    for (int i = 0; i < b->k; ++i) {
        size_t h = lsm__mix(x + (uint64_t)i * 0xdeadbeefULL) % b->m;
        b->bits[h / 8] |= (uint8_t)(1 << (h & 7));
    }
}
static bool lsm__bloom_check(const LsmBloom *b, uint64_t x) {
    for (int i = 0; i < b->k; ++i) {
        size_t h = lsm__mix(x + (uint64_t)i * 0xdeadbeefULL) % b->m;
        if (!((b->bits[h / 8] >> (h & 7)) & 1)) return false;
    }
    return true;
}

typedef struct { uint64_t k; int v; } LsmKV;
static int lsm__cmp(const void *a, const void *b) {
    uint64_t x = ((const LsmKV *)a)->k, y = ((const LsmKV *)b)->k;
    return x < y ? -1 : x > y ? 1 : 0;
}

static inline void lsm_init(LSM *l) { memset(l, 0, sizeof(*l)); }

static void lsm__sst_build(LsmSSTable *s, uint64_t *keys, int *values, size_t n) {
    LsmKV *kv = (LsmKV *)malloc(n * sizeof(LsmKV));
    for (size_t i = 0; i < n; ++i) { kv[i].k = keys[i]; kv[i].v = values[i]; }
    qsort(kv, n, sizeof(LsmKV), lsm__cmp);
    s->keys = (uint64_t *)malloc(n * sizeof(uint64_t));
    s->values = (int *)malloc(n * sizeof(int));
    for (size_t i = 0; i < n; ++i) { s->keys[i] = kv[i].k; s->values[i] = kv[i].v; }
    s->n = n;
    free(kv);
    lsm__bloom_init(&s->bloom, 10 * n, 7);
    for (size_t i = 0; i < n; ++i) lsm__bloom_add(&s->bloom, s->keys[i]);
}

static inline void lsm_flush(LSM *l) {
    if (l->mt_n == 0) return;
    if (l->n_ssts >= LSM_MAX_SSTABLES) return;
    memmove(&l->ssts[1], &l->ssts[0], l->n_ssts * sizeof(LsmSSTable));
    lsm__sst_build(&l->ssts[0], l->mt_keys, l->mt_values, l->mt_n);
    l->n_ssts++;
    l->mt_n = 0;
}

static inline void lsm_put(LSM *l, uint64_t key, int value) {
    for (size_t i = 0; i < l->mt_n; ++i)
        if (l->mt_keys[i] == key) { l->mt_values[i] = value; return; }
    l->mt_keys[l->mt_n] = key;
    l->mt_values[l->mt_n] = value;
    l->mt_n++;
    if (l->mt_n == LSM_MEMTABLE_LIMIT) lsm_flush(l);
}

static bool lsm__sst_get(const LsmSSTable *s, uint64_t k, int *out) {
    if (!lsm__bloom_check(&s->bloom, k)) return false;
    size_t lo = 0, hi = s->n;
    while (lo < hi) {
        size_t m = (lo + hi) / 2;
        if (s->keys[m] < k) lo = m + 1; else hi = m;
    }
    if (lo < s->n && s->keys[lo] == k) { *out = s->values[lo]; return true; }
    return false;
}

static inline bool lsm_get(const LSM *l, uint64_t key, int *out) {
    for (size_t i = 0; i < l->mt_n; ++i)
        if (l->mt_keys[i] == key) { *out = l->mt_values[i]; return true; }
    for (int i = 0; i < l->n_ssts; ++i)
        if (lsm__sst_get(&l->ssts[i], key, out)) return true;
    return false;
}

static inline void lsm_free(LSM *l) {
    for (int i = 0; i < l->n_ssts; ++i) {
        free(l->ssts[i].keys); free(l->ssts[i].values);
        free(l->ssts[i].bloom.bits);
    }
}

#endif /* LSM_TOY_H */
