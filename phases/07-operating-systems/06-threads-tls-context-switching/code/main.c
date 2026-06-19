#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <pthread.h>
#include <unistd.h>
#include <time.h>
#include <setjmp.h>
#include <sys/time.h>

/*
 * Threading demos: creation, race conditions, TLS,
 * context switch cost measurement, manual context switch.
 * Compile: gcc -o thread_demo main.c -lpthread
 * Run:     ./thread_demo
 */

/* ---------- Helper: timing ---------- */
/* Uncomment to use for custom timing:
static double now_ms(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return tv.tv_sec * 1000.0 + tv.tv_usec / 1000.0;
} */

/* ---------- create_threads ---------- */
static void *thread_func(void *arg) {
    int id = *(int *)arg;
    printf("  Thread %d: PID=%d, TID=%lu\n", id, getpid(),
           (unsigned long)pthread_self());
    return NULL;
}

static void create_threads(void) {
    printf("=== Thread Creation Demo ===\n\n");

    pthread_t threads[4];
    int ids[4] = {0, 1, 2, 3};

    for (int i = 0; i < 4; i++) {
        pthread_create(&threads[i], NULL, thread_func, &ids[i]);
    }

    for (int i = 0; i < 4; i++) {
        pthread_join(threads[i], NULL);
    }
    printf("\n");
}

/* ---------- shared_counter_race ---------- */

static int shared_counter = 0;
static pthread_mutex_t counter_mutex = PTHREAD_MUTEX_INITIALIZER;

static void *increment_unsafe(void *arg) {
    int iterations = *(int *)arg;
    for (int i = 0; i < iterations; i++) {
        shared_counter++;  /* race condition! */
    }
    return NULL;
}

static void *increment_safe(void *arg) {
    int iterations = *(int *)arg;
    for (int i = 0; i < iterations; i++) {
        pthread_mutex_lock(&counter_mutex);
        shared_counter++;
        pthread_mutex_unlock(&counter_mutex);
    }
    return NULL;
}

static void shared_counter_race(void) {
    printf("=== Shared Counter Race Condition Demo ===\n\n");

    int iterations = 100000;
    pthread_t t1, t2;

    /* Without mutex */
    shared_counter = 0;
    pthread_create(&t1, NULL, increment_unsafe, &iterations);
    pthread_create(&t2, NULL, increment_unsafe, &iterations);
    pthread_join(t1, NULL);
    pthread_join(t2, NULL);

    printf("Without mutex: expected %d, got %d (lost %d increments)\n",
           iterations * 2, shared_counter,
           iterations * 2 - shared_counter);

    /* With mutex */
    shared_counter = 0;
    pthread_create(&t1, NULL, increment_safe, &iterations);
    pthread_create(&t2, NULL, increment_safe, &iterations);
    pthread_join(t1, NULL);
    pthread_join(t2, NULL);

    printf("With mutex:    expected %d, got %d\n\n",
           iterations * 2, shared_counter);
}

/* ---------- tls_demo ---------- */

__thread int tls_value = 0;

static void *tls_worker(void *arg) {
    int id = *(int *)arg;

    /* Each thread sets its own TLS value */
    tls_value = id * 100;

    printf("  Thread %d: tls_value = %d\n", id, tls_value);

    /* Small sleep to let other threads run */
    usleep(10000);

    /* Read it back — still our own value */
    printf("  Thread %d: tls_value still = %d (not affected by other threads)\n",
           id, tls_value);
    return NULL;
}

static void tls_demo(void) {
    printf("=== Thread-Local Storage (TLS) Demo ===\n\n");

    pthread_t threads[4];
    int ids[4] = {1, 2, 3, 4};

    for (int i = 0; i < 4; i++) {
        pthread_create(&threads[i], NULL, tls_worker, &ids[i]);
    }

    for (int i = 0; i < 4; i++) {
        pthread_join(threads[i], NULL);
    }
    printf("\n");
}

/* ---------- context_switch_cost ---------- */

static void *busy_switch(void *arg) {
    volatile int *flag = (volatile int *)arg;
    int count = 0;

    while (*flag) {
        /* Yield to allow context switch */
        sched_yield();
        count++;
    }

    return (void *)(long)count;
}

static void context_switch_cost(void) {
    printf("=== Context Switch Cost Measurement ===\n\n");

    /*
     * Two threads ping-pong via sched_yield().
     * Each yield triggers a context switch.
     * We measure how many switches happen in 1 second.
     */
    volatile int running = 1;
    pthread_t t1, t2;

    pthread_create(&t1, NULL, busy_switch, (void *)&running);
    pthread_create(&t2, NULL, busy_switch, (void *)&running);

    sleep(1);
    running = 0;

    long count1, count2;
    pthread_join(t1, (void **)&count1);
    pthread_join(t2, (void **)&count2);

    long total_switches = count1 + count2;
    double cost_us = 1000000.0 / total_switches;

    printf("Total yields in 1 second: %ld\n", total_switches);
    printf("Approx context switch cost: %.2f microseconds\n", cost_us);
    printf("(This includes yield overhead, not just register save/restore)\n\n");
}

/* ---------- manual context switch (setjmp/longjmp) ---------- */

static jmp_buf env_main;
static jmp_buf env_coroutine;
static int coroutine_step = 0;

static void coroutine(void) {
    if (setjmp(env_coroutine) == 0) {
        /* Initial call — return to main */
        longjmp(env_main, 1);
    }

    /* Resumed here by longjmp from main */
    printf("  Coroutine: step %d\n", ++coroutine_step);
    longjmp(env_main, 1);

    printf("  Coroutine: step %d\n", ++coroutine_step);
    longjmp(env_main, 1);

    printf("  Coroutine: step %d\n", ++coroutine_step);
    longjmp(env_main, 1);
}

static void manual_context_switch(void) {
    printf("=== Manual Context Switch (setjmp/longjmp) ===\n\n");

    char stack[4096];
    /* In a real coroutine implementation, stack[] would be used
       as the coroutine's stack by adjusting SP before longjmp. */
    (void)stack;

    /*
     * setjmp/longjmp let us implement cooperative context switching
     * without pthreads. This is how user-space thread libraries
     * worked before kernel threads existed.
     *
     * Limitation: setjmp/longjmp don't truly switch stacks on all
     * platforms. A real coroutine implementation would adjust SP
     * to point into the coroutine's stack.
     */

    if (setjmp(env_main) == 0) {
        /* First call — start the coroutine */
        coroutine();
    }

    /* We get here via longjmp from coroutine */
    printf("  Main: resumed from coroutine (step %d)\n", coroutine_step);

    longjmp(env_coroutine, 1); /* resume coroutine */
    printf("  Main: resumed again (step %d)\n", coroutine_step);

    longjmp(env_coroutine, 1); /* resume coroutine */
    printf("  Main: resumed final time (step %d)\n", coroutine_step);

    printf("\nNote: This shows cooperative switching.\n");
    printf("Preemptive switching (like OS threads) requires timer signals.\n\n");
}

int main(void) {
    printf("Threads, TLS, and Context Switching\n");
    printf("====================================\n\n");

    create_threads();
    shared_counter_race();
    tls_demo();
    context_switch_cost();
    manual_context_switch();

    printf("Done.\n");
    return 0;
}
