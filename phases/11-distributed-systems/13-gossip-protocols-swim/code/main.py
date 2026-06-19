import random
import copy
import math
from enum import Enum, auto
from dataclasses import dataclass
from typing import Optional


class MemberState(Enum):
    ALIVE = auto()
    SUSPECT = auto()
    DEAD = auto()


@dataclass
class MembershipEntry:
    state: MemberState
    incarnation: int

    def dominates(self, other: "MembershipEntry") -> bool:
        if self.state == MemberState.DEAD and other.state != MemberState.DEAD:
            return True
        if other.state == MemberState.DEAD and self.state != MemberState.DEAD:
            return False
        if self.incarnation > other.incarnation:
            return True
        if self.incarnation < other.incarnation:
            return False
        severity_order = {MemberState.ALIVE: 0, MemberState.SUSPECT: 1, MemberState.DEAD: 2}
        return severity_order[self.state] > severity_order[other.state]


class SWIMMember:
    def __init__(self, node_id: str):
        self.node_id = node_id
        self.incarnation = 0
        self.state = MemberState.ALIVE
        self.membership: dict[str, MembershipEntry] = {}
        self.alive = True
        self.slow = False
        self.slow_drop_rate = 0.0
        self.piggyback_buffer: list[tuple[str, MembershipEntry]] = []
        self._needs_refutation = False

    def init_peers(self, peer_ids: list[str]):
        for pid in peer_ids:
            if pid != self.node_id:
                self.membership[pid] = MembershipEntry(MemberState.ALIVE, 0)

    def mark_suspect(self, target: str, incarnation: int):
        entry = MembershipEntry(MemberState.SUSPECT, incarnation)
        self._merge(target, entry)

    def mark_alive(self, target: str, incarnation: int):
        entry = MembershipEntry(MemberState.ALIVE, incarnation)
        self._merge(target, entry)

    def mark_dead(self, target: str, incarnation: int):
        entry = MembershipEntry(MemberState.DEAD, incarnation)
        self._merge(target, entry)

    def refute_if_suspected(self) -> bool:
        if not self._needs_refutation and self.state != MemberState.SUSPECT:
            return False
        self.incarnation += 1
        self._needs_refutation = False
        self.state = MemberState.ALIVE
        refutation = MembershipEntry(MemberState.ALIVE, self.incarnation)
        self.piggyback_buffer.append((self.node_id, copy.deepcopy(refutation)))
        return True

    def receive_piggyback(self, updates: list[tuple[str, MembershipEntry]]):
        for nid, entry in updates:
            self._merge(nid, copy.deepcopy(entry))

    def get_piggyback(self, max_items: int = 5) -> list[tuple[str, MembershipEntry]]:
        result = self.piggyback_buffer[-max_items:]
        self.piggyback_buffer = self.piggyback_buffer[:-max_items] if len(self.piggyback_buffer) > max_items else []
        return result

    def alive_peers(self, exclude: Optional[set[str]] = None) -> list[str]:
        exclude = (exclude or set()) | {self.node_id}
        return [nid for nid, e in self.membership.items()
                if e.state in (MemberState.ALIVE, MemberState.SUSPECT) and nid not in exclude]

    def _merge(self, nid: str, incoming: MembershipEntry):
        if nid == self.node_id:
            if incoming.state == MemberState.SUSPECT and incoming.incarnation >= self.incarnation:
                self._needs_refutation = True
            elif incoming.state == MemberState.ALIVE and incoming.incarnation > self.incarnation:
                pass
            return
        existing = self.membership.get(nid)
        if existing is None:
            self.membership[nid] = copy.deepcopy(incoming)
            self.piggyback_buffer.append((nid, copy.deepcopy(incoming)))
            return
        if incoming.dominates(existing):
            self.membership[nid] = copy.deepcopy(incoming)
            self.piggyback_buffer.append((nid, copy.deepcopy(incoming)))


class SWIMCluster:
    def __init__(self, node_count: int, drop_rate: float = 0.0,
                 indirect_probe_count: int = 3, suspicion_timeout: int = 4,
                 seed: int = 42):
        self.rng = random.Random(seed)
        self.drop_rate = drop_rate
        self.indirect_probe_count = indirect_probe_count
        self.suspicion_timeout = suspicion_timeout
        self.round_num = 0
        self.suspicion_start: dict[str, int] = {}
        self.members: dict[str, SWIMMember] = {}

        all_ids = [f"node-{i}" for i in range(node_count)]
        for nid in all_ids:
            m = SWIMMember(nid)
            m.init_peers(all_ids)
            self.members[nid] = m

    def _log(self, msg: str, events: list[str] | None = None):
        line = f"[R{self.round_num:3d}] {msg}"
        if events is not None:
            events.append(line)

    def _can_respond(self, node_id: str) -> bool:
        m = self.members[node_id]
        if not m.alive:
            return False
        if m.slow and self.rng.random() < m.slow_drop_rate:
            return False
        if self.rng.random() < self.drop_rate:
            return False
        return True

    def _ping(self, src: str, dst: str) -> tuple[bool, list[tuple[str, MembershipEntry]]]:
        if not self._can_respond(dst):
            return False, []
        dst_member = self.members[dst]
        piggyback = dst_member.get_piggyback()
        return True, piggyback

    def _indirect_probe(self, pinger: str, target: str) -> tuple[bool, list[tuple[str, MembershipEntry]]]:
        pinger_member = self.members[pinger]
        candidates = pinger_member.alive_peers(exclude={target})
        if not candidates:
            return False, []
        k = min(self.indirect_probe_count, len(candidates))
        proxies = self.rng.sample(candidates, k)
        all_updates: list[tuple[str, MembershipEntry]] = []
        for proxy in proxies:
            if not self._can_respond(proxy):
                continue
            proxy_member = self.members[proxy]
            all_updates.extend(proxy_member.get_piggyback())
            if self._can_respond(target):
                target_member = self.members[target]
                all_updates.extend(target_member.get_piggyback())
                return True, all_updates
        return False, all_updates

    def kill_node(self, node_id: str):
        self.members[node_id].alive = False
        self.members[node_id].state = MemberState.DEAD

    def slow_node(self, node_id: str, drop_rate: float = 0.5):
        self.members[node_id].slow = True
        self.members[node_id].slow_drop_rate = drop_rate

    def run_round(self) -> list[str]:
        self.round_num += 1
        events: list[str] = []

        alive_ids = [nid for nid, m in self.members.items() if m.alive]
        order = list(alive_ids)
        self.rng.shuffle(order)

        for nid in order:
            member = self.members[nid]
            targets = member.alive_peers()
            if not targets:
                continue
            target = self.rng.choice(targets)
            target_inc = self.members[target].incarnation

            ok, piggyback = self._ping(nid, target)
            if ok:
                member.mark_alive(target, self.members[target].incarnation)
                member.receive_piggyback(piggyback)
                self._log(f"{nid} ─ping─→ {target} ✓", events)

                src_pb = member.get_piggyback()
                self.members[target].receive_piggyback(src_pb)
            else:
                indirect_ok, indirect_pb = self._indirect_probe(nid, target)
                if indirect_ok:
                    member.mark_alive(target, self.members[target].incarnation)
                    member.receive_piggyback(indirect_pb)
                    self._log(f"{nid} ─indirect─→ {target} ✓", events)
                    src_pb = member.get_piggyback()
                    self.members[target].receive_piggyback(src_pb)
                else:
                    member.mark_suspect(target, target_inc)
                    self.suspicion_start.setdefault(target, self.round_num)
                    self._log(f"{nid} ⊗ suspects {target}", events)

        for nid in order:
            member = self.members[nid]
            if member.alive and member.refute_if_suspected():
                self._log(f"{nid} REFUTES (inc → {member.incarnation})", events)

        for target_id in list(self.suspicion_start.keys()):
            start_round = self.suspicion_start[target_id]
            if self.members[target_id].alive:
                continue
            if self.round_num - start_round >= self.suspicion_timeout:
                for nid in alive_ids:
                    self.members[nid].mark_dead(
                        target_id,
                        self.members[nid].membership.get(
                            target_id, MembershipEntry(MemberState.SUSPECT, 0)
                        ).incarnation
                    )
                self._log(f"{target_id} confirmed DEAD", events)
                del self.suspicion_start[target_id]

        return events

    def run_rounds(self, n: int) -> list[list[str]]:
        return [self.run_round() for _ in range(n)]

    def view_from(self, viewer_id: str) -> dict[str, tuple[str, int]]:
        viewer = self.members[viewer_id]
        result = {}
        for nid, m in self.members.items():
            if nid == viewer_id:
                status = "ALIVE" if m.alive else "DEAD"
                result[nid] = (status, m.incarnation)
            else:
                entry = viewer.membership.get(nid)
                if entry:
                    result[nid] = (entry.state.name, entry.incarnation)
                else:
                    result[nid] = ("UNKNOWN", 0)
        return result

    def print_view(self, viewer_id: str | None = None, title: str = ""):
        if title:
            print(f"\n  {title}")
        if viewer_id is None:
            viewer_id = next(nid for nid, m in self.members.items() if m.alive)
        view = self.view_from(viewer_id)
        print(f"  {'Node':<10} {'Actual':<8} {'Viewed':<10} {'Inc':<5}")
        print(f"  {'-'*35}")
        for nid in sorted(self.members.keys()):
            m = self.members[nid]
            actual = "DEAD" if not m.alive else ("SLOW" if m.slow else "ALIVE")
            viewed, inc = view[nid]
            print(f"  {nid:<10} {actual:<8} {viewed:<10} {inc:<5}")


def demo_node_death():
    print("=" * 70)
    print("DEMO 1: Node Death — Suspicion → Confirmed Death")
    print("=" * 70)

    c = SWIMCluster(node_count=10, seed=42, suspicion_timeout=4)

    print("\n  Initial state (10 nodes, all alive):")
    c.print_view()

    print("\n  Warming up for 3 rounds...")
    c.run_rounds(3)

    print("\n  --- KILLING node-7 ---")
    c.kill_node("node-7")

    suspected_at = None
    confirmed_at = None
    for i in range(20):
        events = c.run_round()
        view = c.view_from("node-0")
        n7_view, n7_inc = view["node-7"]

        if n7_view == "SUSPECT" and suspected_at is None:
            suspected_at = c.round_num
            print(f"\n  Round {c.round_num}: node-7 SUSPECTED (first detection)")
        if n7_view == "DEAD" and confirmed_at is None:
            confirmed_at = c.round_num
            print(f"  Round {c.round_num}: node-7 CONFIRMED DEAD")

    print(f"\n  Round {confirmed_at}: node-7 death confirmed in {confirmed_at - suspected_at} rounds after suspicion")

    c.print_view(title="Final state")
    return c


def demo_slow_refutation():
    print("\n" + "=" * 70)
    print("DEMO 2: Suspicion Refutation — False Positive Recovered via Incarnation")
    print("=" * 70)

    c = SWIMCluster(node_count=10, seed=123, suspicion_timeout=20)

    print("\n  Initial state (10 nodes):")
    c.print_view()

    c.run_rounds(3)

    print("\n  --- Simulating: node-0 and node-1 suspect node-3 ---")
    print("  (This could happen from a transient network issue)")
    c.members["node-0"].mark_suspect("node-3", 0)
    c.members["node-1"].mark_suspect("node-3", 0)

    print("\n  Directly sending suspicion to node-3 (simulating gossip delivery):")
    print("  node-3 receives piggyback: {(node-3, SUSPECT, inc=0)}")
    suspicion_update = [("node-3", MembershipEntry(MemberState.SUSPECT, 0))]
    c.members["node-3"].receive_piggyback(suspicion_update)

    c.print_view(title="After suspicion (node-0's view)")

    print("\n  Running gossip rounds — node-3 refutes and cluster converges...")
    for i in range(10):
        events = c.run_round()
        node3 = c.members["node-3"]
        view = c.view_from("node-0")
        n3_view, n3_inc = view["node-3"]

        all_aware = sum(
            1 for nid, m in c.members.items()
            if nid != "node-3" and m.alive and
            m.membership.get("node-3") and
            m.membership["node-3"].state == MemberState.ALIVE and
            m.membership["node-3"].incarnation >= 1
        )

        print(f"  Round {c.round_num}: node-3 inc={node3.incarnation}, "
              f"viewed={n3_view} (inc={n3_inc}), "
              f"nodes_with_refutation={all_aware}/9")

        if n3_view == "ALIVE" and n3_inc > 0:
            print(f"\n  ✓ node-3 refuted suspicion! Incarnation: 0 → {n3_inc}")
            print("    The SUSPECT entry was overridden by ALIVE with higher incarnation.")
            break

    c.print_view(title="Final state after refutation")

    c.print_view(title="Final state")


def demo_convergence():
    print("\n" + "=" * 70)
    print("DEMO 3: Gossip Convergence Speed — O(log N)")
    print("=" * 70)

    c = SWIMCluster(node_count=20, seed=99, suspicion_timeout=4)

    c.run_rounds(2)
    print(f"\n  Cluster: 20 nodes")
    print("  Killing node-15...")

    c.kill_node("node-15")

    for i in range(25):
        c.run_round()
        alive = [m for m in c.members.values() if m.alive]
        aware = sum(
            1 for m in alive
            if m.membership.get("node-15") and
            m.membership["node-15"].state in (MemberState.SUSPECT, MemberState.DEAD)
        )
        total = len(alive)
        if i < 4 or aware > 0 or (i + 1) % 3 == 0:
            print(f"  Round {c.round_num}: {aware}/{total} nodes aware of node-15 failure")

        if aware == total:
            log_n = math.log(20, 4)
            print(f"\n  ✓ Full convergence in {c.round_num} rounds "
                  f"(theoretical O(log N) ≈ log₄(20) ≈ {log_n:.1f})")
            break


def demo_anti_entropy():
    print("\n" + "=" * 70)
    print("DEMO 4: Piggyback Dissemination — Membership Spreads on Existing Messages")
    print("=" * 70)

    c = SWIMCluster(node_count=8, seed=77, suspicion_timeout=3)

    c.run_rounds(2)

    c.kill_node("node-5")
    print("  Killed node-5\n")

    node0 = c.members["node-0"]
    node0.mark_suspect("node-5", 0)
    print("  node-0 directly observes node-5 failure and marks it SUSPECT")

    for i in range(10):
        c.run_round()
        alive = [m for m in c.members.values() if m.alive]
        aware = sum(
            1 for m in alive
            if m.membership.get("node-5") and
            m.membership["node-5"].state in (MemberState.SUSPECT, MemberState.DEAD)
        )
        print(f"  Round {c.round_num}: {aware}/{len(alive)} nodes know about node-5")

    c.print_view(title="Final state")


if __name__ == "__main__":
    demo_node_death()
    demo_slow_refutation()
    demo_convergence()
    demo_anti_entropy()

    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print("""
SWIM Protocol Components:
  1. Failure Detection: ping → ack, with indirect probe on timeout
  2. Suspicion: Suspect → refute with higher incarnation → Alive
                Suspect → confirmation timeout → Dead
  3. Dissemination: membership updates piggyback on ping/ack messages

Key Properties:
  - O(N) messages per round (each node pings one target)
  - O(log N) rounds for cluster-wide convergence
  - False positives mitigated by indirect probing + suspicion refutation
  - No central coordinator, no all-to-all heartbeats
  - Incarnation numbers prevent stale info from overriding fresh state

Production Systems Using SWIM:
  - HashiCorp memberlist (Go) — powers Consul, Serf, Nomad
  - Apache Cassandra — gossip for membership and schema propagation
  - Serf — SWIM + Lifeguard for reduced false positives
""")