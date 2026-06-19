/* main.c — FNV-1a, splitmix64, tabulation, SipHash-1-3 with avalanche tests. */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <time.h>

/* ============================================================ */
/* FNV-1a                                                        */
/* ============================================================ */
static uint64_t fnv1a(const void *data, size_t n) {
    const uint8_t *p = (const uint8_t *)data;
    uint64_t h = 0xcbf29ce484222325ULL;
    for (size_t i = 0; i < n; ++i) {
        h ^= p[i];
        h *= 0x100000001b3ULL;
    }
    return h;
}

/* ============================================================ */
/* splitmix64                                                    */
/* ============================================================ */
static uint64_t mix64(uint64_t x) {
    x += 0x9e3779b97f4a7c15ULL;
    x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9ULL;
    x = (x ^ (x >> 27)) * 0x94d049bb133111ebULL;
    return x ^ (x >> 31);
}

/* ============================================================ */
/* Tabulation hashing (8 byte slices)                            */
/* ============================================================ */
static uint64_t TAB[8][256];

static void tab_init(uint64_t seed) {
    /* Fill with random using splitmix as PRNG */
    uint64_t s = seed;
    for (int i = 0; i < 8; ++i)
        for (int j = 0; j < 256; ++j) {
            s = mix64(s + j + i * 257);
            TAB[i][j] = s;
        }
}

static uint64_t tab_hash(uint64_t key) {
    uint64_t h = 0;
    for (int i = 0; i < 8; ++i)
        h ^= TAB[i][(key >> (i * 8)) & 0xff];
    return h;
}

/* ============================================================ */
/* SipHash-1-3 (paper-faithful, 8-byte input)                    */
/* ============================================================ */
#define ROTL(x, b) (((x) << (b)) | ((x) >> (64 - (b))))
#define SIPROUND(v0, v1, v2, v3) do { \
    v0 += v1; v1 = ROTL(v1, 13); v1 ^= v0; v0 = ROTL(v0, 32); \
    v2 += v3; v3 = ROTL(v3, 16); v3 ^= v2; \
    v0 += v3; v3 = ROTL(v3, 21); v3 ^= v0; \
    v2 += v1; v1 = ROTL(v1, 17); v1 ^= v2; v2 = ROTL(v2, 32); \
} while (0)

static uint64_t siphash_1_3(const uint8_t *in, size_t inlen, uint64_t k0, uint64_t k1) {
    uint64_t v0 = 0x736f6d6570736575ULL ^ k0;
    uint64_t v1 = 0x646f72616e646f6dULL ^ k1;
    uint64_t v2 = 0x6c7967656e657261ULL ^ k0;
    uint64_t v3 = 0x7465646279746573ULL ^ k1;

    const uint8_t *end = in + (inlen - (inlen % 8));
    size_t left = inlen & 7;
    uint64_t b = ((uint64_t)inlen) << 56;

    for (; in != end; in += 8) {
        uint64_t m;
        memcpy(&m, in, 8);
        v3 ^= m;
        SIPROUND(v0, v1, v2, v3);                /* c = 1 */
        v0 ^= m;
    }
    for (size_t i = 0; i < left; ++i) b |= ((uint64_t)in[i]) << (i * 8);
    v3 ^= b;
    SIPROUND(v0, v1, v2, v3);
    v0 ^= b;
    v2 ^= 0xff;
    SIPROUND(v0, v1, v2, v3);                    /* d = 3 finalization rounds */
    SIPROUND(v0, v1, v2, v3);
    SIPROUND(v0, v1, v2, v3);
    return v0 ^ v1 ^ v2 ^ v3;
}

static uint64_t siphash_u64(uint64_t key, uint64_t k0, uint64_t k1) {
    return siphash_1_3((const uint8_t *)&key, 8, k0, k1);
}

/* ============================================================ */
/* Avalanche test: bits in output flipped when one input bit flips */
/* ============================================================ */
typedef uint64_t (*HashU64)(uint64_t);

static double avalanche_score(HashU64 h, int trials) {
    /* For each input bit, average fraction of output bits flipped. */
    long flips_total = 0;
    for (int t = 0; t < trials; ++t) {
        uint64_t x = mix64((uint64_t)t * 7919 + 0xdeadbeef);
        for (int b = 0; b < 64; ++b) {
            uint64_t y = x ^ (1ULL << b);
            uint64_t d = h(x) ^ h(y);
            flips_total += __builtin_popcountll(d);
        }
    }
    /* trials × 64 bit-flips, each compares 64 output bits.
       Ideal: 50% of output bits flip → flips/total_bits = 0.5 */
    double total_bits = (double)trials * 64.0 * 64.0;
    return flips_total / total_bits;
}

static uint64_t bad_mul31(uint64_t x) { return x * 31; }
static uint64_t fnv1a_u64(uint64_t x) { return fnv1a(&x, sizeof(x)); }
static uint64_t mix64_u64(uint64_t x) { return mix64(x); }
static uint64_t tab_u64(uint64_t x)   { return tab_hash(x); }
static uint64_t siphash_default(uint64_t x) {
    return siphash_u64(x, 0x0706050403020100ULL, 0x0f0e0d0c0b0a0908ULL);
}

/* ============================================================ */
/* Bench throughput                                              */
/* ============================================================ */
static double now(void) {
    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

static void bench(const char *label, HashU64 h, int n) {
    double t = now();
    uint64_t sink = 0;
    for (int i = 0; i < n; ++i) sink ^= h((uint64_t)i * 0x9e3779b97f4a7c15ULL);
    double dt = now() - t;
    fprintf(stderr, "  (sink=%lx)\n", (unsigned long)sink);    /* defeat DCE */
    printf("  %-22s  %.1f ns/hash  (%.1f Mhash/s)\n", label, dt * 1e9 / n, n / 1e6 / dt);
}

int main(void) {
    tab_init(0x12345678);

    printf("== Avalanche scores (ideal 0.500) ==\n");
    printf("  bad x*31          : %.3f\n", avalanche_score(bad_mul31, 1000));
    printf("  FNV-1a            : %.3f\n", avalanche_score(fnv1a_u64, 1000));
    printf("  splitmix64        : %.3f\n", avalanche_score(mix64_u64, 1000));
    printf("  Tabulation        : %.3f\n", avalanche_score(tab_u64, 1000));
    printf("  SipHash-1-3       : %.3f\n", avalanche_score(siphash_default, 1000));

    printf("\n== Throughput (8-byte input, 10M iterations) ==\n");
    const int N = 10000000;
    bench("FNV-1a (8B)",    fnv1a_u64, N);
    bench("splitmix64",     mix64_u64, N);
    bench("Tabulation",     tab_u64,   N);
    bench("SipHash-1-3 (8B)", siphash_default, N);

    return 0;
}
