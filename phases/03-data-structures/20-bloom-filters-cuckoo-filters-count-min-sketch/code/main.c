/* main.c — Bloom filter + Count-Min sketch (Cuckoo filter sketched in docs). */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <time.h>
#include <math.h>

static uint64_t mix64(uint64_t x) {
    x += 0x9e3779b97f4a7c15ULL;
    x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
    x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
    return x ^ (x >> 31);
}

/* ============================================================ */
/* Bloom filter                                                  */
/* ============================================================ */
typedef struct {
    uint8_t *bits;
    size_t   m;             /* size in bits */
    int      k;             /* number of hash functions */
} Bloom;

static void bloom_init(Bloom *b, size_t m, int k) {
    b->m = m; b->k = k;
    b->bits = calloc((m + 7) / 8, 1);
}

static void bloom_set_bit(Bloom *b, size_t i) { b->bits[i / 8] |= (uint8_t)(1 << (i & 7)); }
static int  bloom_get_bit(Bloom *b, size_t i) { return (b->bits[i / 8] >> (i & 7)) & 1; }

static void bloom_add(Bloom *b, uint64_t x) {
    for (int i = 0; i < b->k; ++i) {
        uint64_t h = mix64(x + (uint64_t)i * 0xdeadbeefULL);
        bloom_set_bit(b, h % b->m);
    }
}

static int bloom_contains(Bloom *b, uint64_t x) {
    for (int i = 0; i < b->k; ++i) {
        uint64_t h = mix64(x + (uint64_t)i * 0xdeadbeefULL);
        if (!bloom_get_bit(b, h % b->m)) return 0;
    }
    return 1;
}

static void bloom_free(Bloom *b) { free(b->bits); }

/* ============================================================ */
/* Count-Min sketch                                              */
/* ============================================================ */
typedef struct {
    long **counts;
    int    w;
    int    d;
} CountMin;

static void cm_init(CountMin *c, int w, int d) {
    c->w = w; c->d = d;
    c->counts = malloc(d * sizeof(long *));
    for (int i = 0; i < d; ++i) c->counts[i] = calloc(w, sizeof(long));
}

static void cm_add(CountMin *c, uint64_t x, long delta) {
    for (int i = 0; i < c->d; ++i) {
        uint64_t h = mix64(x + (uint64_t)i * 0xcafef00dULL);
        c->counts[i][h % c->w] += delta;
    }
}

static long cm_estimate(CountMin *c, uint64_t x) {
    long m = -1;
    for (int i = 0; i < c->d; ++i) {
        uint64_t h = mix64(x + (uint64_t)i * 0xcafef00dULL);
        long v = c->counts[i][h % c->w];
        if (m == -1 || v < m) m = v;
    }
    return m;
}

static void cm_free(CountMin *c) {
    for (int i = 0; i < c->d; ++i) free(c->counts[i]);
    free(c->counts);
}

/* ============================================================ */
/* Demo + FPR experiment                                         */
/* ============================================================ */
int main(void) {
    /* Bloom: n=10000, target ε=0.01 → m ≈ 96000, k = 7 */
    const int n_in = 10000;
    const int n_out = 100000;
    Bloom b; bloom_init(&b, 96000, 7);
    for (uint64_t i = 0; i < (uint64_t)n_in; ++i) bloom_add(&b, i);

    /* False-positive: query items NOT inserted (range [n_in, n_in + n_out)). */
    int fp = 0;
    for (uint64_t i = (uint64_t)n_in; i < (uint64_t)(n_in + n_out); ++i)
        if (bloom_contains(&b, i)) fp++;

    /* Verify no false negatives. */
    int fn = 0;
    for (uint64_t i = 0; i < (uint64_t)n_in; ++i)
        if (!bloom_contains(&b, i)) fn++;

    double observed_fpr = (double)fp / n_out;
    /* Theoretical: (1 - e^(-kn/m))^k for k=7, n=10000, m=96000 */
    double theoretical = pow(1.0 - exp(-7.0 * n_in / 96000.0), 7);

    printf("== Bloom filter (m=96000, k=7, n=%d) ==\n", n_in);
    printf("  false negatives: %d  (must be 0)\n", fn);
    printf("  false positives: %d / %d = %.4f\n", fp, n_out, observed_fpr);
    printf("  theoretical FPR: %.4f\n", theoretical);
    bloom_free(&b);

    /* Count-Min sketch */
    CountMin c; cm_init(&c, 256, 4);
    /* Heavy hitter: insert 'item 42' a lot. */
    for (int i = 0; i < 1000; ++i) cm_add(&c, 42, 1);
    /* Background noise. */
    for (int i = 0; i < 100; ++i)
        for (int j = 0; j < 10; ++j) cm_add(&c, (uint64_t)i, 1);

    printf("\n== Count-Min sketch (w=256, d=4) ==\n");
    printf("  estimate(42) = %ld  (true count: 1000 + 10 = 1010)\n", cm_estimate(&c, 42));
    printf("  estimate(7)  = %ld  (true count: 10)\n", cm_estimate(&c, 7));
    printf("  estimate(999) = %ld  (true count: 0)\n", cm_estimate(&c, 999));
    cm_free(&c);

    return 0;
}
