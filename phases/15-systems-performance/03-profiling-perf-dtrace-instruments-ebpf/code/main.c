#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <math.h>

#define ARRAY_SIZE (8 * 1024 * 1024)
#define MATRIX_SIZE 256
#define BENCH_ITERATIONS 3

static double time_now(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

static int *shuffle_indices(int n) {
    int *indices = malloc(n * sizeof(int));
    if (!indices) { perror("malloc"); exit(1); }
    for (int i = 0; i < n; i++) indices[i] = i;
    for (int i = n - 1; i > 0; i--) {
        int j = rand() % (i + 1);
        int tmp = indices[i];
        indices[i] = indices[j];
        indices[j] = tmp;
    }
    return indices;
}

static long long sequential_access(const int *arr, int n) {
    long long sum = 0;
    for (int i = 0; i < n; i++) {
        sum += arr[i];
    }
    return sum;
}

static long long random_access(const int *arr, const int *indices, int n) {
    long long sum = 0;
    for (int i = 0; i < n; i++) {
        sum += arr[indices[i]];
    }
    return sum;
}

static long long branchy(const int *data, int n) {
    long long count = 0;
    for (int i = 0; i < n; i++) {
        if (data[i] > 0)
            count += data[i];
        else
            count -= data[i];
    }
    return count;
}

static void matrix_multiply(const double *A, const double *B, double *C, int n) {
    for (int i = 0; i < n; i++) {
        for (int k = 0; k < n; k++) {
            double a_ik = A[i * n + k];
            for (int j = 0; j < n; j++) {
                C[i * n + j] += a_ik * B[k * n + j];
            }
        }
    }
}

static void fill_sequential(int *arr, int n) {
    for (int i = 0; i < n; i++) arr[i] = i % 97;
}

static void fill_random(int *arr, int n) {
    for (int i = 0; i < n; i++) arr[i] = (rand() % 200) - 100;
}

static void fill_matrix(double *M, int n) {
    for (int i = 0; i < n * n; i++) M[i] = (double)(rand() % 100) / 100.0;
}

static void run_benchmark(const char *name, long long (*func)(void), double target_time) {
    double elapsed = 0.0;
    long long total_iters = 0;
    long long result = 0;

    int iters = 1;
    while (elapsed < target_time) {
        iters *= 2;
        double t0 = time_now();
        for (int i = 0; i < iters; i++) {
            result = func();
        }
        double t1 = time_now();
        elapsed = t1 - t0;
        total_iters = iters;
    }

    double per_iter_us = (elapsed / total_iters) * 1e6;
    printf("  %-20s  %8.2f us/iter  (result=%lld, iters=%lld)\n",
           name, per_iter_us, result, total_iters);
}

static int *g_arr = NULL;
static int *g_indices = NULL;
static int *g_branchdata = NULL;
static double *g_A = NULL, *g_B = NULL, *g_C = NULL;
static int g_n;
static int g_mn;

static long long bench_sequential(void) { return sequential_access(g_arr, g_n); }
static long long bench_random(void) { return random_access(g_arr, g_indices, g_n); }
static long long bench_branchy(void) { return branchy(g_branchdata, g_n); }
static long long bench_matrix(void) {
    memset(g_C, 0, g_mn * g_mn * sizeof(double));
    matrix_multiply(g_A, g_B, g_C, g_mn);
    return (long long)g_C[0];
}

int main(int argc, char **argv) {
    srand(42);
    g_n = ARRAY_SIZE;
    g_mn = MATRIX_SIZE;

    g_arr = malloc(g_n * sizeof(int));
    g_indices = shuffle_indices(g_n);
    g_branchdata = malloc(g_n * sizeof(int));
    g_A = malloc(g_mn * g_mn * sizeof(double));
    g_B = malloc(g_mn * g_mn * sizeof(double));
    g_C = malloc(g_mn * g_mn * sizeof(double));

    if (!g_arr || !g_indices || !g_branchdata || !g_A || !g_B || !g_C) {
        perror("malloc");
        return 1;
    }

    fill_sequential(g_arr, g_n);
    fill_random(g_branchdata, g_n);
    fill_matrix(g_A, g_mn);
    fill_matrix(g_B, g_mn);

    int run_sequential = 0, run_random = 0, run_branchy = 0, run_matrix = 0;

    if (argc <= 1 || strcmp(argv[1], "all") == 0) {
        run_sequential = run_random = run_branchy = run_matrix = 1;
    } else {
        for (int i = 1; i < argc; i++) {
            if (strcmp(argv[i], "sequential") == 0) run_sequential = 1;
            else if (strcmp(argv[i], "random") == 0) run_random = 1;
            else if (strcmp(argv[i], "branchy") == 0) run_branchy = 1;
            else if (strcmp(argv[i], "matrix") == 0) run_matrix = 1;
            else {
                fprintf(stderr, "Unknown benchmark: %s\n"
                    "Usage: %s [all|sequential|random|branchy|matrix]\n",
                    argv[i], argv[0]);
                return 1;
            }
        }
    }

    printf("Profiling benchmark suite (array_size=%d, matrix_size=%d)\n\n", g_n, g_mn);

    if (run_sequential) run_benchmark("sequential_access", bench_sequential, 0.5);
    if (run_random)     run_benchmark("random_access",     bench_random,     0.5);
    if (run_branchy)    run_benchmark("branchy",           bench_branchy,    0.5);
    if (run_matrix)     run_benchmark("matrix_multiply",    bench_matrix,     0.5);

    free(g_arr);
    free(g_indices);
    free(g_branchdata);
    free(g_A);
    free(g_B);
    free(g_C);

    return 0;
}