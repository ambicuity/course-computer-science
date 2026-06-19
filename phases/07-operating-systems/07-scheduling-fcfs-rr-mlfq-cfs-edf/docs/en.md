# Lesson 07: Scheduling — FCFS, RR, MLFQ, CFS, EDF

## Core Concepts

The CPU scheduler decides which ready process runs next. Every multiprogrammed OS needs one. The scheduler's choices shape system performance in measurable ways.

**Goals (often competing):**
- **Throughput** — maximize processes completed per unit time
- **Turnaround time** — minimize time from submission to completion
- **Response time** — minimize time from submission to first response
- **Fairness** — no process starves; all get a proportional share

No single algorithm optimizes all four simultaneously. The choice depends on workload: batch, interactive, or real-time.

## First-Come-First-Serve (FCFS)

The simplest policy: run processes in arrival order. A FIFO queue holds ready processes. The CPU runs the head until it blocks or finishes.

```
Arrival  Burst
  P1      0     24
  P2      1      3
  P3      2      3

Gantt: |====P1====|==P2==|==P3==|
       0         24    27    30
```

**Turnaround:** P1 = 24, P2 = 26, P3 = 28. Average = 26.
**Convoy effect:** one long CPU burst holds up many short processes. I/O-bound processes wait behind CPU-bound ones. Poor response time for short jobs.

FCFS is non-preemptive. Once a process starts, it runs to completion or I/O block. Simple to implement. Fair in a narrow sense but unfair to short jobs.

## Round Robin (RR)

Each process gets a fixed **time quantum** (typically 10–100 ms). When the quantum expires, the process is preempted and moved to the back of the ready queue.

```
Quantum = 4

Gantt: |P1|P2|P3|P1|P2|P3|P1|P1|
       0  4  8 12 16 20 24 28 32
```

**Key tradeoffs:**
- Small quantum → more context switches, better response time
- Large quantum → fewer switches, approaches FCFS behavior
- Rule of thumb: 80% of bursts should finish within one quantum

RR is preemptive and fair. Every process gets equal CPU share. Response time is good for interactive workloads. Turnaround time depends on quantum size — smaller quantum increases average turnaround.

## Multi-Level Feedback Queue (MLFQ)

Multiple priority queues. Processes start in the highest queue. If a process uses its full quantum, it drops to a lower queue. Lower-priority queues get larger quanta.

```
Queue 0 (high): quantum = 8ms
Queue 1 (med):  quantum = 16ms
Queue 2 (low):  quantum = 32ms
```

**Rules:**
1. New processes enter the top queue
2. If a process uses its full quantum, demote it
3. If a process gives up CPU before quantum expires, it stays (I/O-bound = interactive)
4. Periodically boost all processes to top queue (prevents starvation)

This lets the scheduler learn workload behavior. Interactive processes stay in high queues (short bursts, good response). CPU-bound processes sink to low queues (long bursts, but they eventually finish). Linux's `O(1)` scheduler and FreeBSD's scheduler use MLFQ variants.

## Completely Fair Scheduler (CFS)

Linux's default scheduler since kernel 2.6.23. Models ideal CPU sharing: N processes each get 1/N of CPU time.

**Mechanism:**
- Each task has **vruntime** (virtual runtime) — actual runtime weighted by nice value
- CFS keeps tasks in a **red-black tree** sorted by vruntime
- Pick the leftmost node (smallest vruntime) — O(log n) insert/delete, O(1) pick
- When a task runs, its vruntime increases; when it sleeps, it doesn't

```
         [P3: v=5]
        /          \
  [P1: v=3]    [P5: v=8]
  /       \
[P2: v=1] [P4: v=4]

Pick: P2 (leftmost, smallest vruntime)
```

CFS is preemptive. A running task is preempted when its vruntime exceeds another task's vruntime by more than the target latency. Nice values adjust weight — a process with nice -5 gets more CPU than one with nice 0.

## Earliest Deadline First (EDF)

For real-time systems. Each task has a deadline. The scheduler always picks the task whose deadline is nearest.

**Optimality:** EDF is optimal among all online schedulers for uniprocessor real-time scheduling. If any algorithm can meet all deadlines, EDF can.

```
Task    Arrival   Burst   Deadline
 T1       0        3        7
 T2       2        2        6
 T3       4        1        8

t=0: run T1 (deadline 7)
t=2: T2 arrives (deadline 6 < 7), preempt T1, run T2
t=4: T2 done, T3 arrives (deadline 8), resume T1
t=5: T1 done, run T3
t=6: T3 done — all deadlines met
```

EDF is used in Linux's `SCHED_DEADLINE` class. If system is overloaded (total utilization > 1), deadlines will be missed — admission control is essential.

## Metrics Compared

| Metric | FCFS | RR (small q) | MLFQ | CFS | EDF |
|--------|------|---------------|------|-----|-----|
| Throughput | High | Medium | Medium | High | High |
| Turnaround | Poor for short | Higher avg | Adaptive | Balanced | Deadline-based |
| Response | Poor | Excellent | Excellent | Good | Deadline-based |
| Fairness | Low | High | Adaptive | High | Deadline-based |
| Preemptive | No | Yes | Yes | Yes | Yes |

## Build It

Write a scheduler simulator in C and Rust. Implement FCFS, Round Robin, and MLFQ. Simulate a set of processes with arrival times and burst lengths. Print Gantt charts and compute average turnaround, wait, and response times for each algorithm.

## Use It

Linux uses CFS for normal processes and EDF (`SCHED_DEADLINE`) for real-time. macOS uses a variant of MLFQ. Windows uses a multi-level feedback queue with 32 priority levels. Android uses CFS with `schedtune` for boosting foreground apps.

## Ship It

Your scheduler simulator should accept a process table (PID, arrival time, burst time, priority) and output Gantt charts and metrics for each algorithm. Compare FCFS vs RR vs MLFQ on the same workload.

## Exercises

### Level 1 — Concept Check
Five processes arrive at t=0 with bursts {6, 3, 1, 7, 4}. Calculate the average turnaround time for FCFS and RR with quantum=2. Show your work.

### Level 2 — Implementation
Extend the simulator to support preemptive SJF (shortest remaining time first). Add it to the comparison table. Handle processes arriving at different times.

### Level 3 — Design
Design a scheduler for a system that runs both interactive (response time < 50ms) and batch (maximize throughput) workloads. Describe your queue structure, priority rules, and how you prevent starvation. Implement it.
