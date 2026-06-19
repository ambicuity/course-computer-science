"""
Parallel Patterns — Map, Reduce, Pipeline, Scan
Phase 13 — Concurrent & Parallel Computing

Demonstrates four fundamental parallel patterns using multiprocessing.Pool:
  1. Map — embarrassingly parallel element-wise transformation
  2. Reduce — parallel tree reduction via chunked partial sums
  3. Pipeline — task parallelism with Process + Queue
  4. Scan — parallel prefix sum (Hillis-Steele)

Run: python main.py
"""

from multiprocessing import Pool, Process, Queue, cpu_count
import time
import math
import os
import sys

ARRAY_SIZE = 10_000_000
SMALL_SIZE = 1_000_000

# ============================================================
# STEP 1 — Parallel Map with Pool.map
# ============================================================

def square(x: int) -> int:
    return x * x + 2 * x + 1


def heavy_func(x: float) -> float:
    result = x
    for _ in range(100):
        result = math.sin(result) * math.cos(result)
    return result


def step1_parallel_map():
    print("=== Step 1: Parallel Map ===")

    data = list(range(SMALL_SIZE))

    t0 = time.perf_counter()
    seq = [square(x) for x in data]
    seq_time = time.perf_counter() - t0

    with Pool() as pool:
        t0 = time.perf_counter()
        par = pool.map(square, data)
        par_time = time.perf_counter() - t0

    assert seq == par, "Results differ!"
    print(f"  f(x)=x²+2x+1, n={SMALL_SIZE}")
    print(f"  Sequential: {seq_time:.4f}s  Parallel: {par_time:.4f}s  "
          f"Speedup: {seq_time / par_time:.2f}x")

    fdata = [float(i) for i in range(ARRAY_SIZE)]
    t0 = time.perf_counter()
    seq_h = [heavy_func(x) for x in fdata[:SMALL_SIZE]]
    seq_h_time = time.perf_counter() - t0

    with Pool() as pool:
        t0 = time.perf_counter()
        par_h = pool.map(heavy_func, fdata[:SMALL_SIZE], chunksize=1000)
        par_h_time = time.perf_counter() - t0

    print(f"  Heavy f(x)=sin(cos(x))×100, n={SMALL_SIZE}")
    print(f"  Sequential: {seq_h_time:.4f}s  Parallel: {par_h_time:.4f}s  "
          f"Speedup: {seq_h_time / par_h_time:.2f}x")
    print()


# ============================================================
# STEP 2 — Parallel Reduce
#
# A full parallel reduce requires a tree combine of partial sums.
# With Pool, the idiom is:
#   1. Split data into chunks
#   2. Map each chunk through a partial reduction
#   3. Combine partial results sequentially (trivial step)
# ============================================================

def partial_sum(chunk):
    return sum(chunk)


def partial_minmax(chunk):
    return min(chunk), max(chunk)


def step2_parallel_reduce():
    print("=== Step 2: Parallel Reduce ===")

    data = list(range(ARRAY_SIZE))

    t0 = time.perf_counter()
    seq_sum = sum(data)
    seq_time = time.perf_counter() - t0

    num_chunks = cpu_count() * 4
    chunk_size = max(1, len(data) // num_chunks)
    chunks = [data[i:i + chunk_size] for i in range(0, len(data), chunk_size)]

    with Pool() as pool:
        t0 = time.perf_counter()
        partials = pool.map(partial_sum, chunks)
        par_time = time.perf_counter() - t0
    par_sum = sum(partials)

    assert seq_sum == par_sum, f"Sums differ: {seq_sum} vs {par_sum}"
    print(f"  Sum (n={ARRAY_SIZE}): seq={seq_time:.4f}s par={par_time:.4f}s "
          f"speedup={seq_time / par_time:.2f}x")

    t0 = time.perf_counter()
    seq_min = min(data)
    seq_max = max(data)
    seq_mm_time = time.perf_counter() - t0

    with Pool() as pool:
        t0 = time.perf_counter()
        minmax_results = pool.map(partial_minmax, chunks)
        par_mm_time = time.perf_counter() - t0
    par_min = min(m for m, _ in minmax_results)
    par_max = max(mx for _, mx in minmax_results)

    assert seq_min == par_min and seq_max == par_max
    print(f"  Min/Max: seq={seq_mm_time:.4f}s par={par_mm_time:.4f}s "
          f"speedup={seq_mm_time / par_mm_time:.2f}x")
    print()


# ============================================================
# STEP 3 — Pipeline with Process + Queue
#
# Stage 1 (producer): generates numbers 0..N
# Stage 2 (filter):   keeps only even numbers
# Stage 3 (doubler):  multiplies by 2
# Stage 4 (collector): verifies and sums results
# ============================================================

def producer(out_q: Queue, n: int):
    for i in range(n):
        out_q.put(i)
    out_q.put(None)


def filter_even(in_q: Queue, out_q: Queue):
    count = 0
    while True:
        val = in_q.get()
        if val is None:
            out_q.put(None)
            break
        if val % 2 == 0:
            out_q.put(val)
            count += 1
    print(f"  Stage 2 (filter): forwarded {count} items")


def doubler(in_q: Queue, out_q: Queue):
    count = 0
    while True:
        val = in_q.get()
        if val is None:
            out_q.put(None)
            break
        out_q.put(val * 2)
        count += 1
    print(f"  Stage 3 (doubler): processed {count} items")


def collector(in_q: Queue, expected: int):
    results = []
    while True:
        val = in_q.get()
        if val is None:
            break
        results.append(val)
    total = sum(results)
    assert total == expected, f"Pipeline mismatch: {total} != {expected}"
    print(f"  Stage 4 (collector): {len(results)} items, sum={total}, correct!")


def step3_pipeline():
    print("=== Step 3: Pipeline ===")

    n = 10_000
    q1 = Queue()
    q2 = Queue()
    q3 = Queue()
    expected = sum(i * 2 for i in range(n) if i % 2 == 0)

    p1 = Process(target=producer, args=(q1, n))
    p2 = Process(target=filter_even, args=(q1, q2))
    p3 = Process(target=doubler, args=(q2, q3))
    p4 = Process(target=collector, args=(q3, expected))

    t0 = time.perf_counter()
    p1.start()
    p2.start()
    p3.start()
    p4.start()
    p1.join()
    p2.join()
    p3.join()
    p4.join()
    elapsed = time.perf_counter() - t0

    print(f"  Pipeline completed in {elapsed:.4f}s")
    print()


# ============================================================
# STEP 4 — Parallel Prefix Scan (Hillis-Steele)
#
# Hillis-Steele algorithm with double buffering.
# Each step reads from the "old" array and writes to the "new" array,
# then swaps.  This ensures no data races within a step.
#
# Work:  W = n log₂ n  (redundant computation)
# Span:  T = log₂ n
#
# Note: Pool.map serializes its arguments, so passing the full
# array to each worker incurs overhead.  For production use,
# prefer shared memory (multiprocessing.shared_memory, Python 3.8+).
# ============================================================

def scan_chunk(args):
    """Process a range of indices for one step of Hillis-Steele."""
    start, end, old, d = args
    result = []
    for i in range(start, end):
        if i >= d:
            result.append(old[i] + old[i - d])
        else:
            result.append(old[i])
    return start, result


def hillis_steele_scan(arr):
    n = len(arr)
    if n <= 1:
        return arr[:]

    old = list(arr)
    d = 1
    num_workers = cpu_count()
    # Aim for ~4× as many chunks as workers for load balancing
    chunk_size = max(1, n // (num_workers * 4))

    with Pool() as pool:
        while d < n:
            chunks = []
            for start in range(0, n, chunk_size):
                end = min(start + chunk_size, n)
                chunks.append((start, end, old, d))

            results = pool.map(scan_chunk, chunks)

            new = [0] * n
            for start_idx, chunk_data in results:
                for j, val in enumerate(chunk_data):
                    new[start_idx + j] = val
            old = new
            d *= 2

    return old


def step4_parallel_scan():
    print("=== Step 4: Parallel Prefix Scan ===")

    n = 50_000
    input_data = list(range(1, n + 1))

    t0 = time.perf_counter()
    acc = 0
    seq = []
    for x in input_data:
        acc += x
        seq.append(acc)
    seq_time = time.perf_counter() - t0

    t0 = time.perf_counter()
    par = hillis_steele_scan(input_data)
    par_time = time.perf_counter() - t0

    if seq == par:
        correct = "✓"
    else:
        correct = "✗"
        for i, (a, b) in enumerate(zip(seq, par)):
            if a != b:
                print(f"  First mismatch at index {i}: seq={a} par={b}")
                break

    print(f"  n={n}")
    print(f"  Sequential: {seq_time:.4f}s  Parallel: {par_time:.4f}s  "
          f"Speedup: {seq_time / par_time:.2f}x")
    print(f"  Correctness: {correct}")
    print()


# ============================================================
# MAIN
# ============================================================

def main():
    print("=" * 55)
    print("  Parallel Patterns — Map, Reduce, Pipeline, Scan")
    print("=" * 55)
    print(f"  CPU cores: {cpu_count()}")
    print(f"  PID: {os.getpid()}")
    print(f"  Python: {sys.version.split()[0]}")
    print()

    step1_parallel_map()
    step2_parallel_reduce()
    step3_pipeline()
    step4_parallel_scan()

    print("All demos complete.")


if __name__ == "__main__":
    main()
