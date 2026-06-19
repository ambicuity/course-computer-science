/*
 * Lesson 22: Real-Time and Embedded OS — RTOS Scheduler Simulation
 * Phase 07 — Operating Systems
 *
 * Simulates RM and EDF scheduling, detects deadline misses,
 * and demonstrates priority inversion with inheritance.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <math.h>

#define MAX_TASKS   8
#define SIM_TIME    200

typedef enum {
    TASK_READY,
    TASK_RUNNING,
    TASK_FINISHED
} TaskState;

typedef struct {
    int       id;
    int       period;
    int       wcet;           /* worst-case execution time */
    int       remaining;      /* ticks left in current instance */
    int       next_release;
    int       next_deadline;
    int       priority;       /* RM rank: higher = shorter period */
    TaskState state;
    int       instances;
    int       misses;
} Task;

/* ---- Rate-Monotonic ---- */

static void rm_priorities(Task tasks[], int n)
{
    /* Sort by period ascending (shortest period = highest priority) */
    for (int i = 1; i < n; i++) {
        Task key = tasks[i];
        int j = i - 1;
        while (j >= 0 && tasks[j].period > key.period) {
            tasks[j + 1] = tasks[j];
            j--;
        }
        tasks[j + 1] = key;
    }
    for (int i = 0; i < n; i++)
        tasks[i].priority = n - i;  /* rank 1..n */
}

static bool rm_schedulable(Task tasks[], int n)
{
    double u = 0.0;
    for (int i = 0; i < n; i++)
        u += (double)tasks[i].wcet / tasks[i].period;
    double bound = n * (pow(2.0, 1.0 / n) - 1.0);
    printf("  Utilization = %.4f, Liu-Layland bound = %.4f\n", u, bound);
    return u <= bound;
}

static void reset_tasks(Task tasks[], int n)
{
    for (int i = 0; i < n; i++) {
        tasks[i].remaining     = 0;
        tasks[i].next_release  = 0;
        tasks[i].next_deadline = 0;
        tasks[i].state         = TASK_READY;
        tasks[i].instances     = 0;
        tasks[i].misses        = 0;
    }
}

static void rm_schedule(Task tasks[], int n)
{
    printf("=== Rate-Monotonic Schedule ===\n");
    rm_priorities(tasks, n);
    printf("  Schedulability: %s\n",
           rm_schedulable(tasks, n) ? "PASS (within Liu-Layland bound)"
                                    : "BOUND EXCEEDED (may still work — test needed)");

    Task work[MAX_TASKS];
    memcpy(work, tasks, n * sizeof(Task));
    reset_tasks(work, n);

    for (int t = 0; t < SIM_TIME; t++) {
        /* Release new instances */
        for (int i = 0; i < n; i++) {
            if (t == work[i].next_release) {
                work[i].remaining     = work[i].wcet;
                work[i].next_deadline = t + work[i].period;
                work[i].next_release  = t + work[i].period;
                work[i].state         = TASK_READY;
                work[i].instances++;
            }
        }

        /* Pick highest-priority ready task */
        int best = -1;
        for (int i = 0; i < n; i++) {
            if (work[i].state == TASK_READY && work[i].remaining > 0) {
                if (best == -1 || work[i].priority > work[best].priority)
                    best = i;
            }
        }

        if (best >= 0) {
            work[best].state = TASK_RUNNING;
            work[best].remaining--;
            printf("  t=%3d: Task %d running (rem=%d)\n",
                   t, work[best].id, work[best].remaining);
            if (work[best].remaining == 0)
                work[best].state = TASK_FINISHED;
        }

        /* Deadline-miss check at end of period */
        for (int i = 0; i < n; i++) {
            if (work[i].remaining > 0 && t + 1 == work[i].next_deadline) {
                work[i].misses++;
                printf("  t=%3d: *** Task %d MISSED DEADLINE ***\n", t + 1, work[i].id);
            }
        }
    }

    printf("  Results:\n");
    for (int i = 0; i < n; i++)
        printf("    Task %d: %d instances, %d misses\n",
               work[i].id, work[i].instances, work[i].misses);
}

/* ---- Earliest-Deadline-First ---- */

static void edf_schedule(Task tasks[], int n)
{
    printf("=== Earliest-Deadline-First Schedule ===\n");

    Task work[MAX_TASKS];
    memcpy(work, tasks, n * sizeof(Task));
    reset_tasks(work, n);

    for (int t = 0; t < SIM_TIME; t++) {
        for (int i = 0; i < n; i++) {
            if (t == work[i].next_release) {
                work[i].remaining     = work[i].wcet;
                work[i].next_deadline = t + work[i].period;
                work[i].next_release  = t + work[i].period;
                work[i].state         = TASK_READY;
                work[i].instances++;
            }
        }

        /* Pick task with earliest absolute deadline */
        int best = -1;
        for (int i = 0; i < n; i++) {
            if (work[i].state == TASK_READY && work[i].remaining > 0) {
                if (best == -1 || work[i].next_deadline < work[best].next_deadline)
                    best = i;
            }
        }

        if (best >= 0) {
            work[best].state = TASK_RUNNING;
            work[best].remaining--;
            printf("  t=%3d: Task %d running (deadline=%d, rem=%d)\n",
                   t, work[best].id, work[best].next_deadline, work[best].remaining);
            if (work[best].remaining == 0)
                work[best].state = TASK_FINISHED;
        }

        /* Deadline-miss check */
        for (int i = 0; i < n; i++) {
            if (work[i].remaining > 0 && t + 1 == work[i].next_deadline) {
                work[i].misses++;
                printf("  t=%3d: *** Task %d MISSED DEADLINE ***\n", t + 1, work[i].id);
            }
        }
    }

    printf("  Results:\n");
    for (int i = 0; i < n; i++)
        printf("    Task %d: %d instances, %d misses\n",
               work[i].id, work[i].instances, work[i].misses);
}

/* ---- Priority Inversion Demo ---- */

typedef struct {
    const char *name;
    int  base_pri;
    int  effective_pri;
    bool holds_mutex;
} Actor;

static void priority_inheritance_demo(void)
{
    printf("=== Priority Inversion Demonstration ===\n\n");

    /* --- Scenario WITHOUT inheritance --- */
    printf("--- Without priority inheritance ---\n");
    printf("  t=0: Low  acquires mutex M\n");
    printf("  t=1: High requests M -> BLOCKED (Low holds it)\n");
    printf("  t=2: Med  arrives   -> preempts Low (Med pri 2 > Low pri 1)\n");
    printf("  t=5: Med  finishes  -> Low resumes\n");
    printf("  t=6: Low  releases M -> High acquires M, runs\n");
    printf("  => High waited 6 ticks (blocked by Med through Low)\n\n");

    /* --- Scenario WITH inheritance --- */
    printf("--- With priority inheritance ---\n");
    printf("  t=0: Low  acquires mutex M\n");
    printf("  t=1: High requests M -> BLOCKED, Low inherits High's priority (1 -> 3)\n");
    printf("  t=2: Med  arrives   -> CANNOT preempt Low (Low effective pri 3 > Med pri 2)\n");
    printf("  t=3: Low  releases M -> Low priority restored (3 -> 1)\n");
    printf("  t=3: High acquires M -> runs immediately\n");
    printf("  => High waited only 2 ticks (Low's actual critical-section work)\n");
}

/* ---- Main ---- */

int main(void)
{
    Task tasks[] = {
        { .id = 1, .period = 10, .wcet = 3 },
        { .id = 2, .period = 15, .wcet = 4 },
        { .id = 3, .period = 35, .wcet = 8 },
    };
    int n = (int)(sizeof(tasks) / sizeof(tasks[0]));

    rm_schedule(tasks, n);
    printf("\n");

    edf_schedule(tasks, n);
    printf("\n");

    priority_inheritance_demo();

    return 0;
}
