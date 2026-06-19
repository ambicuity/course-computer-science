from dataclasses import dataclass
from enum import Enum


class VCRelation(Enum):
    BEFORE = "before"
    AFTER = "after"
    CONCURRENT = "concurrent"
    IDENTICAL = "identical"


class VectorClock:
    def __init__(self, processes: list[str]):
        self.processes = list(processes)
        self._index = {name: i for i, name in enumerate(self.processes)}
        self.counters = [0] * len(processes)

    def increment(self, process: str) -> "VectorClock":
        self.counters[self._index[process]] += 1
        return self

    def merge(self, other: "VectorClock") -> "VectorClock":
        for i in range(len(self.counters)):
            self.counters[i] = max(self.counters[i], other.counters[i])
        return self

    def compare(self, other: "VectorClock") -> VCRelation:
        le = all(a <= b for a, b in zip(self.counters, other.counters))
        ge = all(a >= b for a, b in zip(self.counters, other.counters))
        eq = all(a == b for a, b in zip(self.counters, other.counters))
        if eq:
            return VCRelation.IDENTICAL
        if le and not ge:
            return VCRelation.BEFORE
        if ge and not le:
            return VCRelation.AFTER
        return VCRelation.CONCURRENT

    def __lt__(self, other: "VectorClock") -> bool:
        return self.compare(other) == VCRelation.BEFORE

    def __gt__(self, other: "VectorClock") -> bool:
        return self.compare(other) == VCRelation.AFTER

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, VectorClock):
            return NotImplemented
        return self.compare(other) == VCRelation.IDENTICAL

    def is_concurrent(self, other: "VectorClock") -> bool:
        return self.compare(other) == VCRelation.CONCURRENT

    def copy(self) -> "VectorClock":
        vc = VectorClock(self.processes)
        vc.counters = list(self.counters)
        return vc

    def __repr__(self) -> str:
        return str(dict(zip(self.processes, self.counters)))


@dataclass
class Message:
    sender: str
    content: str
    vc: VectorClock
    sender_seq: int = 0


class Process:
    def __init__(self, name: str, all_processes: list[str]):
        self.name = name
        self.vc = VectorClock(all_processes)
        self.delivered: list[Message] = []

    def local_event(self) -> None:
        self.vc.increment(self.name)

    def send(self, content: str) -> Message:
        self.vc.increment(self.name)
        return Message(sender=self.name, content=content, vc=self.vc.copy())

    def receive(self, msg: Message) -> None:
        self.vc.merge(msg.vc)
        self.vc.increment(self.name)
        self.delivered.append(msg)


class CausalBroadcast:
    def __init__(self, process_names: list[str]):
        self.process_names = list(process_names)
        self.idx = {name: i for i, name in enumerate(process_names)}
        self.n = len(process_names)
        self.broadcast_num: dict[str, int] = {name: 0 for name in process_names}
        self.buffers: dict[str, list[Message]] = {name: [] for name in process_names}

    def broadcast(self, sender: str, content: str, processes: dict[str, Process]) -> tuple[Message, dict[str, str]]:
        sender_proc = processes[sender]
        sender_proc.vc.increment(sender)
        self.broadcast_num[sender] += 1
        msg = Message(
            sender=sender,
            content=content,
            vc=sender_proc.vc.copy(),
            sender_seq=self.broadcast_num[sender],
        )
        sender_proc.delivered.append(msg)

        results = {sender: "SELF-DELIVER"}
        for name in self.process_names:
            if name != sender:
                results[name] = self._try_deliver(name, msg, processes)
        return msg, results

    def _check_deps(self, dest: str, msg: Message, processes: dict[str, Process]) -> bool:
        dest_vc = processes[dest].vc
        for i, proc in enumerate(self.process_names):
            if proc == msg.sender:
                if msg.sender_seq > dest_vc.counters[i] + 1:
                    return False
            else:
                if msg.vc.counters[i] > dest_vc.counters[i]:
                    return False
        return True

    def _do_deliver(self, dest: str, msg: Message, processes: dict[str, Process]) -> None:
        proc = processes[dest]
        proc.vc.merge(msg.vc)
        proc.vc.increment(proc.name)
        proc.delivered.append(msg)

    def _try_deliver(self, dest: str, msg: Message, processes: dict[str, Process]) -> str:
        if self._check_deps(dest, msg, processes):
            self._do_deliver(dest, msg, processes)
            return f"DELIVERED '{msg.content}'"
        else:
            self.buffers[dest].append(msg)
            return f"BUFFERED '{msg.content}' (waiting for causal predecessors)"

    def inject_message(self, dest: str, msg: Message, processes: dict[str, Process]) -> str:
        return self._try_deliver(dest, msg, processes)

    def flush_buffer(self, dest: str, processes: dict[str, Process]) -> list[str]:
        delivered_msgs = []
        progress = True
        while progress:
            progress = False
            remaining = []
            for msg in self.buffers[dest]:
                if self._check_deps(dest, msg, processes):
                    self._do_deliver(dest, msg, processes)
                    delivered_msgs.append(
                        f"DELIVERED '{msg.content}' from {msg.sender}"
                    )
                    progress = True
                else:
                    remaining.append(msg)
            self.buffers[dest] = remaining
        return delivered_msgs


@dataclass
class Version:
    value: str
    vc: VectorClock


class ConflictDetector:
    @staticmethod
    def classify(v1: Version, v2: Version) -> VCRelation:
        return v1.vc.compare(v2.vc)

    @staticmethod
    def detect(v1: Version, v2: Version) -> tuple[bool, str]:
        relation = v1.vc.compare(v2.vc)
        if relation == VCRelation.CONCURRENT:
            return True, (
                f"CONFLICT: '{v1.value}' and '{v2.value}' are concurrent "
                f"(neither caused the other)"
            )
        if relation == VCRelation.BEFORE:
            return False, f"ORDERED: '{v1.value}' → '{v2.value}' (v1 caused v2)"
        if relation == VCRelation.AFTER:
            return False, f"ORDERED: '{v2.value}' → '{v1.value}' (v2 caused v1)"
        return False, f"IDENTICAL: same causal state '{v1.value}'"

    @staticmethod
    def resolve_lww(
        v1: Version, v2: Version, ts1: float, ts2: float
    ) -> Version:
        return v1 if ts1 >= ts2 else v2


def sep(title: str) -> None:
    print()
    print("=" * 64)
    print(title)
    print("=" * 64)


def demo_vc_evolution() -> None:
    sep("SCENARIO 1: Vector Clock Evolution")

    procs = ["P0", "P1", "P2"]
    p0 = Process("P0", procs)
    p1 = Process("P1", procs)
    p2 = Process("P2", procs)

    print(f"Initial: P0={p0.vc}  P1={p1.vc}  P2={p2.vc}")

    p0.local_event()
    print(f"P0 local event → P0={p0.vc}")

    msg1 = p0.send("hello from P0")
    print(f"P0 sends '{msg1.content}' → P0={p0.vc}, msg vc={msg1.vc}")

    p1.receive(msg1)
    print(f"P1 receives '{msg1.content}' → P1={p1.vc}")

    p1.local_event()
    print(f"P1 local event → P1={p1.vc}")

    msg2 = p1.send("reply from P1")
    print(f"P1 sends '{msg2.content}' → P1={p1.vc}, msg vc={msg2.vc}")

    p2.local_event()
    print(f"P2 local event (independent) → P2={p2.vc}")

    print()
    print("Comparisons:")
    print(f"  P0 < P1? {p0.vc < p1.vc}  (P0's events causally precede P1's)")
    print(f"  P0 || P2? {p0.vc.is_concurrent(p2.vc)}  (P0 and P2 are independent)")
    print(f"  P1 > P0? {p1.vc > p0.vc}  (P1's events came after P0's)")


def demo_conflict_detection() -> None:
    sep("SCENARIO 2: Concurrent Updates and Conflict Detection")

    procs = ["A", "B", "C"]

    vc_a = VectorClock(procs)
    vc_a.increment("A")
    vc_a.increment("A")
    version_a = Version("balance=200", vc_a)

    vc_c = VectorClock(procs)
    vc_c.increment("C")
    version_c = Version("balance=300", vc_c)

    print("Three replicas store key 'balance':")
    print(f"  Version A: '{version_a.value}' vc={version_a.vc}")
    print(f"  Version C: '{version_c.value}' vc={version_c.vc}")

    detector = ConflictDetector()
    is_conflict, explanation = detector.detect(version_a, version_c)
    print(f"\n  {explanation}")
    print(f"  Is conflict? {is_conflict}")

    vc_a2 = VectorClock(procs)
    vc_a2.increment("A")
    vc_a2.increment("A")
    version_a2 = Version("balance=250", vc_a2)

    is_conflict2, explanation2 = detector.detect(version_a, version_a2)
    print(f"\n  Same replica, sequential writes:")
    print(f"  {explanation2}")
    print(f"  Is conflict? {is_conflict2}")

    print("\nResolution strategies:")
    resolved = ConflictDetector.resolve_lww(version_a, version_c, 100.0, 200.0)
    print(f"  LWW (ts_A=100, ts_C=200): keep '{resolved.value}'")
    print(f"  Application merge: balance∈{{200, 300}} (preserve both as set)")
    print(f"  CRDT: merge deltas automatically (preview of lesson 12)")


def demo_causal_broadcast() -> None:
    sep("SCENARIO 3: Causal Broadcast — Enforcing Delivery Order")

    procs = ["P0", "P1", "P2"]

    print("Three processes connected by CBCAST.")
    print("Causal broadcast ensures: if m1 → m2 (causally),")
    print("then every process delivers m1 before m2.\n")

    p0_full = Process("P0", procs)
    p1_full = Process("P1", procs)
    p2_full = Process("P2", procs)
    processes_full = {"P0": p0_full, "P1": p1_full, "P2": p2_full}
    cb_full = CausalBroadcast(procs)

    print("--- Normal delivery (in order) ---\n")
    print("P0 broadcasts 'm1: init'")
    msg_m1, r = cb_full.broadcast("P0", "m1: init", processes_full)
    for name in procs:
        print(f"  {name}: {r[name]}")

    print("\nP0 broadcasts 'm2: data'")
    msg_m2, r = cb_full.broadcast("P0", "m2: data", processes_full)
    for name in procs:
        print(f"  {name}: {r[name]}")

    print("\nP1 broadcasts 'm3: ack' (after receiving m1, m2)")
    msg_m3, r = cb_full.broadcast("P1", "m3: ack", processes_full)
    for name in procs:
        print(f"  {name}: {r[name]}")

    print(f"\nAll delivered at P2 (in order):")
    for m in p2_full.delivered:
        print(f"  '{m.content}' from {m.sender}, vc={m.vc}")

    msg_m1 = p0_full.delivered[0]
    msg_m2 = p0_full.delivered[1]

    print("\n" + "=" * 64)
    print("--- Out-of-order delivery (CBCAST buffers) ---")
    print("=" * 64)
    print()
    print("Same messages, but a new P2 receives m3 BEFORE m1 and m2.")
    print("CBCAST must buffer m3 until m1 and m2 arrive first.\n")

    p0_ooo = Process("P0", procs)
    p1_ooo = Process("P1", procs)
    p2_ooo = Process("P2", procs)
    processes_ooo = {"P0": p0_ooo, "P1": p1_ooo, "P2": p2_ooo}
    cb_ooo = CausalBroadcast(procs)

    print(f"m1 vc={msg_m1.vc}")
    print(f"m2 vc={msg_m2.vc}")
    print(f"m3 vc={msg_m3.vc} (P1's VC carries P0≥2: causally depends on m1,m2)")
    print()

    print("P2 receives m3 first (out of order):")
    result = cb_ooo.inject_message("P2", msg_m3, processes_ooo)
    print(f"  {result}")
    print(f"  P2's vc is {p2_ooo.vc}, but m3 carries P0≥2 → not met → BUFFERED")
    print()

    print("P2 receives m1 next:")
    result = cb_ooo.inject_message("P2", msg_m1, processes_ooo)
    print(f"  {result}")

    print("\nP2 receives m2:")
    result = cb_ooo.inject_message("P2", msg_m2, processes_ooo)
    print(f"  {result}")

    print("\nFlushing buffer (m3's P0 dependency now met):")
    delivered = cb_ooo.flush_buffer("P2", processes_ooo)
    for d in delivered:
        print(f"  {d}")

    print(f"\nDelivery order at P2 (causal order preserved):")
    for m in p2_ooo.delivered:
        print(f"  '{m.content}' from {m.sender}")
    print()
    if len(p2_ooo.delivered) == 3:
        print("CBCAST successfully buffered m3 and delivered it after m1 and m2,")
        print("preserving causal order despite network reordering.")
    else:
        print(f"Delivered {len(p2_ooo.delivered)} of 3 expected messages.")


def demo_lamport_vs_vector() -> None:
    sep("SCENARIO 4: Why Lamport Clocks Are Not Enough")

    procs = ["A", "B"]

    vc_a = VectorClock(procs)
    vc_a.increment("A")
    vc_a.increment("A")

    vc_b = VectorClock(procs)
    vc_b.increment("B")

    print("Two independent events:")
    print(f"  Event at A: vc={vc_a}  (A did 2 local events)")
    print(f"  Event at B: vc={vc_b}  (B did 1 local event)")
    print()

    lc_a, lc_b = 2, 1
    print(f"Lamport timestamps: LC(A)={lc_a}, LC(B)={lc_b}")
    print(f"  LC(A) > LC(B)  →  looks like A happened 'after' B?")
    print(f"  But A and B are INDEPENDENT — neither caused the other!")
    print()
    print(f"Vector clocks reveal the truth:")
    print(f"  VC(A)={vc_a.counters}  vs  VC(B)={vc_b.counters}")
    print(f"  A[0]={vc_a.counters[0]} > B[0]={vc_b.counters[0]}  AND  "
          f"A[1]={vc_a.counters[1]} < B[1]={vc_b.counters[1]}")
    print(f"  → incomparable → CONCURRENT")
    print()
    print("Key insight:")
    print("  Lamport: LC(a) < LC(b) ⟹ 'maybe a→b, maybe not' (inconclusive)")
    print("  Vector:  VC(a) < VC(b) ⟹ 'definitely a→b'          (conclusive)")
    print("  Vector:  VC(a) || VC(b) ⟹ 'definitely concurrent'   (conclusive)")


if __name__ == "__main__":
    print("Vector Clocks and Causal Order")
    demo_vc_evolution()
    demo_conflict_detection()
    demo_causal_broadcast()
    demo_lamport_vs_vector()