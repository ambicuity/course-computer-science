/*
 * MPI and Distributed-Memory Parallelism
 * Phase 13 — Concurrent & Parallel Computing
 *
 * Build steps (run all four with mpirun -np 4):
 *   Step 1 — Hello world: rank + size
 *   Step 2 — Ping-pong:   point-to-point round-trip
 *   Step 3 — Sum reduce:  scatter + reduce (collectives)
 *   Step 4 — Jacobi:      2-D stencil with halo exchange
 *
 * Compile: mpicc -O2 -Wall -o mpi_bin code/main.c
 * Run:     mpirun -np 4 ./mpi_bin
 */
#include <mpi.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#define NROWS  64
#define NCOLS  64
#define PINGS  5
#define MAX_ITER 2000
#define TOL     1e-6

/* ------------------------------------------------------------------ */
/*  Step 1 — Hello World                                               */
/* ------------------------------------------------------------------ */
static void step1_hello(void) {
    int rank, size;
    MPI_Comm_rank(MPI_COMM_WORLD, &rank);
    MPI_Comm_size(MPI_COMM_WORLD, &size);

    if (rank == 0)
        printf("--- Step 1: Hello World (%d processes) ---\n", size);
    printf("  Hello from rank %d of %d\n", rank, size);
}

/* ------------------------------------------------------------------ */
/*  Step 2 — Ping-Pong                                                 */
/* ------------------------------------------------------------------ */
static void step2_pingpong(void) {
    int rank, size;
    MPI_Comm_rank(MPI_COMM_WORLD, &rank);
    MPI_Comm_size(MPI_COMM_WORLD, &size);

    if (rank == 0)
        printf("--- Step 2: Ping-Pong (%d pings) ---\n", PINGS);

    /* Only rank 0 and 1 participate; others skip */
    if (rank > 1)
        return;

    int buf;
    MPI_Status status;
    double t_start = MPI_Wtime();

    for (int i = 0; i < PINGS; i++) {
        if (rank == 0) {
            buf = i;
            MPI_Send(&buf, 1, MPI_INT, 1, 0, MPI_COMM_WORLD);
            MPI_Recv(&buf, 1, MPI_INT, 1, 0, MPI_COMM_WORLD, &status);
            printf("  rank 0 received %d back from rank 1\n", buf);
        } else if (rank == 1) {
            MPI_Recv(&buf, 1, MPI_INT, 0, 0, MPI_COMM_WORLD, &status);
            buf *= 2;  /* double and return */
            MPI_Send(&buf, 1, MPI_INT, 0, 0, MPI_COMM_WORLD);
        }
    }

    /* Demonstrate MPI_Sendrecv — a single call for simultaneous send+receive.
     * This is the deadlock-free alternative when both ranks need to exchange. */
    if (rank == 0) {
        int send_val = 99, recv_val;
        MPI_Sendrecv(&send_val, 1, MPI_INT, 1, 1,
                     &recv_val, 1, MPI_INT, 1, 1,
                     MPI_COMM_WORLD, &status);
        printf("  rank 0: Sendrecv sent %d, received %d\n", send_val, recv_val);
    } else if (rank == 1) {
        int send_val = 88, recv_val;
        MPI_Sendrecv(&send_val, 1, MPI_INT, 0, 1,
                     &recv_val, 1, MPI_INT, 0, 1,
                     MPI_COMM_WORLD, &status);
        printf("  rank 1: Sendrecv sent %d, received %d\n", send_val, recv_val);
    }

    double t_end = MPI_Wtime();
    if (rank == 0)
        printf("  Ping-pong + Sendrecv took %.3f ms\n", (t_end - t_start) * 1e3);
}

/* ------------------------------------------------------------------ */
/*  Step 3 — Parallel Sum Reduction (Scatter + Reduce)                */
/* ------------------------------------------------------------------ */
static void step3_sum_reduce(void) {
    int rank, size;
    MPI_Comm_rank(MPI_COMM_WORLD, &rank);
    MPI_Comm_size(MPI_COMM_WORLD, &size);

    if (rank == 0)
        printf("--- Step 3: Parallel Sum Reduction ---\n");

    int n_per_proc = 8;              /* elements per rank after scatter */
    int total_n    = n_per_proc * size;

    float *data     = NULL;
    float *local    = (float *)malloc(n_per_proc * sizeof(float));
    float local_sum = 0.0f;

    if (rank == 0) {
        data = (float *)malloc(total_n * sizeof(float));
        for (int i = 0; i < total_n; i++)
            data[i] = (float)(i + 1);
        printf("  Input data: [%d numbers, 1..%d]\n", total_n, total_n);
    }

    MPI_Scatter(data, n_per_proc, MPI_FLOAT,
                local, n_per_proc, MPI_FLOAT,
                0, MPI_COMM_WORLD);

    for (int i = 0; i < n_per_proc; i++)
        local_sum += local[i];

    float global_sum = 0.0f;
    MPI_Reduce(&local_sum, &global_sum, 1, MPI_FLOAT,
               MPI_SUM, 0, MPI_COMM_WORLD);

    if (rank == 0) {
        float expected = (float)(total_n) * (total_n + 1.0f) / 2.0f;
        printf("  Local sum on root = %.1f\n", local_sum);
        printf("  Global sum (root)  = %.1f\n", global_sum);
        printf("  Expected           = %.1f\n", expected);
    }

    free(local);
    if (rank == 0) free(data);

    /* Demonstrate MPI_Allreduce on every rank */
    float all_sum = 0.0f;
    MPI_Allreduce(&local_sum, &all_sum, 1, MPI_FLOAT,
                  MPI_SUM, MPI_COMM_WORLD);
    printf("  [rank %d] Allreduce global sum = %.1f\n", rank, all_sum);
}

/* ------------------------------------------------------------------ */
/*  Step 4 — Jacobi Iteration with Halo Exchange                      */
/* ------------------------------------------------------------------ */

/*
 * 1-D row decomposition:
 *   local_nrows = NROWS / size  (assumes NROWS divisible by size)
 *   Each rank owns a "slab" of rows plus one halo row above and below.
 *
 *   Grid layout (size=4, NROWS=64):
 *     rank 0: rows  0..15  + halo from rank 1 (row 16)
 *     rank 1: rows 16..31  + halo from rank 0 (row 15) and rank 2 (row 32)
 *     rank 2: rows 32..47  + halo from rank 1 (row 31) and rank 3 (row 48)
 *     rank 3: rows 48..63  + halo from rank 2 (row 47)
 *
 *   u[local_nrows+2][NCOLS]  — 2 extra rows for halos (top/bottom).
 *   Row 0        = halo from rank-1 (bottom row of neighbour above)
 *   Row 1..local = owned interior
 *   Row local+1  = halo from rank+1 (top row of neighbour below)
 */

static void step4_jacobi(void) {
    int rank, size;
    MPI_Comm_rank(MPI_COMM_WORLD, &rank);
    MPI_Comm_size(MPI_COMM_WORLD, &size);

    int local_nrows = NROWS / size;
    if (NROWS % size != 0 && rank == 0) {
        fprintf(stderr, "NROWS must be divisible by size (%d %% %d != 0)\n",
                NROWS, size);
        return;
    }

    if (rank == 0)
        printf("--- Step 4: Jacobi 2-D (%dx%d grid, %d procs) ---\n",
               NROWS, NCOLS, size);

    /* Allocate grid with halos — contiguous in memory */
    int ld = NCOLS;  /* leading dimension (row stride) */
    float *u        = (float *)calloc((local_nrows + 2) * ld, sizeof(float));
    float *u_new    = (float *)calloc((local_nrows + 2) * ld, sizeof(float));

    /* Initial condition: u = 0 interior, boundary = 1 on top edge (row 0 of global grid)
       and bottom edge (row NROWS-1) */
    int global_row0 = rank * local_nrows;
    for (int r = 1; r <= local_nrows; r++) {
        int gr = global_row0 + (r - 1);
        for (int c = 0; c < NCOLS; c++) {
            double x = (double)c / (NCOLS - 1);
            double y = (double)gr / (NROWS - 1);
            /* sin(pi*x) * sin(pi*y) initial guess */
            u[r * ld + c] = sin(M_PI * x) * sinh(M_PI * (1.0 - y));
        }
    }
    /* enforce Dirichlet BCs: top edge = 1, bottom edge = 0 (handled by initial below) */
    /* We'll fix boundaries each iteration via the stencil — they are set by the initial condition */

    int left  = (rank > 0)          ? rank - 1 : MPI_PROC_NULL;
    int right = (rank < size - 1)   ? rank + 1 : MPI_PROC_NULL;

    MPI_Request reqs[4];

    int iter;
    double norm = 0.0;

    for (iter = 0; iter < MAX_ITER; iter++) {
        /* --- Halo exchange (non-blocking) --- */
        /* Post receives first */
        MPI_Irecv(&u[0 * ld],              NCOLS, MPI_FLOAT, left,  0,
                  MPI_COMM_WORLD, &reqs[0]);
        MPI_Irecv(&u[(local_nrows + 1) * ld], NCOLS, MPI_FLOAT, right, 0,
                  MPI_COMM_WORLD, &reqs[1]);

        /* Send halo rows */
        MPI_Isend(&u[1 * ld],              NCOLS, MPI_FLOAT, left,  0,
                  MPI_COMM_WORLD, &reqs[2]);
        MPI_Isend(&u[local_nrows * ld],    NCOLS, MPI_FLOAT, right, 0,
                  MPI_COMM_WORLD, &reqs[3]);

        /* Wait for all comms to finish */
        MPI_Waitall(4, reqs, MPI_STATUSES_IGNORE);

        /* --- Jacobi stencil update on interior --- */
        norm = 0.0;
        for (int r = 1; r <= local_nrows; r++) {
            for (int c = 1; c < NCOLS - 1; c++) {
                u_new[r * ld + c] = 0.25f * (u[(r - 1) * ld + c] +
                                             u[(r + 1) * ld + c] +
                                             u[r * ld + (c - 1)] +
                                             u[r * ld + (c + 1)]);
                double diff = fabs(u_new[r * ld + c] - u[r * ld + c]);
                if (diff > norm) norm = diff;
            }
        }

        /* Apply Dirichlet BC on left/right walls (column 0 and NCOLS-1) */
        /* These stay at zero (or their initial value) */
        for (int r = 1; r <= local_nrows; r++) {
            u_new[r * ld + 0]       = 0.0f;
            u_new[r * ld + NCOLS - 1] = 0.0f;
        }

        /* Swap pointers */
        float *tmp = u;
        u = u_new;
        u_new = tmp;

        if (norm < TOL) break;

        /* For large grids, print every 200 iterations to avoid IO storm */
        if (rank == 0 && (iter % 200 == 0 || iter == MAX_ITER - 1))
            printf("  iter %4d  ||diff||_inf = %.2e\n", iter, norm);
    }

    if (rank == 0)
        printf("  Converged at iteration %d (norm=%.2e)\n", iter, norm);

    /* Gather a sample value to verify correctness across ranks */
    float local_val = u[1 * ld + NCOLS / 2];  /* centre column, first owned row */
    float root_val;
    MPI_Reduce(&local_val, &root_val, 1, MPI_FLOAT, MPI_SUM, 0, MPI_COMM_WORLD);
    if (rank == 0) {
        printf("  Sum of centre-column samples across ranks = %.6f\n", root_val);
        printf("  (Each rank reported u[local_row1][NCOLS/2])\n");
    }

    free(u);
    free(u_new);
}

/* ------------------------------------------------------------------ */
/*  Error-handling helper — checks every MPI call                     */
/* ------------------------------------------------------------------ */
static void check_mpi(int rc, int line, const char *file) {
    if (rc != MPI_SUCCESS) {
        char err[MPI_MAX_ERROR_STRING];
        int len;
        MPI_Error_string(rc, err, &len);
        fprintf(stderr, "MPI error at %s:%d: %s\n", file, line, err);
        MPI_Abort(MPI_COMM_WORLD, rc);
    }
}
#define CHECK(expr)  check_mpi((expr), __LINE__, __FILE__)

/* ------------------------------------------------------------------ */
/*  Main — run all four steps                                         */
/* ------------------------------------------------------------------ */
int main(int argc, char **argv) {
    MPI_Init(&argc, &argv);

    int rank;
    MPI_Comm_rank(MPI_COMM_WORLD, &rank);

    /* seed RNG differently per rank so we can show data ownership */
    srand(42 + rank);

    step1_hello();
    CHECK(MPI_Barrier(MPI_COMM_WORLD));

    step2_pingpong();
    CHECK(MPI_Barrier(MPI_COMM_WORLD));

    step3_sum_reduce();
    CHECK(MPI_Barrier(MPI_COMM_WORLD));

    step4_jacobi();

    MPI_Finalize();
    return 0;
}
