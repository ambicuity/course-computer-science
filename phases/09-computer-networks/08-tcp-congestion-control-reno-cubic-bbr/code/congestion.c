/*
 * TCP Congestion Control — Reno Implementation
 * Phase 09 — Computer Networks
 *
 * Implements Reno slow start + congestion avoidance.
 * Tracks cwnd and ssthresh through simulated packet loss events.
 *
 * Compile: gcc -Wall -Wextra -lm -o congestion congestion.c
 * Run: ./congestion
 */

#include <stdio.h>
#include <stdlib.h>
#include <math.h>

#define MSS 1460
#define INITIAL_SSTHRESH (64 * MSS)
#define SIM_ROUNDS 200
#define LOSS_INTERVAL 30  /* simulate loss every N rounds */

typedef enum {
    PHASE_SLOW_START,
    PHASE_CONGESTION_AVOIDANCE,
    PHASE_FAST_RECOVERY,
} congestion_phase_t;

typedef struct {
    double cwnd;               /* congestion window in bytes */
    double ssthresh;           /* slow start threshold */
    int duplicate_acks;
    int in_recovery;
    congestion_phase_t phase;
    int round;
} reno_t;

static const char *phase_name(congestion_phase_t p) {
    switch (p) {
        case PHASE_SLOW_START:           return "SLOW_START";
        case PHASE_CONGESTION_AVOIDANCE: return "CONG_AVOID";
        case PHASE_FAST_RECOVERY:        return "FAST_RECOV";
        default:                         return "UNKNOWN";
    }
}

static void reno_init(reno_t *r) {
    r->cwnd = MSS;
    r->ssthresh = INITIAL_SSTHRESH;
    r->duplicate_acks = 0;
    r->in_recovery = 0;
    r->phase = PHASE_SLOW_START;
    r->round = 0;
}

/* Called for each ACK received (simplified: one per RTT round) */
static void reno_on_ack(reno_t *r) {
    if (r->in_recovery) {
        r->cwnd += MSS; /* inflate during recovery */
        return;
    }

    if (r->cwnd < r->ssthresh) {
        /* Slow start: double per RTT */
        r->cwnd += MSS;
        r->phase = PHASE_SLOW_START;
    } else {
        /* Congestion avoidance: linear increase */
        r->cwnd += (double)(MSS * MSS) / r->cwnd;
        r->phase = PHASE_CONGESTION_AVOIDANCE;
    }
}

/* Called when a timeout is detected */
static void reno_on_timeout(reno_t *r) {
    r->ssthresh = r->cwnd / 2;
    if (r->ssthresh < 2 * MSS) r->ssthresh = 2 * MSS;
    r->cwnd = MSS;
    r->duplicate_acks = 0;
    r->in_recovery = 0;
    r->phase = PHASE_SLOW_START;
}

/* Called when a duplicate ACK is received */
static void reno_on_dup_ack(reno_t *r) {
    r->duplicate_acks++;
    if (r->duplicate_acks >= 3) {
        /* Fast retransmit + fast recovery */
        r->ssthresh = r->cwnd / 2;
        if (r->ssthresh < 2 * MSS) r->ssthresh = 2 * MSS;
        r->cwnd = r->ssthresh + 3 * MSS;
        r->in_recovery = 1;
        r->phase = PHASE_FAST_RECOVERY;
        r->duplicate_acks = 0;
    }
}

/* Print state at each round */
static void reno_print(const reno_t *r) {
    printf("Round %3d | cwnd=%8.0f (%5.1f MSS) | ssthresh=%8.0f (%5.1f MSS) | %s\n",
           r->round,
           r->cwnd, r->cwnd / MSS,
           r->ssthresh, r->ssthresh / MSS,
           phase_name(r->phase));
}

/* ASCII chart row */
static void print_bar(double value, double max_val, int width) {
    int filled = (int)(value / max_val * width);
    if (filled > width) filled = width;
    if (filled < 0) filled = 0;
    printf("  |");
    for (int i = 0; i < filled; i++) printf("█");
    for (int i = filled; i < width; i++) printf(" ");
    printf("| %.0f\n", value);
}

int main(void) {
    printf("TCP Reno Congestion Control Simulator\n");
    printf("======================================\n\n");

    reno_t r;
    reno_init(&r);

    double history[SIM_ROUNDS];
    int hcount = 0;

    printf("%-8s | %-20s | %-20s | %s\n",
           "Round", "cwnd", "ssthresh", "Phase");
    printf("%-8s-+-%-20s-+-%-20s-+-%s\n",
           "--------", "--------------------", "--------------------",
           "----------------");

    for (int i = 0; i < SIM_ROUNDS; i++) {
        r.round = i;

        /* Simulate loss events at regular intervals */
        if (i > 0 && i % LOSS_INTERVAL == 0) {
            if (i % (LOSS_INTERVAL * 3) == 0) {
                printf("  >>> LOSS EVENT (timeout) at round %d\n", i);
                reno_on_timeout(&r);
            } else {
                /* 3 duplicate ACKs */
                reno_on_dup_ack(&r);
                reno_on_dup_ack(&r);
                reno_on_dup_ack(&r);
                printf("  >>> LOSS EVENT (3 dup ACKs) at round %d\n", i);
            }
        }

        reno_on_ack(&r);

        if (i % 5 == 0) {
            reno_print(&r);
        }

        if (hcount < SIM_ROUNDS) {
            history[hcount++] = r.cwnd;
        }
    }

    /* Print final summary chart */
    printf("\n");
    printf("Final cwnd history (sampled):\n");
    printf("=====================================\n");

    double max_cwnd = 0;
    for (int i = 0; i < hcount; i++) {
        if (history[i] > max_cwnd) max_cwnd = history[i];
    }

    int step = hcount / 40;
    if (step < 1) step = 1;
    for (int i = 0; i < hcount; i += step) {
        printf("  R%3d: ", i);
        print_bar(history[i] / MSS, max_cwnd / MSS, 50);
    }

    printf("\nFinal cwnd: %.0f bytes (%.1f MSS)\n", r.cwnd, r.cwnd / MSS);
    printf("Final ssthresh: %.0f bytes (%.1f MSS)\n", r.ssthresh, r.ssthresh / MSS);

    return 0;
}
