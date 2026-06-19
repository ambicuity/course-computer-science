# Lesson 09: Page Replacement — LRU, Clock, ARC

## Core Concepts

When physical memory is full and a new page must be loaded, the OS must **evict** (replace) an existing page. The choice of which page to evict directly impacts performance: evicting a frequently used page causes unnecessary future page faults.

**Reference string:** a sequence of page numbers accessed by a process. Used to evaluate replacement algorithms.

## Optimal (Belady's Algorithm)

Evict the page that will not be used for the longest time in the future. This is the theoretical best — it minimizes page faults.

```
Frames: 3
Reference: 7 0 1 2 0 3 0 4 2 3 0 3 2 1 2 0 1 7 0 1

Optimal: evict page used furthest in future
Page faults: 9 (minimum possible)
```

**Problem:** requires knowledge of the future. Cannot be implemented in a real system. Serves as a benchmark: if your algorithm produces close to optimal, it's good.

## FIFO (First-In-First-Out)

Evict the page that has been in memory the longest. Simple queue: new pages enter at the tail, eviction removes from the head.

```
Frames: 3
Reference: 1 2 3 4 1 2 5 1 2 3 4 5

Step  Ref  Frames       Evict  Fault?
 1     1  [1]              -     ✓
 2     2  [1,2]            -     ✓
 3     3  [1,2,3]          -     ✓
 4     4  [4,2,3]          1     ✓
 5     1  [4,1,3]          2     ✓
 6     2  [4,1,2]          3     ✓
 7     5  [5,1,2]          4     ✓
 8     1  [5,1,2]          -     -
 9     2  [5,1,2]          -     -
10     3  [5,3,2]          1     ✓
11     4  [5,3,4]          2     ✓
12     5  [5,3,4]          -     -
Total page faults: 10
```

**Belady's anomaly:** increasing the number of frames can *increase* the number of page faults with FIFO. This does not happen with stack algorithms (LRU, optimal).

```
Reference: 1 2 3 4 1 2 5 1 2 3 4 5
3 frames → 9 faults
4 frames → 10 faults (more frames, more faults!)
```

## LRU (Least Recently Used)

Evict the page that has not been accessed for the longest time. Uses **recency** of access as a proxy for future use.

```
Reference: 1 2 3 4 1 2 5 1 2 3 4 5

Step  Ref  Frames       Evict  Fault?
 4     4  [4,2,3]          1     ✓  (1 was least recent)
 5     1  [4,1,3]          2     ✓
 6     2  [4,1,2]          3     ✓
 7     5  [5,1,2]          4     ✓
...
```

**Implementation approaches:**
- **Timestamp:** record last access time per page, scan for minimum — O(n) per access
- **Stack:** doubly-linked list, move accessed page to head, evict tail — O(1) per access
- **Hardware support:** reference + dirty bits help approximate

LRU is a **stack algorithm**: the set of pages in k frames is always a subset of pages in k+1 frames. This prevents Belady's anomaly.

## Clock Algorithm

A practical approximation of LRU. Uses a circular list and a single **reference bit** per page.

```
       ┌───┐
   ┌───┤ 0 ├───┐
   │   └───┘   │
┌──┴──┐     ┌──┴──┐
│  1  │     │  3  │    hand → 0
└──┬──┘     └──┬──┘
   │   ┌───┐   │
   └───┤ 2 ├───┘
       └───┘

Clock hand sweeps clockwise.
```

**Algorithm:**
1. On page fault, check the page at the hand position
2. If reference bit = 1: clear it, advance hand
3. If reference bit = 0: evict this page
4. Repeat until a victim is found

```
Reference: 1 2 3 4 1 2 5 1 2 3 4 5
Frames: 3

Access 1: fault, frames=[1,r] hand=1
Access 2: fault, frames=[1,r; 2,r] hand=2
Access 3: fault, frames=[1,r; 2,r; 3,r] hand=0
Access 4: fault, sweep: 1(r→0), 2(r→0), 3(r→0), evict 1
          frames=[4,r; 2,0; 3,0] hand=1
...
```

**Complexity:** O(1) amortized per access. The hand rarely needs to do a full sweep because reference bits are periodically cleared by the OS.

**Enhanced clock:** use both reference and dirty bits. Prefer evicting pages that are neither referenced nor dirty (clean, unused) over dirty pages.

| Ref | Dirty | Priority to evict |
|-----|-------|-------------------|
| 0 | 0 | First (clean, unused) |
| 0 | 1 | Second (dirty, unused) |
| 1 | 0 | Third (clean, recently used) |
| 1 | 1 | Last (dirty, recently used) |

## ARC (Adaptive Replacement Cache)

Combines **recency** (LRU) and **frequency** (LFU) with an adaptive balance parameter.

**Structure:**
- **T1** (recency): LRU list of pages seen only once recently
- **T2** (frequency): LRU list of pages seen at least twice
- **B1** (ghost recency): evicted from T1, tracks recency misses
- **B2** (ghost frequency): evicted from T2, tracks frequency misses
- **p**: adaptive target size for T1

**Adaptation:**
- If a miss hits in B1 (ghost recency): increase p → favor recency
- If a miss hits in B2 (ghost frequency): decrease p → favor frequency
- The cache dynamically adjusts to workload characteristics

```
Cache size c = 4, p = 2

Access 1 (miss):    T1=[1]
Access 2 (miss):    T1=[2,1]
Access 3 (miss):    evict from T1 (1), B1=[1], T1=[3,2]
Access 1 (miss in B1): hit ghost recency → p=3, T1=[1,3,2], T2=[]
Access 4 (miss):    T1 full → evict, T1=[4,1,3]
Access 1 (hit):     move T1→T2, T1=[4,3], T2=[1]
```

ARC is self-tuning. It performs well for both scan-resistant and frequency-biased workloads.

## Comparison

| Algorithm | Optimal? | Belady's Anomaly? | Implementation | Use Case |
|-----------|----------|-------------------|----------------|----------|
| Optimal | Yes | No | Impossible | Benchmark |
| FIFO | No | Yes | Simple queue | Baseline |
| LRU | No | No | Stack/list | Theoretical ideal |
| Clock | Approx LRU | No | Circular + bits | Linux, practical |
| ARC | No | No | Two LRU + ghosts | ZFS, databases |

## Build It

Implement all four replacement algorithms in C and Rust. Feed each the same reference string and frame count. Compare page fault counts.

## Use It

Linux uses a variant of the clock algorithm with active/inactive lists. FreeBSD uses a variant of ARC. ZFS uses ARC for its block cache. Windows uses a complex working-set-based algorithm with clock-like aging.

## Ship It

Your page replacement library should accept a reference string and frame count, run each algorithm, and output the page fault count and eviction sequence. Include a comparison table.

## Exercises

### Level 1 — Concept Check
Given reference string `1 2 3 4 1 2 5 1 2 3 4 5` with 3 frames, calculate the number of page faults for FIFO, LRU, and Optimal.

### Level 2 — Implementation
Implement the enhanced clock algorithm that considers both reference and dirty bits. Test with a reference string that has write operations marked. Compare fault count with the basic clock.

### Level 3 — Design
Design a page replacement system that detects sequential scan patterns (e.g., full table scans in databases) and protects the buffer pool from being flushed. Describe how your algorithm distinguishes scans from random access, and implement it.
