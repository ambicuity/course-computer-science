# Lesson 22: Real-Time and Embedded OS

## Core Concepts

A real-time operating system (RTOS) is one where **correctness depends not only on the result but on when the result is delivered**. If a computation finishes but arrives too late, the system has failed.

This is fundamentally different from a general-purpose OS like Linux or Windows, where throughput and average-case latency matter more than guaranteed timing.

## Hard Real-Time vs. Soft Real-Time

**Hard real-time** systems have deadlines that **must not be missed**. Missing a deadline means system failure, potentially with catastrophic consequences.

| Domain | Deadline miss consequence | Typical deadline |
|--------|--------------------------|-----------------|
| Airbag controller | Occupant death | 1–5 ms |
| Pacemaker | Cardiac event | 100–300 ms |
| Fly-by-wire control | Aircraft instability | 1–10 ms |
| Nuclear reactor SCRAM | Uncontrolled reaction | 1–100 ms |

Hard real-time systems are **formally verified** for schedulability before deployment. You prove mathematically that every task meets every deadline under worst-case conditions.

**Soft real-time** systems have deadlines that **should be met most of the time**. Missing a deadline degrades quality but does not cause failure.

| Domain | Deadline miss consequence | Typical deadline |
|--------|--------------------------|-----------------|
| Video frame delivery | Frame drop, jitter | 16.7 ms (60 fps) |
| Audio playback | Buffer underrun, pop | 5–20 ms |
| Game engine tick | Stutter, input lag | 16.7 ms |
| Stock trading system | Missed opportunity | 1–100 µs |

Soft real-time systems use statistical guarantees: "99.9% of frames delivered within 20 ms."

## Real-Time Scheduling

The central problem: given a set of tasks with known periods and execution times, can we guarantee all deadlines are met?

**Task model:** Each task τ_i has:
- **Period (T_i)** — time between successive activations
- **Worst-case execution time (C_i)** — longest the task can take on the target hardware
- **Deadline (D_i)** — usually equals the period

**Utilization:** U = Σ(C_i / T_i). The fraction of CPU time consumed by all tasks. If U > 1, the system is definitely not schedulable.

### Rate-Monotonic (RM)

**Fixed-priority, static assignment.** Priority is inversely proportional to period — shorter period means higher priority.

```
Task  Period  WCET  Priority
τ1    10      3     HIGH (shortest period)
τ2    15      4     MEDIUM
τ3    35      8     LOW (longest period)
```

RM is **optimal among fixed-priority algorithms**: if any fixed-priority schedule can meet all deadlines, RM can too.

**Schedulability test (Liu & Layland bound):**
A task set with n tasks is schedulable under RM if:

```
U ≤ n(2^(1/n) - 1)
```

| n  | Bound  |
|----|--------|
| 1  | 1.000  |
| 2  | 0.828  |
| 3  | 0.780  |
| 4  | 0.757  |
| ∞  | 0.693 (ln 2) |

This bound is **pessimistic** — many task sets with U > 0.693 are schedulable, but the bound guarantees it without simulation.

### Earliest-Deadline-First (EDF)

**Dynamic priority.** At every scheduling decision, run the task whose deadline is nearest, regardless of period.

EDF is **optimal among all algorithms**: if any schedule can meet all deadlines, EDF can. It can schedule task sets up to **U ≤ 1.0** (100% utilization).

```
Time  Ready tasks     EDF picks
0     τ1(d=10),τ2(d=15)  τ1 (closer deadline)
3     τ2(d=15)           τ2
7     (idle until t=10)
10    τ1(d=20),τ2(d=30)  τ1
```

EDF is harder to implement in practice because priorities change at runtime. Preemption overhead is higher.

### Priority Inheritance Protocol

**The priority inversion problem:** A low-priority task holds a lock. A high-priority task tries to acquire the same lock and blocks. Meanwhile, a medium-priority task preempts the low-priority task (since it has higher priority). The high-priority task now waits on the medium-priority task, through no fault of its own.

This famously caused the Mars Pathfinder resets in 1997.

```
Priority inversion timeline:
  t=0   Low acquires Mutex M
  t=1   High tries to acquire M → BLOCKS
  t=2   Medium arrives → preempts Low
  t=5   Medium finishes
  t=6   Low resumes, finally releases M
  t=7   High acquires M, runs
  
  High waited 7 ticks instead of 1 (Low's remaining time).
```

**Priority inheritance:** When High blocks on a mutex held by Low, Low temporarily **inherits High's priority**. Medium cannot preempt Low, so Low finishes its critical section faster and High gets the resource sooner.

```
With priority inheritance:
  t=0   Low acquires Mutex M
  t=1   High tries to acquire M → BLOCKS, Low inherits HIGH priority
  t=2   Medium arrives → cannot preempt Low (Low is now HIGH)
  t=3   Low releases M → priority drops back to LOW
  t=3   High acquires M, runs
  
  High waited only 2 ticks (Low's actual remaining work).
```

## Embedded OS

Embedded systems run on constrained hardware: limited RAM (often KB, not MB), no MMU, small flash storage. The OS must be tiny, deterministic, and reliable.

### Key Embedded RTOSes

**FreeRTOS** — The most widely deployed RTOS. Runs on microcontrollers from ESP32 to STM32. Kernel is ~9K lines of C. MIT licensed. Used by AWS IoT, industrial controllers, medical devices.

**Zephyr** — Linux Foundation project. Modern, modular RTOS with device tree support, networking stack, and BLE. Growing ecosystem. Apache 2.0 licensed.

**QNX** — Commercial microkernel RTOS. POSIX-compliant. Used in automotive (digital instrument clusters), medical devices, and BlackBerry phones. Rock-solid reliability.

### RTOS Kernel Components

```
┌─────────────────────────────────┐
│         Application             │
├─────────────────────────────────┤
│  Tasks  │  Semaphores │  Queues │  ← IPC primitives
├─────────┴─────────────┴─────────┤
│         Scheduler               │  ← RM, EDF, or priority-based
├─────────────────────────────────┤
│    Timer    │  Memory Pools     │  ← Tick interrupt, fixed-block alloc
├─────────────────────────────────┤
│    Hardware Abstraction Layer   │
├─────────────────────────────────┤
│         Hardware (MCU)          │
└─────────────────────────────────┘
```

No virtual memory. No demand paging. No swap. Memory is allocated from **pools** of fixed-size blocks — no fragmentation, deterministic allocation time. The scheduler is tick-driven (SysTick on ARM) or event-driven. IPC uses semaphores, mutexes, message queues — all with bounded wait times.

## Build It: RTOS Scheduler Simulation

We simulate an RTOS scheduler in userspace C. The simulation tracks task state, computes RM priorities, runs the RM and EDF algorithms, detects deadline misses, and demonstrates priority inversion with inheritance.

### Step 1: Data Structures

```c
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
    TASK_WAITING,
    TASK_FINISHED
} TaskState;

typedef struct {
    int     id;
    int     period;
    int     wcet;
    int     deadline;
    int     remaining;      // execution time left in current instance
    int     next_release;
    int     next_deadline;
    int     priority;       // higher = more important (RM: shorter period)
    TaskState state;
    int     instances;      // total instances released
    int     misses;         // deadline misses
} Task;
```

### Step 2: Rate-Monotonic Scheduling

```c
void rm_priorities(Task tasks[], int n) {
    // Sort by period ascending → assign priority descending
    for (int i = 0; i < n; i++)
        tasks[i].priority = tasks[i].period; // lower period = higher priority
    // Simple insertion sort by period
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
        tasks[i].priority = n - i; // rank 1..n
}

bool rm_schedulable(Task tasks[], int n) {
    double u = 0.0;
    for (int i = 0; i < n; i++)
        u += (double)tasks[i].wcet / tasks[i].period;
    double bound = n * (pow(2.0, 1.0 / n) - 1.0);
    printf("  Utilization = %.4f, Liu-Layland bound = %.4f\n", u, bound);
    return u <= bound;
}

void rm_schedule(Task tasks[], int n) {
    printf("=== Rate-Monotonic Schedule ===\n");
    rm_priorities(tasks, n);
    printf("  Schedulability: %s\n", rm_schedulable(tasks, n) ? "PASS" : "FAIL (may still work)");

    // Reset state
    for (int i = 0; i < n; i++) {
        tasks[i].remaining = 0;
        tasks[i].next_release = 0;
        tasks[i].state = TASK_READY;
        tasks[i].instances = 0;
        tasks[i].misses = 0;
    }

    for (int t = 0; t < SIM_TIME; t++) {
        // Release new instances
        for (int i = 0; i < n; i++) {
            if (t == tasks[i].next_release) {
                tasks[i].remaining = tasks[i].wcet;
                tasks[i].next_deadline = t + tasks[i].period;
                tasks[i].next_release = t + tasks[i].period;
                tasks[i].state = TASK_READY;
                tasks[i].instances++;
            }
        }

        // Pick highest-priority ready task
        int best = -1;
        for (int i = 0; i < n; i++) {
            if (tasks[i].state == TASK_READY && tasks[i].remaining > 0) {
                if (best == -1 || tasks[i].priority > tasks[best].priority)
                    best = i;
            }
        }

        if (best >= 0) {
            tasks[best].state = TASK_RUNNING;
            tasks[best].remaining--;
            printf("  t=%3d: Task %d running (rem=%d)\n", t, tasks[best].id, tasks[best].remaining);
            if (tasks[best].remaining == 0)
                tasks[best].state = TASK_FINISHED;
        }

        // Check deadline misses at period boundaries
        for (int i = 0; i < n; i++) {
            if (t == tasks[i].next_deadline - tasks[i].period + tasks[i].period - 1) {
                if (tasks[i].state != TASK_FINISHED && tasks[i].remaining > 0) {
                    tasks[i].misses++;
                    printf("  t=%3d: *** Task %d MISSED DEADLINE ***\n", t, tasks[i].id);
                }
            }
        }
    }

    printf("  Results:\n");
    for (int i = 0; i < n; i++)
        printf("    Task %d: %d instances, %d misses\n",
               tasks[i].id, tasks[i].instances, tasks[i].misses);
}
```

### Step 3: Earliest-Deadline-First Scheduling

```c
void edf_schedule(Task tasks[], int n) {
    printf("=== Earliest-Deadline-First Schedule ===\n");

    for (int i = 0; i < n; i++) {
        tasks[i].remaining = 0;
        tasks[i].next_release = 0;
        tasks[i].state = TASK_READY;
        tasks[i].instances = 0;
        tasks[i].misses = 0;
    }

    for (int t = 0; t < SIM_TIME; t++) {
        for (int i = 0; i < n; i++) {
            if (t == tasks[i].next_release) {
                tasks[i].remaining = tasks[i].wcet;
                tasks[i].next_deadline = t + tasks[i].period;
                tasks[i].next_release = t + tasks[i].period;
                tasks[i].state = TASK_READY;
                tasks[i].instances++;
            }
        }

        // Pick task with earliest deadline
        int best = -1;
        for (int i = 0; i < n; i++) {
            if (tasks[i].state == TASK_READY && tasks[i].remaining > 0) {
                if (best == -1 || tasks[i].next_deadline < tasks[best].next_deadline)
                    best = i;
            }
        }

        if (best >= 0) {
            tasks[best].state = TASK_RUNNING;
            tasks[best].remaining--;
            printf("  t=%3d: Task %d running (deadline=%d, rem=%d)\n",
                   t, tasks[best].id, tasks[best].next_deadline, tasks[best].remaining);
            if (tasks[best].remaining == 0)
                tasks[best].state = TASK_FINISHED;
        }

        for (int i = 0; i < n; i++) {
            if (tasks[i].remaining > 0 && t >= tasks[i].next_deadline) {
                tasks[i].misses++;
                printf("  t=%3d: *** Task %d MISSED DEADLINE ***\n", t, tasks[i].id);
                tasks[i].state = TASK_FINISHED;
            }
        }
    }

    printf("  Results:\n");
    for (int i = 0; i < n; i++)
        printf("    Task %d: %d instances, %d misses\n",
               tasks[i].id, tasks[i].instances, tasks[i].misses);
}
```

### Step 4: Priority Inversion Demonstration

```c
typedef struct {
    const char *name;
    int priority;       // 1=low, 2=med, 3=high
    int effective_pri;  // changes with inheritance
    bool holds_mutex;
    int wait_time;
} Actor;

void priority_inheritance_demo(void) {
    printf("=== Priority Inversion Demo ===\n\n");

    // --- WITHOUT inheritance ---
    printf("--- Without priority inheritance ---\n");
    Actor low  = {"Low",  1, 1, false, 0};
    Actor med  = {"Med",  2, 2, false, 0};
    Actor high = {"High", 3, 3, false, 0};

    low.holds_mutex = true;
    int mutex_owner_idx = 0;
    Actor *actors[] = {&low, &med, &high};

    printf("  t=0: Low acquires mutex M\n");
    printf("  t=1: High requests M → BLOCKED (Low holds it)\n");

    // Med preempts Low (Med has higher base priority)
    printf("  t=2: Med arrives → preempts Low (Med pri %d > Low pri %d)\n", med.priority, low.priority);
    printf("  t=5: Med finishes (Low delayed by Med)\n");
    low.wait_time += 3; // delayed by Med
    printf("  t=6: Low resumes, releases M\n");
    low.holds_mutex = false;
    printf("  t=7: High acquires M, runs\n");
    printf("  => High waited 6 ticks (suffered priority inversion via Med)\n\n");

    // --- WITH inheritance ---
    printf("--- With priority inheritance ---\n");
    low.holds_mutex = true;
    low.effective_pri = 1;
    med.effective_pri = 2;
    high.effective_pri = 3;

    printf("  t=0: Low acquires mutex M\n");
    printf("  t=1: High requests M → BLOCKED (Low holds it)\n");
    printf("  => Low inherits High's priority: %d → %d\n", low.effective_pri, high.effective_pri);
    low.effective_pri = high.effective_pri;

    printf("  t=2: Med arrives → CANNOT preempt Low (Low effective pri %d >= Med pri %d)\n",
           low.effective_pri, med.priority);
    printf("  t=3: Low finishes critical section, releases M\n");
    printf("  => Low priority restored: %d → 1\n", low.effective_pri);
    low.effective_pri = 1;
    low.holds_mutex = false;

    printf("  t=3: High acquires M, runs immediately\n");
    printf("  => High waited only 2 ticks (Low's actual work)\n");
}
```

### Step 5: Main — Run Everything

```c
int main(void) {
    // Example task set
    Task tasks[] = {
        { .id = 1, .period = 10, .wcet = 3 },
        { .id = 2, .period = 15, .wcet = 4 },
        { .id = 3, .period = 35, .wcet = 8 },
    };
    int n = sizeof(tasks) / sizeof(tasks[0]);

    rm_schedule(tasks, n);
    printf("\n");

    edf_schedule(tasks, n);
    printf("\n");

    priority_inheritance_demo();

    return 0;
}
```

## Use It

**Automotive:** Engine control units (ECUs) run AUTOSAR-compliant RTOSes. Engine timing events (spark injection, fuel pump) are hard real-time — missed deadlines damage the engine. Infotainment is soft real-time.

**Aerospace:** Flight control computers use triple-redundant RTOSes (often VxWorks or INTEGRITY). The Mars rover runs VxWorks — and priority inheritance was added to it after the Pathfinder incident.

**Industrial control:** PLCs running real-time kernels control conveyor belts, robotic arms, and chemical processes. Sampling rates of 1–10 kHz require deterministic sub-millisecond response.

**FreeRTOS internals:** `tasks.c` contains the scheduler. `xTaskIncrementTick()` handles timer interrupts. `vTaskSwitchContext()` performs context switching. The ready list is a priority-ordered linked list.

## Read the Source

- `FreeRTOS/tasks.c` — the core scheduler: `xTaskIncrementTick()`, `vTaskSwitchContext()`
- `FreeRTOS/queue.c` — message queues with blocking send/receive
- `Zephyr/kernel/sched.c` — multi-queue ready list, priority-based preemption
- `QNX source` — microkernel with message-passing IPC

## Ship It

The RTOS scheduler simulation is the reusable artifact. Compile with:

```bash
gcc -o rtos_sim main.c -lm
./rtos_sim
```

The output shows RM scheduling decisions, EDF scheduling, deadline misses, and the priority inversion scenario.

## Exercises

1. **Easy** — Modify the task set to `{T=6,C=2}, {T=8,C=1}, {T=12,C=3}`. Run both RM and EDF. Report which (if any) deadlines are missed.

2. **Medium** — Implement **Deadline Monotonic** scheduling (priority based on relative deadline, not period). When D < T, DM differs from RM. Add a test case where DM succeeds but RM fails.

3. **Hard** — Implement the **Priority Ceiling Protocol** (PCP). Each mutex has a ceiling equal to the highest priority of any task that may lock it. A task's effective priority is raised to the ceiling when it acquires any mutex. Compare the worst-case blocking time against priority inheritance.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Hard real-time | "Guaranteed deadlines" | System failure if any deadline is missed; formal schedulability proof required |
| Soft real-time | "Usually meets deadlines" | Quality degrades on miss; statistical guarantee acceptable |
| RM (Rate-Monotonic) | "Priority by period" | Fixed-priority assignment: shorter period → higher priority; optimal among fixed-priority schemes |
| EDF (Earliest-Deadline-First) | "Dynamic priority" | Schedule task with nearest deadline next; optimal among all algorithms (up to U ≤ 1) |
| Utilization (U) | "CPU load" | Σ(C_i/T_i); fraction of CPU consumed by periodic tasks |
| Priority inversion | "Low blocks high" | Low-priority task holds resource needed by high-priority task; medium-priority tasks extend the blocking |
| Priority inheritance | "Borrow priority" | Mutex holder temporarily inherits priority of highest-priority waiter, reducing unbounded blocking |
| WCET | "Worst-case time" | Worst-case execution time — measured or estimated upper bound for a task on target hardware |

## Further Reading

- Liu, C.L. and Layland, J.W. (1973). "Scheduling Algorithms for Multiprogramming in a Hard-Real-Time Environment." *JACM*, 20(1).
- Buttazzo, G.C. (2011). *Hard Real-Time Computing Systems*. Springer.
- FreeRTOS documentation: https://www.freertos.org/implementation/
- "What Really Happened on Mars?" — Mike Jones, 1997. Priority inversion on Pathfinder.
