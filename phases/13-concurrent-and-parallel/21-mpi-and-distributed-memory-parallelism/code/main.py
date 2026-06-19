"""
MPI and Distributed-Memory Parallelism
Phase 13 — Concurrent & Parallel Computing

Build steps (run with mpirun -np 4):
  Step 1 — Hello world: rank + size
  Step 2 — Ping-pong:   point-to-point round-trip
  Step 3 — Sum reduce:  scatter + reduce (collectives)
  Step 4 — Jacobi:      2-D stencil with halo exchange

Requires mpi4py:  pip install mpi4py
Run:  mpirun -np 4 python3 code/main.py
"""

from mpi4py import MPI
import math
import sys

NROWS = 64
NCOLS = 64
PINGS = 5
MAX_ITER = 2000
TOL = 1e-6

comm = MPI.COMM_WORLD
rank = comm.Get_rank()
size = comm.Get_size()


def step1_hello():
    if rank == 0:
        print(f"--- Step 1: Hello World ({size} processes) ---")
    print(f"  Hello from rank {rank} of {size}")


def step2_pingpong():
    if rank == 0:
        print(f"--- Step 2: Ping-Pong ({PINGS} pings) ---")

    if rank > 1:
        return

    t_start = MPI.Wtime()

    for i in range(PINGS):
        if rank == 0:
            buf = i
            comm.send(buf, dest=1, tag=0)
            buf = comm.recv(source=1, tag=0)
            print(f"  rank 0 received {buf} back from rank 1")
        elif rank == 1:
            buf = comm.recv(source=0, tag=0)
            buf *= 2
            comm.send(buf, dest=0, tag=0)

    # Demonstrate Sendrecv via send+recv pair (mpi4py has no direct Sendrecv)
    if rank == 0:
        comm.send(99, dest=1, tag=1)
        recv_val = comm.recv(source=1, tag=1)
        print(f"  rank 0: send/recv pair sent 99, received {recv_val}")
    elif rank == 1:
        recv_val = comm.recv(source=0, tag=1)
        comm.send(88, dest=0, tag=1)
        print(f"  rank 1: send/recv pair sent 88, received {recv_val}")

    t_end = MPI.Wtime()
    if rank == 0:
        print(f"  Ping-pong + send/recv pair took {(t_end - t_start) * 1e3:.3f} ms")


def step3_sum_reduce():
    if rank == 0:
        print("--- Step 3: Parallel Sum Reduction ---")

    n_per_proc = 8
    total_n = n_per_proc * size

    if rank == 0:
        data = [float(i + 1) for i in range(total_n)]
        print(f"  Input data: [{total_n} numbers, 1..{total_n}]")
    else:
        data = None

    local = comm.scatter(data, root=0)
    local_sum = sum(local)

    global_sum = comm.reduce(local_sum, op=MPI.SUM, root=0)

    if rank == 0:
        expected = total_n * (total_n + 1) / 2.0
        print(f"  Local sum on root = {local_sum:.1f}")
        print(f"  Global sum (root)  = {global_sum:.1f}")
        print(f"  Expected           = {expected:.1f}")

    all_sum = comm.allreduce(local_sum, op=MPI.SUM)
    print(f"  [rank {rank}] Allreduce global sum = {all_sum:.1f}")


def step4_jacobi():
    if rank == 0:
        print(f"--- Step 4: Jacobi 2-D ({NROWS}x{NCOLS} grid, {size} procs) ---")

    if NROWS % size != 0:
        if rank == 0:
            print(f"ERROR: NROWS ({NROWS}) must be divisible by size ({size})",
                  file=sys.stderr)
        return

    local_nrows = NROWS // size
    global_row0 = rank * local_nrows

    # Allocate grid with halos: u[local_nrows + 2][NCOLS]
    u = [[0.0] * NCOLS for _ in range(local_nrows + 2)]
    u_new = [[0.0] * NCOLS for _ in range(local_nrows + 2)]

    # Initial guess: sin(pi*x) * sinh(pi*(1-y))
    for r in range(1, local_nrows + 1):
        gr = global_row0 + (r - 1)
        for c in range(NCOLS):
            x = c / (NCOLS - 1)
            y = gr / (NROWS - 1)
            u[r][c] = math.sin(math.pi * x) * math.sinh(math.pi * (1.0 - y))

    left = rank - 1 if rank > 0 else MPI.PROC_NULL
    right = rank + 1 if rank < size - 1 else MPI.PROC_NULL

    # Helper: convert list-of-lists to contiguous buffer and back for MPI
    # (We keep list-of-lists for readability; mpi4py handles it fine with Python pickles.
    #  For performance, use numpy arrays + mpi4py.fortran or mpi4py.numpy.)

    converged = False
    for it in range(MAX_ITER):
        # --- Non-blocking halo exchange ---
        reqs = []

        reqs.append(comm.Irecv(u[0], source=left, tag=0))
        reqs.append(comm.Irecv(u[local_nrows + 1], source=right, tag=0))

        reqs.append(comm.Isend(u[1], dest=left, tag=0))
        reqs.append(comm.Isend(u[local_nrows], dest=right, tag=0))

        MPI.Request.Waitall(reqs)

        # --- Jacobi stencil update ---
        norm = 0.0
        for r in range(1, local_nrows + 1):
            u_r = u[r]
            u_rm1 = u[r - 1]
            u_rp1 = u[r + 1]
            un_r = u_new[r]
            for c in range(1, NCOLS - 1):
                val = 0.25 * (u_rm1[c] + u_rp1[c] + u_r[c - 1] + u_r[c + 1])
                un_r[c] = val
                diff = abs(val - u_r[c])
                if diff > norm:
                    norm = diff

        # Dirichlet BC on left/right walls
        for r in range(1, local_nrows + 1):
            u_new[r][0] = 0.0
            u_new[r][NCOLS - 1] = 0.0

        u, u_new = u_new, u

        if norm < TOL:
            converged = True

        if rank == 0 and (it % 200 == 0 or converged or it == MAX_ITER - 1):
            print(f"  iter {it:4d}  ||diff||_inf = {norm:.2e}")

        if converged:
            break

    if rank == 0:
        status = "Converged" if converged else "Did NOT converge"
        print(f"  {status} at iteration {it} (norm={norm:.2e})")

    # Verify: gather centre-column samples
    local_val = u[1][NCOLS // 2]
    root_val = comm.reduce(local_val, op=MPI.SUM, root=0)
    if rank == 0:
        print(f"  Sum of centre-column samples across ranks = {root_val:.6f}")
        print("  (Each rank reported u[local_row1][NCOLS/2])")


def main():
    step1_hello()
    comm.Barrier()

    step2_pingpong()
    comm.Barrier()

    step3_sum_reduce()
    comm.Barrier()

    step4_jacobi()


if __name__ == "__main__":
    main()
