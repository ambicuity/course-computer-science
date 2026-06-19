/*
 * Semaphores and the Classics (Producer/Consumer, Dining)
 * Phase 13 — Concurrent & Parallel Computing
 *
 * Demonstrates three classic synchronization problems using POSIX
 * semaphores (semaphore.h) and pthreads.
 *
 *   1. Producer–Consumer  (bounded buffer)
 *   2. Dining Philosophers (resource-ordering deadlock prevention)
 *   3. Readers–Writers    (first problem: readers-preference)
 *
 * Build:
 *   gcc -pthread -o prodcon main.c && ./prodcon
 */

#define _POSIX_C_SOURCE 200112L

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>
#include <pthread.h>
#include <semaphore.h>
#include <time.h>
#include <errno.h>
#include <fcntl.h>

/* ------------------------------------------------------------------ */
/*  Portable semaphore helpers                                         */
/*                                                                     */
/*  macOS has deprecated sem_init/sem_destroy (they return ENOSYS).    */
/*  sem_open with unique names works on both macOS and Linux.          */
/* ------------------------------------------------------------------ */

typedef struct {
    sem_t *ptr;
    char  name[64];
} portable_sem;

static void psem_init(portable_sem *ps, int pshared, unsigned int value,
                      const char *label, int id) {
    (void)pshared;
    /* macOS limits semaphore names to PSEMNAMLEN (31 chars),
     * so keep this short: /p6_X_YYYY */
    snprintf(ps->name, sizeof(ps->name), "/p6_%s_%d", label, id);
    sem_unlink(ps->name);
    ps->ptr = sem_open(ps->name, O_CREAT | O_EXCL, 0644, value);
    if (ps->ptr == SEM_FAILED) { perror("sem_open"); exit(EXIT_FAILURE); }
}

static void psem_wait(portable_sem *ps) {
    if (sem_wait(ps->ptr) != 0) { perror("sem_wait"); exit(EXIT_FAILURE); }
}

static void psem_post(portable_sem *ps) {
    if (sem_post(ps->ptr) != 0) { perror("sem_post"); exit(EXIT_FAILURE); }
}

static void psem_destroy(portable_sem *ps) {
    sem_close(ps->ptr);
    sem_unlink(ps->name);
}

/* ------------------------------------------------------------------ */
/*  Utility helpers                                                    */
/* ------------------------------------------------------------------ */

#define CHECK(expr, msg) do {                                          \
    if ((expr) != 0) { perror(msg); exit(EXIT_FAILURE); }              \
} while (0)

#define NANO 1000000000L

static void nsleep(long ns) {
    struct timespec ts = { .tv_sec = ns / NANO, .tv_nsec = ns % NANO };
    nanosleep(&ts, NULL);
}

/* ------------------------------------------------------------------ */
/*  1. Producer–Consumer (bounded buffer)                              */
/* ------------------------------------------------------------------ */

#define PROD_BUF_SIZE 8
#define PROD_ITEMS    32

typedef struct {
    int buf[PROD_BUF_SIZE];
    int head, tail;
    portable_sem empty;
    portable_sem full;
    pthread_mutex_t lock;
} BoundedBuffer;

static void bb_init(BoundedBuffer *bb) {
    memset(bb, 0, sizeof(*bb));
    psem_init(&bb->empty, 0, PROD_BUF_SIZE, "bb_empty", 0);
    psem_init(&bb->full,  0, 0,              "bb_full",  0);
    pthread_mutex_init(&bb->lock, NULL);
}

static void bb_destroy(BoundedBuffer *bb) {
    psem_destroy(&bb->empty);
    psem_destroy(&bb->full);
    pthread_mutex_destroy(&bb->lock);
}

static void bb_put(BoundedBuffer *bb, int val) {
    psem_wait(&bb->empty);
    pthread_mutex_lock(&bb->lock);
    bb->buf[bb->tail % PROD_BUF_SIZE] = val;
    bb->tail++;
    pthread_mutex_unlock(&bb->lock);
    psem_post(&bb->full);
}

static int bb_get(BoundedBuffer *bb) {
    psem_wait(&bb->full);
    pthread_mutex_lock(&bb->lock);
    int val = bb->buf[bb->head % PROD_BUF_SIZE];
    bb->head++;
    pthread_mutex_unlock(&bb->lock);
    psem_post(&bb->empty);
    return val;
}

typedef struct {
    BoundedBuffer *bb;
    int id;
    int items;
} ProdConArg;

static void *producer_thread(void *arg) {
    ProdConArg *a = (ProdConArg *)arg;
    for (int i = 0; i < a->items; i++) {
        int val = a->id * 1000 + i;
        bb_put(a->bb, val);
        printf("[Producer %d] put %d\n", a->id, val);
        nsleep(1000000 + rand() % 5000000);
    }
    return NULL;
}

static void *consumer_thread(void *arg) {
    ProdConArg *a = (ProdConArg *)arg;
    for (int i = 0; i < a->items; i++) {
        int val = bb_get(a->bb);
        printf("[Consumer %d] got %d\n", a->id, val);
        nsleep(2000000 + rand() % 8000000);
    }
    return NULL;
}

static void run_producer_consumer(void) {
    printf("\n========== Producer–Consumer (Bounded Buffer) ==========\n\n");

    BoundedBuffer bb;
    bb_init(&bb);

    int nproducers = 2, nconsumers = 2;
    int items_per   = PROD_ITEMS / (nproducers + nconsumers);

    pthread_t producers[4], consumers[4];
    ProdConArg pargs[4], cargs[4];

    int total_produced = 0, total_consumed = 0;
    for (int i = 0; i < nproducers; i++) {
        pargs[i].bb    = &bb;
        pargs[i].id    = i + 1;
        pargs[i].items = items_per;
        total_produced += items_per;
        CHECK(pthread_create(&producers[i], NULL, producer_thread, &pargs[i]),
              "pthread_create producer");
    }
    for (int i = 0; i < nconsumers; i++) {
        cargs[i].bb    = &bb;
        cargs[i].id    = i + 1;
        cargs[i].items = items_per;
        total_consumed += items_per;
        CHECK(pthread_create(&consumers[i], NULL, consumer_thread, &cargs[i]),
              "pthread_create consumer");
    }

    for (int i = 0; i < nproducers; i++)
        pthread_join(producers[i], NULL);
    for (int i = 0; i < nconsumers; i++)
        pthread_join(consumers[i], NULL);

    printf("\nProducer–consumer finished: %d produced, %d consumed\n",
           total_produced, total_consumed);
    bb_destroy(&bb);
}

/* ------------------------------------------------------------------ */
/*  2. Dining Philosophers                                             */
/* ------------------------------------------------------------------ */

#define NPHIL 5

static portable_sem chopsticks[NPHIL];

static void phil_init(void) {
    for (int i = 0; i < NPHIL; i++) {
        char label[16];
        snprintf(label, sizeof(label), "chop_%d", i);
        psem_init(&chopsticks[i], 0, 1, label, i);
    }
}

static void phil_destroy(void) {
    for (int i = 0; i < NPHIL; i++)
        psem_destroy(&chopsticks[i]);
}

static void think(int id) {
    printf("Philosopher %d is thinking...\n", id);
    nsleep(2000000 + rand() % 5000000);
}

static void eat(int id) {
    printf("Philosopher %d is eating...\n", id);
    nsleep(1000000 + rand() % 3000000);
}

static void *philosopher_thread(void *arg) {
    int id = *(int *)arg;
    int left, right;

    /* Resource-ordering: pick up lower-numbered chopstick first.
     * This breaks circular wait and prevents deadlock. */
    if (id < (id + 1) % NPHIL) {
        left  = id;
        right = (id + 1) % NPHIL;
    } else {
        left  = (id + 1) % NPHIL;
        right = id;
    }

    for (int i = 0; i < 3; i++) {
        think(id);

        psem_wait(&chopsticks[left]);
        printf("Philosopher %d picked up chopstick %d (left)\n", id, left);
        psem_wait(&chopsticks[right]);
        printf("Philosopher %d picked up chopstick %d (right)\n", id, right);

        eat(id);

        psem_post(&chopsticks[left]);
        psem_post(&chopsticks[right]);
        printf("Philosopher %d put down chopsticks\n", id);
    }
    return NULL;
}

static void run_dining_philosophers(void) {
    printf("\n========== Dining Philosophers ==========\n\n");

    phil_init();

    pthread_t philosophers[NPHIL];
    int ids[NPHIL];
    for (int i = 0; i < NPHIL; i++) {
        ids[i] = i;
        CHECK(pthread_create(&philosophers[i], NULL, philosopher_thread, &ids[i]),
              "pthread_create philosopher");
    }

    for (int i = 0; i < NPHIL; i++)
        pthread_join(philosophers[i], NULL);

    phil_destroy();
    printf("\nDining philosophers finished — no deadlock.\n");
}

/* ------------------------------------------------------------------ */
/*  3. Readers–Writers (first problem: readers-preference)             */
/* ------------------------------------------------------------------ */

#define RW_READERS 4
#define RW_WRITERS 2
#define RW_OPS     6

static portable_sem rw_mutex;       /* writer exclusivity */
static portable_sem rw_count_mutex; /* protects read_count */
static int   rw_read_count;

static int   rw_shared_data;

static void rw_init(void) {
    rw_read_count = 0;
    rw_shared_data = 0;
    psem_init(&rw_mutex, 0, 1, "rw_mutex", 0);
    psem_init(&rw_count_mutex, 0, 1, "rw_count", 0);
}

static void rw_destroy(void) {
    psem_destroy(&rw_mutex);
    psem_destroy(&rw_count_mutex);
}

static void *reader_thread(void *arg) {
    int id = *(int *)arg;
    for (int i = 0; i < RW_OPS; i++) {
        psem_wait(&rw_count_mutex);
        rw_read_count++;
        if (rw_read_count == 1)
            psem_wait(&rw_mutex);
        psem_post(&rw_count_mutex);

        int val = rw_shared_data;
        printf("[Reader %d] read value = %d (concurrent readers: %d)\n",
               id, val, rw_read_count);
        nsleep(500000 + rand() % 2000000);

        psem_wait(&rw_count_mutex);
        rw_read_count--;
        if (rw_read_count == 0)
            psem_post(&rw_mutex);
        psem_post(&rw_count_mutex);

        nsleep(1000000 + rand() % 3000000);
    }
    return NULL;
}

static void *writer_thread(void *arg) {
    int id = *(int *)arg;
    for (int i = 0; i < RW_OPS; i++) {
        /* Entry section */
        psem_wait(&rw_mutex);

        rw_shared_data++;
        printf("[Writer %d] wrote value = %d\n", id, rw_shared_data);
        nsleep(1000000 + rand() % 3000000);

        psem_post(&rw_mutex);

        nsleep(2000000 + rand() % 5000000);
    }
    return NULL;
}

static void run_readers_writers(void) {
    printf("\n========== Readers–Writers (readers-preference) ==========\n\n");

    rw_init();

    pthread_t readers[RW_READERS], writers[RW_WRITERS];
    int reader_ids[RW_READERS], writer_ids[RW_WRITERS];

    for (int i = 0; i < RW_READERS; i++) {
        reader_ids[i] = i + 1;
        CHECK(pthread_create(&readers[i], NULL, reader_thread, &reader_ids[i]),
              "pthread_create reader");
    }
    for (int i = 0; i < RW_WRITERS; i++) {
        writer_ids[i] = i + 1;
        CHECK(pthread_create(&writers[i], NULL, writer_thread, &writer_ids[i]),
              "pthread_create writer");
    }

    for (int i = 0; i < RW_READERS; i++)
        pthread_join(readers[i], NULL);
    for (int i = 0; i < RW_WRITERS; i++)
        pthread_join(writers[i], NULL);

    rw_destroy();
    printf("\nReaders–writers finished — final value: %d\n", rw_shared_data);
}

/* ------------------------------------------------------------------ */
/*  Main entry point                                                   */
/* ------------------------------------------------------------------ */

int main(void) {
    srand((unsigned)time(NULL));

    run_producer_consumer();
    run_dining_philosophers();
    run_readers_writers();

    printf("\nAll classic problems completed successfully.\n");
    return 0;
}
