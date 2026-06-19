/*
 * Lesson 14: Synchronization in the Kernel — Spinlocks, RCU
 *
 * Implements spinlock, mutex, RCU, and seqlock in userspace.
 * Benchmarks spinlock vs mutex under contention.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <time.h>
#include <unistd.h>
#include <sched.h>

/* ─── Helpers ──────────────────────────────────────────────────── */

static inline double now_sec(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

/* ─── Spinlock ─────────────────────────────────────────────────── */

typedef struct {
    atomic_flag flag;
} spinlock_t;

static void spinlock_init(spinlock_t *s) {
    atomic_flag_clear(&s->flag);
}

static void spin_lock(spinlock_t *s) {
    while (atomic_flag_test_and_set_explicit(&s->flag, memory_order_acquire))
        ; /* spin */
}

static void spin_unlock(spinlock_t *s) {
    atomic_flag_clear_explicit(&s->flag, memory_order_release);
}

/* ─── Mutex (simplified using pthread mutex) ───────────────────── */

typedef struct {
    pthread_mutex_t mtx;
} mutex_t;

static void mutex_init(mutex_t *m) {
    pthread_mutex_init(&m->mtx, NULL);
}

static void mutex_lock(mutex_t *m) {
    pthread_mutex_lock(&m->mtx);
}

static void mutex_unlock(mutex_t *m) {
    pthread_mutex_unlock(&m->mtx);
}

/* ─── Seqlock ──────────────────────────────────────────────────── */

typedef struct {
    atomic_uint seq;
    pthread_mutex_t writer_mtx;
} seqlock_t;

static void seqlock_init(seqlock_t *sl) {
    atomic_store(&sl->seq, 0);
    pthread_mutex_init(&sl->writer_mtx, NULL);
}

static void seqlock_write_begin(seqlock_t *sl) {
    pthread_mutex_lock(&sl->writer_mtx);
    atomic_fetch_add_explicit(&sl->seq, 1, memory_order_release);
}

static void seqlock_write_end(seqlock_t *sl) {
    atomic_fetch_add_explicit(&sl->seq, 1, memory_order_release);
    pthread_mutex_unlock(&sl->writer_mtx);
}

static unsigned seqlock_read_begin(seqlock_t *sl) {
    unsigned s;
    do {
        s = atomic_load_explicit(&sl->seq, memory_order_acquire);
        if (s & 1) sched_yield(); /* write in progress */
    } while (s & 1);
    atomic_thread_fence(memory_order_acquire);
    return s;
}

static bool seqlock_read_retry(seqlock_t *sl, unsigned start) {
    atomic_thread_fence(memory_order_acquire);
    return atomic_load_explicit(&sl->seq, memory_order_acquire) != start;
}

/* ─── RCU Demo ─────────────────────────────────────────────────── */

typedef struct {
    int value;
    int version;
} rcu_data_t;

static _Atomic rcu_data_t *global_rcu_data;
static atomic_int grace_epoch;

static void rcu_reader(int id) {
    /* Read-side critical section: load pointer, read data */
    rcu_data_t local;
    atomic_store(&grace_epoch, atomic_load(&grace_epoch) + 1);
    rcu_data_t *p = (rcu_data_t *)atomic_load(
        (_Atomic(uintptr_t) *)&global_rcu_data);
    if (p) {
        local = *p;
        printf("  [Reader %d] version=%d value=%d (epoch %d)\n",
               id, local.version, local.value,
               atomic_load(&grace_epoch));
    }
}

static void rcu_writer(int new_value) {
    rcu_data_t *old = (rcu_data_t *)atomic_load(
        (_Atomic(uintptr_t) *)&global_rcu_data);
    rcu_data_t *n = malloc(sizeof(rcu_data_t));
    n->value = new_value;
    n->version = old ? old->version + 1 : 1;
    atomic_store((_Atomic(uintptr_t) *)&global_rcu_data, (uintptr_t)n);
    printf("  [Writer] published version=%d value=%d\n", n->version, n->value);
    /* In real kernel: synchronize_rcu() then kfree(old) */
    usleep(1000); /* simulate grace period */
    free(old);
}

/* ─── Benchmark: Spinlock vs Mutex ─────────────────────────────── */

typedef struct {
    int             id;
    int             ops;
    spinlock_t     *spin;
    mutex_t        *mtx;
    atomic_long    *counter_spin;
    atomic_long    *counter_mtx;
    bool            use_spin;
} bench_arg_t;

static void *bench_thread(void *arg) {
    bench_arg_t *b = arg;
    for (int i = 0; i < b->ops; i++) {
        if (b->use_spin) {
            spin_lock(b->spin);
            (*b->counter_spin)++;
            /* tiny critical section */
            spin_unlock(b->spin);
        } else {
            mutex_lock(b->mtx);
            (*b->counter_mtx)++;
            mutex_unlock(b->mtx);
        }
    }
    return NULL;
}

static void run_benchmark(int nthreads, int ops_per_thread) {
    spinlock_t spin;
    mutex_t mtx;
    spinlock_init(&spin);
    mutex_init(&mtx);
    atomic_long counter_s = 0, counter_m = 0;

    /* Spinlock benchmark */
    pthread_t threads[32];
    bench_arg_t args[32];
    double t0 = now_sec();
    for (int i = 0; i < nthreads; i++) {
        args[i] = (bench_arg_t){ i, ops_per_thread, &spin, &mtx,
                                  &counter_s, &counter_m, true };
        pthread_create(&threads[i], NULL, bench_thread, &args[i]);
    }
    for (int i = 0; i < nthreads; i++)
        pthread_join(threads[i], NULL);
    double spin_time = now_sec() - t0;

    /* Mutex benchmark */
    t0 = now_sec();
    for (int i = 0; i < nthreads; i++) {
        args[i].use_spin = false;
        pthread_create(&threads[i], NULL, bench_thread, &args[i]);
    }
    for (int i = 0; i < nthreads; i++)
        pthread_join(threads[i], NULL);
    double mutex_time = now_sec() - t0;

    printf("  %2d threads: spinlock=%.3f s (%.0f Kops/s)  mutex=%.3f s (%.0f Kops/s)\n",
           nthreads,
           spin_time, (counter_s / spin_time) / 1000.0,
           mutex_time, (counter_m / mutex_time) / 1000.0);
}

/* ─── Main Demo ────────────────────────────────────────────────── */

int main(void) {
    printf("=== Lesson 14: Kernel Synchronization Demo ===\n\n");

    /* Spinlock demo */
    printf("--- Spinlock ---\n");
    spinlock_t sl;
    spinlock_init(&sl);
    spin_lock(&sl);
    printf("  spin_lock acquired\n");
    spin_unlock(&sl);
    printf("  spin_unlock released\n\n");

    /* Seqlock demo */
    printf("--- Seqlock ---\n");
    seqlock_t sq;
    seqlock_init(&sq);
    /* Writer */
    int shared_val = 42;
    seqlock_write_begin(&sq);
    shared_val = 100;
    seqlock_write_end(&sq);
    printf("  Writer set shared_val = %d\n", shared_val);
    /* Reader */
    unsigned s = seqlock_read_begin(&sq);
    int val = shared_val;
    bool retry = seqlock_read_retry(&sq, s);
    printf("  Reader read %d (retry=%s)\n\n", val, retry ? "yes" : "no");

    /* RCU demo */
    printf("--- RCU ---\n");
    global_rcu_data = malloc(sizeof(rcu_data_t));
    ((rcu_data_t *)global_rcu_data)->value = 1;
    ((rcu_data_t *)global_rcu_data)->version = 0;
    atomic_store(&grace_epoch, 0);

    for (int r = 0; r < 3; r++)
        rcu_reader(r);
    rcu_writer(42);
    rcu_writer(99);
    for (int r = 0; r < 3; r++)
        rcu_reader(r);
    free((void *)atomic_load(
        (_Atomic(uintptr_t) *)&global_rcu_data));
    printf("\n");

    /* Benchmark */
    printf("--- Spinlock vs Mutex Benchmark ---\n");
    printf("  (100000 ops per thread, tiny critical section)\n");
    for (int n = 1; n <= 8; n *= 2)
        run_benchmark(n, 100000);

    return 0;
}
