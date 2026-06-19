# MPI and Distributed-Memory Parallelism — Outputs

This directory contains reusable reference artifacts for the MPI lesson.

## Artifacts

### `main.c` — MPI Reference in C

`code/main.c` implements all four build steps in a single file:

1. **Step 1 — Hello World** – `MPI_Init`, `MPI_Comm_rank`, `MPI_Comm_size`, `MPI_Finalize`.
2. **Step 2 — Ping-Pong** – `MPI_Send` / `MPI_Recv` between rank 0 and rank 1.
3. **Step 3 — Parallel Sum Reduction** – `MPI_Scatter` + `MPI_Reduce` + `MPI_Allreduce`.
4. **Step 4 — Jacobi 2-D** – Domain-decomposed grid with non-blocking halo exchange (`MPI_Isend` / `MPI_Irecv` / `MPI_Waitall`).

Compile: `mpicc -O2 -Wall -o mpi_bin code/main.c`
Run: `mpirun -np 4 ./mpi_bin`

### `main.py` — MPI Reference in Python

Same four steps using `mpi4py`.

Run: `mpirun -np 4 python3 code/main.py`

## Cheat-Sheet Patterns

### SPMD Skeleton

```c
MPI_Init(&argc, &argv);
int rank, size;
MPI_Comm_rank(MPI_COMM_WORLD, &rank);
MPI_Comm_size(MPI_COMM_WORLD, &size);
/* work */
MPI_Finalize();
```

### Point-to-Point

```c
MPI_Send(buf, count, MPI_TYPE, dest, tag, MPI_COMM_WORLD);
MPI_Recv(buf, count, MPI_TYPE, src,  tag, MPI_COMM_WORLD, &status);
```

### Collective Reduction

```c
MPI_Scatter(sendbuf, n, MPI_FLOAT, recvbuf, n, MPI_FLOAT, 0, MPI_COMM_WORLD);
/* compute local_sum */
MPI_Reduce(&local_sum, &global_sum, 1, MPI_FLOAT, MPI_SUM, 0, MPI_COMM_WORLD);
MPI_Allreduce(&local_sum, &all_sum, 1, MPI_FLOAT, MPI_SUM, MPI_COMM_WORLD);
```

### Non-blocking Halo Exchange

```c
MPI_Irecv(halo_top,   NCOLS, MPI_FLOAT, left,  tag, comm, &req[0]);
MPI_Irecv(halo_bottom,NCOLS, MPI_FLOAT, right, tag, comm, &req[1]);
MPI_Isend(own_top,    NCOLS, MPI_FLOAT, left,  tag, comm, &req[2]);
MPI_Isend(own_bottom, NCOLS, MPI_FLOAT, right, tag, comm, &req[3]);
MPI_Waitall(4, reqs, MPI_STATUSES_IGNORE);
/* stencil update on interior */
```
