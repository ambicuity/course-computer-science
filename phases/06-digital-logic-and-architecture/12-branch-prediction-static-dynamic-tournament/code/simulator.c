/*
 * Branch Prediction Simulator
 * ---------------------------
 * Reads a trace of (PC, taken/not-taken) and evaluates multiple predictor
 * types: always-taken, BTFN, 1-bit, 2-bit bimodal, gshare.
 *
 * Trace format (one per line):  <hex_pc>  <T|NT>
 * Example:
 *   0x1000 T
 *   0x1004 NT
 *   0x1000 T
 *
 * Usage:
 *   gcc -o sim simulator.c -Wall -O2
 *   ./sim                        (uses built-in sample trace)
 *   ./sim trace.txt              (reads from file)
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

/* ---- Configuration ------------------------------------------------------- */
#define TABLE_BITS   10
#define TABLE_SIZE   (1 << TABLE_BITS)
#define HIST_BITS    10
#define MAX_TRACE    100000

/* ---- Trace entry --------------------------------------------------------- */
typedef struct {
    uint32_t pc;
    int      taken;   /* 1 = taken, 0 = not-taken */
} branch_t;

/* ---- Predictor state ----------------------------------------------------- */
typedef struct {
    /* Bimodal table (2-bit counters) */
    uint8_t bimodal[TABLE_SIZE];

    /* Gshare: global history register + table */
    uint16_t ghr;
    uint8_t  gshare[TABLE_SIZE];
} predictors_t;

/* ---- Helper: extract table index from PC --------------------------------- */
static unsigned pc_index(uint32_t pc) {
    /* Word-aligned: drop lowest 2 bits, take TABLE_BITS bits */
    return (pc >> 2) & (TABLE_SIZE - 1);
}

/* ---- 2-bit saturating counter update ------------------------------------- */
static uint8_t counter_update(uint8_t counter, int outcome) {
    if (outcome) {
        return (counter < 3) ? counter + 1 : counter;
    } else {
        return (counter > 0) ? counter - 1 : counter;
    }
}

/* ---- Predict functions --------------------------------------------------- */
static int bimodal_predict(uint8_t *table, uint32_t pc) {
    return table[pc_index(pc)] >= 2;  /* MSB: 2 or 3 → taken */
}

static int gshare_predict(uint8_t *table, uint16_t ghr, uint32_t pc) {
    unsigned idx = pc_index(pc) ^ (ghr & (TABLE_SIZE - 1));
    return table[idx] >= 2;
}

static int btfn_predict(uint32_t pc, uint32_t prev_pc) {
    /* Backward branch → taken, forward → not taken */
    return (pc < prev_pc);
}

/* ---- Sample trace (used when no file is provided) ------------------------ */
static branch_t sample_trace[] = {
    {0x1000, 1},  /* loop branch, taken */
    {0x1004, 0},  /* forward branch, not taken */
    {0x1000, 1},
    {0x1008, 1},  /* backward (loop), taken */
    {0x1000, 1},
    {0x1004, 0},
    {0x1008, 1},
    {0x1000, 1},
    {0x1004, 1},  /* sometimes taken */
    {0x1008, 0},  /* loop exit, not taken */
    {0x1000, 1},
    {0x1000, 1},
    {0x1000, 1},
    {0x1004, 0},
    {0x1008, 1},
    {0x1000, 1},
    {0x1004, 1},
    {0x1008, 1},
    {0x1000, 0},  /* outer loop exit */
    {0x1004, 0},
};
#define SAMPLE_LEN (sizeof(sample_trace) / sizeof(sample_trace[0]))

/* ---- Read trace from file ------------------------------------------------ */
static int read_trace(const char *path, branch_t *trace, int max_entries) {
    FILE *f = fopen(path, "r");
    if (!f) {
        perror(path);
        return 0;
    }
    int n = 0;
    char buf[32];
    while (n < max_entries && fscanf(f, "%31s", buf) == 1) {
        char dir;
        if (fscanf(f, " %c", &dir) != 1) break;
        trace[n].pc    = (uint32_t)strtoul(buf, NULL, 0);
        trace[n].taken = (dir == 'T' || dir == 't');
        n++;
    }
    fclose(f);
    return n;
}

/* ---- Run simulation ------------------------------------------------------ */
static void simulate(branch_t *trace, int count) {
    predictors_t p;
    memset(&p, 0, sizeof(p));
    /* Initialize bimodal to weakly-taken (2) */
    for (int i = 0; i < TABLE_SIZE; i++) {
        p.bimodal[i] = 2;
        p.gshare[i]  = 2;
    }

    int correct_always  = 0;
    int correct_btfn    = 0;
    int correct_1bit    = 0;
    int correct_2bit    = 0;
    int correct_gshare  = 0;

    /* 1-bit predictor table */
    uint8_t onebit[TABLE_SIZE];
    memset(onebit, 1, sizeof(onebit)); /* start predicting taken */

    uint32_t prev_pc = 0;

    for (int i = 0; i < count; i++) {
        uint32_t pc = trace[i].pc;
        int taken   = trace[i].taken;

        /* Always-taken */
        if (taken) correct_always++;

        /* BTFN */
        if (btfn_predict(pc, prev_pc) == taken) correct_btfn++;

        /* 1-bit */
        unsigned idx1 = pc_index(pc);
        if ((int)onebit[idx1] == taken) {
            correct_1bit++;
        }
        onebit[idx1] = taken;

        /* 2-bit bimodal */
        if (bimodal_predict(p.bimodal, pc) == taken) correct_2bit++;
        p.bimodal[pc_index(pc)] = counter_update(p.bimodal[pc_index(pc)], taken);

        /* Gshare */
        if (gshare_predict(p.gshare, p.ghr, pc) == taken) correct_gshare++;
        unsigned gidx = pc_index(pc) ^ (p.ghr & (TABLE_SIZE - 1));
        p.gshare[gidx] = counter_update(p.gshare[gidx], taken);
        p.ghr = ((p.ghr << 1) | taken) & ((1 << HIST_BITS) - 1);

        prev_pc = pc;
    }

    /* ---- Report ---------------------------------------------------------- */
    printf("Branch Prediction Simulator\n");
    printf("===========================\n");
    printf("Trace entries: %d\n", count);
    printf("Table size:    %d entries (%d-bit index)\n\n", TABLE_SIZE, TABLE_BITS);
    printf("%-25s %10s %s\n", "Predictor", "Correct", "Accuracy");
    printf("%-25s %10d %6.2f%%\n", "Always-Taken",  correct_always,
           100.0 * correct_always  / count);
    printf("%-25s %10d %6.2f%%\n", "BTFN",          correct_btfn,
           100.0 * correct_btfn    / count);
    printf("%-25s %10d %6.2f%%\n", "1-Bit",         correct_1bit,
           100.0 * correct_1bit    / count);
    printf("%-25s %10d %6.2f%%\n", "2-Bit Bimodal", correct_2bit,
           100.0 * correct_2bit    / count);
    printf("%-25s %10d %6.2f%%\n", "Gshare (10-bit)", correct_gshare,
           100.0 * correct_gshare  / count);
}

/* ---- Main ---------------------------------------------------------------- */
int main(int argc, char *argv[]) {
    branch_t trace[MAX_TRACE];
    int count;

    if (argc > 1) {
        count = read_trace(argv[1], trace, MAX_TRACE);
        if (count == 0) {
            fprintf(stderr, "Failed to read trace from '%s'\n", argv[1]);
            return 1;
        }
    } else {
        /* Use built-in sample trace */
        memcpy(trace, sample_trace, sizeof(sample_trace));
        count = SAMPLE_LEN;
        printf("(Using built-in sample trace — pass a file argument for a custom trace)\n\n");
    }

    simulate(trace, count);
    return 0;
}
