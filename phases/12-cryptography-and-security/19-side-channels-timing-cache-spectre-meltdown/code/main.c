/*
 * Side-Channels — Timing, Cache, Spectre/Meltdown
 * Phase 12 — Cryptography & Security
 *
 * Compile: gcc -O2 -o sidechannel main.c
 * Run:     ./sidechannel
 *
 * Demonstrates:
 *   1. Variable-time memcmp timing attack (secret byte-by-byte recovery)
 *   2. Constant-time comparison (timing independent of mismatch position)
 *   3. Flush+Reload cache side-channel (detect victim memory access)
 *   4. Spectre v1 gadget (speculative bounds-check bypass, conceptual)
 *
 * Requires x86-64 with RDTSC and CLFLUSH support.
 */

#define _GNU_SOURCE
#include <stdio.h>
#include <stdint.h>
#include <inttypes.h>
#include <string.h>
#include <stdlib.h>
#include <time.h>

#if defined(__x86_64__) || defined(__i386__)
#include <x86intrin.h>
#else
#error "This lesson requires x86-64 with RDTSC and CLFLUSH intrinsics."
#error "Compile on an Intel or AMD x86-64 machine: gcc -O2 -o sidechannel main.c"
#endif

#define CACHE_LINE_SIZE 64
#define ARRAY_SIZE 256
#define SECRET_LEN 16
#define TRIALS_PER_BYTE 5000
#define WARMUP 1000

static volatile int sink;

/* ------------------------------------------------------------------ */
/*  Cycle-accurate timing helpers                                      */
/* ------------------------------------------------------------------ */

static inline uint64_t tick_begin(void)
{
    _mm_lfence();
    uint64_t t = __rdtsc();
    _mm_lfence();
    return t;
}

static inline uint64_t tick_end(void)
{
    _mm_lfence();
    uint64_t t = __rdtsc();
    _mm_lfence();
    return t;
}

/* ------------------------------------------------------------------ */
/*  Part 1 — Variable-Time Timing Attack                               */
/* ------------------------------------------------------------------ */

static int naive_memcmp(const void *a, const void *b, size_t len)
{
    const uint8_t *pa = (const uint8_t *)a;
    const uint8_t *pb = (const uint8_t *)b;
    for (size_t i = 0; i < len; i++) {
        if (pa[i] != pb[i])
            return -1;
    }
    return 0;
}

static uint64_t time_naive(const uint8_t *a, const uint8_t *b, size_t len)
{
    uint64_t start = tick_begin();
    sink = naive_memcmp(a, b, len);
    return tick_end() - start;
}

static void show_timing_gradient(void)
{
    uint8_t ref[SECRET_LEN];
    uint8_t mod[SECRET_LEN];
    memset(ref, 0x42, SECRET_LEN);
    memset(mod, 0x42, SECRET_LEN);

    printf("Timing gradient (naive memcmp, cycles, avg of 10000):\n");
    for (int pos = 0; pos < SECRET_LEN; pos += 4) {
        mod[pos] = 0xFF;
        uint64_t sum = 0;
        for (int t = 0; t < 10000; t++)
            sum += time_naive(ref, mod, SECRET_LEN);
        printf("  mismatch at [%2d]: %5" PRIu64 " cycles\n", pos, (uint64_t)(sum / 10000));
        mod[pos] = 0x42;
    }
    printf("\n");
}

static void recover_secret(uint8_t *secret, size_t len)
{
    uint8_t guess[SECRET_LEN];
    memset(guess, 0, SECRET_LEN);

    printf("Recovering %zu-byte secret byte-by-byte...\n\n", len);
    for (size_t pos = 0; pos < len; pos++) {
        uint64_t best_time = 0;
        uint8_t best_byte = 0;

        for (int cand = 0; cand < 256; cand++) {
            guess[pos] = (uint8_t)cand;

            uint64_t sum = 0;
            for (int t = 0; t < TRIALS_PER_BYTE; t++) {
                sum += time_naive(secret, guess, SECRET_LEN);
                _mm_clflush(guess);
                _mm_clflush(secret);
                _mm_lfence();
            }

            uint64_t avg = sum / TRIALS_PER_BYTE;
            if (avg > best_time) {
                best_time = avg;
                best_byte = (uint8_t)cand;
            }
        }

        guess[pos] = best_byte;
        char ok = (best_byte == secret[pos]) ? 'Y' : 'n';
        printf("  pos %2zu: guessed 0x%02x  actual 0x%02x  %c  (best= %5" PRIu64 " cyc)\n",
               pos, best_byte, secret[pos], ok, (uint64_t)(best_time / TRIALS_PER_BYTE));
    }
}

static void demo_timing_attack(void)
{
    uint8_t secret[SECRET_LEN];
    srand((unsigned)time(NULL));
    for (size_t i = 0; i < SECRET_LEN; i++)
        secret[i] = (uint8_t)(rand() & 0xFF);

    printf("============================================\n");
    printf("  Step 1 — Timing Attack\n");
    printf("============================================\n\n");
    printf("Secret: ");
    for (size_t i = 0; i < SECRET_LEN; i++)
        printf("%02x", secret[i]);
    printf("\n\n");

    show_timing_gradient();
    recover_secret(secret, SECRET_LEN);
    printf("\n");
}

/* ------------------------------------------------------------------ */
/*  Part 2 — Constant-Time Comparison                                  */
/* ------------------------------------------------------------------ */

static int constant_time_memcmp(const void *a, const void *b, size_t len)
{
    const uint8_t *pa = (const uint8_t *)a;
    const uint8_t *pb = (const uint8_t *)b;
    uint8_t diff = 0;
    for (size_t i = 0; i < len; i++)
        diff |= (pa[i] ^ pb[i]);
    return (int)diff;
}

static uint64_t time_constant(const uint8_t *a, const uint8_t *b, size_t len)
{
    uint64_t start = tick_begin();
    sink = constant_time_memcmp(a, b, len);
    return tick_end() - start;
}

static void demo_constant_time(void)
{
    uint8_t ref[SECRET_LEN];
    uint8_t mod[SECRET_LEN];
    memset(ref, 0x42, SECRET_LEN);
    memset(mod, 0x42, SECRET_LEN);

    printf("============================================\n");
    printf("  Step 2 — Constant-Time Comparison\n");
    printf("============================================\n\n");

    uint8_t x[4] = {1, 2, 3, 4};
    uint8_t y[4] = {1, 2, 3, 4};
    uint8_t z[4] = {1, 2, 3, 5};
    printf("Correctness: equal(%d,%d)=%d   unequal(%d,%d)=%d\n\n",
           4, 4, constant_time_memcmp(x, y, 4),
           4, 4, constant_time_memcmp(x, z, 4));

    printf("Timing gradient (constant-time memcmp, cycles, avg of 10000):\n");
    for (int pos = 0; pos < SECRET_LEN; pos += 4) {
        mod[pos] = 0xFF;
        uint64_t sum = 0;
        for (int t = 0; t < 10000; t++)
            sum += time_constant(ref, mod, SECRET_LEN);
        printf("  mismatch at [%2d]: %5" PRIu64 " cycles\n", pos, (uint64_t)(sum / 10000));
        mod[pos] = 0x42;
    }
    printf("\n");
}

/* ------------------------------------------------------------------ */
/*  Part 3 — Cache Side-Channel (Flush+Reload)                         */
/* ------------------------------------------------------------------ */

static volatile uint8_t shared_array[ARRAY_SIZE * CACHE_LINE_SIZE];

__attribute__((noinline))
static void victim(int secret_byte)
{
    sink = (int)shared_array[secret_byte * CACHE_LINE_SIZE];
}

static int spy(void)
{
    uint64_t best_time = UINT64_MAX;
    int best_index = 0;

    for (int i = 0; i < ARRAY_SIZE; i++) {
        uint64_t start = tick_begin();
        sink = (int)shared_array[i * CACHE_LINE_SIZE];
        uint64_t elapsed = tick_end() - start;

        if (elapsed < best_time) {
            best_time = elapsed;
            best_index = i;
        }
    }
    return best_index;
}

static void calibrate_cache(void)
{
    int mid = ARRAY_SIZE / 2;

    /* Cold miss */
    _mm_clflush((void *)&shared_array[mid * CACHE_LINE_SIZE]);
    _mm_lfence();
    uint64_t t1 = tick_begin();
    sink = (int)shared_array[mid * CACHE_LINE_SIZE];
    uint64_t miss = tick_end() - t1;

    /* Hot hit — same line is now in L1 */
    _mm_lfence();
    uint64_t t2 = tick_begin();
    sink = (int)shared_array[mid * CACHE_LINE_SIZE];
    uint64_t hit = tick_end() - t2;

    printf("  Cache miss:  %4" PRIu64 " cycles\n", miss);
    printf("  Cache hit:   %4" PRIu64 " cycles\n", hit);
    printf("  Differential: %4" PRIu64 " cycles (%.1fx)\n\n",
           miss - hit, (double)miss / (double)hit);
}

static void demo_cache_side_channel(void)
{
    printf("============================================\n");
    printf("  Step 3 — Flush+Reload Cache Attack\n");
    printf("============================================\n\n");

    memset((void *)shared_array, 1, sizeof(shared_array));
    _mm_mfence();

    printf("Calibrating cache hit/miss timing:\n");
    calibrate_cache();

    int trials = 25;
    int correct = 0;

    printf("Flush+Reload attack (%d trials):\n", trials);
    for (int t = 0; t < trials; t++) {
        int secret_byte = rand() % ARRAY_SIZE;

        /* Spy flushes every cache line */
        for (int i = 0; i < ARRAY_SIZE; i++)
            _mm_clflush((void *)&shared_array[i * CACHE_LINE_SIZE]);
        _mm_mfence();
        _mm_lfence();

        /* Victim accesses one line */
        victim(secret_byte);
        _mm_lfence();

        /* Spy probes: which line is cached? */
        int detected = spy();

        if (detected == secret_byte)
            correct++;
        printf("  trial %2d: victim=%3d  spy=%3d  %s\n",
               t, secret_byte, detected,
               detected == secret_byte ? "OK" : "miss");
    }
    printf("\nAccuracy: %d/%d (%d%%)\n\n",
           correct, trials, correct * 100 / trials);
}

/* ------------------------------------------------------------------ */
/*  Part 4 — Spectre v1 (Conceptual)                                   */
/* ------------------------------------------------------------------ */

static size_t array1_size = 16;
static uint8_t array1[32];
static volatile uint8_t array2[ARRAY_SIZE * CACHE_LINE_SIZE];

__attribute__((noinline))
static void spectre_victim(size_t x)
{
    if (x < array1_size)
        sink = (int)array2[array1[x] * CACHE_LINE_SIZE];
}

static void train_predictor(void)
{
    for (int i = 0; i < 2000; i++)
        spectre_victim(0);
}

static int spectre_probe(void)
{
    uint64_t best_time = UINT64_MAX;
    int best_index = 0;

    for (int i = 0; i < ARRAY_SIZE; i++) {
        uint64_t start = tick_begin();
        sink = (int)array2[i * CACHE_LINE_SIZE];
        uint64_t elapsed = tick_end() - start;

        if (elapsed < best_time) {
            best_time = elapsed;
            best_index = i;
        }
    }
    return best_index;
}

static void demo_spectre(void)
{
    printf("============================================\n");
    printf("  Step 4 — Spectre v1 (Conceptual)\n");
    printf("============================================\n\n");

    for (int i = 0; i < 16; i++)
        array1[i] = (uint8_t)i;
    array1[16] = 0xAB;
    array1[17] = 0xCD;
    array1[18] = 0xEF;

    memset((void *)array2, 1, sizeof(array2));
    _mm_mfence();

    printf("  array1 accessible: [0..15]\n");
    printf("  array1[16] (secret): 0x%02x\n", array1[16]);
    printf("  Probe buffer: array2[0..255] x 64B cache lines\n\n");

    printf("Training branch predictor ...\n");
    train_predictor();

    printf("Flushing array1_size, evicting array2 ...\n");
    _mm_clflush(&array1_size);
    _mm_lfence();
    for (int i = 0; i < ARRAY_SIZE; i++)
        _mm_clflush((void *)&array2[i * CACHE_LINE_SIZE]);
    _mm_mfence();
    _mm_lfence();

    printf("Calling spectre_victim(16) — out-of-bounds index ...\n");
    spectre_victim(16);
    _mm_lfence();

    int cached = spectre_probe();
    printf("\nFastest array2 index after probe: %d\n", cached);
    printf("Expected (array1[16]):               %d\n", array1[16]);

    if (cached == (int)array1[16])
        printf("\n  Spectre-like leak detected! The cached index matches array1[16].\n");
    else
        printf("\n  No leak detected (expected on modern CPUs with mitigations).\n"
               "  The gadget structure is correct; on older pre-2018 hardware\n"
               "  without IBRS/KPTI this would reveal array1[16].\n");

    printf("\n");
}

/* ------------------------------------------------------------------ */
/*  Main                                                               */
/* ------------------------------------------------------------------ */

int main(void)
{
    printf("============================================\n");
    printf("  Side-Channel Analysis Toolkit\n");
    printf("  Phase 12: Cryptography & Security\n");
    printf("  gcc -O2 -o sidechannel main.c && ./sidechannel\n");
    printf("============================================\n\n");

    demo_timing_attack();
    demo_constant_time();
    demo_cache_side_channel();
    demo_spectre();

    printf("============================================\n");
    printf("  All demonstrations complete.\n");
    printf("============================================\n");
    return 0;
}
