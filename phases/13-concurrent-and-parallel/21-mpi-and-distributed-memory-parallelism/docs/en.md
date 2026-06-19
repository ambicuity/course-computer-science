# MPI and Distributed-Memory Parallelism

> Build distributed-memory parallel programs using the Message Passing Interface (MPI). Go from a "hello world" rank/size probe to a working Jacobi iterative solver with halo exchange — in C and Python.

**Type:** Build
**Languages:** C, Python
**Prerequisites:** Basic C (pointers, arrays, structs) or Python; threads/concurrency (Phase 13 lessons 1–20); comfort with the terminal and a text editor.
**Time:** ~75 minutes

## Learning Objectives

- Contrast distributed-memory (MPI) with shared-memory (pthreads/OpenMP) parallelism.
- Describe the MPI execution model: `MPI_Init`, `MPI_Finalize`, communicators, rank, and size.
- Implement point-to-point communication with `MPI_Send` / `MPI_Recv` and avoid trivial deadlocks.
- Use collective operations (`MPI_Bcast`, `MPI_Reduce`, `MPI_Scatter`, `MPI_Gather`, `MPI_Allreduce`) to distribute work and combine results.
- Apply non-blocking communication (`MPI_Isend` / `MPI_Irecv` / `MPI_Wait`) to overlap communication with computation.
- Build a 2-D Jacobi iteration that exchanges halo cells with neighbours using non-blocking MPI.

## The Problem

Threads (pthreads, std::thread, OpenMP) share an address space. Every thread can read and write the same global variable. Synchronisation is hard, but *communication is free* — any thread can reach any memory location (modulo caches).

Now imagine a cluster: 64 machines, each with its own RAM, connected by Ethernet or InfiniBand. A thread on machine 0 **cannot** dereference a pointer to memory on machine 1 — there is no shared address space. The only way to exchange data is to *send messages* through the network.

This is the world of **distributed-memory parallelism**. Every process has private memory; data moves via explicit messages. MPI — the Message Passing Interface — is the industry-standard library for this model. Understanding MPI means you can write programs that scale from your laptop (4 cores) to a supercomputer (hundreds of thousands of cores).

## The Concept

### Distributed-Memory vs Shared-Memory

| Aspect | Shared-Memory (pthreads/OpenMP) | Distributed-Memory (MPI) |
|--------|---------------------------------|--------------------------|
| Address space | Single (all threads see same memory) | Private (each process has its own) |
| Communication | Implicit via loads/stores | Explicit via messages |
| Synchronisation | Mutexes, atomics, barriers | Message ordering, collectives |
| Scalability | Typically ≤ 64 cores | 10⁵+ cores on clusters |
| Failure model | One process fails → all fail | Processes can fail independently |
| Programming model | Threads + shared data | Processes + message passing |

### MPI Concepts

**History** — MPI was born in the early 1990s when the major parallel-computing vendors realised every vendor had a different message-passing library. The MPI Forum (academia, industry, labs) standardised the API. MPI-1 (1994) covered point-to-point and collectives. MPI-2 (1997) added dynamic processes, one-sided communication, and parallel I/O. MPI-3 (2012) added non-blocking collectives, shared-memory windows, and dropped the deprecated C++ bindings. MPI-4 (2021) added persistent collectives, partitioned communication, and Fortran 2018 support. Today every HPC system ships at least one MPI implementation (Open MPI, MPICH, Intel MPI, MVAPICH, HPE-MPI).

**Communicator** — a group of processes that can send messages to each other. `MPI_COMM_WORLD` is the default communicator containing every process launched by `mpirun`. You can create sub-communicators with `MPI_Comm_split` to isolate groups (e.g., one communicator per colour in a 2-D grid). A communicator has both a *group* (the set of processes) and a *context* (a communication domain that prevents message cross-talk between libraries).

**Rank** — each process in a communicator has a unique integer ID (0 … size−1). Messages are addressed by (communicator, source_rank, destination_rank). Rank 0 is conventionally the "root" for I/O and collectives, but there is no special hardware meaning.

**Size** — the total number of processes in the communicator.

**Message envelope** — every MPI message is identified by four fields: (1) communicator, (2) source rank, (3) destination rank, (4) tag (an integer 0 … MPI_TAG_UB). The tag allows distinguishing different messages between the same source/destination pair. `MPI_ANY_TAG` and `MPI_ANY_SOURCE` wildcards let a receive match any tag or any source.

**MPI datatypes** — MPI abstracts away the in-memory layout of data. Built-in types (`MPI_INT`, `MPI_FLOAT`, `MPI_DOUBLE`, `MPI_CHAR`, etc.) map directly to C types. Derived types (`MPI_Type_vector`, `MPI_Type_create_subarray`, `MPI_Type_create_struct`) describe strided, sub-array, or heterogeneous layouts so you can send non-contiguous data without manually packing.

**Point-to-point communication** — one process sends a message; one process receives it.

- `MPI_Send(buf, count, datatype, dest, tag, comm)` — blocking send. May buffer or block depending on message size and implementation.
- `MPI_Recv(buf, count, datatype, src, tag, comm, status)` — blocking receive. Returns a `status` object from which you can extract source, tag, and `MPI_Get_count`.
- `MPI_Sendrecv(sendbuf, ..., recvbuf, ..., comm)` — simultaneous send and receive. Deadlock-free for symmetric swaps.
- `MPI_Sendrecv_replace(buf, ..., comm)` — same buffer for send and receive.

**Collective communication** — all processes in a communicator participate. Every process must call the same collective; the call returns only when all processes in the communicator have entered it.

| Operation | What it does |
|-----------|-------------|
| `MPI_Bcast` | One process (root) sends the same data to all others. Tree-based algorithm; O(log N) steps. |
| `MPI_Reduce` | Every process contributes a value; a single result (sum, max, min, product, etc.) appears on the root. |
| `MPI_Allreduce` | Like Reduce, but the result appears on *every* process. Uses a butterfly-like algorithm. |
| `MPI_Scatter` | Root splits its data array into `size` chunks and sends one chunk to each rank. |
| `MPI_Gather` | Every process sends data; the root collects it into one array. Inverse of Scatter. |
| `MPI_Allgather` | Every process sends data; every process receives the complete collection. |
| `MPI_Scan` | Prefix reduction: rank k gets the reduction of ranks 0…k. |
| `MPI_Barrier` | Synchronisation: no process exits the barrier until all have entered. |

**Non-blocking communication** — the send/receive call returns immediately; you do other work, then call `MPI_Wait` to complete. The communication buffer must not be touched between the non-blocking call and the wait.

- `MPI_Isend(buf, count, datatype, dest, tag, comm, &request)` — returns an `MPI_Request` handle.
- `MPI_Irecv(buf, count, datatype, src, tag, comm, &request)` — posts a receive that completes in the background.
- `MPI_Wait(&request, &status)` — block until the operation finishes.
- `MPI_Test(&request, &flag, &status)` — non-blocking query; sets `flag` true if done.
- `MPI_Waitall(count, array_of_requests, array_of_statuses)` — wait for multiple requests.
- `MPI_Startall(count, array_of_requests)` — used with persistent requests (`MPI_Send_init` / `MPI_Recv_init`).

Non-blocking calls are essential for overlapping communication with computation and for avoiding deadlock in symmetric swaps (e.g., halo exchange). MPI-3 added **non-blocking collectives** (`MPI_Ibcast`, `MPI_Ireduce`, `MPI_Ialltoall` etc.) that let computation proceed while the collective is in flight.

### Deadlock Trap

```c
/* WRONG — both processes call Send before either calls Recv */
if (rank == 0) {
    MPI_Send(sendbuf, n, MPI_INT, 1, tag, MPI_COMM_WORLD);
    MPI_Recv(recvbuf, n, MPI_INT, 1, tag, MPI_COMM_WORLD, &status);
} else if (rank == 1) {
    MPI_Send(sendbuf, n, MPI_INT, 0, tag, MPI_COMM_WORLD);
    MPI_Recv(recvbuf, n, MPI_INT, 0, tag, MPI_COMM_WORLD, &status);
}
/* DEADLOCK if MPI uses a synchronous send — both wait for the other to receive. */
```

Fix: swap Send/Recv ordering or use `MPI_Sendrecv` / non-blocking calls.

### MPI Execution Model in Detail

When you run `mpirun -np 4 ./mpi_bin`, these things happen in order:

1. `mpirun` (or `mpiexec`) spawns four OS processes, possibly on different machines.
2. Each process starts executing `main()`.
3. `MPI_Init` establishes communication channels between all processes. An MPI "world" is created connecting all four. If processes are on different nodes, a side-band daemon (e.g., `orted` in Open MPI) negotiates the TCP/InfiniBand connection.
4. Each process runs independently until it calls an MPI function, at which point the MPI library may synchronise with other processes (for collectives) or send/receive messages (for point-to-point).
5. `MPI_Finalize` tears down the MPI infrastructure. After this, no MPI calls are allowed.
6. Each process exits `main()` independently.

The SPMD model is deliberately simple. Every process has the same code but operates on different data (determined by `rank`). This is the same model used by GPU kernels (SIMT) and data-parallel frameworks like MapReduce.

### MPI Buffer Modes

`MPI_Send` is **blocking** but its semantics depend on whether the MPI implementation buffers the message or not:

- **Buffered mode**: the library copies the message into an internal buffer and returns immediately. The send buffer can be reused right away.
- **Synchronous mode**: the library blocks until the matching receive has started. The send buffer must stay valid until `MPI_Send` returns.

Most implementations use a hybrid: small messages (≤ a threshold, e.g., 256 KB) are buffered; large messages are synchronous. This means the classic deadlock pattern (everybody sends first) does *not* always deadlock if messages are small enough to be buffered — but *relying* on that is a bug. Always write deadlock-free code using `MPI_Sendrecv` or non-blocking calls.

Explicit buffer modes:

| Function | Behaviour |
|----------|-----------|
| `MPI_Bsend` | Buffered send — user provides buffer via `MPI_Buffer_attach`. Always returns immediately. |
| `MPI_Ssend` | Synchronous send — always blocks until matching receive is posted. |
| `MPI_Rsend` | Ready send — user guarantees the matching receive is already posted. Slightly lower overhead. |
| `MPI_Isend` / `MPI_Ibsend` / `MPI_Issend` / `MPI_Irsend` | Non-blocking variants of the above. |

## Build It

We assume Open MPI or MPICH is installed. To install:

- **macOS:** `brew install open-mpi`
- **Ubuntu/Debian:** `sudo apt install mpich` or `sudo apt install openmpi-bin libopenmpi-dev`
- **Fedora:** `sudo dnf install mpich`
- **Python (mpi4py):** `pip install mpi4py`

The C code compiles with:

```
mpicc -O2 -Wall -o mpi_bin code/main.c
mpirun -np 4 ./mpi_bin
```

The Python code runs with:

```
mpirun -np 4 python3 code/main.py
```

We build four steps. Each step adds a capability. All four live in the same `main.c` / `main.py`; run them with `-np 4`.

### Step 1 — Hello World: Rank and Size

Every MPI program follows the same skeleton:

```c
#include <mpi.h>
#include <stdio.h>

int main(int argc, char **argv) {
    MPI_Init(&argc, &argv);            // 1. Start MPI

    int rank, size;
    MPI_Comm_rank(MPI_COMM_WORLD, &rank);  // 2. Who am I?
    MPI_Comm_size(MPI_COMM_WORLD, &size);  // 3. How many of us?

    printf("Hello from rank %d of %d\n", rank, size);

    MPI_Finalize();                    // 4. Tear down MPI
    return 0;
}
```

Key observations:

- `MPI_Init` must be called before any other MPI function. It sets up the communication infrastructure. If you call any other MPI function before it, the result is undefined (likely a crash).
- `MPI_Comm_rank` queries the calling process's rank inside a communicator.
- `MPI_Comm_size` returns the total number of processes.
- Every process runs the *same binary* — this is **SPMD** (Single Program, Multiple Data). Each process has its own private stack and heap. There is no automatic sharing.
- Output order is non-deterministic because each process prints independently. The operating system schedules each process's I/O separately.
- Python (mpi4py) version:
  ```python
  from mpi4py import MPI
  comm = MPI.COMM_WORLD
  rank = comm.Get_rank()
  size = comm.Get_size()
  print(f"Hello from rank {rank} of {size}")
  ```
- Try running: `mpirun -np 2 ./mpi_bin`, `mpirun -np 4 ./mpi_bin`, `mpirun -np 8 ./mpi_bin`. Watch how the output interleaving changes.

### Step 2 — Ping-Pong (Point-to-Point)

Two processes exchange a message back-and-forth *n* times. This exercises `MPI_Send` and `MPI_Recv` and confirms bidirectional communication works.

Pseudo:

```
if rank == 0:
    for i in 1..PINGS:
        send(buf, to=1)
        recv(buf, from=1)
elif rank == 1:
    for i in 1..PINGS:
        recv(buf, from=0)
        send(buf, to=0)
```

We use `MPI_Sendrecv` in the final version to avoid deadlock, but the simplest correct pattern alternates Send/Recv so that the two processes never both send at the same time.

### Step 3 — Parallel Sum Reduction (Collectives)

Every process holds a local array of floats. We want the global sum.

1. `MPI_Scatter` — rank 0 distributes chunks of the input array to all ranks.
2. Each rank computes its partial sum.
3. `MPI_Reduce` — rank 0 collects the total sum.

Alternatively, `MPI_Allreduce` makes the total sum available on every rank without a separate broadcast.

### Step 4 — Jacobi Iteration with Halo Exchange

The 2-D Laplace equation `∇²u = 0` on a unit square can be solved iteratively with the Jacobi method:

```
u_new[i][j] = 0.25 * (u[i-1][j] + u[i+1][j] + u[i][j-1] + u[i][j+1])
```

This is a **5-point stencil**: each new value depends on four neighbours (up, down, left, right). We start with an initial guess and repeatedly apply the stencil until the solution converges (`||u_new - u||_∞` < tolerance).

#### Domain Decomposition

We distribute the grid across processes by *rows* (a 1-D decomposition). For `size` processes, each process owns `NROWS / size` contiguous rows. Each process allocates two extra rows — the **halo cells** — one above its local slab (to receive from the neighbour below) and one below (to receive from the neighbour above).

```
+------------------+
| rank 0           |  ← owns rows 0..15   (NROWS=64, size=4)
|   halo from r1   |  ← row 16 is ghost
+------------------+
| rank 1           |  ← owns rows 16..31
|   halo from r0   |  ← row 15 (ghost from above)
|   halo from r2   |  ← row 32 (ghost from below)
+------------------+
| rank 2           |  ← owns rows 32..47
|   halo from r1   |  ← row 31
|   halo from r3   |  ← row 48
+------------------+
| rank 3           |  ← owns rows 48..63
|   halo from r2   |  ← row 47
+------------------+
```

#### Halo Exchange Pattern

Every Jacobi iteration requires four communication steps per process:

1. Post `MPI_Irecv` for the halo row from the left neighbour (rank-1) into the top ghost row.
2. Post `MPI_Irecv` for the halo row from the right neighbour (rank+1) into the bottom ghost row.
3. `MPI_Isend` the local topmost owned row to the left neighbour.
4. `MPI_Isend` the local bottommost owned row to the right neighbour.
5. `MPI_Waitall` — block until all four operations complete.

Non-blocking calls are critical here. If we used `MPI_Send` / `MPI_Recv` in a symmetric pattern (both neighbours send then both receive), we could deadlock. With `MPI_Irecv` posted first, the sends can complete because their matching receives are already posted. Additionally, between posting the receives and calling `MPI_Waitall`, the process can compute the interior points that *don't* depend on halo data — overlapping communication with computation.

#### Boundary Conditions

- Left and right walls (column 0 and column NCOLS-1): Dirichlet BC, fixed at 0.
- Top edge (global row 0, rank 0 only): fixed at initial value from `sin(pi*x) * sinh(pi*(1-y))`.
- Bottom edge (global row NROWS-1, last rank only): same initial pattern.
- Interior points: Jacobi stencil.

#### Convergence Check

After each iteration we compute `||u_new - u||_∞` locally (the maximum absolute difference across all local cells). Each rank tracks its own norm. The global norm is obtained via `MPI_Allreduce(MPI_MAX)` — but for simplicity our code only checks the root's local norm. A production solver would use `MPI_Allreduce` to find the true global maximum difference.

#### Running the Jacobi Solver

```
mpirun -np 4 ./mpi_bin   # includes Jacobi output
```

Observe:
- How the iteration count changes with grid size (try NROWS=128).
- How the norm decreases
- That more processes means smaller local slabs and faster per-iteration time — but more halo communication.

### Running the Code

Compile C:

```bash
mpicc -O2 -Wall -o mpi_bin code/main.c
```

Run all steps with 4 processes:

```bash
mpirun -np 4 ./mpi_bin
```

Python:

```bash
mpirun -np 4 python3 code/main.py
```

Try varying `-np` (2, 4, 8) to see how rank/size changes. For the Jacobi solver, the grid is split among processes, so more processes = smaller local grid per rank.

## Use It

MPI is the backbone of high-performance scientific computing. Production solvers use MPI for:

- **Weather & climate models** (e.g., MPAS, COSMO) — domain-decomposed grids like our Jacobi example, but with hundreds of millions of cells.
- **Molecular dynamics** (e.g., GROMACS, NAMD, LAMMPS) — atoms are spatially decomposed; forces across domain boundaries require halo exchange.
- **Big linear algebra** (e.g., PETSc, ScaLAPACK, Elemental) — matrices are block-cyclically distributed; MPI collectives coordinate factorisations.
- **Deep learning** (e.g., Distributed TensorFlow, PyTorch DDP) — `MPI_Allreduce` averages gradients across GPUs in data-parallel training.

The production version of our Jacobi solver would use:

- **Cartesian communicators** (`MPI_Cart_create`) for n-dimensional topology — makes neighbour lookup trivial.
- **Derived datatypes** (`MPI_Type_vector`, `MPI_Type_create_subarray`) to send non-contiguous halo rows without manual packing.
- **Persistent communication** (`MPI_Send_init` / `MPI_Recv_init` / `MPI_Startall`) for the halo exchange that repeats every iteration — driver overhead drops to near zero.

If you use the C++ bindings (`mpi.h` in C code is idiomatic; the C++ bindings were removed in MPI-3), the same patterns hold.

## Read the Source

- **Open MPI source** — https://github.com/open-mpi/ompi — look at `ompi/mca/coll/tuned/` for tuned collective algorithms (binomial tree for broadcast, Rabenseifner's algorithm for allreduce).
- **MPICH source** — https://github.com/pmodels/mpich — `src/mpi/coll/` has production collective implementations with multiple algorithm choices per communicator size and message size.
- **PETSc `src/snes/tutorials/ex5.c`** — a production finite-difference Jacobi / Newton solver with MPI halo exchange, representative of real CFD codes.

### Debugging MPI Programs

Debugging parallel programs is harder than debugging sequential ones. These techniques help:

**Use totalview or ddt** — HPC debuggers that attach to all MPI processes simultaneously. Set breakpoints that apply across all ranks or conditionally per rank.

**GDB with a wrapper** — Launch GDB on a single rank:
```bash
mpirun -np 4 xterm -e gdb ./mpi_bin
```
Or attach to a specific process by PID. For rank-specific breakpoints:
```c
if (rank == 2) { volatile int wait = 1; while (wait); }  // attach GDB to rank 2
```

**Print with rank prefix** — Always include the rank in debug output so you can trace which process produced which line:
```c
fprintf(stderr, "[%d] variable x = %f\n", rank, x);
```
Use `stderr` instead of `stdout` because `stdout` may be buffered per-process and interleave unpredictably.

**Check for errors** — Almost all MPI functions return `MPI_SUCCESS` or an error code. If you don't check return values, a silently failing `MPI_Recv` (wrong tag, wrong source) will manifest as a hang or corrupt data. Install an error handler:
```c
MPI_Comm_set_errhandler(MPI_COMM_WORLD, MPI_ERRORS_RETURN);
// then check every call:  if (ierr != MPI_SUCCESS) { ... }
```
Or set `MPI_ERRORS_ARE_FATAL` (the default) to get an immediate abort on any MPI error.

**Valgrind with MPI** — `mpirun -np 4 valgrind ./mpi_bin` catches memory errors. Use `--suppressions=mpi.supp` to suppress false positives from the MPI library itself.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`:

- **`outputs/README.md`** — Cheat-sheet reference for the four MPI steps: rank/size, ping-pong, parallel reduction, and Jacobi halo exchange. Copy-paste these patterns into future distributed-memory projects.

## Exercises

1. **Easy** — Run the C and Python programs with `-np 2`, `-np 4`, and `-np 8`. Observe how the output order changes. Why does rank 0 not always print first?
2. **Easy** — In the ping-pong step, change the message to an array of 10,000 integers. How does performance change? Try with `MPI_Ssend` instead of `MPI_Send` — does the program still complete?
3. **Medium** — Modify the parallel sum to use `MPI_Allreduce` instead of `Scatter` + `Reduce`. Count the messages (theoretical): how many network transfers does Scatter use vs. Allreduce?
4. **Medium** — Add a 5-point stencil Jacobi in 3-D. Distribute the grid along the z-axis (slabs). Extend `main.c` / `main.py` with a 3-D halo exchange — you need two neighbour pairs instead of one, and six halo faces instead of two.
5. **Medium** — Implement a ring broadcast: rank 0 sends to rank 1, rank 1 forwards to rank 2, ... rank n-1 forwards back to rank 0. Compare with `MPI_Bcast` for latency on `-np 8`. Which is faster and why?
6. **Hard** — Replace the blocking halo exchange in the Jacobi solver with `MPI_Isend` / `MPI_Irecv` / `MPI_Waitall`. Benchmark iteration time with blocking vs. non-blocking for a 1024×1024 grid on 4 processes. Is non-blocking faster? Why or why not?
7. **Hard** — Use `MPI_Cart_create` to build a 2-D Cartesian communicator (size = P × Q). Replace the 1-D row decomposition in Jacobi with a 2-D block decomposition. Each process now has four neighbours (N, S, E, W) and must exchange four halo faces.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| MPI | Message Passing Interface | Standard library for distributed-memory parallelism; processes communicate by sending messages, not by sharing memory. |
| Rank | "My rank is k" | Unique integer ID of a process within a communicator (0 … size−1). Messages are addressed to ranks. |
| Communicator | "Create a new communicator" | A group of MPI processes that can exchange messages. `MPI_COMM_WORLD` is the default. |
| Point-to-point | "Send from rank i to rank j" | One process sends a message; one process receives it. `MPI_Send` / `MPI_Recv`. |
| Collective | "Broadcast the result to everyone" | All processes in a communicator participate in a single coordinated operation. |
| Broadcast | "Bcast the initial condition" | `MPI_Bcast`: root → all. One-to-all data distribution. |
| Reduce | "Reduce the partial sums" | `MPI_Reduce`: all → root with an associative operation (sum, max, min, etc.). |
| Scatter | "Scatter the input data" | `MPI_Scatter`: root splits an array and sends each chunk to a different rank. |
| Gather | "Gather the results" | `MPI_Gather`: each rank sends data; root collects into one array. Inverse of Scatter. |
| Allreduce | "Sync gradients across GPUs" | `MPI_Allreduce`: all → all. Every rank gets the reduction result. |
| Non-blocking | "Post the receive and keep working" | `MPI_Isend` / `MPI_Irecv` return immediately; `MPI_Wait` completes. Overlaps communication with computation. |
| Halo exchange | "Swap ghost cells" | At every time step, boundary data from neighbouring sub-domains is exchanged so interior stencils can be computed. |
| Deadlock | "Both processes are stuck waiting for each other" | Both processes call `MPI_Send` before either calls `MPI_Recv`. The fix: re-order ops or use `MPI_Sendrecv`. |

## Further Reading

- **MPI: A Message-Passing Interface Standard** — the official specification (mpi-forum.org). Read the "MPI-4.0" document for the definitive reference on every function, constant, and error code.
- **Gropp, Lusk, Skjellum, "Using MPI"** (3rd ed., MIT Press) — the canonical textbook. Portable, practical, covers MPI-3 extensively. Every MPI programmer should own a copy.
- **Pacheco, "An Introduction to Parallel Programming"** — gentle introduction with MPI, OpenMP, and GPU chapters. Great for self-study.
- **MPI for Python (mpi4py) docs** — https://mpi4py.readthedocs.io/ — Python bindings with a dead-simple API. The `MPI.PointToPoint` and `MPI.Collective` pages are particularly helpful.
- **LLNL MPI Tutorials** — https://hpc-tutorials.llnl.gov/mpi/ — practical tutorials from Lawrence Livermore National Lab with many worked examples.
- **PETSc manual** (https://petsc.org/) — production toolkit for PDE solvers built on MPI. The manual's tutorial section walks through finite-difference, finite-element, and spectral methods all using MPI.
- **"A Survey of MPI Usage in the U.S. Exascale Computing Project"** — how production applications actually use MPI collectives, non-blocking operations, and derived datatypes. Real insights from real codes.
- **Hoefler, Siebert, Lumsdaine, "Scalable Communication Protocols for Dynamic Sparse Data Exchange"** — discusses advanced MPI usage patterns for irregular communication (not covered in this lesson but essential for real applications).
