/*
 * bloom.h — single-header Bloom filter.
 *
 *   Bloom b; bloom_init(&b, 96000, 7);
 *   bloom_add(&b, key);
 *   if (bloom_contains(&b, key)) ...     // may have false positives
 *   bloom_free(&b);
 *
 * Use bloom_params() to derive (m, k) from target n and FPR.
 */
#ifndef BLOOM_H
#define BLOOM_H

#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <math.h>

typedef struct { uint8_t *bits; size_t m; int k; } Bloom;

static inline uint64_t bloom__mix(uint64_t x) {
    x += 0x9e3779b97f4a7c15ULL;
    x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
    x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
    return x ^ (x >> 31);
}

static inline void bloom_init(Bloom *b, size_t m, int k) {
    b->m = m; b->k = k;
    b->bits = (uint8_t *)calloc((m + 7) / 8, 1);
}

/* Compute optimal m and k for n items and target FPR ε. */
static inline void bloom_params(size_t n, double eps, size_t *m_out, int *k_out) {
    double m = -(double)n * log(eps) / (log(2.0) * log(2.0));
    *m_out = (size_t)(m + 0.5);
    *k_out = (int)(((double)*m_out / n) * log(2.0) + 0.5);
    if (*k_out < 1) *k_out = 1;
}

static inline void bloom_add(Bloom *b, uint64_t x) {
    for (int i = 0; i < b->k; ++i) {
        size_t h = bloom__mix(x + (uint64_t)i * 0xdeadbeefULL) % b->m;
        b->bits[h / 8] |= (uint8_t)(1 << (h & 7));
    }
}

static inline bool bloom_contains(const Bloom *b, uint64_t x) {
    for (int i = 0; i < b->k; ++i) {
        size_t h = bloom__mix(x + (uint64_t)i * 0xdeadbeefULL) % b->m;
        if (!((b->bits[h / 8] >> (h & 7)) & 1)) return false;
    }
    return true;
}

static inline void bloom_free(Bloom *b) { free(b->bits); b->bits = NULL; }

#endif /* BLOOM_H */
