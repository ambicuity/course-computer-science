#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

#define MAX_PROCS 64
#define MLFQ_LEVELS 3
#define MAX_TIME 500

typedef enum { READY, RUNNING, DONE } State;

typedef struct {
    int pid;
    int arrival;
    int burst;
    int remaining;
    int priority;
    State state;
    int start_time;
    int finish_time;
    int wait_time;
    int response_time;
    bool started;
} Process;

typedef struct {
    int pid;
    int start;
    int end;
} GanttEntry;

static GanttEntry gantt[MAX_TIME * 2];
static int gantt_len;

static void gantt_push(int pid, int start, int end) {
    if (gantt_len > 0 && gantt[gantt_len - 1].pid == pid &&
        gantt[gantt_len - 1].end == start) {
        gantt[gantt_len - 1].end = end;
    } else {
        gantt[gantt_len].pid = pid;
        gantt[gantt_len].start = start;
        gantt[gantt_len].end = end;
        gantt_len++;
    }
}

static void print_gantt(const char *label) {
    printf("\n%s Gantt Chart:\n", label);
    printf("|");
    for (int i = 0; i < gantt_len; i++)
        printf(" P%d (%d-%d) |", gantt[i].pid, gantt[i].start, gantt[i].end);
    printf("\n");
}

static void print_metrics(Process procs[], int n) {
    double total_turn = 0, total_wait = 0, total_resp = 0;
    printf("\n%-5s %-8s %-6s %-8s %-8s\n", "PID", "Turnaround", "Wait", "Response", "Finish");
    printf("-------------------------------------------\n");
    for (int i = 0; i < n; i++) {
        int turn = procs[i].finish_time - procs[i].arrival;
        int wait = turn - procs[i].burst;
        int resp = procs[i].start_time - procs[i].arrival;
        total_turn += turn;
        total_wait += wait;
        total_resp += resp;
        printf("P%-4d %-8d %-6d %-8d %-8d\n",
               procs[i].pid, turn, wait, resp, procs[i].finish_time);
    }
    printf("-------------------------------------------\n");
    printf("Avg   %-8.2f %-6.2f %-8.2f\n",
           total_turn / n, total_wait / n, total_resp / n);
}

static int cmp_arrival(const void *a, const void *b) {
    const Process *pa = (const Process *)a;
    const Process *pb = (const Process *)b;
    if (pa->arrival != pb->arrival) return pa->arrival - pb->arrival;
    return pa->pid - pb->pid;
}

static void copy_procs(Process dst[], Process src[], int n) {
    memcpy(dst, src, n * sizeof(Process));
    for (int i = 0; i < n; i++) {
        dst[i].remaining = dst[i].burst;
        dst[i].state = READY;
        dst[i].started = false;
    }
}

/* ── FCFS ─────────────────────────────────────────────── */

void fcfs(Process orig[], int n) {
    Process p[MAX_PROCS];
    copy_procs(p, orig, n);
    qsort(p, n, sizeof(Process), cmp_arrival);
    gantt_len = 0;

    int time = 0, done = 0;
    while (done < n) {
        int idx = -1;
        for (int i = 0; i < n; i++) {
            if (p[i].state == READY && p[i].arrival <= time) {
                idx = i;
                break;
            }
        }
        if (idx < 0) { time++; continue; }

        if (!p[idx].started) {
            p[idx].started = true;
            p[idx].start_time = time;
            p[idx].response_time = time - p[idx].arrival;
        }
        p[idx].state = RUNNING;
        int start = time;
        time += p[idx].burst;
        p[idx].finish_time = time;
        p[idx].remaining = 0;
        p[idx].state = DONE;
        done++;
        gantt_push(p[idx].pid, start, time);
    }
    print_gantt("FCFS");
    print_metrics(p, n);
}

/* ── Round Robin ──────────────────────────────────────── */

void round_robin(Process orig[], int n, int quantum) {
    Process p[MAX_PROCS];
    copy_procs(p, orig, n);
    qsort(p, n, sizeof(Process), cmp_arrival);
    gantt_len = 0;

    int queue[MAX_PROCS], head = 0, tail = 0, qsize = 0;
    bool inq[MAX_PROCS] = {false};

    #define ENQ(i) do { queue[tail] = i; tail = (tail + 1) % MAX_PROCS; qsize++; inq[i] = true; } while (0)
    #define DEQ()   ({ int _i = queue[head]; head = (head + 1) % MAX_PROCS; qsize--; inq[_i] = false; _i; })

    int time = 0, done = 0, next = 0;
    while (done < n) {
        while (next < n && p[next].arrival <= time) {
            if (!inq[next] && p[next].state != DONE) { ENQ(next); }
            next++;
        }
        if (qsize == 0) { time++; continue; }

        int idx = DEQ();
        if (!p[idx].started) {
            p[idx].started = true;
            p[idx].start_time = time;
        }
        p[idx].state = RUNNING;
        int run = p[idx].remaining < quantum ? p[idx].remaining : quantum;
        int start = time;
        time += run;
        p[idx].remaining -= run;

        while (next < n && p[next].arrival <= time) {
            if (!inq[next] && p[next].state != DONE) { ENQ(next); }
            next++;
        }

        if (p[idx].remaining == 0) {
            p[idx].state = DONE;
            p[idx].finish_time = time;
            done++;
        } else {
            p[idx].state = READY;
            ENQ(idx);
        }
        gantt_push(p[idx].pid, start, time);
    }
    char label[32];
    snprintf(label, sizeof label, "RR (q=%d)", quantum);
    print_gantt(label);
    print_metrics(p, n);
    #undef ENQ
    #undef DEQ
}

/* ── MLFQ ─────────────────────────────────────────────── */

void mlfq(Process orig[], int n) {
    Process p[MAX_PROCS];
    copy_procs(p, orig, n);
    qsort(p, n, sizeof(Process), cmp_arrival);
    gantt_len = 0;

    int quantums[MLFQ_LEVELS] = {8, 16, 32};
    int queues[MLFQ_LEVELS][MAX_PROCS];
    int heads[MLFQ_LEVELS] = {0}, tails[MLFQ_LEVELS] = {0}, sizes[MLFQ_LEVELS] = {0};
    int level[MAX_PROCS];
    for (int i = 0; i < n; i++) level[i] = 0;

    #define ENQ(q, i) do { queues[q][tails[q]] = i; tails[q] = (tails[q]+1)%MAX_PROCS; sizes[q]++; } while(0)
    #define DEQ(q)    ({ int _i = queues[q][heads[q]]; heads[q]=(heads[q]+1)%MAX_PROCS; sizes[q]--; _i; })

    int time = 0, done = 0, next = 0;
    int boost_interval = 100, last_boost = 0;

    while (done < n) {
        if (time - last_boost >= boost_interval) {
            for (int i = 0; i < n; i++) {
                if (p[i].state != DONE) level[i] = 0;
            }
            /* Re-enqueue all ready at level 0 */
            for (int lv = 1; lv < MLFQ_LEVELS; lv++) {
                int sz = sizes[lv];
                for (int j = 0; j < sz; j++) {
                    int idx = DEQ(lv);
                    if (p[idx].state == READY || p[idx].state == RUNNING) {
                        ENQ(0, idx);
                    }
                }
            }
            last_boost = time;
        }

        while (next < n && p[next].arrival <= time) {
            ENQ(0, next);
            next++;
        }

        int lv = -1;
        for (int q = 0; q < MLFQ_LEVELS; q++) {
            if (sizes[q] > 0) { lv = q; break; }
        }
        if (lv < 0) { time++; continue; }

        int idx = DEQ(lv);
        if (!p[idx].started) {
            p[idx].started = true;
            p[idx].start_time = time;
        }
        p[idx].state = RUNNING;
        int run = p[idx].remaining < quantums[lv] ? p[idx].remaining : quantums[lv];
        int start = time;
        time += run;
        p[idx].remaining -= run;

        while (next < n && p[next].arrival <= time) {
            ENQ(0, next);
            next++;
        }

        if (p[idx].remaining == 0) {
            p[idx].state = DONE;
            p[idx].finish_time = time;
            done++;
        } else {
            p[idx].state = READY;
            if (lv < MLFQ_LEVELS - 1) level[idx]++;
            ENQ(level[idx], idx);
        }
        gantt_push(p[idx].pid, start, time);
    }
    print_gantt("MLFQ");
    print_metrics(p, n);
    #undef ENQ
    #undef DEQ
}

/* ── Main ─────────────────────────────────────────────── */

int main(void) {
    Process procs[] = {
        { .pid=1, .arrival=0, .burst=24, .priority=0 },
        { .pid=2, .arrival=1, .burst=3,  .priority=0 },
        { .pid=3, .arrival=2, .burst=3,  .priority=0 },
        { .pid=4, .arrival=3, .burst=12, .priority=0 },
        { .pid=5, .arrival=5, .burst=6,  .priority=0 },
    };
    int n = sizeof(procs) / sizeof(procs[0]);

    printf("=== Process Table ===\n");
    printf("%-5s %-8s %-6s\n", "PID", "Arrival", "Burst");
    for (int i = 0; i < n; i++)
        printf("P%-4d %-8d %-6d\n", procs[i].pid, procs[i].arrival, procs[i].burst);

    fcfs(procs, n);
    round_robin(procs, n, 4);
    mlfq(procs, n);

    printf("\n=== Algorithm Comparison ===\n");
    printf("FCFS:    Simple, convoy effect, poor response for short jobs\n");
    printf("RR(q=4): Fair, good response, more context switches\n");
    printf("MLFQ:    Adaptive, favors interactive, prevents starvation via boost\n");

    return 0;
}
