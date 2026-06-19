import time
import random
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional


class ConsistencyMode(Enum):
    CP = "CP"
    AP = "AP"


class ConsistencyLevel(Enum):
    ONE = "ONE"
    QUORUM = "QUORUM"
    ALL = "ALL"


@dataclass
class Node:
    name: str
    data: dict = field(default_factory=dict)
    is_alive: bool = True
    partition_group: Optional[int] = None
    vector_clock: dict = field(default_factory=dict)
    reject_count: int = 0
    stale_read_count: int = 0

    def read(self, key: str) -> Optional[str]:
        return self.data.get(key)

    def write(self, key: str, value: str, vc: dict | None = None):
        self.data[key] = value
        if vc is not None:
            self.vector_clock = dict(vc)
            node_id = self.name
            self.vector_clock[node_id] = self.vector_clock.get(node_id, 0) + 1
        else:
            self.vector_clock[self.name] = self.vector_clock.get(self.name, 0) + 1


@dataclass
class WriteResult:
    accepted: bool
    node: str
    key: str
    value: str
    reason: str = ""


@dataclass
class ReadResult:
    value: Optional[str]
    node: str
    stale: bool
    reason: str = ""


class CAPSimulator:
    def __init__(self, node_names: list[str], mode: ConsistencyMode):
        self.nodes = {name: Node(name=name) for name in node_names}
        self.mode = mode
        self.partition_active = False
        self.partition_groups: dict[int, list[Node]] = {}
        self.operation_log: list[str] = []

    def _can_communicate(self, from_node: Node, to_node: Node) -> bool:
        if not self.partition_active:
            return True
        if from_node.partition_group is None or to_node.partition_group is None:
            return True
        return from_node.partition_group == to_node.partition_group

    def introduce_partition(self, group_assignment: dict[str, int]):
        self.partition_active = True
        self.partition_groups = {}
        for name, group_id in group_assignment.items():
            node = self.nodes[name]
            node.partition_group = group_id
            self.partition_groups.setdefault(group_id, []).append(node)
        group_names = {}
        for gid, nodes in self.partition_groups.items():
            group_names[gid] = [n.name for n in nodes]
        self.operation_log.append(
            f"PARTITION: {group_names}"
        )

    def heal_partition(self):
        self.partition_active = False
        for node in self.nodes.values():
            node.partition_group = None
        self.partition_groups = {}
        self.operation_log.append("PARTITION HEALED")

    def _majority_group(self) -> Optional[int]:
        if not self.partition_active:
            return None
        total = len(self.nodes)
        for gid, nodes in self.partition_groups.items():
            if len(nodes) > total / 2:
                return gid
        return None

    def write(self, key: str, value: str, target_node: str) -> WriteResult:
        node = self.nodes[target_node]
        if not self.partition_active:
            node.write(key, value)
            self.operation_log.append(
                f"WRITE OK  node={target_node} key={key} value={value}"
            )
            return WriteResult(accepted=True, node=target_node, key=key, value=value)

        majority = self._majority_group()
        node_group = node.partition_group

        if self.mode == ConsistencyMode.CP:
            if majority is None or node_group != majority:
                node.reject_count += 1
                self.operation_log.append(
                    f"WRITE REJECTED node={target_node} key={key} value={value} "
                    f"reason=minority_partition"
                )
                return WriteResult(
                    accepted=False, node=target_node, key=key, value=value,
                    reason="CP mode: rejecting write on minority partition"
                )
            node.write(key, value)
            self.operation_log.append(
                f"WRITE OK  node={target_node} key={key} value={value}"
            )
            return WriteResult(accepted=True, node=target_node, key=key, value=value)

        else:
            node.write(key, value)
            self.operation_log.append(
                f"WRITE OK  node={target_node} key={key} value={value} "
                f"(stale on other partition until heal)"
            )
            return WriteResult(accepted=True, node=target_node, key=key, value=value)

    def read(self, key: str, target_node: str) -> ReadResult:
        node = self.nodes[target_node]

        if not self.partition_active:
            value = node.read(key)
            self.operation_log.append(
                f"READ OK   node={target_node} key={key} value={value}"
            )
            return ReadResult(value=value, node=target_node, stale=False)

        majority = self._majority_group()
        node_group = node.partition_group

        if self.mode == ConsistencyMode.CP:
            if majority is None or node_group != majority:
                node.reject_count += 1
                self.operation_log.append(
                    f"READ REJECTED node={target_node} key={key} reason=minority_partition"
                )
                return ReadResult(
                    value=None, node=target_node, stale=False,
                    reason="CP mode: rejecting read on minority partition"
                )
            value = node.read(key)
            self.operation_log.append(
                f"READ OK   node={target_node} key={key} value={value}"
            )
            return ReadResult(value=value, node=target_node, stale=False)

        else:
            value = node.read(key)
            latest_value = self._latest_value_for_key(key)
            is_stale = value != latest_value if latest_value is not None else False
            if is_stale:
                node.stale_read_count += 1
            self.operation_log.append(
                f"READ OK   node={target_node} key={key} value={value}"
                + (" (STALE)" if is_stale else "")
            )
            return ReadResult(value=value, node=target_node, stale=is_stale)

    def _latest_value_for_key(self, key: str) -> Optional[str]:
        values = []
        for node in self.nodes.values():
            if node.data.get(key) is not None:
                values.append((node.vector_clock.get(node.name, 0), node.data[key]))
        if not values:
            return None
        values.sort(key=lambda x: x[0], reverse=True)
        return values[0][1]

    def replicate(self):
        if self.partition_active:
            self.operation_log.append("REPLICATE: skipped (partition active)")
            return
        keys = set()
        for node in self.nodes.values():
            keys.update(node.data.keys())
        for key in keys:
            latest_node = None
            latest_vc_sum = -1
            for node in self.nodes.values():
                vc_sum = sum(node.vector_clock.values())
                if key in node.data and vc_sum > latest_vc_sum:
                    latest_vc_sum = vc_sum
                    latest_node = node
            if latest_node is not None:
                for node in self.nodes.values():
                    if node.name != latest_node.name:
                        old_val = node.data.get(key, "<none>")
                        node.data[key] = latest_node.data[key]
                        node.vector_clock = dict(latest_node.vector_clock)
                        if old_val != latest_node.data[key]:
                            self.operation_log.append(
                                f"REPLICATE {key}={latest_node.data[key]} "
                                f"from {latest_node.name} to {node.name} "
                                f"(was {old_val})"
                            )

    def state_summary(self) -> str:
        lines = []
        lines.append(f"{'Node':<10} {'Data':<25} {'Rejects':<10} {'Stale Reads':<12} {'VC'}")
        lines.append("-" * 70)
        for node in self.nodes.values():
            data_str = str(node.data) if node.data else "{}"
            vc_str = str(dict(node.vector_clock)) if node.vector_clock else "{}"
            lines.append(
                f"{node.name:<10} {data_str:<25} {node.reject_count:<10} "
                f"{node.stale_read_count:<12} {vc_str}"
            )
        return "\n".join(lines)


class PACELCExplorer:
    def __init__(self, profile: str):
        self.profile = profile
        self.partition_mode, self.normal_mode = self._parse_profile(profile)
        self.latency_baseline_ns = 500_000
        self.partition_log: list[str] = []

    def _parse_profile(self, profile: str):
        parts = profile.strip().split("/")
        if len(parts) != 2:
            raise ValueError(f"Invalid PACELC profile: {profile}. Use format like PA/EL or PC/EC")
        p_choice = parts[0].strip().upper()
        e_choice = parts[1].strip().upper()
        if p_choice not in ("PA", "PC"):
            raise ValueError(f"Partition choice must be PA or PC, got {p_choice}")
        if e_choice not in ("EL", "EC"):
            raise ValueError(f"Normal operation choice must be EL or EC, got {e_choice}")
        return p_choice, e_choice

    def read_latency(self, level: ConsistencyLevel) -> float:
        base = self.latency_baseline_ns
        n_replicas = 5
        if level == ConsistencyLevel.ONE:
            return base
        elif level == ConsistencyLevel.QUORUM:
            return base * (1 + (n_replicas // 2) * 0.1)
        elif level == ConsistencyLevel.ALL:
            return base * (1 + n_replicas * 0.1)
        return base

    def behavior_during_partition(self, key: str) -> str:
        if self.partition_mode == "PA":
            return (
                f"AVAILABILITY over CONSISTENCY for key '{key}':\n"
                f"  - Both partitions serve reads and writes\n"
                f"  - Reads may return stale data\n"
                f"  - Writes on different partitions will diverge\n"
                f"  - Reconciliation required on heal"
            )
        else:
            return (
                f"CONSISTENCY over AVAILABILITY for key '{key}':\n"
                f"  - Only majority partition serves requests\n"
                f"  - Minority partition rejects all reads and writes\n"
                f"  - Data remains consistent but unavailable to minority"
            )

    def behavior_during_normal(self, key: str) -> str:
        if self.normal_mode == "EL":
            latency = self.read_latency(ConsistencyLevel.ONE)
            return (
                f"LATENCY over CONSISTENCY for key '{key}':\n"
                f"  - Reads from nearest replica (~{latency / 1_000_000:.2f} ms)\n"
                f"  - May return stale data from local replica\n"
                f"  - Replication is asynchronous\n"
                f"  - Eventual consistency during normal operation"
            )
        else:
            latency = self.read_latency(ConsistencyLevel.QUORUM)
            return (
                f"CONSISTENCY over LATENCY for key '{key}':\n"
                f"  - Reads require quorum (~{latency / 1_000_000:.2f} ms)\n"
                f"  - Always return latest committed value\n"
                f"  - Synchronous replication before acknowledgment\n"
                f"  - Strong consistency during normal operation"
            )

    def compare_profiles(self, profiles: list[str]) -> str:
        lines = ["PACELC Profile Comparison", "=" * 72]
        lines.append(
            f"{'Profile':<10} {'Partition':<25} {'Normal Op':<25} {'Best For'}"
        )
        lines.append("-" * 72)
        descriptions = {
            "PA/EL": ("Available (may be stale)", "Low latency (may be stale)", "High-availability web apps"),
            "PA/EC": ("Available (may be stale)", "Strong consistency", "Mixed-workload systems"),
            "PC/EL": ("Reject minority (consistent)", "Low latency (may be stale)", "Rare; unusual combo"),
            "PC/EC": ("Reject minority (consistent)", "Strong consistency", "Financial systems, config mgmt"),
        }
        for p in profiles:
            desc = descriptions.get(p, ("—", "—", "—"))
            lines.append(f"{p:<10} {desc[0]:<25} {desc[1]:<25} {desc[2]}")
        return "\n".join(lines)

    def full_report(self, key: str = "user_42") -> str:
        lines = [
            f"PACELC Explorer: {self.profile} Profile",
            "=" * 72,
            "",
            "DURING PARTITION (P → A or C):",
            self.behavior_during_partition(key),
            "",
            "DURING NORMAL OPERATION (E → L or C):",
            self.behavior_during_normal(key),
            "",
            "Latency comparison (5-node cluster):",
        ]
        for level in ConsistencyLevel:
            lat = self.read_latency(level)
            lines.append(f"  {level.value:<10} {lat / 1_000_000:.3f} ms")
        return "\n".join(lines)


def demo_cap_simulation():
    print("=" * 72)
    print("DEMO 1: CAP Trade-off Simulator (3-node cluster)")
    print("=" * 72)
    print()

    for mode in [ConsistencyMode.CP, ConsistencyMode.AP]:
        print(f"\n--- {mode.value} Mode ---\n")
        sim = CAPSimulator(["N1", "N2", "N3"], mode)

        for node_name in ["N1", "N2", "N3"]:
            sim.write(key="balance", value="100", target_node=node_name)
        sim.replicate()

        print(f"Initial state (all nodes agree: balance=100):")
        print(sim.state_summary())
        print()

        print("Introducing partition: {N1, N2} | {N3}")
        sim.introduce_partition({"N1": 0, "N2": 0, "N3": 1})

        print("Writing balance=200 to N1 (majority):")
        r1 = sim.write(key="balance", value="200", target_node="N1")
        print(f"  Result: accepted={r1.accepted} reason={r1.reason or 'ok'}")

        print("Writing balance=150 to N3 (minority):")
        r2 = sim.write(key="balance", value="150", target_node="N3")
        print(f"  Result: accepted={r2.accepted} reason={r2.reason or 'ok'}")

        print("Reading balance from N2 (majority):")
        r3 = sim.read(key="balance", target_node="N2")
        print(f"  Result: value={r3.value} stale={r3.stale} reason={r3.reason or 'ok'}")

        print("Reading balance from N3 (minority):")
        r4 = sim.read(key="balance", target_node="N3")
        print(f"  Result: value={r4.value} stale={r4.stale} reason={r4.reason or 'ok'}")

        print("\nState during partition:")
        print(sim.state_summary())
        print()

        print("Healing partition...")
        sim.heal_partition()
        sim.replicate()

        print("State after healing and reconciliation:")
        print(sim.state_summary())
        print()

    print("\nOperation Log:")
    print("-" * 72)


def demo_five_node_cluster():
    print("\n" + "=" * 72)
    print("DEMO 2: 5-Node Cluster Under Partition (3 vs 2)")
    print("=" * 72)
    print()

    for mode in [ConsistencyMode.CP, ConsistencyMode.AP]:
        print(f"\n--- {mode.value} Mode (5-node cluster) ---\n")
        sim = CAPSimulator(["N1", "N2", "N3", "N4", "N5"], mode)

        for n in ["N1", "N2", "N3", "N4", "N5"]:
            sim.write(key="counter", value="0", target_node=n)
        sim.replicate()

        print("Initial: all nodes have counter=0")
        print(sim.state_summary())
        print()

        print("Partition: {N1, N2, N3} | {N4, N5}")
        sim.introduce_partition({"N1": 0, "N2": 0, "N3": 0, "N4": 1, "N5": 1})

        print("Majority partition writes:")
        for val, node in [("10", "N1"), ("20", "N2"), ("30", "N3")]:
            r = sim.write(key="counter", value=val, target_node=node)
            print(f"  write counter={val} to {node}: accepted={r.accepted}")

        print("\nMinority partition writes:")
        for val, node in [("99", "N4"), ("88", "N5")]:
            r = sim.write(key="counter", value=val, target_node=node)
            print(f"  write counter={val} to {node}: accepted={r.accepted} {r.reason}")

        print("\nReads during partition:")
        for node in ["N1", "N4"]:
            r = sim.read(key="counter", target_node=node)
            print(f"  read from {node}: value={r.value} stale={r.stale} {r.reason}")

        print("\nState during partition:")
        print(sim.state_summary())

        print("\nHealing partition and reconciling...")
        sim.heal_partition()
        sim.replicate()

        print("State after healing:")
        print(sim.state_summary())
        print()


def demo_pacelc():
    print("\n" + "=" * 72)
    print("DEMO 3: PACELC Profiles")
    print("=" * 72)
    print()

    profiles = ["PA/EL", "PC/EC", "PA/EC", "PC/EL"]
    for profile in profiles:
        explorer = PACELCExplorer(profile)
        print(explorer.full_report())
        print()

    print(explorer.compare_profiles(profiles))
    print()


def demo_consistency_models():
    print("=" * 72)
    print("DEMO 4: Consistency Models — What You Get When You Sacrifice C")
    print("=" * 72)
    print()

    models = [
        ("Linearizability",
         "Every read sees the most recent write. Total order matches real time.",
         "etcd, ZooKeeper, HBase (during normal operation in CP/EC mode)",
         "Read waits for quorum agreement. Latest value guaranteed."),
        ("Sequential Consistency",
         "Operations appear in some total order respecting each client's program order.",
         "Some Raft configurations with stale reads",
         "Reads may see stale values from another client, but each client's own writes are ordered."),
        ("Causal Consistency",
         "Causally related operations appear in order. Concurrent writes can differ across clients.",
         "Cassandra with lightweight transactions, Riak",
         "You see your own writes. Concurrent writes may disagree across replicas."),
        ("Eventual Consistency",
         "If no new writes, all replicas eventually converge. No bound on staleness.",
         "Cassandra (default), DynamoDB (eventually consistent reads), Couchbase",
         "Reads may return arbitrarily stale data. Convergence is guaranteed but timing is not."),
    ]

    lines = ["Consistency Models (Strong → Weak)", "-" * 72]
    for name, definition, examples, implication in models:
        lines.append(f"\n{name}:")
        lines.append(f"  Definition:  {definition}")
        lines.append(f"  Examples:    {examples}")
        lines.append(f"  Implication: {implication}")

    lines.append("\n" + "-" * 72)
    lines.append("The gap between linearizability and eventual consistency is where bugs live.")
    lines.append("CAP says you must choose during a partition. PACELC says you're always choosing.")
    print("\n".join(lines))
    print()


def demo_cap_misconceptions():
    print("=" * 72)
    print("DEMO 5: Common CAP Misconceptions")
    print("=" * 72)
    print()

    misconceptions = [
        (
            '"You can pick two of C, A, P"',
            "The P is mandatory because network partitions *will* happen. You can't choose CA. The real choice is CP (sacrifice availability) or AP (sacrifice consistency) *during a partition*.",
        ),
        (
            '"CAP means eventual consistency is unavoidable"',
            "CAP only forces the choice during partitions. During normal operation (99.9%+ of the time), you can have strong consistency. PACELC makes this explicit: the E branch is the one that matters most.",
        ),
        (
            '"AP systems are always available"',
            "AP systems are available *during a partition*. They can still have outages, bugs, and overload. Availability in CAP means 'a non-failing node returns a non-error response,' not 'the system never goes down.'",
        ),
        (
            '"CP systems are always consistent"',
            "CP systems guarantee linearizability only when there's no partition. The consistency guarantee is that they refuse to serve possibly-stale data — by returning errors. The data is consistent because some requests were rejected.",
        ),
        (
            '"Eventual consistency is good enough for everything"',
            "Eventual consistency means reads can return arbitrarily stale data. For user account balances, inventory counts, and access control lists, this is a bug, not a feature. PACELC reminds you that you're choosing latency *during normal operation* — not just during partitions.",
        ),
    ]

    for myth, reality in misconceptions:
        print(f"  MYTH: {myth}")
        print(f"  FACT: {reality}")
        print()


if __name__ == "__main__":
    demo_cap_simulation()
    demo_five_node_cluster()
    demo_pacelc()
    demo_consistency_models()
    demo_cap_misconceptions()