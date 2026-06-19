/*
 * siphash.h — single-header SipHash-1-3 / SipHash-2-4.
 *
 * Usage:
 *     uint64_t k0 = ..., k1 = ...;        // 128-bit secret key
 *     uint64_t h = siphash13(msg, len, k0, k1);
 *     uint64_t h2 = siphash24(msg, len, k0, k1);
 *
 * SipHash-1-3 (c=1, d=3): hash-table default, fastest.
 * SipHash-2-4 (c=2, d=4): more conservative, used in Python and OpenBSD.
 *
 * License: MIT (rewrite of public-domain reference).
 */
#ifndef SIPHASH_H
#define SIPHASH_H

#include <stdint.h>
#include <stddef.h>
#include <string.h>

#define SIPHASH__ROTL(x, b) (((x) << (b)) | ((x) >> (64 - (b))))
#define SIPHASH__ROUND(v0, v1, v2, v3) do {              \
    v0 += v1; v1 = SIPHASH__ROTL(v1, 13); v1 ^= v0; v0 = SIPHASH__ROTL(v0, 32); \
    v2 += v3; v3 = SIPHASH__ROTL(v3, 16); v3 ^= v2;       \
    v0 += v3; v3 = SIPHASH__ROTL(v3, 21); v3 ^= v0;       \
    v2 += v1; v1 = SIPHASH__ROTL(v1, 17); v1 ^= v2; v2 = SIPHASH__ROTL(v2, 32); \
} while (0)

static inline uint64_t siphash__core(int c, int d, const uint8_t *in, size_t inlen,
                                     uint64_t k0, uint64_t k1) {
    uint64_t v0 = 0x736f6d6570736575ULL ^ k0;
    uint64_t v1 = 0x646f72616e646f6dULL ^ k1;
    uint64_t v2 = 0x6c7967656e657261ULL ^ k0;
    uint64_t v3 = 0x7465646279746573ULL ^ k1;
    const uint8_t *end = in + (inlen - (inlen % 8));
    size_t left = inlen & 7;
    uint64_t b = ((uint64_t)inlen) << 56;
    for (; in != end; in += 8) {
        uint64_t m; memcpy(&m, in, 8);
        v3 ^= m;
        for (int i = 0; i < c; ++i) SIPHASH__ROUND(v0, v1, v2, v3);
        v0 ^= m;
    }
    for (size_t i = 0; i < left; ++i) b |= ((uint64_t)in[i]) << (i * 8);
    v3 ^= b;
    for (int i = 0; i < c; ++i) SIPHASH__ROUND(v0, v1, v2, v3);
    v0 ^= b;
    v2 ^= 0xff;
    for (int i = 0; i < d; ++i) SIPHASH__ROUND(v0, v1, v2, v3);
    return v0 ^ v1 ^ v2 ^ v3;
}

static inline uint64_t siphash13(const void *in, size_t inlen, uint64_t k0, uint64_t k1) {
    return siphash__core(1, 3, (const uint8_t *)in, inlen, k0, k1);
}

static inline uint64_t siphash24(const void *in, size_t inlen, uint64_t k0, uint64_t k1) {
    return siphash__core(2, 4, (const uint8_t *)in, inlen, k0, k1);
}

#endif /* SIPHASH_H */
