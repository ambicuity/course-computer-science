#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <limits.h>

#define MAX_PAGES 256

/* ── Optimal (Belady's) ──────────────────────────────── */

int optimal_replace(int pages[], int n, int frames) {
    int *mem = malloc(frames * sizeof(int));
    int faults = 0;

    for (int i = 0; i < frames; i++) mem[i] = -1;

    for (int i = 0; i < n; i++) {
        bool hit = false;
        for (int j = 0; j < frames; j++) {
            if (mem[j] == pages[i]) { hit = true; break; }
        }
        if (hit) continue;

        /* Fault */
        faults++;
        int empty = -1;
        for (int j = 0; j < frames; j++) {
            if (mem[j] == -1) { empty = j; break; }
        }
        if (empty >= 0) {
            mem[empty] = pages[i];
            continue;
        }

        /* Evict page used furthest in future */
        int farthest = -1, evict = 0;
        for (int j = 0; j < frames; j++) {
            int next_use = INT_MAX;
            for (int k = i + 1; k < n; k++) {
                if (pages[k] == mem[j]) { next_use = k; break; }
            }
            if (next_use > farthest) {
                farthest = next_use;
                evict = j;
            }
        }
        mem[evict] = pages[i];
    }
    free(mem);
    return faults;
}

/* ── FIFO ─────────────────────────────────────────────── */

int fifo_replace(int pages[], int n, int frames) {
    int *mem = malloc(frames * sizeof(int));
    int head = 0, faults = 0;

    for (int i = 0; i < frames; i++) mem[i] = -1;

    for (int i = 0; i < n; i++) {
        bool hit = false;
        for (int j = 0; j < frames; j++) {
            if (mem[j] == pages[i]) { hit = true; break; }
        }
        if (hit) continue;

        faults++;
        mem[head] = pages[i];
        head = (head + 1) % frames;
    }
    free(mem);
    return faults;
}

/* ── LRU ──────────────────────────────────────────────── */

int lru_replace(int pages[], int n, int frames) {
    int *mem = malloc(frames * sizeof(int));
    int *last_used = malloc(frames * sizeof(int));
    int faults = 0;

    for (int i = 0; i < frames; i++) {
        mem[i] = -1;
        last_used[i] = -1;
    }

    for (int i = 0; i < n; i++) {
        bool hit = false;
        for (int j = 0; j < frames; j++) {
            if (mem[j] == pages[i]) {
                hit = true;
                last_used[j] = i;
                break;
            }
        }
        if (hit) continue;

        faults++;
        int empty = -1;
        for (int j = 0; j < frames; j++) {
            if (mem[j] == -1) { empty = j; break; }
        }
        if (empty >= 0) {
            mem[empty] = pages[i];
            last_used[empty] = i;
            continue;
        }

        /* Evict least recently used */
        int lru_idx = 0, lru_time = last_used[0];
        for (int j = 1; j < frames; j++) {
            if (last_used[j] < lru_time) {
                lru_time = last_used[j];
                lru_idx = j;
            }
        }
        mem[lru_idx] = pages[i];
        last_used[lru_idx] = i;
    }
    free(mem);
    free(last_used);
    return faults;
}

/* ── Clock ────────────────────────────────────────────── */

int clock_replace(int pages[], int n, int frames) {
    int *mem = malloc(frames * sizeof(int));
    bool *ref = malloc(frames * sizeof(bool));
    int hand = 0, faults = 0;

    for (int i = 0; i < frames; i++) {
        mem[i] = -1;
        ref[i] = false;
    }

    for (int i = 0; i < n; i++) {
        bool hit = false;
        for (int j = 0; j < frames; j++) {
            if (mem[j] == pages[i]) {
                hit = true;
                ref[j] = true;
                break;
            }
        }
        if (hit) continue;

        faults++;
        int empty = -1;
        for (int j = 0; j < frames; j++) {
            if (mem[j] == -1) { empty = j; break; }
        }
        if (empty >= 0) {
            mem[empty] = pages[i];
            ref[empty] = true;
            continue;
        }

        /* Clock sweep */
        while (ref[hand]) {
            ref[hand] = false;
            hand = (hand + 1) % frames;
        }
        mem[hand] = pages[i];
        ref[hand] = true;
        hand = (hand + 1) % frames;
    }
    free(mem);
    free(ref);
    return faults;
}

/* ── ARC (simplified) ─────────────────────────────────── */

typedef struct {
    int page;
    int age;
} ArcEntry;

int arc_replace(int pages[], int n, int frames) {
    int c = frames;
    int p = 0;
    int faults = 0;

    /* T1, T2: cached pages. B1, B2: ghost entries */
    ArcEntry *t1 = calloc(c * 2, sizeof(ArcEntry));
    ArcEntry *t2 = calloc(c * 2, sizeof(ArcEntry));
    ArcEntry *b1 = calloc(c * 2, sizeof(ArcEntry));
    ArcEntry *b2 = calloc(c * 2, sizeof(ArcEntry));
    int t1n = 0, t2n = 0, b1n = 0, b2n = 0;
    int tick = 0;

    #define FIND(arr, n, pg) ({ int _r = -1; for(int _i=0;_i<(n);_i++){if((arr)[_i].page==(pg)){_r=_i;break;}} _r; })
    #define REMOVE(arr, n, idx) do { (arr)[idx] = (arr)[--(n)]; } while(0)
    #define INSERT(arr, n, pg) do { (arr)[(n)].page = (pg); (arr)[(n)++].age = tick++; } while(0)

    for (int i = 0; i < n; i++) {
        int pg = pages[i];
        tick++;

        /* Hit in T1: move to T2 */
        int idx = FIND(t1, t1n, pg);
        if (idx >= 0) {
            REMOVE(t1, t1n, idx);
            INSERT(t2, t2n, pg);
            continue;
        }

        /* Hit in T2: update age */
        idx = FIND(t2, t2n, pg);
        if (idx >= 0) {
            t2[idx].age = tick;
            continue;
        }

        /* Miss */
        faults++;

        /* Check ghost lists */
        idx = FIND(b1, b1n, pg);
        if (idx >= 0) {
            /* Ghost recency hit: increase p */
            int delta = (b2n >= b1n) ? 1 : b1n / (b2n > 0 ? b2n : 1);
            p = (p + delta < c) ? p + delta : c;
            REMOVE(b1, b1n, idx);
            INSERT(t2, t2n, pg);
            continue;
        }

        idx = FIND(b2, b2n, pg);
        if (idx >= 0) {
            /* Ghost frequency hit: decrease p */
            int delta = (b1n >= b2n) ? 1 : b2n / (b1n > 0 ? b1n : 1);
            p = (p > delta) ? p - delta : 0;
            REMOVE(b2, b2n, idx);
            INSERT(t2, t2n, pg);
            continue;
        }

        /* True miss */
        int l1 = t1n + b1n;
        if (l1 == c) {
            if (t1n < c) {
                /* Evict from B1 */
                int oldest = 0;
                for (int j = 1; j < b1n; j++)
                    if (b1[j].age < b1[oldest].age) oldest = j;
                REMOVE(b1, b1n, oldest);
            } else {
                /* Evict from T1 */
                int oldest = 0;
                for (int j = 1; j < t1n; j++)
                    if (t1[j].age < t1[oldest].age) oldest = j;
                REMOVE(t1, t1n, oldest);
            }
        } else {
            int total = t1n + t2n + b1n + b2n;
            if (total >= c) {
                if (t1n + t2n + b1n + b2n >= 2 * c) {
                    if (b2n > 0) {
                        int oldest = 0;
                        for (int j = 1; j < b2n; j++)
                            if (b2[j].age < b2[oldest].age) oldest = j;
                        REMOVE(b2, b2n, oldest);
                    } else {
                        int oldest = 0;
                        for (int j = 1; j < b1n; j++)
                            if (b1[j].age < b1[oldest].age) oldest = j;
                        REMOVE(b1, b1n, oldest);
                    }
                }
            }
        }

        /* Evict from T1 if cache full */
        if (t1n + t2n >= c) {
            if (t1n > 0 && t1n > p) {
                int oldest = 0;
                for (int j = 1; j < t1n; j++)
                    if (t1[j].age < t1[oldest].age) oldest = j;
                INSERT(b1, b1n, t1[oldest].page);
                REMOVE(t1, t1n, oldest);
            } else if (t2n > 0) {
                int oldest = 0;
                for (int j = 1; j < t2n; j++)
                    if (t2[j].age < t2[oldest].age) oldest = j;
                INSERT(b2, b2n, t2[oldest].page);
                REMOVE(t2, t2n, oldest);
            }
        }

        INSERT(t1, t1n, pg);
    }

    free(t1); free(t2); free(b1); free(b2);
    #undef FIND
    #undef REMOVE
    #undef INSERT
    return faults;
}

/* ── Benchmark ────────────────────────────────────────── */

static void run_benchmark(const char *label, int pages[], int n, int frames) {
    printf("%-12s frames=%d  ref_len=%d\n", label, frames, n);
    printf("  Optimal:  %d faults\n", optimal_replace(pages, n, frames));
    printf("  FIFO:     %d faults\n", fifo_replace(pages, n, frames));
    printf("  LRU:      %d faults\n", lru_replace(pages, n, frames));
    printf("  Clock:    %d faults\n", clock_replace(pages, n, frames));
    printf("  ARC:      %d faults\n", arc_replace(pages, n, frames));
    printf("\n");
}

/* ── Main ─────────────────────────────────────────────── */

int main(void) {
    printf("=== Page Replacement Simulator ===\n\n");

    /* Classic test */
    int ref1[] = {1,2,3,4,1,2,5,1,2,3,4,5};
    int n1 = sizeof(ref1) / sizeof(ref1[0]);
    run_benchmark("Classic", ref1, n1, 3);
    run_benchmark("Classic", ref1, n1, 4);

    /* Belady's anomaly demonstration */
    printf("Belady's Anomaly (FIFO):\n");
    int ref2[] = {1,2,3,4,1,2,5,1,2,3,4,5};
    int n2 = sizeof(ref2) / sizeof(ref2[0]);
    printf("  3 frames: %d faults\n", fifo_replace(ref2, n2, 3));
    printf("  4 frames: %d faults\n", fifo_replace(ref2, n2, 4));
    printf("\n");

    /* Locality test */
    int ref3[] = {1,1,1,2,2,2,3,3,3,1,1,4,4,4,2,2,5,5,5,1,1,1};
    int n3 = sizeof(ref3) / sizeof(ref3[0]);
    run_benchmark("Locality", ref3, n3, 3);

    /* Scan test */
    int ref4[50];
    for (int i = 0; i < 50; i++) ref4[i] = i % 10;
    run_benchmark("Scan(10)", ref4, 50, 4);

    return 0;
}
