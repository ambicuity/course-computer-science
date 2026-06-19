/*
 * Condition Variables and Monitors
 * Phase 13 — Concurrent & Parallel Computing
 *
 * Build It, Steps 1 & 2:
 *   1. Broken Busy-Wait — a consumer spins on a flag (100% CPU).
 *   2. Producer-Consumer with pthread_cond — proper bounded buffer
 *      including lost-wakeup bug, broadcast, and spurious-wakeup handling.
 *
 * Compile:  gcc -std=c11 -Wall -Wextra -pedantic -o main main.c -lpthread
 * Usage:    ./main
 *
 * Step 1 runs first (watch CPU with top/htop), then Step 2 demonstrates
 * the CV-based bounded buffer.
 */

#define _POSIX_C_SOURCE 200809L

#include <pthread.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

/* ================================================================== */
/*  Step 1 — Broken Busy-Wait                                         */
/* ================================================================== */
/*
 * A mutex protects shared data, but it cannot express "wait until X is
 * true."  Without a condition variable, threads must resort to spinning
 * — checking a flag in a tight loop, burning 100% of a CPU core.
 *
 * This step demonstrates the problem.  Watch CPU usage while it runs.
 */

static volatile int busy_flag = 0;

static void* busy_waiter(void* arg) {
    (void)arg;
    fprintf(stderr, "[BusyWait] Consumer spinning... (watch CPU)\n");

    /* BAD: spins at 100% CPU until the flag changes.  No yield,
     * no sleep, no signal — just pure waste. */
    while (busy_flag == 0) {
        /* tight loop — pegs a core */
    }

    fprintf(stderr, "[BusyWait] Consumer: flag is set!  "
            "(1 second of 100%% CPU wasted)\n");
    return NULL;
}

static void* busy_setter(void* arg) {
    (void)arg;
    sleep(1);                   /* guarantee the waiter spins for 1 s */
    busy_flag = 1;
    fprintf(stderr, "[BusyWait] Producer: flag set to 1\n");
    return NULL;
}

static void run_busy_wait(void) {
    pthread_t t1, t2;
    pthread_create(&t1, NULL, busy_waiter, NULL);
    pthread_create(&t2, NULL, busy_setter, NULL);
    pthread_join(t1, NULL);
    pthread_join(t2, NULL);
    busy_flag = 0;              /* reset for cleanliness */
}

/* ================================================================== */
/*  Step 2 — Bounded Buffer with pthread_cond_t                       */
/* ================================================================== */
/*
 * A bounded (fixed-capacity) buffer shared between producers and
 * consumers.  Uses TWO condition variables:
 *
 *   can_consume  — "the buffer is not empty"
 *   can_produce  — "the buffer is not full"
 *
 * Each cv has its own wait queue: consumers wait on can_consume,
 * producers wait on can_produce.  This avoids the thundering-herd
 * problem where a broadcast would wake both groups unnecessarily.
 *
 * Key correctness properties:
 *   - Every wait() is in a while-loop (Mesa semantics + spurious wakeups).
 *   - Signal is called while holding the lock (lost-wakeup avoidance).
 *   - Signal is called AFTER modifying the state (the condition changed).
 */

#define BUF_CAPACITY 8

typedef struct {
    int buf[BUF_CAPACITY];
    size_t head;                /* read index  (dequeue from head) */
    size_t tail;                /* write index (enqueue at tail)   */
    size_t count;               /* number of items currently in buffer */
    pthread_mutex_t lock;
    pthread_cond_t can_consume; /* "buffer not empty" — consumers wait here */
    pthread_cond_t can_produce; /* "buffer not full"  — producers wait here */
} bounded_buffer_t;

static void bb_init(bounded_buffer_t* bb) {
    memset(bb->buf, 0, sizeof(bb->buf));
    bb->head = 0;
    bb->tail = 0;
    bb->count = 0;
    pthread_mutex_init(&bb->lock, NULL);
    pthread_cond_init(&bb->can_consume, NULL);
    pthread_cond_init(&bb->can_produce, NULL);
}

static void bb_destroy(bounded_buffer_t* bb) {
    pthread_mutex_destroy(&bb->lock);
    pthread_cond_destroy(&bb->can_consume);
    pthread_cond_destroy(&bb->can_produce);
}

/*
 * bb_put — insert an item.  Blocks if the buffer is full.
 *
 * The while-loop is essential: even after pthread_cond_wait returns,
 * the condition (count < capacity) may be false because:
 *   1. Spurious wakeup — the OS woke us for no reason.
 *   2. Mesa semantics — another consumer may have been woken first
 *      and filled the buffer again before we re-acquired the lock.
 */
static void bb_put(bounded_buffer_t* bb, int item) {
    pthread_mutex_lock(&bb->lock);
    while (bb->count == BUF_CAPACITY) {
        /* atomically: unlock + sleep.  On wake: re-lock. */
        pthread_cond_wait(&bb->can_produce, &bb->lock);
    }

    /* Critical section: modify shared state. */
    bb->buf[bb->tail] = item;
    bb->tail = (bb->tail + 1) % BUF_CAPACITY;
    bb->count++;

    /* The condition "buffer not empty" just became true.  Signal one
     * consumer.  The signal is queued only if a consumer is waiting;
     * if none are, the next consumer will find the item immediately. */
    pthread_cond_signal(&bb->can_consume);
    pthread_mutex_unlock(&bb->lock);
}

/*
 * bb_get — remove and return an item.  Blocks if the buffer is empty.
 */
static int bb_get(bounded_buffer_t* bb) {
    pthread_mutex_lock(&bb->lock);
    while (bb->count == 0) {
        pthread_cond_wait(&bb->can_consume, &bb->lock);
    }

    int item = bb->buf[bb->head];
    bb->head = (bb->head + 1) % BUF_CAPACITY;
    bb->count--;

    /* The condition "buffer not full" just became true.  Signal one
     * producer that there's room again. */
    pthread_cond_signal(&bb->can_produce);
    pthread_mutex_unlock(&bb->lock);
    return item;
}

/* ---- Test harness: multiple producers + consumers ---- */

typedef struct {
    bounded_buffer_t* bb;
    int id;
    int items;
} worker_arg_t;

static void* producer_thread(void* arg) {
    worker_arg_t* wa = (worker_arg_t*)arg;
    for (int i = 0; i < wa->items; i++) {
        int val = i + wa->id * 10000;   /* unique per producer */
        bb_put(wa->bb, val);
        fprintf(stderr, "[Producer %d] put %d\n", wa->id, val);
        /* Small delay to make the interleaving interesting. */
        nanosleep(&(struct timespec){0, 5000000L}, NULL); /* 5 ms */
    }
    return NULL;
}

static void* consumer_thread(void* arg) {
    worker_arg_t* wa = (worker_arg_t*)arg;
    for (int i = 0; i < wa->items; i++) {
        int val = bb_get(wa->bb);
        fprintf(stderr, "[Consumer %d] got %d\n", wa->id, val);
        nanosleep(&(struct timespec){0, 3000000L}, NULL); /* 3 ms */
    }
    return NULL;
}

/* ---- Lost-Wakeup Demonstration ---- */
/*
 * THE LOST-WAKEUP BUG:
 * If a thread calls pthread_cond_signal BEFORE any thread is waiting
 * on that CV, the signal is a no-op — it has zero effect.  If the
 * consumer then calls pthread_cond_wait, it will sleep forever because
 * the signal that would wake it already happened and was discarded.
 *
 * This is the single most common bug with condition variables.
 *
 * The correct pattern is:
 *   1. Lock the mutex.
 *   2. Modify shared state.
 *   3. Signal the CV.
 *   4. Unlock the mutex.
 *
 * And on the receiver side:
 *   1. Lock the mutex.
 *   2. while (condition false) { wait(&cv, &lock); }
 *   3. Use the shared state.
 *   4. Unlock.
 *
 * The signal must happen AFTER the wait side is already waiting, or
 * the signal is lost.  In a well-structured program the mutex ensures
 * this: the producer can't signal until it holds the lock, and the
 * consumer releases the lock atomically when it calls wait().
 *
 * But if the producer manages to run the entire critical section
 * (lock → modify → signal → unlock) before the consumer ever enters
 * the wait, the signal is lost.
 */

#if 0
/* Uncomment this block, recompile, and run.  The consumer will hang. */
static void* buggy_producer(void* arg) {
    bounded_buffer_t* bb = (bounded_buffer_t*)arg;

    /* Producer runs FIRST — no consumer is waiting yet. */
    pthread_mutex_lock(&bb->lock);
    bb->buf[bb->tail] = 42;
    bb->tail = (bb->tail + 1) % BUF_CAPACITY;
    bb->count = 1;
    pthread_cond_signal(&bb->can_consume);  /* LOST! nobody listening */
    pthread_mutex_unlock(&bb->lock);
    fprintf(stderr, "[BuggyProducer] signaled — but nobody was listening!\n");
    return NULL;
}

static void* buggy_consumer(void* arg) {
    bounded_buffer_t* bb = (bounded_buffer_t*)arg;
    sleep(1);  /* arrive late — the signal already happened */

    pthread_mutex_lock(&bb->lock);
    while (bb->count == 0) {
        /* MAY WAIT FOREVER — the signal was already sent and lost. */
        pthread_cond_wait(&bb->can_consume, &bb->lock);
    }
    int val = bb->buf[bb->head];
    bb->head = (bb->head + 1) % BUF_CAPACITY;
    bb->count--;
    pthread_mutex_unlock(&bb->lock);
    fprintf(stderr, "[BuggyConsumer] got %d\n", val);
    return NULL;
}

static void run_lost_wakeup(void) {
    bounded_buffer_t bb;
    bb_init(&bb);

    pthread_t p, c;
    /* Start producer FIRST so it runs and signals before consumer waits. */
    pthread_create(&p, NULL, buggy_producer, &bb);
    pthread_create(&c, NULL, buggy_consumer, &bb);
    pthread_join(p, NULL);
    pthread_join(c, NULL);   /* ← this join will hang */

    bb_destroy(&bb);
}
#endif

/* ---- Broadcast Demonstration ---- */
/*
 * pthread_cond_broadcast wakes EVERY thread waiting on the CV.
 * Use broadcast when the condition change could satisfy MULTIPLE
 * waiters.  Example: a single "shutdown" flag that should wake
 * all worker threads.
 *
 * In contrast, pthread_cond_signal wakes exactly one waiter (or
 * none if the queue is empty).  Using signal when multiple waiters
 * could proceed is a correctness bug — only one wakes, the rest
 * sleep forever.
 */

static volatile int shutdown_flag = 0;

typedef struct {
    int id;
    pthread_mutex_t* lock;
    pthread_cond_t* shutdown_cv;
} worker_shutdown_arg_t;

static void* worker_with_shutdown(void* arg) {
    worker_shutdown_arg_t* wa = (worker_shutdown_arg_t*)arg;

    pthread_mutex_lock(wa->lock);
    while (!shutdown_flag) {
        pthread_cond_wait(wa->shutdown_cv, wa->lock);
    }
    pthread_mutex_unlock(wa->lock);

    fprintf(stderr, "[Worker %d] received shutdown signal\n", wa->id);
    return NULL;
}

static void run_broadcast_demo(void) {
    fprintf(stderr, "\n--- Broadcast Demo ---\n");

    pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
    pthread_cond_t shutdown_cv = PTHREAD_COND_INITIALIZER;
    pthread_t workers[4];
    worker_shutdown_arg_t args[4];

    /* Start 4 workers, all waiting on the same CV. */
    for (int i = 0; i < 4; i++) {
        args[i] = (worker_shutdown_arg_t){i, &lock, &shutdown_cv};
        pthread_create(&workers[i], NULL, worker_with_shutdown, &args[i]);
    }

    nanosleep(&(struct timespec){0, 50000000L}, NULL); /* 50 ms — let workers start */

    /* Signal shutdown.  pthread_cond_signal(&shutdown_cv) would wake
     * only ONE worker, leaving the other three asleep forever.
     * broadcast wakes all four. */
    pthread_mutex_lock(&lock);
    shutdown_flag = 1;
    pthread_cond_broadcast(&shutdown_cv);
    pthread_mutex_unlock(&lock);

    for (int i = 0; i < 4; i++)
        pthread_join(workers[i], NULL);

    shutdown_flag = 0;
    pthread_mutex_destroy(&lock);
    pthread_cond_destroy(&shutdown_cv);
}

/* ---- Spurious Wakeup Simulation ---- */
/*
 * POSIX explicitly allows pthread_cond_wait to return even though
 * nobody called signal or broadcast.  This is called a SPURIOUS
 * WAKEUP.  The canonical while-loop pattern handles it.
 *
 * To simulate: we can't force the OS to produce a spurious wakeup,
 * but we can show what happens if a thread is woken and the condition
 * is still false.
 */

typedef struct {
    int id;
    bounded_buffer_t* bb;
} spurious_worker_arg_t;

static void* spurious_consumer(void* arg) {
    spurious_worker_arg_t* wa = (spurious_worker_arg_t*)arg;

    pthread_mutex_lock(&wa->bb->lock);
    /* Without the while-loop, this would crash on spurious wakeup: */
    int count = wa->bb->count;
    if (count == 0) {
        pthread_cond_wait(&wa->bb->can_consume, &wa->bb->lock);
    }
    /* BUG: if this was a spurious wakeup, bb->count is still 0,
     * and the following access reads garbage from the buffer. */
    int item = wa->bb->buf[wa->bb->head];
    pthread_mutex_unlock(&wa->bb->lock);

    fprintf(stderr, "[SpuriousDemo Consumer %d] got %d "
            "(may be garbage from empty buffer!)\n", wa->id, item);
    return NULL;
}

static void show_spurious_wakeup_danger(void) {
    fprintf(stderr, "\n--- Spurious Wakeup Danger ---\n");
    fprintf(stderr, "This code uses if() instead of while().  "
            "On spurious wakeup, it reads garbage.\n");

    bounded_buffer_t bb;
    bb_init(&bb);

    /* Start a consumer that uses the WRONG pattern (if, not while). */
    pthread_t c;
    spurious_worker_arg_t ca = {1, &bb};
    pthread_create(&c, NULL, spurious_consumer, &ca);
    pthread_join(c, NULL);

    /* Note: not actually triggering a spurious wakeup here (the OS
     * doesn't guarantee one), but the code is buggy regardless. */
    fprintf(stderr, "The if-pattern is BROKEN — always use while.\n\n");
    bb_destroy(&bb);
}

/* ---- Integrate Step 2 into a clean run ---- */

static void run_bounded_buffer(void) {
    bounded_buffer_t bb;
    bb_init(&bb);

    pthread_t producers[2];
    pthread_t consumers[3];

    worker_arg_t pa[] = {{&bb, 1, 5}, {&bb, 2, 5}};
    worker_arg_t ca[] = {{&bb, 1, 4}, {&bb, 2, 3}, {&bb, 3, 3}};

    /* Launch 2 producers and 3 consumers.  The buffer capacity is 8,
     * tighter than the 10 total items, so producers will block on
     * can_produce and consumers will block on can_consume — both CVs
     * get exercised. */
    for (int i = 0; i < 2; i++)
        pthread_create(&producers[i], NULL, producer_thread, &pa[i]);
    for (int i = 0; i < 3; i++)
        pthread_create(&consumers[i], NULL, consumer_thread, &ca[i]);

    for (int i = 0; i < 2; i++)
        pthread_join(producers[i], NULL);
    for (int i = 0; i < 3; i++)
        pthread_join(consumers[i], NULL);

    bb_destroy(&bb);
    fprintf(stderr, "[BoundedBuffer] All done.  "
            "%d items produced, %d consumed.\n", 10, 10);
}

/* ================================================================== */
/*  Main                                                              */
/* ================================================================== */

int main(void) {
    fprintf(stderr, "=== Condition Variables and Monitors ===\n\n");

    fprintf(stderr, "=== Step 1: Broken Busy-Wait ===\n");
    fprintf(stderr, "(Watch CPU usage with top/htop/Activity Monitor "
            "— one core will spike to 100%%)\n");
    run_busy_wait();
    fprintf(stderr, "Spinning wastes CPU.  A condition variable "
            "lets the thread sleep instead.\n\n");

    fprintf(stderr, "=== Step 2: Bounded Buffer (pthread_cond) ===\n");
    fprintf(stderr, "Two CVs: can_consume (empty→not-empty) "
            "and can_produce (full→not-full).\n");
    run_bounded_buffer();
    fprintf(stderr, "\n");

    run_broadcast_demo();
    fprintf(stderr, "\n");

    show_spurious_wakeup_danger();

    fprintf(stderr, "\n=== Summary ===\n");
    fprintf(stderr, "  Step 1: 100%% CPU spinning — never do this.\n");
    fprintf(stderr, "  Step 2: pthread_cond_wait/signal — threads sleep "
            "until work arrives.\n");
    fprintf(stderr, "  Broadcast: wakes ALL waiters on a CV.\n");
    fprintf(stderr, "  Key rule: ALWAYS use while(), never if(), "
            "around pthread_cond_wait.\n");
    return 0;
}
