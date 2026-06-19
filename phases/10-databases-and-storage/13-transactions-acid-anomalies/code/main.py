from enum import Enum
from dataclasses import dataclass, field
from typing import Optional


class IsolationLevel(Enum):
    READ_UNCOMMITTED = 1
    READ_COMMITTED = 2
    REPEATABLE_READ = 3
    SERIALIZABLE = 4


@dataclass
class Txn:
    txn_id: int
    isolation: IsolationLevel
    snapshot: dict = field(default_factory=dict)
    writes: dict = field(default_factory=dict)
    active: bool = True


class TransactionManager:
    def __init__(self, isolation: IsolationLevel, data: dict = None):
        self.isolation = isolation
        self.data = data or {}
        self.txns: dict[int, Txn] = {}
        self.next_id = 0

    def begin(self) -> int:
        txn_id = self.next_id
        self.next_id += 1
        txn = Txn(txn_id=txn_id, isolation=self.isolation)
        if self.isolation in (IsolationLevel.REPEATABLE_READ, IsolationLevel.SERIALIZABLE):
            txn.snapshot = dict(self.data)
        txn.writes = {}
        self.txns[txn_id] = txn
        return txn_id

    def _latest_uncommitted(self, key: str) -> Optional[int]:
        latest_txn = -1
        latest_val = None
        for tid, txn in self.txns.items():
            if txn.active and key in txn.writes and tid > latest_txn:
                latest_txn = tid
                latest_val = txn.writes[key]
        return latest_val

    def read(self, txn_id: int, key: str) -> Optional[int]:
        txn = self.txns[txn_id]
        if not txn.active:
            raise ValueError(f"Transaction {txn_id} is not active")
        if key in txn.writes:
            return txn.writes[key]
        if self.isolation == IsolationLevel.READ_UNCOMMITTED:
            uncommitted = self._latest_uncommitted(key)
            if uncommitted is not None:
                return uncommitted
            return self.data.get(key)
        if self.isolation in (IsolationLevel.REPEATABLE_READ, IsolationLevel.SERIALIZABLE):
            return txn.snapshot.get(key)
        return self.data.get(key)

    def write(self, txn_id: int, key: str, value: int):
        txn = self.txns[txn_id]
        if not txn.active:
            raise ValueError(f"Transaction {txn_id} is not active")
        txn.writes[key] = value

    def commit(self, txn_id: int) -> bool:
        txn = self.txns[txn_id]
        if not txn.active:
            return False
        txn.active = False
        for key, value in txn.writes.items():
            self.data[key] = value
        return True

    def rollback(self, txn_id: int):
        txn = self.txns.get(txn_id)
        if txn:
            txn.active = False
            txn.writes.clear()

    def schedule_run(self, schedule: list[dict]):
        for step in schedule:
            txn_id = step['txn']
            op = step['op']
            key = step.get('key')
            value = step.get('value')
            if op == 'r':
                val = self.read(txn_id, key)
                step['result'] = val
            elif op == 'w':
                self.write(txn_id, key, value)
            elif op == 'commit':
                self.commit(txn_id)
            elif op == 'rollback':
                self.rollback(txn_id)


def demo_dirty_read():
    print("=== Dirty Read ===")
    print("Isolation: READ UNCOMMITTED")
    print("T1 writes X=200 (uncommitted). T2 reads X -> sees 200 (dirty). T1 rolls back.")
    print("Expected: T2 sees 200 during T1's transaction, then 100 after rollback.\n")

    tm = TransactionManager(IsolationLevel.READ_UNCOMMITTED, {"x": 100})
    t1 = tm.begin()
    t2 = tm.begin()

    schedule = [
        {'txn': t1, 'op': 'w', 'key': 'x', 'value': 200},
        {'txn': t2, 'op': 'r', 'key': 'x'},
        {'txn': t1, 'op': 'rollback'},
    ]
    tm.schedule_run(schedule)
    print(f"  T2 dirty read result: {schedule[1]['result']}")
    print(f"  Final data after rollback: {tm.data}")


def demo_dirty_read_prevented():
    print("=== Dirty Read Prevented (READ COMMITTED) ===")
    print("Same schedule but at READ COMMITTED.")
    print("Expected: T2 never sees the uncommitted write.\n")

    tm = TransactionManager(IsolationLevel.READ_COMMITTED, {"x": 100})
    t1 = tm.begin()
    t2 = tm.begin()

    schedule = [
        {'txn': t1, 'op': 'w', 'key': 'x', 'value': 200},
        {'txn': t2, 'op': 'r', 'key': 'x'},
        {'txn': t1, 'op': 'rollback'},
    ]
    tm.schedule_run(schedule)
    print(f"  T2 read result (should see 100, not 200): {schedule[1]['result']}")
    print(f"  Final data: {tm.data}")


def demo_nonrepeatable_read():
    print("=== Non-Repeatable Read ===")
    print("Isolation: READ COMMITTED")
    print("T1 reads X=100. T2 writes X=200 and commits. T1 reads X again -> sees 200.")
    print("Expected: T1 sees different values for the same read.\n")

    tm = TransactionManager(IsolationLevel.READ_COMMITTED, {"x": 100})
    t1 = tm.begin()
    t2 = tm.begin()

    schedule = [
        {'txn': t1, 'op': 'r', 'key': 'x'},
        {'txn': t2, 'op': 'w', 'key': 'x', 'value': 200},
        {'txn': t2, 'op': 'commit'},
        {'txn': t1, 'op': 'r', 'key': 'x'},
        {'txn': t1, 'op': 'commit'},
    ]
    tm.schedule_run(schedule)
    print(f"  T1 first read:  {schedule[0]['result']}")
    print(f"  T1 second read: {schedule[3]['result']}")


def demo_nonrepeatable_read_prevented():
    print("=== Non-Repeatable Read Prevented (REPEATABLE READ) ===")
    print("Same schedule at REPEATABLE READ.")
    print("Expected: T1 sees the same value both times (snapshot at begin).\n")

    tm = TransactionManager(IsolationLevel.REPEATABLE_READ, {"x": 100})
    t1 = tm.begin()
    t2 = tm.begin()

    schedule = [
        {'txn': t1, 'op': 'r', 'key': 'x'},
        {'txn': t2, 'op': 'w', 'key': 'x', 'value': 200},
        {'txn': t2, 'op': 'commit'},
        {'txn': t1, 'op': 'r', 'key': 'x'},
        {'txn': t1, 'op': 'commit'},
    ]
    tm.schedule_run(schedule)
    print(f"  T1 first read:  {schedule[0]['result']}")
    print(f"  T1 second read: {schedule[3]['result']}")


def demo_phantom_read():
    print("=== Phantom Read ===")
    print("Isolation: READ COMMITTED")
    print("T1 scans keys a..c, sees 3 rows.")
    print("T2 inserts 'd' and commits.")
    print("T1 re-scans -> sees 4 rows (phantom 'd' appeared).\n")

    tm = TransactionManager(IsolationLevel.READ_COMMITTED, {"a": 1, "b": 2, "c": 3})
    t1 = tm.begin()
    t2 = tm.begin()

    def scan_global(lo: str, hi: str):
        return {k: v for k, v in tm.data.items() if lo <= k <= hi}

    result1 = scan_global("a", "d")
    print(f"  T1 first scan (a..d): {result1}")

    tm.write(t2, "d", 4)
    tm.commit(t2)

    result2 = scan_global("a", "d")
    print(f"  T1 second scan (a..d) after T2 insert: {result2}")
    print(f"  Phantom row 'd' appeared at READ COMMITTED!\n")

    print("--- Phantom Read at REPEATABLE READ (MVCC prevents it) ---")
    print("Same scenario, but T1 has snapshot from begin.")
    print("(In PostgreSQL, REPEATABLE READ uses MVCC snapshot for all reads,")
    print(" so phantoms are also prevented — stronger than SQL standard minimum.)\n")

    tm2 = TransactionManager(IsolationLevel.REPEATABLE_READ, {"a": 1, "b": 2, "c": 3})
    t1b = tm2.begin()
    t2b = tm2.begin()

    snap1 = dict(tm2.txns[t1b].snapshot)
    print(f"  T1 snapshot at begin: {snap1}")

    tm2.write(t2b, "d", 4)
    tm2.commit(t2b)
    print(f"  After T2 commit, global data: {tm2.data}")

    snap1b = tm2.txns[t1b].snapshot
    print(f"  T1 still sees snapshot: {snap1b}")
    print(f"  Phantom prevented by MVCC snapshot (though SQL standard says RR allows phantoms).")


def demo_lost_update():
    print("=== Lost Update ===")
    print("Isolation: READ COMMITTED")
    print("T1 reads counter=100, increments locally (+10), writes 110.")
    print("T2 reads counter=100 (before T1 commit), increments locally (+20), writes 120.")
    print("T2's write overwrites T1's. Final: 120 instead of 130.\n")

    tm = TransactionManager(IsolationLevel.READ_COMMITTED, {"counter": 100})
    t1 = tm.begin()
    t2 = tm.begin()

    v1 = tm.read(t1, "counter")
    v2 = tm.read(t2, "counter")

    schedule = [
        {'txn': t1, 'op': 'w', 'key': 'counter', 'value': v1 + 10},
        {'txn': t1, 'op': 'commit'},
        {'txn': t2, 'op': 'w', 'key': 'counter', 'value': v2 + 20},
        {'txn': t2, 'op': 'commit'},
    ]
    tm.schedule_run(schedule)
    print(f"  Expected counter: {100 + 10 + 20}")
    print(f"  Actual   counter: {tm.data['counter']}")


def demo_write_skew():
    print("=== Write Skew ===")
    print("Isolation: REPEATABLE READ")
    print("Constraint: at least one doctor (A or B) must be on call.")
    print("T1 sees both on call, takes A off call.")
    print("T2 sees both on call, takes B off call.")
    print("Both commits -> nobody on call.\n")

    tm = TransactionManager(IsolationLevel.REPEATABLE_READ, {"a_oncall": 1, "b_oncall": 1})
    t1 = tm.begin()
    t2 = tm.begin()

    schedule = [
        {'txn': t1, 'op': 'w', 'key': 'a_oncall', 'value': 0},
        {'txn': t1, 'op': 'commit'},
        {'txn': t2, 'op': 'w', 'key': 'b_oncall', 'value': 0},
        {'txn': t2, 'op': 'commit'},
    ]
    tm.schedule_run(schedule)
    print(f"  A on call: {tm.data['a_oncall']}, B on call: {tm.data['b_oncall']}")


def demo_read_skew():
    print("=== Read Skew ===")
    print("Isolation: READ COMMITTED")
    print("T1 reads X then Y. T2 writes both X and Y between T1's reads.")
    print("T1 sees a state that never existed (old X, new Y).\n")

    tm = TransactionManager(IsolationLevel.READ_COMMITTED, {"x": 50, "y": 100})
    t1 = tm.begin()
    t2 = tm.begin()

    schedule = [
        {'txn': t1, 'op': 'r', 'key': 'x'},
        {'txn': t2, 'op': 'w', 'key': 'x', 'value': 150},
        {'txn': t2, 'op': 'w', 'key': 'y', 'value': 200},
        {'txn': t2, 'op': 'commit'},
        {'txn': t1, 'op': 'r', 'key': 'y'},
        {'txn': t1, 'op': 'commit'},
    ]
    tm.schedule_run(schedule)
    print(f"  T1 read X: {schedule[0]['result']}")
    print(f"  T1 read Y: {schedule[4]['result']}")
    print(f"  Pair ({schedule[0]['result']}, {schedule[4]['result']}) never coexisted!")


def summary():
    print("\n" + "=" * 60)
    print("SUMMARY: Anomaly × Isolation Level")
    print("=" * 60)
    print(f"{'Anomaly':<25} {'RU':<5} {'RC':<5} {'RR':<5} {'SER':<5}")
    print("-" * 45)
    for anomaly, ru, rc, rr, ser in [
        ("Dirty Read", "YES", "no", "no", "no"),
        ("Non-repeatable Read", "YES", "YES", "no", "no"),
        ("Phantom Read", "YES", "YES", "YES*", "no"),
        ("Lost Update", "YES", "YES", "no*", "no"),
        ("Read Skew", "YES", "YES", "no", "no"),
        ("Write Skew", "YES", "YES", "YES", "no"),
    ]:
        print(f"{anomaly:<25} {ru:<5} {rc:<5} {rr:<5} {ser:<5}")
    print("\nRU=READ UNCOMMITTED, RC=READ COMMITTED, RR=REPEATABLE READ, SER=SERIALIZABLE")
    print("* In practice depends on implementation (MVCC vs locking).")


def main():
    demo_dirty_read()
    print()
    demo_dirty_read_prevented()
    print()
    demo_nonrepeatable_read()
    print()
    demo_nonrepeatable_read_prevented()
    print()
    demo_phantom_read()
    print()
    demo_lost_update()
    print()
    demo_write_skew()
    print()
    demo_read_skew()
    print()
    summary()


if __name__ == "__main__":
    main()
