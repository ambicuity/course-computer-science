/* main.c — dynamic array (Vec<int>) with doubling growth + cost accounting. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <assert.h>

typedef struct {
    int    *data;
    size_t  len;
    size_t  cap;
    size_t  resizes;
    size_t  total_copies;
} Vec;

static void vec_init(Vec *v) { memset(v, 0, sizeof(*v)); }

static void vec_reserve(Vec *v, size_t new_cap) {
    if (new_cap <= v->cap) return;
    v->data = realloc(v->data, new_cap * sizeof(int));
    assert(v->data);
    v->cap = new_cap;
}

static void vec_push_factor(Vec *v, int x, double factor) {
    if (v->len == v->cap) {
        size_t new_cap = v->cap == 0 ? 4 : (size_t)(v->cap * factor + 0.5);
        if (new_cap == v->cap) new_cap = v->cap + 1;        /* safety */
        v->total_copies += v->len;
        v->resizes++;
        vec_reserve(v, new_cap);
    }
    v->data[v->len++] = x;
}

static void vec_push_plus_k(Vec *v, int x, size_t k) {
    if (v->len == v->cap) {
        size_t new_cap = v->cap + k;
        v->total_copies += v->len;
        v->resizes++;
        vec_reserve(v, new_cap);
    }
    v->data[v->len++] = x;
}

static void vec_free(Vec *v) { free(v->data); memset(v, 0, sizeof(*v)); }

int main(void) {
    const size_t N = 1000000;

    printf("== Dynamic array growth comparison: %zu pushes ==\n\n", N);

    /* 2× growth */
    Vec v2; vec_init(&v2);
    for (size_t i = 0; i < N; ++i) vec_push_factor(&v2, (int)i, 2.0);
    printf("2.0× growth: cap=%zu  resizes=%zu  total_copies=%zu  amortized=%.2f writes/push\n",
           v2.cap, v2.resizes, v2.total_copies, (double)(N + v2.total_copies) / N);
    vec_free(&v2);

    /* 1.5× growth */
    Vec v15; vec_init(&v15);
    for (size_t i = 0; i < N; ++i) vec_push_factor(&v15, (int)i, 1.5);
    printf("1.5× growth: cap=%zu  resizes=%zu  total_copies=%zu  amortized=%.2f writes/push\n",
           v15.cap, v15.resizes, v15.total_copies, (double)(N + v15.total_copies) / N);
    vec_free(&v15);

    /* +8 growth (BAD) */
    Vec vk; vec_init(&vk);
    for (size_t i = 0; i < N; ++i) vec_push_plus_k(&vk, (int)i, 8);
    printf("+8   growth: cap=%zu  resizes=%zu  total_copies=%zu  amortized=%.2f writes/push  (≈ N/16 → O(n))\n",
           vk.cap, vk.resizes, vk.total_copies, (double)(N + vk.total_copies) / N);
    vec_free(&vk);

    /* Bench: reserve up-front vs push-only */
    printf("\n== Bench: push-only vs reserve+push (N=%zu, repeated 5×) ==\n", N);
    struct timespec t0, t1;

    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int r = 0; r < 5; ++r) {
        Vec v; vec_init(&v);
        for (size_t i = 0; i < N; ++i) vec_push_factor(&v, (int)i, 2.0);
        vec_free(&v);
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);
    double t_push = (t1.tv_sec - t0.tv_sec) + (t1.tv_nsec - t0.tv_nsec) * 1e-9;

    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int r = 0; r < 5; ++r) {
        Vec v; vec_init(&v);
        vec_reserve(&v, N);
        for (size_t i = 0; i < N; ++i) vec_push_factor(&v, (int)i, 2.0);
        vec_free(&v);
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);
    double t_reserve = (t1.tv_sec - t0.tv_sec) + (t1.tv_nsec - t0.tv_nsec) * 1e-9;

    printf("  push-only:       %.3fs  (%.1f ns/push)\n", t_push,    t_push / 5 * 1e9 / N);
    printf("  reserve + push:  %.3fs  (%.1f ns/push)  speedup %.2f×\n",
           t_reserve, t_reserve / 5 * 1e9 / N, t_push / t_reserve);
    return 0;
}
