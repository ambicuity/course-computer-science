/*
 * Locks — Mutex, RW Lock, Spinlock, Ticket Lock
 * Phase 13 — Concurrent & Parallel Computing
 *
 * From-scratch implementations:
 *   1. Spinlock       — atomic_flag test-and-set loop
 *   2. Ticket lock    — atomic_fetch_add + atomic_load for FIFO fairness
 *   3. Mutex          — CAS spin loop + sched_yield fallback
 *   4. Benchmark      — wall-clock comparison, 1/2/4 threads
 *
 * Compile:
 *   clang -std=c11 -pthread -O2 -o locks_bench main.c   (macOS)
 *   gcc   -std=c11 -pthread -O2 -o locks_bench main.c   (Linux)
 *   perf stat ./locks_bench                              (Linux: see cycles)
 *
 * Run:
 *   ./locks_bench
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <pthread.h>
#include <sched.h>
#include <time.h>

/* ──────────────────────────────────────────────
   Helpers
   ────────────────────────────────────────────── */

#if defined(__x86_64__) || defined(__i386__)
  #define PAUSE() __builtin_ia32_pause()
#else
  #define PAUSE() ((void)0)
#endif

#define NS_PER_SEC 1000000000ULL

static double timespec_to_sec(struct timespec ts) {
    return (double)ts.tv_sec + (double)ts.tv_nsec / NS_PER_SEC;
}

static double wall_clock(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return timespec_to_sec(ts);
}

/* ──────────────────────────────────────────────
   1. Spinlock — atomic_flag test-and-set
   ────────────────────────────────────────────── */

typedef struct {
    atomic_flag flag;
} spinlock_t;

static void spinlock_init(spinlock_t *l) {
    atomic_flag_clear(&l->flag);
}

static void spinlock_lock(spinlock_t *l) {
    while (atomic_flag_test_and_set(&l->flag)) {
        PAUSE();
    }
}

static void spinlock_unlock(spinlock_t *l) {
    atomic_flag_clear(&l->flag);
}

/* ──────────────────────────────────────────────
   2. Ticket lock — fair FIFO via atomics
   ────────────────────────────────────────────── */

typedef struct {
    atomic_uint ticket;
    atomic_uint turn;
} ticketlock_t;

static void ticketlock_init(ticketlock_t *l) {
    atomic_store(&l->ticket, 0);
    atomic_store(&l->turn, 0);
}

static void ticketlock_lock(ticketlock_t *l) {
    unsigned my = atomic_fetch_add(&l->ticket, 1);
    while (atomic_load(&l->turn) != my) {
        PAUSE();
    }
}

static void ticketlock_unlock(ticketlock_t *l) {
    atomic_fetch_add(&l->turn, 1);
}

/* ──────────────────────────────────────────────
   3. Mutex — CAS + sched_yield backoff
      (On Linux, replace sched_yield with futex)
   ────────────────────────────────────────────── */

typedef struct {
    atomic_int locked;    /* 0 = free, 1 = held */
} mutex_t;

static void mutex_init(mutex_t *m) {
    atomic_store(&m->locked, 0);
}

static void mutex_lock(mutex_t *m) {
    unsigned spins = 0;
    for (;;) {
        int expected = 0;
        if (atomic_compare_exchange_weak(&m->locked, &expected, 1))
            return;
        if (++spins > 100) {
            spins = 0;
            sched_yield();
        } else {
            PAUSE();
        }
    }
}

static void mutex_unlock(mutex_t *m) {
    atomic_store(&m->locked, 0);
}

/* ──────────────────────────────────────────────
   Benchmark harness
   ────────────────────────────────────────────── */

typedef struct {
    void *lock;
    void (*lock_fn)(void*);
    void (*unlock_fn)(void*);
    const char *name;
    volatile unsigned long long *counter;  /* shared counter */
    int thread_id;
    int num_threads;
    unsigned long long iterations;
} bench_arg_t;

static void* bench_thread(void *arg) {
    bench_arg_t *ba = (bench_arg_t*)arg;
    for (unsigned long long i = 0; i < ba->iterations; i++) {
        ba->lock_fn(ba->lock);
        (*(volatile unsigned long long*)ba->counter)++;
        ba->unlock_fn(ba->lock);
    }
    return NULL;
}

/* type-erased wrappers so we can use a common bench_thread */

static void spinlock_lock_wrap(void *l) { spinlock_lock((spinlock_t*)l); }
static void spinlock_unlock_wrap(void *l) { spinlock_unlock((spinlock_t*)l); }
static void ticketlock_lock_wrap(void *l) { ticketlock_lock((ticketlock_t*)l); }
static void ticketlock_unlock_wrap(void *l) { ticketlock_unlock((ticketlock_t*)l); }
static void mutex_lock_wrap(void *l) { mutex_lock((mutex_t*)l); }
static void mutex_unlock_wrap(void *l) { mutex_unlock((mutex_t*)l); }

/* empty lock: no synchronization at all (race demo) */
static void null_lock(void *l) { (void)l; }
static void null_unlock(void *l) { (void)l; }

static double run_bench(const char *name,
                        void *lock,
                        void (*lock_fn)(void*),
                        void (*unlock_fn)(void*),
                        int num_threads,
                        unsigned long long iters_per_thread,
                        volatile unsigned long long *counter)
{
    pthread_t *threads = malloc(num_threads * sizeof(pthread_t));
    bench_arg_t *args   = malloc(num_threads * sizeof(bench_arg_t));

    double t0 = wall_clock();

    for (int i = 0; i < num_threads; i++) {
        args[i].lock        = lock;
        args[i].lock_fn     = lock_fn;
        args[i].unlock_fn   = unlock_fn;
        args[i].name        = name;
        args[i].counter     = counter;
        args[i].thread_id   = i;
        args[i].num_threads = num_threads;
        args[i].iterations  = iters_per_thread;
        pthread_create(&threads[i], NULL, bench_thread, &args[i]);
    }

    for (int i = 0; i < num_threads; i++) {
        pthread_join(threads[i], NULL);
    }

    double elapsed = wall_clock() - t0;

    free(threads);
    free(args);
    return elapsed;
}

static void bench_one(const char *name,
                      void *lock,
                      void (*lock_fn)(void*),
                      void (*unlock_fn)(void*),
                      int max_threads,
                      unsigned long long iters_per_thread)
{
    printf("  %-14s", name);
    for (int t = 1; t <= max_threads; t++) {
        volatile unsigned long long counter = 0;
        double sec = run_bench(name, lock, lock_fn, unlock_fn,
                               t, iters_per_thread, &counter);
        unsigned long long expected = t * iters_per_thread;
        double ops_per_sec = (double)(t * iters_per_thread) / sec;

        printf(" | T=%d: %5.2fs  %9.0f ops/s  %s",
               t, sec, ops_per_sec,
               (counter == expected) ? "" : "***RACE*** ");
    }
    printf("\n");
}

/* ──────────────────────────────────────────────
   Race demo (no lock)
   ────────────────────────────────────────────── */

static void demo_race(int num_threads, unsigned long long iters) {
    printf("\n=== Race Demo (No Lock) ===\n");
    unsigned long long expected = (unsigned long long)num_threads * iters;
    volatile unsigned long long counter = 0;
    double sec = run_bench("no-lock", NULL, null_lock, null_unlock,
                           num_threads, iters, &counter);
    printf("  Expected: %llu\n", expected);
    printf("  Actual:   %llu\n", counter);
    printf("  Time:     %.3fs\n", sec);
    printf("  Lost updates: %llu\n", expected - counter);
}

/* ──────────────────────────────────────────────
   Benchmark suite
   ────────────────────────────────────────────── */

static void demo_benchmarks(void) {
    printf("\n=== Benchmark: Lock Throughput ===\n");
    printf("  Each thread does 1M lock+inc+unlock cycles\n");
    printf("  Measurements are wall-clock seconds and ops/sec\n\n");

    unsigned long long iters = 1000000;
    int max_threads = 4;

    spinlock_t    sl;    spinlock_init(&sl);
    ticketlock_t  tl;    ticketlock_init(&tl);
    mutex_t       m;     mutex_init(&m);

    bench_one("Spinlock",   &sl, spinlock_lock_wrap,   spinlock_unlock_wrap,   max_threads, iters);
    bench_one("TicketLock", &tl, ticketlock_lock_wrap, ticketlock_unlock_wrap, max_threads, iters);
    bench_one("Mutex",      &m,  mutex_lock_wrap,      mutex_unlock_wrap,      max_threads, iters);
}

/* ──────────────────────────────────────────────
   Deadlock demonstration (lock ordering)
   ────────────────────────────────────────────── */

static pthread_mutex_t dl_a = PTHREAD_MUTEX_INITIALIZER;
static pthread_mutex_t dl_b = PTHREAD_MUTEX_INITIALIZER;

static void* deadlock_ab(void *arg) {
    (void)arg;
    pthread_mutex_lock(&dl_a);
    printf("  Thread A: got lock A, waiting for B...\n");
    struct timespec ts = {.tv_sec = 0, .tv_nsec = 50000000}; /* 50 ms */
    nanosleep(&ts, NULL);
    pthread_mutex_lock(&dl_b);  /* deadlocks if B holds A */
    printf("  Thread A: got both locks\n");
    pthread_mutex_unlock(&dl_b);
    pthread_mutex_unlock(&dl_a);
    return NULL;
}

static void* deadlock_ba(void *arg) {
    (void)arg;
    pthread_mutex_lock(&dl_b);
    pthread_mutex_lock(&dl_a);  /* deadlock */
    printf("  Thread B: got both locks\n");
    pthread_mutex_unlock(&dl_a);
    pthread_mutex_unlock(&dl_b);
    return NULL;
}

static void demo_deadlock(void) {
    printf("\n=== Deadlock Demo ===\n");
    printf("  Thread A: lock A -> lock B\n");
    printf("  Thread B: lock B -> lock A\n");
    printf("  (This will hang — press Ctrl-C or skip with 'N')\n");
    printf("  Run? (Y/N): ");
    fflush(stdout);

    int c = getchar();
    if (c != 'Y' && c != 'y') {
        printf("  Skipped.\n");
        return;
    }

    pthread_t a, b;
    pthread_create(&a, NULL, deadlock_ab, NULL);
    pthread_create(&b, NULL, deadlock_ba, NULL);
    pthread_join(a, NULL);
    pthread_join(b, NULL);
}

/* ──────────────────────────────────────────────
   Priority inversion simulation
   ────────────────────────────────────────────── */

static pthread_mutex_t pi_lock = PTHREAD_MUTEX_INITIALIZER;

static void* pi_low(void *arg) {
    (void)arg;
    printf("  Low-pri thread: acquiring lock...\n");
    pthread_mutex_lock(&pi_lock);
    printf("  Low-pri thread: holds lock, working...\n");
    struct timespec ts = {.tv_sec = 0, .tv_nsec = 200000000}; /* 200ms */
    nanosleep(&ts, NULL);
    printf("  Low-pri thread: releasing lock\n");
    pthread_mutex_unlock(&pi_lock);
    return NULL;
}

static void* pi_high(void *arg) {
    (void)arg;
    struct timespec ts = {.tv_sec = 0, .tv_nsec = 50000000}; /* 50ms */
    nanosleep(&ts, NULL);
    printf("  High-pri thread: trying to acquire lock...\n");
    double t0 = wall_clock();
    pthread_mutex_lock(&pi_lock);
    double waited = wall_clock() - t0;
    printf("  High-pri thread: got lock after %.3fs\n", waited);
    pthread_mutex_unlock(&pi_lock);
    return NULL;
}

static void demo_priority_inversion(void) {
    printf("\n=== Priority Inversion Simulation ===\n");
    printf("  High-pri thread waits for low-pri thread holding lock\n");
    printf("  (On a real RT system with medium-pri preemption this\n");
    printf("   wait would be unbounded — priority inheritance fixes it)\n");

    pthread_t low, high;
    pthread_create(&low, NULL, pi_low, NULL);
    pthread_create(&high, NULL, pi_high, NULL);
    pthread_join(low, NULL);
    pthread_join(high, NULL);
}

/* ──────────────────────────────────────────────
   main
   ────────────────────────────────────────────── */

int main(void) {
    printf("═══════════════════════════════════════════════\n");
    printf("  Locks — Mutex, RW Lock, Spinlock, Ticket Lock\n");
    printf("═══════════════════════════════════════════════\n");

    demo_race(2, 1000000);
    demo_benchmarks();
    demo_deadlock();
    demo_priority_inversion();

    printf("\nNote: On Linux, run 'perf stat ./locks_bench' to see\n");
    printf("  cycle counts, context switches, and cache misses.\n");
    printf("  This reveals spinlock overhead versus mutex blocking.\n\n");
    return 0;
}
