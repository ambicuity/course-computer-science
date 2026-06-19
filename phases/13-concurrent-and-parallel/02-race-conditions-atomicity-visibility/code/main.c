/*
 * Race Conditions, Atomicity, Visibility
 * Phase 13 — Concurrent & Parallel Computing
 *
 * Race condition demos in C:
 *   Part 1 — Counter race (broken)
 *   Part 2 — Mutex fix
 *   Part 3 — C11 atomic fix
 *   Part 4 — Visibility broken (flag-based)
 *   Part 5 — Visibility fixed with atomics
 *
 * Compile:
 *   gcc -O0 -pthread -std=c11 -o race main.c    (counter race manifests)
 *   gcc -O2 -pthread -std=c11 -o race main.c    (visibility bug manifests; optional)
 * Run:     ./race
 *
 * To detect races: gcc -fsanitize=thread -O1 -g -pthread -std=c11 -o race main.c
 */

#include <stdio.h>
#include <stdlib.h>
#include <pthread.h>
#include <stdatomic.h>
#include <unistd.h>

#define NUM_INCREMENTS 10000000
#define NUM_THREADS 4
#define SPIN_LIMIT 2000000000

/* ────────────────────────────────────────────────────────
   Part 1 — Counter Race (no synchronization)
   ──────────────────────────────────────────────────────── */

volatile int plain_counter = 0;

void* increment_race(void* arg) {
    for (int i = 0; i < NUM_INCREMENTS; i++) {
        /* volatile forces separate LOAD/ADD/STORE — no RMW fusion */
        plain_counter = plain_counter + 1;
    }
    return NULL;
}

void demo_counter_race(void) {
    printf("=== Demo 1: Counter Race ===\n");
    plain_counter = 0;

    pthread_t threads[NUM_THREADS];
    for (int i = 0; i < NUM_THREADS; i++)
        pthread_create(&threads[i], NULL, increment_race, NULL);
    for (int i = 0; i < NUM_THREADS; i++)
        pthread_join(threads[i], NULL);

    printf("  Expected: %d\n", NUM_INCREMENTS * NUM_THREADS);
    printf("  Actual:   %d\n", plain_counter);
    printf("  (Lost updates due to unsynchronized increment)\n\n");
}

/* ────────────────────────────────────────────────────────
   Part 2 — Fix with Mutex
   ──────────────────────────────────────────────────────── */

pthread_mutex_t mutex_lock;
int mutex_counter = 0;

void* increment_mutex(void* arg) {
    for (int i = 0; i < NUM_INCREMENTS; i++) {
        pthread_mutex_lock(&mutex_lock);
        mutex_counter++;
        pthread_mutex_unlock(&mutex_lock);
    }
    return NULL;
}

void demo_mutex_fix(void) {
    printf("=== Demo 2: Mutex Fix ===\n");
    mutex_counter = 0;
    pthread_mutex_init(&mutex_lock, NULL);

    pthread_t threads[NUM_THREADS];
    for (int i = 0; i < NUM_THREADS; i++)
        pthread_create(&threads[i], NULL, increment_mutex, NULL);
    for (int i = 0; i < NUM_THREADS; i++)
        pthread_join(threads[i], NULL);

    printf("  Expected: %d\n", NUM_INCREMENTS * NUM_THREADS);
    printf("  Actual:   %d\n", mutex_counter);
    printf("  (Correct — mutex serializes access)\n\n");

    pthread_mutex_destroy(&mutex_lock);
}

/* ────────────────────────────────────────────────────────
   Part 3 — Fix with C11 Atomics
   ──────────────────────────────────────────────────────── */

atomic_int atomic_counter = 0;

void* increment_atomic(void* arg) {
    for (int i = 0; i < NUM_INCREMENTS; i++) {
        atomic_counter++;
    }
    return NULL;
}

void demo_atomic_fix(void) {
    printf("=== Demo 3: Atomic Fix ===\n");
    atomic_store(&atomic_counter, 0);

    pthread_t threads[NUM_THREADS];
    for (int i = 0; i < NUM_THREADS; i++)
        pthread_create(&threads[i], NULL, increment_atomic, NULL);
    for (int i = 0; i < NUM_THREADS; i++)
        pthread_join(threads[i], NULL);

    printf("  Expected: %d\n", NUM_INCREMENTS * NUM_THREADS);
    printf("  Actual:   %d\n", atomic_load(&atomic_counter));
    printf("  (Correct — atomic increment is indivisible)\n\n");
}

/* ────────────────────────────────────────────────────────
   Part 4 — Visibility Broken (flag-based shutdown)
   ────────────────────────────────────────────────────────
   Without proper memory ordering, the compiler/CPU can
   reorder stores. On ARM/Power, consumer may see ready=1
   but read data=0. With -O2, compiler may hoist the
   non-atomic 'ready' load out of the while loop, causing
   an infinite spin.
   ──────────────────────────────────────────────────────── */

int vis_data = 0;
int vis_ready = 0;

void* vis_producer(void* arg) {
    vis_data = 42;
    vis_ready = 1;  /* Can be reordered before vis_data=42 */
    return NULL;
}

void* vis_consumer(void* arg) {
    long spins = 0;
    while (vis_ready == 0) {
        spins++;
        if (spins > SPIN_LIMIT) {
            printf("  RACE: consumer timed out — never saw ready==1\n");
            printf("  (Compiler likely hoisted the load out of the loop)\n");
            return NULL;
        }
    }
    printf("  exited spin loop after %ld iterations\n", spins);
    printf("  read vis_data = %d", vis_data);
    if (vis_data != 42) printf(" (RACE: reordering detected!)");
    printf("\n");
    return NULL;
}

void demo_visibility_broken(void) {
    printf("=== Demo 4: Visibility Broken ===\n");
    vis_data = 0;
    vis_ready = 0;

    pthread_t producer, consumer;
    pthread_create(&consumer, NULL, vis_consumer, NULL);
    usleep(100000);
    pthread_create(&producer, NULL, vis_producer, NULL);

    pthread_join(producer, NULL);
    pthread_join(consumer, NULL);

    printf("  WARNING: This code has undefined behavior.\n");
    printf("  On x86 it often appears to work (TSO hides the issue).\n");
    printf("  On ARM/Power the consumer may read vis_data = 0.\n");
    printf("  Compile with -O2 and no volatile — may hang.\n\n");
}

/* ────────────────────────────────────────────────────────
   Part 5 — Visibility Fixed with C11 Atomics
   ────────────────────────────────────────────────────────
   Use release/acquire semantics to establish happens-before:
   - producer: data=42  HAPPENS-BEFORE  ready.store(1, release)
   - consumer: ready.load(acquire)  HAPPENS-BEFORE  read(data)
   Transitivity ensures the consumer sees data==42.
   ──────────────────────────────────────────────────────── */

int vis_data_fixed = 0;
atomic_int vis_ready_fixed = 0;

void* vis_producer_fixed(void* arg) {
    vis_data_fixed = 42;
    atomic_store_explicit(&vis_ready_fixed, 1, memory_order_release);
    return NULL;
}

void* vis_consumer_fixed(void* arg) {
    while (atomic_load_explicit(&vis_ready_fixed, memory_order_acquire) == 0);
    printf("  read vis_data_fixed = %d\n", vis_data_fixed);
    printf("  (Correct — acquire sees the release's writes)\n");
    return NULL;
}

void demo_visibility_fixed(void) {
    printf("=== Demo 5: Visibility Fixed (Atomics + Ordering) ===\n");
    vis_data_fixed = 0;
    atomic_store_explicit(&vis_ready_fixed, 0, memory_order_relaxed);

    pthread_t producer, consumer;
    pthread_create(&consumer, NULL, vis_consumer_fixed, NULL);
    usleep(100000);
    pthread_create(&producer, NULL, vis_producer_fixed, NULL);

    pthread_join(producer, NULL);
    pthread_join(consumer, NULL);

    printf("  Release/acquire guarantees visibility across threads.\n\n");
}

/* ────────────────────────────────────────────────────────
   main
   ──────────────────────────────────────────────────────── */

int main(void) {
    printf("════════════════════════════════════════════\n");
    printf("  Race Conditions, Atomicity, Visibility\n");
    printf("════════════════════════════════════════════\n\n");

    demo_counter_race();
    demo_mutex_fix();
    demo_atomic_fix();
    demo_visibility_broken();
    demo_visibility_fixed();

    printf("All demos complete.\n");
    return 0;
}
