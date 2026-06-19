/* main.c — profiler target.
 *
 * Three functions, three different cost profiles:
 *   hot_inner_loop:  small, called many times — most samples should land here
 *   medium_work:     moderate
 *   light_work:      tiny
 *
 * Build for sampling profilers: gcc -O2 -g main.c -o profile-target
 * Build for valgrind clarity:    gcc -O0 -g main.c -o profile-target-dbg
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define N_OUTER  10000
#define N_INNER  10000

/* Opaque the pointer through inline asm so the compiler can't reason about it.
 * Then strlen() really executes every iteration — that's the point. */
static inline const char *opaque(const char *p) {
#if defined(__GNUC__) || defined(__clang__)
    __asm__ volatile("" : "+r"(p) ::);   /* tells compiler p is "unknown" */
#endif
    return p;
}

static int hot_inner_loop(const char *s) {
    int sum = 0;
    for (int i = 0; i < N_INNER; ++i) {
        sum += (int)strlen(opaque(s)) ^ i;
    }
    return sum;
}

static int medium_work(int x) {
    int s = 0;
    for (int i = 0; i < 500; ++i) {
        s += (x ^ i) * (i + 1);
    }
    return s;
}

static void light_work(int *acc) {
    *acc = (*acc * 31 + 7) & 0xFFFFFF;
}

int main(void) {
    /* A long string to make strlen non-trivial. */
    char buf[1024];
    memset(buf, 'a', sizeof(buf) - 1);
    buf[sizeof(buf) - 1] = '\0';

    long total = 0;
    int acc = 1;

    struct timespec t0, t1;
    clock_gettime(CLOCK_MONOTONIC, &t0);

    for (int i = 0; i < N_OUTER; ++i) {
        total += hot_inner_loop(buf);
        total += medium_work(i);
        light_work(&acc);
    }

    clock_gettime(CLOCK_MONOTONIC, &t1);

    double elapsed = (t1.tv_sec - t0.tv_sec) + (t1.tv_nsec - t0.tv_nsec) / 1e9;

    printf("total=%ld acc=%d elapsed=%.3fs\n", total, acc, elapsed);
    return 0;
}
