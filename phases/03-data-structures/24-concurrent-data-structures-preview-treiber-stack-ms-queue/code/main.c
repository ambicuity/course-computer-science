/* main.c — Treiber lock-free stack + mutex stack benchmark.
 * Memory reclamation: we INTENTIONALLY leak popped nodes for safety (no hazard
 * pointers). A real Treiber stack uses epoch GC or hazard pointers — see Phase 13.
 */
#include <stdio.h>
#include <stdlib.h>
#include <stdatomic.h>
#include <pthread.h>
#include <time.h>
#include <stdint.h>

/* ============================================================ */
/* Treiber stack (lock-free)                                    */
/* ============================================================ */
typedef struct TNode {
    int           value;
    struct TNode *next;
} TNode;

typedef struct {
    _Atomic(TNode *) head;
} Treiber;

static void treiber_push(Treiber *s, int v) {
    TNode *n = malloc(sizeof(TNode));
    n->value = v;
    TNode *old_head = atomic_load_explicit(&s->head, memory_order_relaxed);
    do {
        n->next = old_head;
    } while (!atomic_compare_exchange_weak_explicit(
        &s->head, &old_head, n,
        memory_order_release, memory_order_relaxed));
}

static int treiber_pop(Treiber *s, int *out) {
    TNode *old_head = atomic_load_explicit(&s->head, memory_order_acquire);
    while (old_head) {
        TNode *next = old_head->next;
        if (atomic_compare_exchange_weak_explicit(
                &s->head, &old_head, next,
                memory_order_acq_rel, memory_order_acquire)) {
            *out = old_head->value;
            /* LEAK: no hazard pointers → can't safely free here */
            return 1;
        }
    }
    return 0;
}

/* ============================================================ */
/* Mutex stack (for comparison)                                 */
/* ============================================================ */
typedef struct MNode { int value; struct MNode *next; } MNode;
typedef struct {
    MNode          *head;
    pthread_mutex_t lock;
} MutexStack;

static void mutex_push(MutexStack *s, int v) {
    MNode *n = malloc(sizeof(MNode));
    n->value = v;
    pthread_mutex_lock(&s->lock);
    n->next = s->head; s->head = n;
    pthread_mutex_unlock(&s->lock);
}

static int mutex_pop(MutexStack *s, int *out) {
    pthread_mutex_lock(&s->lock);
    if (!s->head) { pthread_mutex_unlock(&s->lock); return 0; }
    MNode *n = s->head; s->head = n->next;
    pthread_mutex_unlock(&s->lock);
    *out = n->value; free(n);
    return 1;
}

/* ============================================================ */
/* Multi-thread test                                             */
/* ============================================================ */
typedef struct {
    void *stack;
    int   n_ops;
    int (*push_fn)(void *, int);
    int (*pop_fn)(void *, int *);
    _Atomic long checksum;
} Worker;

#define N_THREADS 4
#define N_OPS_PER 250000

static int treiber_push_v(void *s, int v) { treiber_push(s, v); return 1; }
static int treiber_pop_v (void *s, int *o) { return treiber_pop(s, o); }
static int mutex_push_v (void *s, int v) { mutex_push(s, v); return 1; }
static int mutex_pop_v  (void *s, int *o) { return mutex_pop(s, o); }

static void *worker(void *arg) {
    Worker *w = arg;
    long sum_in = 0, sum_out = 0;
    for (int i = 0; i < w->n_ops; ++i) {
        int v = (int)((uintptr_t)w + i);
        w->push_fn(w->stack, v);
        sum_in += v;
        int popped;
        if (w->pop_fn(w->stack, &popped)) sum_out += popped;
    }
    /* drain anything left we missed */
    int popped;
    while (w->pop_fn(w->stack, &popped)) sum_out += popped;
    atomic_fetch_add_explicit(&w->checksum, sum_in - sum_out, memory_order_relaxed);
    return NULL;
}

static double now(void) {
    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec * 1e-9;
}

static double bench(void *stack, int (*pf)(void *, int), int (*qf)(void *, int *)) {
    pthread_t threads[N_THREADS];
    Worker workers[N_THREADS];
    _Atomic long shared_checksum;
    atomic_store(&shared_checksum, 0);

    for (int i = 0; i < N_THREADS; ++i) {
        workers[i] = (Worker){ stack, N_OPS_PER, pf, qf, 0 };
        atomic_store(&workers[i].checksum, 0);
    }
    double t = now();
    for (int i = 0; i < N_THREADS; ++i) pthread_create(&threads[i], NULL, worker, &workers[i]);
    for (int i = 0; i < N_THREADS; ++i) pthread_join(threads[i], NULL);
    double elapsed = now() - t;
    long total_diff = 0;
    for (int i = 0; i < N_THREADS; ++i)
        total_diff += atomic_load(&workers[i].checksum);
    printf("    (checksum diff = %ld; should be 0)\n", total_diff);
    return elapsed;
}

int main(void) {
    Treiber t = { 0 };
    atomic_init(&t.head, NULL);
    printf("== Treiber lock-free stack ==\n");
    double t_treiber = bench(&t, treiber_push_v, treiber_pop_v);
    printf("    %d threads × %d ops = %d total in %.3fs (%.0f Mops/s)\n",
           N_THREADS, N_OPS_PER, N_THREADS * N_OPS_PER * 2,
           t_treiber, (double)(N_THREADS * N_OPS_PER * 2) / 1e6 / t_treiber);

    MutexStack m;
    m.head = NULL;
    pthread_mutex_init(&m.lock, NULL);
    printf("\n== Mutex-protected stack ==\n");
    double t_mutex = bench(&m, mutex_push_v, mutex_pop_v);
    printf("    %d threads × %d ops = %d total in %.3fs (%.0f Mops/s)\n",
           N_THREADS, N_OPS_PER, N_THREADS * N_OPS_PER * 2,
           t_mutex, (double)(N_THREADS * N_OPS_PER * 2) / 1e6 / t_mutex);
    pthread_mutex_destroy(&m.lock);

    printf("\nMutex / Treiber ratio: %.2fx (>1 means mutex faster)\n", t_treiber / t_mutex);
    printf("Note: lock-free guarantees PROGRESS (no deadlock), not always speed.\n");
    printf("Under heavy contention CAS-retry storms can make Treiber slower than\n");
    printf("a well-tuned mutex. Lock-free wins for: large critical sections,\n");
    printf("priority-inversion-sensitive code, real-time systems.\n");
    return 0;
}
