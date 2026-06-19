# Lesson 15: Deadlock — Detection, Prevention, Avoidance

## The Problem

Two or more processes each hold a resource and wait for a resource held by the other. None can proceed. The system — or a subset of it — freezes. Deadlocks waste resources, stall users, and can cascade in large systems. Every OS and database must handle them.

## Four Necessary Conditions

A deadlock can occur only if **all four** of these conditions hold simultaneously:

1. **Mutual Exclusion:** At least one resource is non-shareable — only one process can use it at a time (e.g., a printer, a mutex).
2. **Hold and Wait:** A process holding at least one resource waits to acquire additional resources held by others.
3. **No Preemption:** Resources cannot be forcibly taken from a process; they must be released voluntarily.
4. **Circular Wait:** A circular chain of processes exists where each waits for a resource held by the next: P1 → P2 → P3 → P1.

## Resource Allocation Graph (RAG)

A directed graph with two node types:
- **Processes:** P1, P2, ...
- **Resources:** R1 (2 instances), R2 (1 instance), ...

Edges:
- **Request edge:** Pi → Rj (Pi is waiting for Rj)
- **Assignment edge:** Rj → Pi (Rj is held by Pi)

**Single-instance resources:** A cycle in the RAG **is** a deadlock.
**Multiple-instance resources:** A cycle is a **necessary but not sufficient** condition. You need further analysis (Banker's algorithm or wait-for graph).

```
Example (single-instance, deadlock):

  P1 ──request──→ R1      P1 holds R2, wants R1
  R1 ──assign───→ P2      P2 holds R1, wants R2
  P2 ──request──→ R2
  R2 ──assign───→ P1

  Cycle: P1 → R1 → P2 → R2 → P1  → DEADLOCK
```

## Prevention

Prevention eliminates one of the four conditions:

| Condition Broken | Strategy | Trade-off |
|-----------------|----------|-----------|
| Mutual Exclusion | Use shareable resources (read-only files, lock-free DS) | Not always possible |
| Hold and Wait | Request all resources at once before execution | Low utilization, starvation risk |
| No Preemption | Allow OS to preempt resources (kill process, reclaim locks) | Rollback complexity |
| Circular Wait | Impose a **resource ordering** — always request resources in increasing order | Restricts programming flexibility |

Resource ordering is the most practical prevention technique. Example: always acquire lock A before lock B. If every thread follows the same order, circular wait is impossible.

```c
// CORRECT: always lock lower-indexed resource first
if (id_a < id_b) { lock(a); lock(b); }
else             { lock(b); lock(a); }
```

## Avoidance — Banker's Algorithm

Avoidance does not prevent deadlock outright. Instead, the OS checks whether granting a request would leave the system in a **safe state** (a state from which all processes can eventually finish). If not, the request is delayed.

### Definitions

- **Available[m]:** instances of each resource type currently free.
- **Max[n][m]:** maximum demand of each process for each resource type.
- **Allocation[n][m]:** resources currently allocated to each process.
- **Need[n][m] = Max − Allocation:** remaining demand.

### Safety Algorithm

```
1. Work = Available; Finish[i] = false for all i
2. Find an i where Finish[i] == false AND Need[i] ≤ Work
3. If found: Work += Allocation[i]; Finish[i] = true; go to 2
4. If all Finish[i] == true → SAFE state
   If no such i exists and some Finish[i] == false → UNSAFE state
```

### Resource-Request Algorithm

When process Pi requests Request[i]:
1. If Request[i] > Need[i] → error (exceeded max claim).
2. If Request[i] > Available → Pi must wait.
3. Pretend to allocate: Available -= Request[i]; Allocation[i] += Request[i]; Need[i] -= Request[i].
4. Run safety algorithm. If safe → grant. If unsafe → revert and make Pi wait.

The Banker's algorithm is conservative — it may deny requests that would not actually cause deadlock. This is the cost of safety guarantees.

## Detection

If prevention and avoidance are not used, the system must **detect** deadlocks periodically and recover.

### Single-Instance: Wait-For Graph

Build a graph with edges Pi → Pj (Pi waits for Pj). A cycle means deadlock. Cycle detection is O(V + E) via DFS.

### Multiple-Instance: Detection Algorithm

Similar to the Banker's safety algorithm but uses the current **Request** matrix instead of Need:

```
1. Work = Available; Finish[i] = (Allocation[i] == 0)
2. Find i where Finish[i] == false AND Request[i] ≤ Work
3. Work += Allocation[i]; Finish[i] = true; go to 2
4. All Finish[i] == true → no deadlock
   Any Finish[i] == false → that process is deadlocked
```

### Python Implementation

```python
def detect_deadlock(available, request, allocation, n, m):
    work = list(available)
    finish = [sum(allocation[i]) == 0 for i in range(n)]
    changed = True
    while changed:
        changed = False
        for i in range(n):
            if not finish[i] and all(request[i][j] <= work[j] for j in range(m)):
                for j in range(m):
                    work[j] += allocation[i][j]
                finish[i] = True
                changed = True
    deadlocked = [i for i in range(n) if not finish[i]]
    return deadlocked  # empty = no deadlock
```

## Recovery

Once deadlock is detected, the system must recover:

1. **Process Termination:** Kill one or more deadlocked processes. Choose the cheapest victim (fewest resources held, lowest priority, least work done).
2. **Resource Preemption:** Take a resource from a process. Requires rollback to a safe state (checkpointing). Must handle starvation — avoid always preempting the same process.
3. **Rollback:** Restore processes to earlier checkpoints and restart. All work since the checkpoint is lost.

## Build It: Banker's Algorithm + Deadlock Detector

The code implements:
- A resource allocation graph with cycle detection (DFS).
- The Banker's algorithm safety check and resource request handler.
- A deadlock detector using the wait-for graph approach.
- Demo scenarios that create and detect deadlocks.

## Use It

Databases use deadlock detection extensively — transactions acquire row locks, and a background thread periodically scans the wait-for graph for cycles. The OS uses prevention (lock ordering in the kernel) and detection (for user-space locks). Understanding these algorithms is critical for building reliable concurrent systems.

## Ship It

A deadlock toolkit combining prevention (resource ordering), avoidance (Banker's), and detection (wait-for graph cycles) demonstrates that deadlock handling is a spectrum — different systems make different trade-offs between performance and safety.

## Exercises

**Level 1 — Cycle Detection:**
Implement a Resource Allocation Graph. Add 5 processes and 3 single-instance resources. Manually create a circular wait. Run your cycle detection to confirm the deadlock and print the deadlocked processes.

**Level 2 — Banker's Algorithm:**
Implement the full Banker's algorithm. Input: Available = [3, 3, 2], Max matrix, Allocation matrix. Compute Need, run the safety algorithm to find a safe sequence. Then simulate a resource request from process 1 and determine whether it can be granted.

**Level 3 — Deadlock Scenario Generator and Resolver:**
Write a program that randomly generates process resource requests over time, builds a wait-for graph, detects deadlocks when they occur, and recovers by selecting the cheapest victim process to terminate. Run 1000 trials and report: deadlock frequency, average processes terminated, and total resource utilization.
