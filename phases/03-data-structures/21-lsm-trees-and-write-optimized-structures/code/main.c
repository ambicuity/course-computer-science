/* main.c — toy LSM tree: memtable + SSTables + per-SSTable Bloom filter. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>
#include <math.h>
#include <time.h>

/* ============================================================ */
/* Bloom (lifted from L20)                                       */
/* ============================================================ */
typedef struct { uint8_t *bits; size_t m; int k; } Bloom;
static uint64_t mix64(uint64_t x) {
    x += 0x9e3779b97f4a7c15ULL;
    x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
    x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
    return x ^ (x >> 31);
}
static void bloom_init(Bloom *b, size_t m, int k) { b->m=m; b->k=k; b->bits=calloc((m+7)/8,1); }
static void bloom_add(Bloom *b, uint64_t x) {
    for (int i=0;i<b->k;++i){size_t h=mix64(x+(uint64_t)i*0xdeadbeef)%b->m; b->bits[h/8]|=(uint8_t)(1<<(h&7));}
}
static bool bloom_check(const Bloom *b, uint64_t x) {
    for (int i=0;i<b->k;++i){size_t h=mix64(x+(uint64_t)i*0xdeadbeef)%b->m; if(!((b->bits[h/8]>>(h&7))&1))return false;}
    return true;
}
static void bloom_free(Bloom *b){free(b->bits);}

/* ============================================================ */
/* SSTable: sorted (key, value) pairs, immutable                */
/* ============================================================ */
typedef struct {
    uint64_t *keys;
    int      *values;          /* INT_MIN = tombstone */
    size_t    n;
    Bloom     bloom;
} SSTable;

#define TOMBSTONE INT32_MIN

static int sst__cmp(const void *a, const void *b) {
    uint64_t x = *(const uint64_t *)a, y = *(const uint64_t *)b;
    return x < y ? -1 : x > y ? 1 : 0;
}

static void sst_build(SSTable *s, uint64_t *keys, int *values, size_t n) {
    /* Sort by key. Use a parallel sort. */
    /* Build (key, val) tmp array, sort by key, split back. */
    typedef struct { uint64_t k; int v; } KV;
    KV *kv = malloc(n * sizeof(KV));
    for (size_t i = 0; i < n; ++i) { kv[i].k = keys[i]; kv[i].v = values[i]; }
    qsort(kv, n, sizeof(KV), sst__cmp);
    s->keys = malloc(n * sizeof(uint64_t));
    s->values = malloc(n * sizeof(int));
    for (size_t i = 0; i < n; ++i) { s->keys[i] = kv[i].k; s->values[i] = kv[i].v; }
    s->n = n;
    free(kv);
    bloom_init(&s->bloom, 10 * n, 7);
    for (size_t i = 0; i < n; ++i) bloom_add(&s->bloom, s->keys[i]);
}

static bool sst_get(const SSTable *s, uint64_t k, int *out, bool use_bloom) {
    if (use_bloom && !bloom_check(&s->bloom, k)) return false;
    size_t lo = 0, hi = s->n;
    while (lo < hi) {
        size_t m = (lo + hi) / 2;
        if (s->keys[m] < k) lo = m + 1;
        else hi = m;
    }
    if (lo < s->n && s->keys[lo] == k) { *out = s->values[lo]; return true; }
    return false;
}

static void sst_free(SSTable *s) {
    free(s->keys); free(s->values); bloom_free(&s->bloom);
}

/* ============================================================ */
/* Memtable: unsorted buffer; flush sorts & makes SSTable      */
/* ============================================================ */
#define MEMTABLE_LIMIT 1000

typedef struct {
    uint64_t keys[MEMTABLE_LIMIT];
    int      values[MEMTABLE_LIMIT];
    size_t   n;
} Memtable;

/* ============================================================ */
/* LSM tree                                                      */
/* ============================================================ */
#define MAX_SSTABLES 16

typedef struct {
    Memtable mt;
    SSTable  ssts[MAX_SSTABLES];      /* newest first */
    int      n_ssts;
} LSM;

static void lsm_init(LSM *l) { memset(l, 0, sizeof(*l)); }

static void lsm_flush(LSM *l) {
    if (l->mt.n == 0) return;                              /* nothing to flush */
    if (l->n_ssts >= MAX_SSTABLES) { /* in real LSM: compaction */ return; }
    memmove(&l->ssts[1], &l->ssts[0], l->n_ssts * sizeof(SSTable));
    sst_build(&l->ssts[0], l->mt.keys, l->mt.values, l->mt.n);
    l->n_ssts++;
    l->mt.n = 0;
}

static void lsm_put(LSM *l, uint64_t key, int value) {
    /* Linear scan memtable to update existing key (cheap at this size). */
    for (size_t i = 0; i < l->mt.n; ++i)
        if (l->mt.keys[i] == key) { l->mt.values[i] = value; return; }
    l->mt.keys[l->mt.n] = key;
    l->mt.values[l->mt.n] = value;
    l->mt.n++;
    if (l->mt.n == MEMTABLE_LIMIT) lsm_flush(l);
}

static bool lsm_get(const LSM *l, uint64_t key, int *out, bool use_bloom) {
    /* Memtable first */
    for (size_t i = 0; i < l->mt.n; ++i)
        if (l->mt.keys[i] == key) { *out = l->mt.values[i]; return true; }
    /* SSTables newest to oldest */
    for (int i = 0; i < l->n_ssts; ++i)
        if (sst_get(&l->ssts[i], key, out, use_bloom)) return true;
    return false;
}

static int lsm_count_bloom_skips(const LSM *l, uint64_t key) {
    int skips = 0;
    for (int i = 0; i < l->n_ssts; ++i)
        if (!bloom_check(&l->ssts[i].bloom, key)) skips++;
    return skips;
}

static void lsm_free(LSM *l) {
    for (int i = 0; i < l->n_ssts; ++i) sst_free(&l->ssts[i]);
}

static double now(void) {
    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

int main(void) {
    LSM l; lsm_init(&l);

    /* Write 10K distinct keys with MEMTABLE_LIMIT=1000 → 10 SSTables. */
    const int N = 10000;
    for (uint64_t i = 0; i < (uint64_t)N; ++i) lsm_put(&l, i * 7 + 1, (int)i);
    lsm_flush(&l);                                      /* flush remaining */
    printf("== LSM tree ==\n");
    printf("  inserted %d keys, n_ssts = %d, memtable.n = %zu\n",
           N, l.n_ssts, l.mt.n);

    /* Verify reads */
    int found = 0;
    for (uint64_t i = 0; i < (uint64_t)N; ++i) {
        int v;
        if (lsm_get(&l, i * 7 + 1, &v, true) && v == (int)i) ++found;
    }
    printf("  reads with Bloom : %d / %d\n", found, N);

    /* Bloom skip demo: query NEW keys (not in any SSTable). */
    int total_skips = 0;
    for (uint64_t k = 70001; k < 71000; ++k) total_skips += lsm_count_bloom_skips(&l, k);
    printf("  for 1000 NEW keys, total Bloom skips: %d / %d (expect ≈%.0f%% skip rate)\n",
           total_skips, 1000 * l.n_ssts, 99.0);

    /* Read bench with vs without Bloom */
    double t = now();
    int hits = 0;
    for (int iter = 0; iter < 10; ++iter)
        for (uint64_t k = 70001; k < 71000; ++k) {
            int v;
            if (lsm_get(&l, k, &v, true)) ++hits;
        }
    double t_bloom = now() - t;

    t = now();
    int hits_nb = 0;
    for (int iter = 0; iter < 10; ++iter)
        for (uint64_t k = 70001; k < 71000; ++k) {
            int v;
            if (lsm_get(&l, k, &v, false)) ++hits_nb;
        }
    double t_nobloom = now() - t;

    printf("\n  reads for 1000 missing keys × 10 iters (in-memory SSTs):\n");
    printf("    with    Bloom: %.4f s  (hits = %d)\n", t_bloom, hits);
    printf("    without Bloom: %.4f s  (hits = %d)\n", t_nobloom, hits_nb);
    printf("    speedup ratio: %.2f×\n", t_nobloom / t_bloom);
    printf("    NOTE: in-memory SSTs are tiny, so Bloom overhead ~ search cost.\n");
    printf("    On disk (multi-MB SSTs), each saved seek is ~100 µs vs 30 ns Bloom.\n");

    lsm_free(&l);
    return 0;
}
