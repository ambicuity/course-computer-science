"""
Isolation Levels — Read Committed → Serializable
Phase 10 — Databases & Storage Systems

A transaction scheduler that enforces different isolation levels and demonstrates
which anomalies occur at each level.
"""

from enum import Enum
from dataclasses import dataclass, field
from typing import Optional, Any


class OperationType(Enum):
    READ = "READ"
    WRITE = "WRITE"
    COMMIT = "COMMIT"
    ABORT = "ABORT"


@dataclass
class Operation:
    txn_id: int
    op_type: OperationType
    key: str = ""
    value: Any = None


@dataclass
class Transaction:
    txn_id: int
    operations: list = field(default_factory=list)
    status: str = "active"  # active, committed, aborted
    snapshot: dict = field(default_factory=dict)


class IsolationLevel(Enum):
    READ_UNCOMMITTED = "READ UNCOMMITTED"
    READ_COMMITTED = "READ COMMITTED"
    REPEATABLE_READ = "REPEATABLE READ"
    SNAPSHOT = "SNAPSHOT"
    SERIALIZABLE = "SERIALIZABLE"


class Database:
    """A simple key-value database with versioned writes."""

    def __init__(self):
        self.data = {}
        self.committed_versions = []
        self.version_counter = 0

    def read(self, key, isolation_level, txn_snapshot=None):
        if key not in self.data:
            return None
        if isolation_level == IsolationLevel.READ_UNCOMMITTED:
            return self.data[key][0]
        elif isolation_level == IsolationLevel.READ_COMMITTED:
            latest_committed = None
            latest_version = -1
            for v, tid, k, val in self.committed_versions:
                if k == key and v > latest_version:
                    latest_committed = val
                    latest_version = v
            return latest_committed
        elif isolation_level in (
            IsolationLevel.REPEATABLE_READ,
            IsolationLevel.SNAPSHOT,
            IsolationLevel.SERIALIZABLE,
        ):
            if txn_snapshot is None:
                return self.data[key][0]
            best = None
            best_version = -1
            sv = txn_snapshot.get("snapshot_version", 0)
            for v, tid, k, val in self.committed_versions:
                if k == key and v <= sv and v > best_version:
                    best = val
                    best_version = v
            return best
        return self.data[key][0]

    def next_version(self):
        """Allocate the next version number without modifying data."""
        self.version_counter += 1
        return self.version_counter

    def apply_write(self, key, value, version):
        """Store a committed write in the data dict."""
        self.data[key] = (value, version)

    def record_committed(self, version, txn_id, key, value):
        """Record a committed version in the version log."""
        self.committed_versions.append((version, txn_id, key, value))


class Scheduler:
    """A transaction scheduler that enforces the specified isolation level."""

    def __init__(self, isolation_level: IsolationLevel):
        self.isolation_level = isolation_level
        self.db = Database()
        self.transactions = {}
        self.active_writes = {}
        self.active_reads = {}
        self.txn_versions = {}
        self.conflict_graph = {}

    def begin_transaction(self, txn_id):
        txn = Transaction(txn_id=txn_id)
        self.transactions[txn_id] = txn
        self.active_writes[txn_id] = {}
        self.active_reads[txn_id] = {}
        self.conflict_graph[txn_id] = set()
        if self.isolation_level in (
            IsolationLevel.REPEATABLE_READ,
            IsolationLevel.SNAPSHOT,
            IsolationLevel.SERIALIZABLE,
        ):
            self.txn_versions[txn_id] = self.db.version_counter

    def read(self, txn_id, key):
        txn = self.transactions.get(txn_id)
        if txn is None or txn.status != "active":
            return None

        if self.isolation_level == IsolationLevel.READ_UNCOMMITTED:
            for tid, writes in self.active_writes.items():
                if key in writes:
                    return writes[key][0]
            return self.db.read(key, self.isolation_level)

        elif self.isolation_level == IsolationLevel.READ_COMMITTED:
            value = self.db.read(key, self.isolation_level)
            if value is not None:
                self.active_reads[txn_id][key] = value
            return value

        elif self.isolation_level == IsolationLevel.REPEATABLE_READ:
            if key in self.active_reads.get(txn_id, {}):
                return self.active_reads[txn_id][key]
            snapshot_info = {"snapshot_version": self.txn_versions.get(txn_id, 0)}
            value = self.db.read(
                key, self.isolation_level, snapshot_info
            )
            if value is not None:
                self.active_reads[txn_id][key] = value
            return value

        elif self.isolation_level == IsolationLevel.SNAPSHOT:
            snapshot_info = {"snapshot_version": self.txn_versions.get(txn_id, 0)}
            return self.db.read(
                key, IsolationLevel.REPEATABLE_READ, snapshot_info
            )

        elif self.isolation_level == IsolationLevel.SERIALIZABLE:
            if key in self.active_reads.get(txn_id, {}):
                return self.active_reads[txn_id][key]
            snapshot_info = {"snapshot_version": self.txn_versions.get(txn_id, 0)}
            value = self.db.read(
                key, IsolationLevel.REPEATABLE_READ, snapshot_info
            )
            if value is not None:
                self.active_reads[txn_id][key] = value
            return value

        return None

    def write(self, txn_id, key, value):
        txn = self.transactions.get(txn_id)
        if txn is None or txn.status != "active":
            return

        version = self.db.next_version()
        self.active_writes[txn_id][key] = (value, version)

        if self.isolation_level == IsolationLevel.SERIALIZABLE:
            for tid, txn_other in self.transactions.items():
                if tid != txn_id and txn_other.status == "active":
                    if key in self.active_reads.get(
                        tid, {}
                    ) or key in self.active_writes.get(tid, {}):
                        self.conflict_graph[txn_id].add(tid)
                        self.conflict_graph[tid].add(txn_id)

    def commit(self, txn_id):
        txn = self.transactions.get(txn_id)
        if txn is None or txn.status != "active":
            return False

        if self.isolation_level == IsolationLevel.SERIALIZABLE:
            if self._has_cycle():
                self.abort(txn_id)
                return False

        for key, (value, version) in self.active_writes[txn_id].items():
            self.db.apply_write(key, value, version)
            self.db.record_committed(version, txn_id, key, value)

        txn.status = "committed"
        self.active_writes[txn_id] = {}
        self.active_reads[txn_id] = {}
        return True

    def abort(self, txn_id):
        txn = self.transactions.get(txn_id)
        if txn is None:
            return
        txn.status = "aborted"
        self.active_writes[txn_id] = {}
        self.active_reads[txn_id] = {}
        if txn_id in self.conflict_graph:
            del self.conflict_graph[txn_id]
        for tid in list(self.conflict_graph):
            self.conflict_graph[tid].discard(txn_id)

    def _has_cycle(self):
        visited = set()
        path = set()

        def dfs(node):
            visited.add(node)
            path.add(node)
            for neighbor in self.conflict_graph.get(node, set()):
                if neighbor not in visited:
                    if dfs(neighbor):
                        return True
                elif neighbor in path:
                    return True
            path.remove(node)
            return False

        for tid in self.conflict_graph:
            if tid not in visited:
                if dfs(tid):
                    return True
        return False


def separator(title):
    print()
    print("=" * 70)
    print(f"  {title}")
    print("=" * 70)


def demo_dirty_read(level):
    print(f"\n--- Dirty Read @ {level.value} ---")
    sched = Scheduler(level)
    sched.begin_transaction(1)
    sched.write(1, "X", 100)
    sched.commit(1)

    sched.begin_transaction(2)
    sched.write(2, "X", 999)
    sched.begin_transaction(3)
    val = sched.read(3, "X")
    if level == IsolationLevel.READ_UNCOMMITTED:
        print(f"  T3 reads X={val} (dirty: saw T2's uncommitted write)")
    else:
        print(f"  T3 reads X={val} (no dirty read)")
    sched.abort(2)
    final_val = sched.read(3, "X") if sched.transactions[3].status == "active" else None
    if final_val is not None:
        print(f"  T3 reads X again after T2 abort: {final_val}")


def demo_nonrepeatable_read(level):
    print(f"\n--- Non-Repeatable Read @ {level.value} ---")
    sched = Scheduler(level)
    sched.begin_transaction(1)
    sched.write(1, "X", 100)
    sched.commit(1)

    sched.begin_transaction(2)
    sched.begin_transaction(3)

    v1 = sched.read(2, "X")
    sched.write(3, "X", 200)
    sched.commit(3)
    v2 = sched.read(2, "X")

    if v1 != v2:
        print(f"  T2: first read X={v1}, second read X={v2} — NON-REPEATABLE READ")
    else:
        print(f"  T2: first read X={v1}, second read X={v2} — repeatable")
    sched.commit(2)


def demo_lost_update(level):
    print(f"\n--- Lost Update @ {level.value} ---")
    sched = Scheduler(level)
    sched.begin_transaction(1)
    sched.write(1, "counter", 100)
    sched.commit(1)

    sched.begin_transaction(2)
    sched.begin_transaction(3)

    v1 = sched.read(2, "counter")
    v2 = sched.read(3, "counter")

    sched.write(2, "counter", v1 + 10)
    sched.write(3, "counter", v2 + 20)
    t2_ok = sched.commit(2)
    t3_ok = sched.commit(3)

    final = sched.db.read("counter", IsolationLevel.READ_COMMITTED)
    expected = 100 + 10 + 20
    if final == expected:
        print(f"  Final counter={final} (expected {expected}) — no lost update")
    elif not t2_ok or not t3_ok:
        print(f"  T2 committed={t2_ok}, T3 committed={t3_ok}, final={final} — SERIALIZABLE aborted one, anomaly prevented")
    else:
        print(f"  Final counter={final} (expected {expected}) — LOST UPDATE")


def demo_write_skew(level):
    print(f"\n--- Write Skew @ {level.value} ---")
    sched = Scheduler(level)
    sched.begin_transaction(1)
    sched.write(1, "a_oncall", 1)
    sched.write(1, "b_oncall", 1)
    sched.commit(1)

    sched.begin_transaction(2)
    sched.begin_transaction(3)

    a1 = sched.read(2, "a_oncall")
    b1 = sched.read(2, "b_oncall")
    a2 = sched.read(3, "a_oncall")
    b2 = sched.read(3, "b_oncall")

    if a1 and b1:
        sched.write(2, "a_oncall", 0)
    if a2 and b2:
        sched.write(3, "b_oncall", 0)

    t2_ok = sched.commit(2)
    t3_ok = sched.commit(3)

    a_final = sched.db.read("a_oncall", IsolationLevel.READ_COMMITTED)
    b_final = sched.db.read("b_oncall", IsolationLevel.READ_COMMITTED)
    if a_final == 0 and b_final == 0:
        outcome = "WRITE SKEW: nobody on call!"
    elif not t2_ok or not t3_ok:
        outcome = f"SERIALIZABLE aborted one, invariant preserved"
    else:
        outcome = "invariant preserved"
    print(f"  A={a_final}, B={b_final} — {outcome}")


def demo_phantom_read(level):
    print(f"\n--- Phantom Read @ {level.value} ---")
    sched = Scheduler(level)
    sched.begin_transaction(0)
    sched.write(0, "a", 10)
    sched.write(0, "b", 20)
    sched.write(0, "c", 30)
    sched.commit(0)

    sched.begin_transaction(2)
    sched.begin_transaction(3)

    count_before = 0
    for k in ["a", "b", "c", "d", "e"]:
        v = sched.read(2, k)
        if v is not None:
            count_before += 1

    sched.write(3, "d", 40)
    sched.commit(3)

    count_after = 0
    for k in ["a", "b", "c", "d", "e"]:
        v = sched.read(2, k)
        if v is not None:
            count_after += 1

    if count_before != count_after:
        print(f"  Before: {count_before} keys, After: {count_after} keys — phantom(s) appeared")
    else:
        print(f"  Before: {count_before} keys, After: {count_after} keys — no phantoms")
    sched.commit(2)


def demo_serializable_abort():
    separator("Serializable: SSI Conflict Detection")
    sched = Scheduler(IsolationLevel.SERIALIZABLE)

    sched.begin_transaction(1)
    sched.begin_transaction(2)

    # Both read X, both write X — creates a cycle in the conflict graph
    sched.read(1, "X")
    sched.read(2, "X")
    sched.write(1, "X", 50)
    sched.write(2, "X", 100)

    sched.commit(1)
    sched.commit(2)

    t1_status = sched.transactions[1].status
    t2_status = sched.transactions[2].status
    print(f"  T1 status: {t1_status}")
    print(f"  T2 status: {t2_status}")
    final_x = sched.db.read("X", IsolationLevel.READ_COMMITTED)
    print(f"  Final X value: {final_x}")
    if "aborted" in (t1_status, t2_status):
        print("  SSI prevented non-serializable execution: one txn aborted")
    else:
        print("  Both committed (no conflict detected)")


def demo_all_levels_summary():
    separator("Anomaly Detection Across All Isolation Levels")
    header = f"{'Anomaly':<25} {'RU':<20} {'RC':<20} {'RR':<20} {'SI':<20} {'SER':<20}"
    print(header)
    print("-" * len(header))

    scenarios = [
        ("Dirty Read", True, False, False, False, False),
        ("Non-repeatable Read", True, True, False, False, False),
        ("Lost Update", True, True, False, False, False),
        ("Write Skew", True, True, True, True, False),
        ("Phantom Read", True, True, False, False, False),
    ]

    for name, ru, rc, rr, si, ser in scenarios:
        ru_str = "YES" if ru else "no"
        rc_str = "YES" if rc else "no"
        rr_str = "YES" if rr else "no"
        si_str = "YES" if si else "no"
        ser_str = "YES" if ser else "no"
        print(f"{name:<25} {ru_str:<20} {rc_str:<20} {rr_str:<20} {si_str:<20} {ser_str:<20}")


def main():
    separator("Isolation Levels — Read Committed → Serializable")

    for level in IsolationLevel:
        separator(f"Demonstrating {level.value}")
        demo_dirty_read(level)
        demo_nonrepeatable_read(level)
        demo_lost_update(level)
        demo_write_skew(level)
        demo_phantom_read(level)

    demo_serializable_abort()
    demo_all_levels_summary()

    print()
    print("Legend: RU=READ UNCOMMITTED, RC=READ COMMITTED,")
    print("RR=REPEATABLE READ, SI=SNAPSHOT, SER=SERIALIZABLE")


if __name__ == "__main__":
    main()
