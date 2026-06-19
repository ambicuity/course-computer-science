"""
Lamport Clock — reusable reference implementation.

Provides Process (with Lamport clock), SimulatedNetwork, and LamportMutex.
Import and reuse in later phases for distributed protocol simulations.
"""

from dataclasses import dataclass
from typing import Optional
import random


class Process:
    def __init__(self, pid: int, name: str = ""):
        self.pid = pid
        self.name = name or f"P{pid}"
        self.lc = 0
        self.log: list[tuple[str, int, int, str]] = []

    def _record(self, event_type: str, label: str) -> tuple[int, int]:
        self.log.append((event_type, self.lc, self.pid, label))
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
        msgs = [m for _, m in self.in_flight]
        self.in_flight.clear()
        return msgs


class LamportMutex:
    def __init__(self, processes: list[Process]):
        self.processes = {p.pid: p for p in processes}
        self.n = len(processes)
        self.queues: dict[int, list[tuple[int, int]]] = {p.pid: [] for p in processes}
        self.acks_received: dict[int, set[int]] = {}

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
        return self.queues[pid][0][1] == pid

    def release(self, pid: int, resource: str = "R"):
        p = self.processes[pid]
        p.internal_event(f"release-{resource}")
        self.queues[pid] = [e for e in self.queues[pid] if e[1] != pid]
        for other_pid in self.processes:
            if other_pid != pid:
                self.queues[other_pid] = [e for e in self.queues[other_pid] if e[1] != pid]


def verify_happens_before(processes: list[Process], causal_pairs: list[tuple[int, int, int, int]]) -> bool:
    result = True
    for src_pid, src_lc, dst_pid, dst_lc in causal_pairs:
        if not (src_lc < dst_lc):
            result = False
    return result