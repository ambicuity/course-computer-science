/* main.c — chaining, linear-probing, and Robin Hood hash tables. */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <time.h>
#include <assert.h>

/* Fast 64-bit mix function (splitmix64). */
static uint64_t mix64(uint64_t x) {
    x += 0x9e3779b97f4a7c15ULL;
    x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
    x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
    return x ^ (x >> 31);
}

/* ============================================================ */
/* Chaining hash table                                          */
/* ============================================================ */

typedef struct CEntry { uint64_t key; int val; struct CEntry *next; } CEntry;
typedef struct {
    CEntry **buckets;
    size_t   cap, len;
} HashChain;

static void hc_init(HashChain *t, size_t cap) {
    size_t c = 16; while (c < cap) c *= 2;
    t->buckets = calloc(c, sizeof(CEntry *));
    t->cap = c; t->len = 0;
}
static void hc_resize(HashChain *t) {
    size_t old_cap = t->cap;
    CEntry **old = t->buckets;
    t->cap = old_cap * 2;
    t->buckets = calloc(t->cap, sizeof(CEntry *));
    for (size_t i = 0; i < old_cap; ++i) {
        CEntry *e = old[i];
        while (e) {
            CEntry *next = e->next;
            size_t idx = mix64(e->key) & (t->cap - 1);
            e->next = t->buckets[idx];
            t->buckets[idx] = e;
            e = next;
        }
    }
    free(old);
}
static void hc_put(HashChain *t, uint64_t key, int val) {
    if (t->len * 4 > t->cap * 3) hc_resize(t);     /* α > 0.75 */
    size_t idx = mix64(key) & (t->cap - 1);
    for (CEntry *e = t->buckets[idx]; e; e = e->next) {
        if (e->key == key) { e->val = val; return; }
    }
    CEntry *n = malloc(sizeof(*n));
    n->key = key; n->val = val; n->next = t->buckets[idx];
    t->buckets[idx] = n;
    t->len++;
}
static int hc_get(const HashChain *t, uint64_t key, int *out) {
    size_t idx = mix64(key) & (t->cap - 1);
    for (CEntry *e = t->buckets[idx]; e; e = e->next) {
        if (e->key == key) { *out = e->val; return 1; }
    }
    return 0;
}
static void hc_free(HashChain *t) {
    for (size_t i = 0; i < t->cap; ++i) {
        CEntry *e = t->buckets[i];
        while (e) { CEntry *next = e->next; free(e); e = next; }
    }
    free(t->buckets);
}

/* ============================================================ */
/* Linear-probing hash table                                    */
/* ============================================================ */

#define LP_EMPTY     0
#define LP_OCCUPIED  1
#define LP_TOMBSTONE 2

typedef struct {
    uint64_t key;
    int      val;
    uint8_t  state;
} LPEntry;

typedef struct {
    LPEntry *buckets;
    size_t   cap, len;
    size_t   tombstones;
} HashOpen;

static void ho_init(HashOpen *t, size_t cap) {
    size_t c = 16; while (c < cap) c *= 2;
    t->buckets = calloc(c, sizeof(LPEntry));
    t->cap = c; t->len = 0; t->tombstones = 0;
}
static void ho_put(HashOpen *t, uint64_t key, int val);
static void ho_resize(HashOpen *t, size_t new_cap) {
    LPEntry *old = t->buckets;
    size_t old_cap = t->cap;
    t->buckets = calloc(new_cap, sizeof(LPEntry));
    t->cap = new_cap; t->len = 0; t->tombstones = 0;
    for (size_t i = 0; i < old_cap; ++i) {
        if (old[i].state == LP_OCCUPIED) ho_put(t, old[i].key, old[i].val);
    }
    free(old);
}
static void ho_put(HashOpen *t, uint64_t key, int val) {
    if ((t->len + t->tombstones) * 2 > t->cap) ho_resize(t, t->cap * 2);  /* α > 0.5 */
    size_t mask = t->cap - 1;
    size_t i = mix64(key) & mask;
    size_t first_tomb = (size_t)-1;
    while (1) {
        if (t->buckets[i].state == LP_EMPTY) {
            size_t target = first_tomb != (size_t)-1 ? first_tomb : i;
            if (t->buckets[target].state == LP_TOMBSTONE) t->tombstones--;
            t->buckets[target] = (LPEntry){ key, val, LP_OCCUPIED };
            t->len++; return;
        }
        if (t->buckets[i].state == LP_OCCUPIED && t->buckets[i].key == key) {
            t->buckets[i].val = val; return;
        }
        if (t->buckets[i].state == LP_TOMBSTONE && first_tomb == (size_t)-1) first_tomb = i;
        i = (i + 1) & mask;
    }
}
static int ho_get(const HashOpen *t, uint64_t key, int *out, size_t *probes_out) {
    size_t mask = t->cap - 1;
    size_t i = mix64(key) & mask;
    size_t probes = 0;
    while (1) {
        probes++;
        if (t->buckets[i].state == LP_EMPTY) { if (probes_out) *probes_out = probes; return 0; }
        if (t->buckets[i].state == LP_OCCUPIED && t->buckets[i].key == key) {
            *out = t->buckets[i].val; if (probes_out) *probes_out = probes; return 1;
        }
        i = (i + 1) & mask;
    }
}
static void ho_free(HashOpen *t) { free(t->buckets); }

/* ============================================================ */
/* Robin Hood hash table                                        */
/* ============================================================ */

typedef struct {
    uint64_t key;
    int      val;
    uint8_t  occupied;
    uint32_t dist;             /* probe distance from ideal slot */
} RHEntry;

typedef struct { RHEntry *buckets; size_t cap, len; } HashRH;

static void hrh_init(HashRH *t, size_t cap) {
    size_t c = 16; while (c < cap) c *= 2;
    t->buckets = calloc(c, sizeof(RHEntry));
    t->cap = c; t->len = 0;
}
static void hrh_put(HashRH *t, uint64_t key, int val);
static void hrh_resize(HashRH *t, size_t new_cap) {
    RHEntry *old = t->buckets;
    size_t old_cap = t->cap;
    t->buckets = calloc(new_cap, sizeof(RHEntry));
    t->cap = new_cap; t->len = 0;
    for (size_t i = 0; i < old_cap; ++i) if (old[i].occupied) hrh_put(t, old[i].key, old[i].val);
    free(old);
}
static void hrh_put(HashRH *t, uint64_t key, int val) {
    if (t->len * 10 > t->cap * 9) hrh_resize(t, t->cap * 2);    /* α > 0.9 */
    size_t mask = t->cap - 1;
    size_t i = mix64(key) & mask;
    RHEntry incoming = { key, val, 1, 0 };
    while (1) {
        if (!t->buckets[i].occupied) { t->buckets[i] = incoming; t->len++; return; }
        if (t->buckets[i].key == incoming.key) { t->buckets[i].val = incoming.val; return; }
        if (t->buckets[i].dist < incoming.dist) {
            RHEntry tmp = t->buckets[i];
            t->buckets[i] = incoming;
            incoming = tmp;
        }
        incoming.dist++;
        i = (i + 1) & mask;
    }
}
static int hrh_get(const HashRH *t, uint64_t key, int *out, size_t *probes_out) {
    size_t mask = t->cap - 1;
    size_t i = mix64(key) & mask;
    size_t probes = 0;
    uint32_t dist = 0;
    while (1) {
        probes++;
        if (!t->buckets[i].occupied || t->buckets[i].dist < dist) {
            if (probes_out) *probes_out = probes; return 0;
        }
        if (t->buckets[i].key == key) {
            *out = t->buckets[i].val; if (probes_out) *probes_out = probes; return 1;
        }
        dist++;
        i = (i + 1) & mask;
    }
}
static void hrh_free(HashRH *t) { free(t->buckets); }

/* ============================================================ */
/* Bench                                                         */
/* ============================================================ */

static double now(void) {
    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

int main(void) {
    const size_t N = 200000;
    uint64_t *keys = malloc(N * sizeof(uint64_t));
    uint64_t s = 12345;
    for (size_t i = 0; i < N; ++i) { s = mix64(s); keys[i] = s; }

    printf("== %zu inserts + %zu lookups ==\n\n", N, N);

    /* Chaining */
    HashChain hc; hc_init(&hc, 32);
    double t = now();
    for (size_t i = 0; i < N; ++i) hc_put(&hc, keys[i], (int)i);
    double t_ins = now() - t;
    t = now();
    long checksum = 0;
    for (size_t i = 0; i < N; ++i) { int v; if (hc_get(&hc, keys[i], &v)) checksum += v; }
    double t_lk = now() - t;
    printf("Chaining   : insert %.3fs (%.0f ns/op)  lookup %.3fs (%.0f ns/op)  cap=%zu  α=%.2f  checksum=%ld\n",
           t_ins, t_ins * 1e9 / N, t_lk, t_lk * 1e9 / N, hc.cap, (double)hc.len / hc.cap, checksum);
    hc_free(&hc);

    /* Linear probing */
    HashOpen ho; ho_init(&ho, 32);
    t = now();
    for (size_t i = 0; i < N; ++i) ho_put(&ho, keys[i], (int)i);
    t_ins = now() - t;
    t = now();
    checksum = 0;
    size_t total_probes = 0, max_probes = 0;
    for (size_t i = 0; i < N; ++i) {
        int v; size_t p = 0;
        if (ho_get(&ho, keys[i], &v, &p)) checksum += v;
        total_probes += p; if (p > max_probes) max_probes = p;
    }
    t_lk = now() - t;
    printf("Linear LP  : insert %.3fs (%.0f ns/op)  lookup %.3fs (%.0f ns/op)  cap=%zu  α=%.2f  avg_probe=%.2f max=%zu\n",
           t_ins, t_ins * 1e9 / N, t_lk, t_lk * 1e9 / N, ho.cap, (double)ho.len / ho.cap,
           (double)total_probes / N, max_probes);
    ho_free(&ho);

    /* Robin Hood */
    HashRH hrh; hrh_init(&hrh, 32);
    t = now();
    for (size_t i = 0; i < N; ++i) hrh_put(&hrh, keys[i], (int)i);
    t_ins = now() - t;
    t = now();
    checksum = 0;
    total_probes = 0; max_probes = 0;
    for (size_t i = 0; i < N; ++i) {
        int v; size_t p = 0;
        if (hrh_get(&hrh, keys[i], &v, &p)) checksum += v;
        total_probes += p; if (p > max_probes) max_probes = p;
    }
    t_lk = now() - t;
    printf("Robin Hood : insert %.3fs (%.0f ns/op)  lookup %.3fs (%.0f ns/op)  cap=%zu  α=%.2f  avg_probe=%.2f max=%zu\n",
           t_ins, t_ins * 1e9 / N, t_lk, t_lk * 1e9 / N, hrh.cap, (double)hrh.len / hrh.cap,
           (double)total_probes / N, max_probes);
    hrh_free(&hrh);

    free(keys);
    return 0;
}
