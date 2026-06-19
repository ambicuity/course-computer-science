import random
from collections import deque
from dataclasses import dataclass, field
from typing import Optional


class Process:
    def __init__(self, pid: int, name: str = ""):
        self.pid = pid
        self.name = name or f"P{pid}"
        self.lc = 0
        self.log: list[tuple[str, int, int, str]] = []

    def _record(self, event_type: str, label: str) -> tuple[int, int]:
        entry = (event_type, self.lc, self.pid, label)
        self.log.append(entry)
        return (self.lc, self.pid)

    def internal_event(self, label: str) -> tuple[int, int]:
        self.lc += 1
        return self._record("internal", label)

    def send_event(self, label: str, dst: int) -> tuple[int, int, int, str]:
        self.lc += 1
        self._record("send", label)
        return (self.lc, self.pid, dst, label)

    def receive_event(self, msg_ts: int, label: str) -> tuple[int, int]:
        self.lc = max(self.lc, msg_ts) + 1
        return self._record("recv", label)


@dataclass
class Message:
    content: tuple[int, int, int, str]
    src: int
    dst: int


class SimulatedNetwork:
    def __init__(self, delay_range: tuple[int, int] = (1, 3), seed: int = 42):
        self.rng = random.Random(seed)
        self.delay_range = delay_range
        self.in_flight: list[tuple[int, Message]] = []
        self.tick = 0

    def send(self, msg: tuple[int, int, int, str], src: int, dst: int):
        delay = self.rng.randint(*self.delay_range)
        arrival = self.tick + delay
        self.in_flight.append((arrival, Message(msg, src, dst)))
        self.in_flight.sort(key=lambda x: x[0])

    def deliver(self) -> list[Message]:
        self.tick += 1
        ready = [(t, m) for t, m in self.in_flight if t <= self.tick]
        self.in_flight = [(t, m) for t, m in self.in_flight if t > self.tick]
        return [m for _, m in ready]

    def deliver_all(self) -> list[Message]:
        all_msgs = [m for _, m in self.in_flight]
        self.in_flight.clear()
        return all_msgs


class LamportMutex:
    def __init__(self, processes: list[Process]):
        self.processes = {p.pid: p for p in processes}
        self.n = len(processes)
        self.queues: dict[int, list[tuple[int, int]]] = {p.pid: [] for p in processes}
        self.acks_received: dict[int, set[int]] = {}
        self.in_cs: Optional[int] = None
        self.cs_order: list[tuple[int, int]] = []

    def request(self, pid: int, resource: str = "R") -> tuple[int, int]:
        p = self.processes[pid]
        ts = p.internal_event(f"request-{resource}")
        self.queues[pid].append(ts)
        self.queues[pid].sort()
        self.acks_received[pid] = {pid}
        for other_pid in self.processes:
            if other_pid != pid:
                self.queues[other_pid].append(ts)
                self.queues[other_pid].sort()
                self.acks_received[pid].add(other_pid)
        return ts

    def can_enter(self, pid: int) -> bool:
        if pid not in self.acks_received:
            return False
        if len(self.acks_received[pid]) < self.n:
            return False
        if not self.queues[pid]:
            return False
        head = self.queues[pid][0]
        return head[1] == pid

    def release(self, pid: int, resource: str = "R"):
        p = self.processes[pid]
        p.internal_event(f"release-{resource}")
        self.queues[pid] = [e for e in self.queues[pid] if e[1] != pid]
        for other_pid in self.processes:
            if other_pid != pid:
                self.queues[other_pid] = [e for e in self.queues[other_pid] if e[1] != pid]
        self.cs_order.append((pid,))
        self.in_cs = None


def visualize_event_diagram(processes: list[Process], messages: list[tuple[int, int, int, str]]):
    max_lc = max(max(e[1] for e in p.log) for p in processes if p.log)
    num_procs = len(processes)
    width = max_lc + 4

    grid: dict[tuple[int, int], str] = {}
    pids = [p.pid for p in processes]
    pid_to_col = {pid: i for i, pid in enumerate(pids)}

    for p in processes:
        col = pid_to_col[p.pid]
        prev_lc = 0
        for event_type, lc, pid, label in p.log:
            row = lc
            if event_type == "internal":
                grid[(row, col)] = f"*{label[:3]}"
            elif event_type == "send":
                grid[(row, col)] = f">{label[:3]}"
            elif event_type == "recv":
                grid[(row, col)] = f"<{label[:3]}"
            prev_lc = lc

    print("\n=== Event Diagram (Lamport Clock Time ─ vertical) ===\n")
    header = "  LC │"
    for p in processes:
        header += f" {p.name:^8}│"
    print(header)
    sep = "─────┼"
    for _ in processes:
        sep += "─────────┼"
    print(sep)

    for t in range(1, max_lc + 1):
        row_str = f" {t:3d} │"
        for col in range(num_procs):
            key = (t, col)
            if key in grid:
                row_str += f" {grid[key]:^7}│"
            else:
                row_str += "    ·    │"
        print(row_str)

    if messages:
        print("\n  Messages:")
        for lc, src, dst, label in messages:
            src_name = next(p.name for p in processes if p.pid == src)
            dst_name = next(p.name for p in processes if p.pid == dst)
            print(f"    LC={lc}: {src_name} → {dst_name} [{label}]")


def verify_lamport_guarantee(processes: list[Process], causal_pairs: list[tuple[int, int, int, int]]):
    print("\n=== Verification: a → b ⟹ LC(a) < LC(b) ===\n")
    all_ok = True
    for src_pid, src_lc, dst_pid, dst_lc in causal_pairs:
        src_name = next(p.name for p in processes if p.pid == src_pid)
        dst_name = next(p.name for p in processes if p.pid == dst_pid)
        if src_lc < dst_lc:
            print(f"  ✓ {src_name}[LC={src_lc}] → {dst_name}[LC={dst_lc}]: "
                  f"LC({src_lc}) < LC({dst_lc})")
        else:
            print(f"  ✗ VIOLATED: {src_name}[LC={src_lc}] → {dst_name}[LC={dst_lc}]: "
                  f"LC({src_lc}) >= LC({dst_lc})")
            all_ok = False
    print(f"\n  Guarantee holds: {all_ok}")
    return all_ok


def demonstrate_concurrent_counterexample():
    print("\n=== Counterexample: LC(a) < LC(b) does NOT mean a → b ===\n")

    p1 = Process(1, "P1")
    p2 = Process(2, "P2")
    for _ in range(3):
        p1.internal_event("warmup")
    p2.internal_event("warmup")
    a = p1.internal_event("a")
    b = p2.internal_event("b")
    print(f"  P1 has done 3 prior events. P1 internal event 'a': LC = {a[0]}")
    print(f"  P2 has done 1 prior event.  P2 internal event 'b': LC = {b[0]}")
    print(f"\n  No message passed between P1 and P2. Events a and b are CONCURRENT.")
    print(f"  Yet LC(a) = {a[0]} > LC(b) = {b[0]}. If we only saw the timestamps,")
    print(f"  we might think b → a. But that's WRONG — they're unrelated.")
    print(f"\n  Conversely, if P1 had fewer prior events, LC(a) could be < LC(b).")
    print(f"  Either way, the LC comparison tells us NOTHING about causality.")

    p1b = Process(1, "P1b")
    p2b = Process(2, "P2b")
    p1b.lc = 5
    a2 = p1b.internal_event("x")
    p2b.lc = 5
    b2 = p2b.internal_event("y")
    print(f"\n  Another case: P1b event x at LC={a2[0]}, P2b event y at LC={b2[0]}")
    print(f"  Same Lamport clock value! Clearly not causally related.")
    print(f"  Vector clocks resolve this: VC(x) = [6, 0], VC(y) = [0, 6].")
    print(f"  Neither dominates the other → confirmed concurrent.")


def simulate_three_processes():
    print("=== Three-Process Lamport Clock Simulation ===\n")

    p1 = Process(1, "P1")
    p2 = Process(2, "P2")
    p3 = Process(3, "P3")
    processes = [p1, p2, p3]
    messages: list[tuple[int, int, int, str]] = []
    causal_pairs: list[tuple[int, int, int, int]] = []

    a_lc, a_pid = p1.internal_event("a")
    causal_pairs.append((a_pid, a_lc, a_pid, a_lc + 1))

    msg1 = p1.send_event("m1→2", dst=2)
    messages.append((msg1[0], msg1[1], msg1[2], msg1[3]))
    sender_lc = msg1[0]
    sender_pid = msg1[1]

    b_lc, _ = p2.internal_event("b")

    recv1_lc, recv1_pid = p2.receive_event(msg1[0], "m1_from1")
    causal_pairs.append((sender_pid, sender_lc, recv1_pid, recv1_lc))

    c_lc, c_pid = p1.internal_event("c")

    msg2 = p1.send_event("m2→3", dst=3)
    messages.append((msg2[0], msg2[1], msg2[2], msg2[3]))
    sender2_lc = msg2[0]
    sender2_pid = msg2[1]

    d_lc, d_pid = p3.receive_event(msg2[0], "m2_from1")
    causal_pairs.append((sender2_pid, sender2_lc, d_pid, d_lc))

    msg3 = p2.send_event("m3→3", dst=3)
    messages.append((msg3[0], msg3[1], msg3[2], msg3[3]))
    sender3_lc = msg3[0]
    sender3_pid = msg3[1]

    f_lc, f_pid = p3.receive_event(msg3[0], "m3_from2")
    causal_pairs.append((sender3_pid, sender3_lc, f_pid, f_lc))

    p3.internal_event("g")

    visualize_event_diagram(processes, messages)
    verify_lamport_guarantee(processes, causal_pairs)


def simulate_mutual_exclusion():
    print("\n\n=== Lamport Timestamp Mutual Exclusion ===\n")

    p1 = Process(1, "P1")
    p2 = Process(2, "P2")
    p3 = Process(3, "P3")
    processes = [p1, p2, p3]

    p1.internal_event("local_work")
    p1.internal_event("local_op")
    p2.internal_event("local_work")
    mutex = LamportMutex(processes)

    print("  Three processes competing for a shared resource.")
    print("  P1 and P3 have done some local work before requesting.\n")

    ts1 = mutex.request(1, "R")
    print(f"  P1 requests R: timestamp = ({ts1[0]}, {ts1[1]})")

    p3.internal_event("local_op")
    ts2 = mutex.request(3, "R")
    print(f"  P3 requests R: timestamp = ({ts2[0]}, {ts2[1]})")

    ts3 = mutex.request(2, "R")
    print(f"  P2 requests R: timestamp = ({ts3[0]}, {ts3[1]})")

    print(f"\n  Queue at P1: {mutex.queues[1]}")
    print(f"  Queue at P2: {mutex.queues[2]}")
    print(f"  Queue at P3: {mutex.queues[3]}")

    print(f"\n  All queues identical (same total order)? "
          f"{mutex.queues[1] == mutex.queues[2] == mutex.queues[3]}")

    entry_order = []
    for ts_pid in sorted(mutex.queues[1], key=lambda x: (x[0], x[1])):
        entry_order.append(f"P{ts_pid[1]}[{ts_pid[0]}]")
    ordered = sorted(mutex.queues[1], key=lambda x: (x[0], x[1]))
    for rank, entry in enumerate(ordered, 1):
        print(f"  {rank}. P{entry[1]} enters CS (LC={entry[0]}, PID={entry[1]})")

    winner_pid = ordered[0][1]
    print(f"\n  P{winner_pid} has lowest (LC, PID) → enters CS first.")
    print(f"  P1 can enter CS? {mutex.can_enter(1)}")
    print(f"  P2 can enter CS? {mutex.can_enter(2)}")
    print(f"  P3 can enter CS? {mutex.can_enter(3)}")

    mutex.release(winner_pid, "R")
    print(f"\n  --- P{winner_pid} releases ---")
    print(f"  Queue at P1: {mutex.queues[1]}")

    next_winner = sorted(mutex.queues[1], key=lambda x: (x[0], x[1]))
    if next_winner:
        next_pid = next_winner[0][1]
        print(f"  P{next_pid} can now enter: {mutex.can_enter(next_pid)}")
        mutex.release(next_pid, "R")
        print(f"  --- P{next_pid} releases ---")

    remaining = sorted(mutex.queues[1], key=lambda x: (x[0], x[1]))
    if remaining:
        last_pid = remaining[0][1]
        print(f"  P{last_pid} enters: {mutex.can_enter(last_pid)}")
        mutex.release(last_pid, "R")
        print(f"  --- P{last_pid} releases ---")

    print(f"\n  CS entry order: {' → '.join(entry_order)}")
    print(f"  No two processes in CS simultaneously: True (by construction)")


def simulate_physical_clock_drift():
    print("\n=== Physical Clock Drift Demonstration ===\n")
    print("  Typical quartz oscillator drift: 10-100 μs/minute")
    print("  After 1 hour: 0.6-6 ms of clock skew")
    print("  After 1 day: 14.4-144 ms of clock skew")
    print()
    print("  NTP corrects but has residuals:")
    print("    LAN:  ±0.1-1 ms    (good for same datacenter)")
    print("    WAN:  ±1-10 ms     (unreliable across regions)")
    print()
    print("  PTP (IEEE 1588) with hardware support:")
    print("    ±0.1-1 μs         (needs special NICs & switches)")
    print()
    print("  GPS time:")
    print("    ±50-100 ns        (needs antenna, clear sky, no spoofing)")
    print()
    print("  Lesson: Physical timestamps give you PROBABILISTIC ordering,")
    print("          not GUARANTEED ordering. For guarantees, use logical time.")

    base_time = 1_000_000
    drift_per_min_us = 50
    minutes = [0, 1, 5, 30, 60, 1440]
    print(f"\n  {'Minutes':>8} │ {'Clock A (μs)':>14} │ {'Clock B (μs)':>14} │ {'Skew (μs)':>10}")
    print(f"  {'─'*8}┼{'─'*16}┼{'─'*16}┼{'─'*12}")
    for m in minutes:
        skew = drift_per_min_us * m
        a = base_time
        b = base_time + skew
        print(f"  {m:>8} │ {a:>14} │ {b:>14} │ {skew:>10}")

    print(f"\n  Even after just 1 minute, clocks disagree by {drift_per_min_us}μs.")
    print(f"  After 1 day, they disagree by {drift_per_min_us * 1440}μs = "
          f"{drift_per_min_us * 1440 / 1000:.1f}ms.")
    print(f"  Two events 'at the same time' could be ordered either way.")


def main():
    print("=" * 66)
    print("  TIME — PHYSICAL, LOGICAL, LAMPORT CLOCKS")
    print("  Phase 11 — Distributed Systems")
    print("=" * 66)

    simulate_physical_clock_drift()
    print("\n" + "=" * 66)
    simulate_three_processes()
    demonstrate_concurrent_counterexample()
    simulate_mutual_exclusion()

    print("\n" + "=" * 66)
    print("  SUMMARY")
    print("=" * 66)
    print("""
  1. Physical time is unreliable for distributed ordering (drift, skew).
  2. Happens-before (→) captures causality: process order + message order + transitivity.
  3. Lamport clocks: LC(a) < LC(b) when a → b. BUT NOT the converse.
  4. Total order: (LC, PID) breaks ties for deterministic ordering.
  5. Mutual exclusion: all processes agree on (LC, PID) ordering. No deadlock.
  6. For detecting concurrency, you need vector clocks (not Lamport clocks).
""")


if __name__ == "__main__":
    main()