#include <stdint.h>
#include <stdio.h>
#include <string.h>

static uint32_t rotr(uint32_t x, int n) {
    return (x >> n) | (x << (32 - n));
}

static uint32_t shr(uint32_t x, int n) {
    return x >> n;
}

#define CH(x, y, z)  (((x) & (y)) ^ ((~(x)) & (z)))
#define MAJ(x, y, z) (((x) & (y)) ^ ((x) & (z)) ^ ((y) & (z)))
#define SIGMA0(x)     (rotr((x), 2) ^ rotr((x), 13) ^ rotr((x), 22))
#define SIGMA1(x)     (rotr((x), 6) ^ rotr((x), 11) ^ rotr((x), 25))
#define sigma0(x)     (rotr((x), 7) ^ rotr((x), 18) ^ shr((x), 3))
#define sigma1(x)     (rotr((x), 17) ^ rotr((x), 19) ^ shr((x), 10))

static const uint32_t K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

typedef struct {
    uint32_t state[8];
    uint8_t  buffer[64];
    uint64_t bitlen;
    uint64_t buflen;
} sha256_ctx;

static void sha256_init(sha256_ctx *ctx) {
    ctx->state[0] = 0x6a09e667;
    ctx->state[1] = 0xbb67ae85;
    ctx->state[2] = 0x3c6ef372;
    ctx->state[3] = 0xa54ff53a;
    ctx->state[4] = 0x510e527f;
    ctx->state[5] = 0x9b05688c;
    ctx->state[6] = 0x1f83d9ab;
    ctx->state[7] = 0x5be0cd19;
    ctx->bitlen = 0;
    ctx->buflen = 0;
}

static void sha256_transform(sha256_ctx *ctx, const uint8_t block[64]) {
    uint32_t W[64];
    uint32_t a, b, c, d, e, f, g, h, t1, t2;
    int i;

    for (i = 0; i < 16; i++) {
        W[i] = ((uint32_t)block[i * 4] << 24)
             | ((uint32_t)block[i * 4 + 1] << 16)
             | ((uint32_t)block[i * 4 + 2] << 8)
             | ((uint32_t)block[i * 4 + 3]);
    }
    for (i = 16; i < 64; i++) {
        W[i] = sigma1(W[i - 2]) + W[i - 7] + sigma0(W[i - 15]) + W[i - 16];
    }

    a = ctx->state[0]; b = ctx->state[1];
    c = ctx->state[2]; d = ctx->state[3];
    e = ctx->state[4]; f = ctx->state[5];
    g = ctx->state[6]; h = ctx->state[7];

    for (i = 0; i < 64; i++) {
        t1 = h + SIGMA1(e) + CH(e, f, g) + K[i] + W[i];
        t2 = SIGMA0(a) + MAJ(a, b, c);
        h = g;
        g = f;
        f = e;
        e = d + t1;
        d = c;
        c = b;
        b = a;
        a = t1 + t2;
    }

    ctx->state[0] += a; ctx->state[1] += b;
    ctx->state[2] += c; ctx->state[3] += d;
    ctx->state[4] += e; ctx->state[5] += f;
    ctx->state[6] += g; ctx->state[7] += h;
}

static void sha256_update(sha256_ctx *ctx, const uint8_t *data, size_t len) {
    size_t i;
    for (i = 0; i < len; i++) {
        ctx->buffer[ctx->buflen++] = data[i];
        if (ctx->buflen == 64) {
            sha256_transform(ctx, ctx->buffer);
            ctx->bitlen += 512;
            ctx->buflen = 0;
        }
    }
}

static void sha256_final(sha256_ctx *ctx, uint8_t hash[32]) {
    uint64_t bitlen = ctx->bitlen + ctx->buflen * 8;
    uint64_t i = ctx->buflen;

    ctx->buffer[i++] = 0x80;

    if (i > 56) {
        while (i < 64) ctx->buffer[i++] = 0;
        sha256_transform(ctx, ctx->buffer);
        i = 0;
    }
    while (i < 56) ctx->buffer[i++] = 0;

    ctx->buffer[56] = (uint8_t)(bitlen >> 56);
    ctx->buffer[57] = (uint8_t)(bitlen >> 48);
    ctx->buffer[58] = (uint8_t)(bitlen >> 40);
    ctx->buffer[59] = (uint8_t)(bitlen >> 32);
    ctx->buffer[60] = (uint8_t)(bitlen >> 24);
    ctx->buffer[61] = (uint8_t)(bitlen >> 16);
    ctx->buffer[62] = (uint8_t)(bitlen >> 8);
    ctx->buffer[63] = (uint8_t)(bitlen);
    sha256_transform(ctx, ctx->buffer);

    for (i = 0; i < 8; i++) {
        hash[i * 4]     = (uint8_t)(ctx->state[i] >> 24);
        hash[i * 4 + 1] = (uint8_t)(ctx->state[i] >> 16);
        hash[i * 4 + 2] = (uint8_t)(ctx->state[i] >> 8);
        hash[i * 4 + 3] = (uint8_t)(ctx->state[i]);
    }
}

static void sha256(const uint8_t *data, size_t len, uint8_t hash[32]) {
    sha256_ctx ctx;
    sha256_init(&ctx);
    sha256_update(&ctx, data, len);
    sha256_final(&ctx, hash);
}

static void print_hash(const uint8_t hash[32]) {
    for (int i = 0; i < 32; i++) {
        printf("%02x", hash[i]);
    }
    printf("\n");
}

static int check_hash(const uint8_t hash[32], const char *expected_hex) {
    for (int i = 0; i < 32; i++) {
        unsigned int byte;
        sscanf(expected_hex + 2 * i, "%02x", &byte);
        if (hash[i] != (uint8_t)byte) return 0;
    }
    return 1;
}

int main(void) {
    uint8_t hash[32];
    int passes = 0;

    printf("SHA-256 Test Vectors\n");
    printf("====================\n\n");

    sha256(NULL, 0, hash);
    printf("SHA-256(\"\")    = ");
    print_hash(hash);
    if (check_hash(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")) {
        printf("[PASS] empty string\n");
        passes++;
    } else {
        printf("[FAIL] empty string\n");
    }

    sha256((const uint8_t *)"abc", 3, hash);
    printf("SHA-256(\"abc\") = ");
    print_hash(hash);
    if (check_hash(hash, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")) {
        printf("[PASS] \"abc\"\n");
        passes++;
    } else {
        printf("[FAIL] \"abc\"\n");
    }

    const char *long_msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    sha256((const uint8_t *)long_msg, strlen(long_msg), hash);
    printf("SHA-256(448-bit) = ");
    print_hash(hash);
    if (check_hash(hash, "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1")) {
        printf("[PASS] 448-bit message\n");
        passes++;
    } else {
        printf("[FAIL] 448-bit message\n");
    }

    printf("\n%d/3 test vectors passed\n", passes);
    return passes == 3 ? 0 : 1;
}