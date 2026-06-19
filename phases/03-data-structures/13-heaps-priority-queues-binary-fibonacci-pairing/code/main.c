/* main.c — Binary min-heap (array-backed) + my_heapsort + heap vs qsort bench. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <assert.h>

typedef struct {
    int    *a;
    size_t  len, cap;
} Heap;

static void swap(int *x, int *y) { int t = *x; *x = *y; *y = t; }

static void heap_init(Heap *h, size_t cap) {
    h->a = malloc(cap * sizeof(int));
    h->len = 0; h->cap = cap;
}

static void sift_up(Heap *h, size_t i) {
    while (i > 0) {
        size_t p = (i - 1) / 2;
        if (h->a[p] <= h->a[i]) break;
        swap(&h->a[p], &h->a[i]);
        i = p;
    }
}

static void sift_down(Heap *h, size_t i) {
    size_t n = h->len;
    while (1) {
        size_t l = 2 * i + 1, r = 2 * i + 2, smallest = i;
        if (l < n && h->a[l] < h->a[smallest]) smallest = l;
        if (r < n && h->a[r] < h->a[smallest]) smallest = r;
        if (smallest == i) return;
        swap(&h->a[i], &h->a[smallest]);
        i = smallest;
    }
}

static void heap_push(Heap *h, int x) {
    if (h->len == h->cap) { h->cap *= 2; h->a = realloc(h->a, h->cap * sizeof(int)); }
    h->a[h->len++] = x;
    sift_up(h, h->len - 1);
}

static int heap_pop(Heap *h) {
    int top = h->a[0];
    h->a[0] = h->a[--h->len];
    if (h->len > 0) sift_down(h, 0);
    return top;
}

/* Floyd's O(n) build_heap */
static void build_heap_from_array(int *a, size_t n, Heap *h) {
    memcpy(h->a, a, n * sizeof(int));
    h->len = n;
    if (n < 2) return;
    for (size_t i = n / 2 - 1; ; --i) {
        sift_down(h, i);
        if (i == 0) break;
    }
}

/* Heapsort: build max-heap, then repeatedly extract max to end.
   We'll build a min-heap then read top → produces sorted ascending in a separate array. */
static void my_heapsort(int *a, size_t n) {
    Heap h; heap_init(&h, n);
    build_heap_from_array(a, n, &h);
    int *out = malloc(n * sizeof(int));
    for (size_t i = 0; i < n; ++i) out[i] = heap_pop(&h);
    memcpy(a, out, n * sizeof(int));
    free(out);
    free(h.a);
}

/* Verify min-heap invariant */
static int verify(const Heap *h) {
    for (size_t i = 1; i < h->len; ++i) {
        size_t p = (i - 1) / 2;
        if (h->a[p] > h->a[i]) return 0;
    }
    return 1;
}

static int cmp_int(const void *a, const void *b) {
    return *(const int *)a - *(const int *)b;
}

static double now(void) {
    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

int main(void) {
    /* Small functional check */
    Heap h; heap_init(&h, 16);
    int seq[] = {5, 1, 9, 3, 7, 2, 8, 4, 6};
    for (size_t i = 0; i < sizeof(seq)/sizeof(seq[0]); ++i) heap_push(&h, seq[i]);
    printf("== Functional check ==\n");
    printf("  peek = %d (expect 1)\n", h.a[0]);
    printf("  invariant: %s\n", verify(&h) ? "OK" : "FAIL");
    printf("  popping: ");
    while (h.len) printf("%d ", heap_pop(&h));
    printf("(should be sorted ascending)\n");
    free(h.a);

    /* O(n) build_heap on adversarial input */
    const int N_build = 1000000;
    int *arr = malloc(N_build * sizeof(int));
    srand(42);
    for (int i = 0; i < N_build; ++i) arr[i] = rand();
    Heap h2; heap_init(&h2, N_build);
    double t0 = now();
    build_heap_from_array(arr, N_build, &h2);
    double t = now() - t0;
    printf("\n== build_heap from %d random ints (Floyd's O(n)) ==\n", N_build);
    printf("  time: %.3fs  invariant: %s\n", t, verify(&h2) ? "OK" : "FAIL");
    free(h2.a);

    /* Heapsort vs qsort */
    const int N = 500000;
    int *a1 = malloc(N * sizeof(int));
    int *a2 = malloc(N * sizeof(int));
    for (int i = 0; i < N; ++i) a1[i] = a2[i] = rand();

    t0 = now();
    my_heapsort(a1, N);
    double t_heap = now() - t0;

    t0 = now();
    qsort(a2, N, sizeof(int), cmp_int);
    double t_qs = now() - t0;

    int eq = (memcmp(a1, a2, N * sizeof(int)) == 0);
    printf("\n== Heapsort vs qsort (%d random ints) ==\n", N);
    printf("  my_heapsort: %.3fs\n", t_heap);
    printf("  qsort   : %.3fs  (typically 2-3x faster due to cache locality)\n", t_qs);
    printf("  results identical: %s\n", eq ? "YES" : "NO");

    free(a1); free(a2); free(arr);
    return 0;
}
