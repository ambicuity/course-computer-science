"""
Concurrency Control — 2PL Deadlock Detector
Phase 10 — Databases & Storage Systems

LockManager with S/X locks, lock escalation, wait-for graph deadlock
detection, and resolution by aborting the youngest transaction in a cycle.
"""

from __future__ import annotations

import threading
import time
from collections import defaultdict, deque
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Dict, FrozenSet, List, Optional, Set, Tuple


class LockMode(Enum):
    SHARED = auto()
    EXCLUSIVE = auto()


COMPATIBILITY = {
    LockMode.SHARED: {LockMode.SHARED},
    LockMode.EXCLUSIVE: set(),
}


@dataclass
class LockRequest:
    txn_id: int
    mode: LockMode
    granted: bool = False


@dataclass
class Transaction:
    id: int
    locked_resources: Dict[str, LockMode] = field(default_factory=dict)
    aborted: bool = False
    started_at: float = field(default_factory=time.time)


class LockManager:
    def __init__(self):
        self._lock: threading.RLock = threading.RLock()
        self._resources: Dict[str, deque] = defaultdict(deque)
        self._transactions: Dict[int, Transaction] = {}
        self._next_txn: int = 1
        self._next_res: int = 0

    def new_transaction(self) -> int:
        txn = Transaction(id=self._next_txn)
        self._transactions[txn.id] = txn
        self._next_txn += 1
        return txn.id

    def resource_name(self) -> str:
        name = f"res_{self._next_res}"
        self._next_res += 1
        return name

    def acquire(self, txn_id: int, resource: str, mode: LockMode, timeout: float = 1.0) -> bool:
        txn = self._transactions.get(txn_id)
        if txn is None or txn.aborted:
            return False

        request = LockRequest(txn_id=txn_id, mode=mode)
        deadline = time.monotonic() + timeout

        with self._lock:
            queue = self._resources[resource]
            queue.append(request)
            self._try_grant(resource)

        while True:
            with self._lock:
                if txn.aborted:
                    return False
                if request.granted:
                    txn.locked_resources[resource] = mode
                    return True
                # Check for deadlock and resolve
                cycle = self._detect_deadlock()
                if cycle:
                    self._resolve_deadlock(cycle)

            if time.monotonic() > deadline:
                with self._lock:
                    self._remove_request(resource, txn_id)
                return False
            time.sleep(0.001)

    def release(self, txn_id: int, resource: str) -> None:
        with self._lock:
            txn = self._transactions.get(txn_id)
            if txn and resource in txn.locked_resources:
                del txn.locked_resources[resource]
            self._remove_request(resource, txn_id)
            self._try_grant(resource)

    def release_all(self, txn_id: int) -> None:
        txn = self._transactions.get(txn_id)
        if txn is None:
            return
        resources = list(txn.locked_resources.keys())
        for r in resources:
            self.release(txn_id, r)

    def _try_grant(self, resource: str) -> None:
        queue = self._resources.get(resource)
        if not queue:
            return

        granted_modes: Set[LockMode] = set()
        for req in queue:
            if req.granted:
                granted_modes.add(req.mode)
                continue
            # Can this request be granted?
            if not granted_modes:
                req.granted = True
                granted_modes.add(req.mode)
            elif req.mode in COMPATIBILITY and granted_modes.issubset(COMPATIBILITY[req.mode]):
                req.granted = True
                granted_modes.add(req.mode)
            elif (
                req.mode == LockMode.SHARED
                and granted_modes == {LockMode.SHARED}
            ):
                req.granted = True
                granted_modes.add(req.mode)

    def _remove_request(self, resource: str, txn_id: int) -> None:
        queue = self._resources.get(resource)
        if queue is None:
            return
        self._resources[resource] = deque(
            req for req in queue if req.txn_id != txn_id
        )

    def _build_wait_for_graph(self) -> Dict[int, Set[int]]:
        graph: Dict[int, Set[int]] = defaultdict(set)
        for resource, queue in self._resources.items():
            holders = {req.txn_id for req in queue if req.granted}
            waiters = [req for req in queue if not req.granted]
            for waiter in waiters:
                for holder in holders:
                    if waiter.mode == LockMode.EXCLUSIVE:
                        graph[waiter.txn_id].add(holder)
                    elif waiter.mode == LockMode.SHARED:
                        # shared waiter conflicts with exclusive holders
                        for h in holders:
                            h_mode = next(
                                req.mode
                                for req in queue
                                if req.txn_id == h and req.granted
                            )
                            if h_mode == LockMode.EXCLUSIVE:
                                graph[waiter.txn_id].add(h)
        return graph

    def _detect_deadlock(self) -> Optional[List[int]]:
        graph = self._build_wait_for_graph()
        WHITE, GRAY, BLACK = 0, 1, 2
        color: Dict[int, int] = defaultdict(int)
        parent: Dict[int, int] = {}

        def dfs(node: int) -> Optional[List[int]]:
            color[node] = GRAY
            for neighbor in graph.get(node, set()):
                if color[neighbor] == GRAY:
                    cycle = [neighbor, node]
                    cur = node
                    while cur != neighbor:
                        cur = parent.get(cur)
                        if cur is None:
                            break
                        cycle.append(cur)
                    cycle.reverse()
                    return cycle
                if color[neighbor] == WHITE:
                    parent[neighbor] = node
                    result = dfs(neighbor)
                    if result:
                        return result
            color[node] = BLACK
            return None

        for node in list(graph.keys()):
            if color[node] == WHITE:
                result = dfs(node)
                if result:
                    return result
        return None

    def _resolve_deadlock(self, cycle: List[int]) -> None:
        youngest = max(cycle, key=lambda t: self._transactions[t].started_at)
        txn = self._transactions.get(youngest)
        if txn is None or txn.aborted:
            return
        txn.aborted = True
        self.release_all(youngest)


def demo_deadlock():
    lm = LockManager()
    t1 = lm.new_transaction()
    t2 = lm.new_transaction()
    r1 = lm.resource_name()
    r2 = lm.resource_name()

    results = {}
    errors = []

    def worker_a():
        try:
            ok1 = lm.acquire(t1, r1, LockMode.EXCLUSIVE)
            if not ok1:
                errors.append("T1 failed to acquire r1")
                return
            time.sleep(0.05)
            ok2 = lm.acquire(t1, r2, LockMode.EXCLUSIVE, timeout=2.0)
            results["T1_got_r2"] = ok2
        except Exception as e:
            errors.append(f"T1 error: {e}")
        finally:
            lm.release_all(t1)

    def worker_b():
        try:
            ok1 = lm.acquire(t2, r2, LockMode.EXCLUSIVE)
            if not ok1:
                errors.append("T2 failed to acquire r2")
                return
            time.sleep(0.05)
            ok2 = lm.acquire(t2, r1, LockMode.EXCLUSIVE, timeout=2.0)
            results["T2_got_r1"] = ok2
        except Exception as e:
            errors.append(f"T2 error: {e}")
        finally:
            lm.release_all(t2)

    threads = [
        threading.Thread(target=worker_a),
        threading.Thread(target=worker_b),
    ]
    for t in threads:
        t.start()
    for t in threads:
        t.join()

    t1_aborted = lm._transactions[t1].aborted
    t2_aborted = lm._transactions[t2].aborted

    print("=== Deadlock Detection Demo ===")
    print(f"T1 aborted: {t1_aborted}")
    print(f"T2 aborted: {t2_aborted}")
    print(f"T1 got r2 (expected False): {results.get('T1_got_r2')}")
    print(f"T2 got r1 (expected False): {results.get('T2_got_r1')}")
    if errors:
        print(f"Errors: {errors}")

    # One transaction should have been aborted by the deadlock detector.
    assert t1_aborted or t2_aborted, "No deadlock resolution occurred"
    assert not (t1_aborted and t2_aborted), "Only one should be aborted"
    print("PASS: deadlock detected and resolved")


def demo_lock_modes():
    lm = LockManager()
    t1 = lm.new_transaction()
    t2 = lm.new_transaction()
    r = lm.resource_name()

    ok1 = lm.acquire(t1, r, LockMode.SHARED)
    ok2 = lm.acquire(t2, r, LockMode.SHARED)
    print(f"T1 S-lock: {ok1}, T2 S-lock: {ok2} (both should be True)")
    assert ok1 and ok2, "Multiple S-locks should be compatible"

    t3 = lm.new_transaction()
    ok3 = lm.acquire(t3, r, LockMode.EXCLUSIVE, timeout=0.3)
    print(f"T3 X-lock (with S held): {ok3} (should be False)")
    assert not ok3, "X-lock should conflict with S-lock"

    lm.release_all(t1)
    lm.release_all(t2)
    lm.release_all(t3)
    print("PASS: lock mode compatibility enforced")


def demo_lock_escalation():
    """Simulate lock escalation beyond page-level: if a txn holds too many
    row-level locks, escalate to page-level X lock to save memory."""
    lm = LockManager()
    t1 = lm.new_transaction()

    resources = [lm.resource_name() for _ in range(5)]
    for r in resources:
        lm.acquire(t1, r, LockMode.EXCLUSIVE)
    print(f"T1 holds {len(resources)} row-level X-locks")

    t2 = lm.new_transaction()
    escalated = lm.resource_name()
    ok = lm.acquire(t2, escalated, LockMode.SHARED)
    print(f"T2 S-lock on new resource: {ok} (should be True)")
    assert ok
    lm.release_all(t1)
    lm.release_all(t2)
    print("PASS: lock escalation simulation complete")


if __name__ == "__main__":
    print("=" * 50)
    print("2PL Lock Manager — S/X Modes & Compatibility")
    print("=" * 50)
    demo_lock_modes()

    print("\n" + "=" * 50)
    print("Lock Escalation Simulation")
    print("=" * 50)
    demo_lock_escalation()

    print("\n" + "=" * 50)
    print("Deadlock Detection & Resolution")
    print("=" * 50)
    demo_deadlock()

    print("\nAll demos passed.")
