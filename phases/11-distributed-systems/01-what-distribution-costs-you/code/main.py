import math
from dataclasses import dataclass, field


@dataclass
class LatencyNumbers:
    reference: dict = field(default_factory=lambda: {
        "l1": ("L1 cache reference", 1, "ns"),
        "l2": ("L2 cache reference", 7, "ns"),
        "mutex": ("Mutex lock/unlock", 25, "ns"),
        "ram": ("Main memory reference", 100, "ns"),
        "compress_1kb": ("Compress 1 KB (Snappy)", 3000, "ns"),
        "ssd_read_4kb": ("Read 4 KB from SSD", 100_000, "ns"),
        "ssd_read_1mb": ("Read 1 MB from SSD", 500_000, "ns"),
        "network_dc": ("Round trip same datacenter", 500_000, "ns"),
        "network_region": ("Round trip same region", 5_000_000, "ns"),
        "network_cross_region": ("Round trip cross-region", 100_000_000, "ns"),
        "network_cross_continent": ("Round trip cross-continent", 150_000_000, "ns"),
    })

    def display(self):
        print("Latency Numbers Every Programmer Should Know")
        print("=" * 60)
        for _key, (name, ns, unit) in self.reference.items():
            if unit == "ns":
                if ns >= 1_000_000:
                    print(f"  {name:40s} ~{ns / 1_000_000:.0f} ms")
                elif ns >= 1_000:
                    print(f"  {name:40s} ~{ns / 1_000:.0f} μs")
                else:
                    print(f"  {name:40s} ~{ns} ns")
        print()

    def get_ns(self, op_key: str) -> float:
        entry = self.reference.get(op_key)
        if entry is None:
            raise ValueError(f"Unknown operation: {op_key}")
        return entry[1]


class LatencySimulator:
    def __init__(self, numbers: LatencyNumbers | None = None):
        self.numbers = numbers or LatencyNumbers()

    def simulate(self, operations: list[str]) -> dict:
        total_ns = 0
        breakdown = []
        for op in operations:
            ns = self.numbers.get_ns(op)
            label = self.numbers.reference[op][0]
            total_ns += ns
            breakdown.append((op, label, ns))

        return {"total_ns": total_ns, "breakdown": breakdown}

    def format_result(self, result: dict) -> str:
        lines = []
        total_ns = result["total_ns"]
        for _op, label, ns in result["breakdown"]:
            lines.append(f"  {label:40s} {self._human_ns(ns):>12s}")
        lines.append(f"  {'─' * 52}")
        lines.append(f"  {'TOTAL':40s} {self._human_ns(total_ns):>12s}")
        return "\n".join(lines)

    def compare(self, local_ops: list[str], remote_ops: list[str]) -> str:
        local = self.simulate(local_ops)
        remote = self.simulate(remote_ops)
        ratio = remote["total_ns"] / local["total_ns"] if local["total_ns"] > 0 else float("inf")
        lines = [
            "Architecture Comparison",
            "=" * 52,
            f"  Local request:  {self._human_ns(local['total_ns']):>12s}",
            f"  Remote request: {self._human_ns(remote['total_ns']):>12s}",
            f"  Remote / Local: {ratio:,.0f}× slower",
        ]
        return "\n".join(lines)

    @staticmethod
    def _human_ns(ns: float) -> str:
        if ns >= 1_000_000:
            return f"{ns / 1_000_000:,.1f} ms"
        elif ns >= 1_000:
            return f"{ns / 1_000:,.1f} μs"
        else:
            return f"{ns:,.0f} ns"


class FailureSimulator:
    def __init__(self, n: int, p: float):
        self.n = n
        self.p = p

    def p_at_least_k(self, k: int) -> float:
        return sum(
            math.comb(self.n, i) * (self.p ** i) * ((1 - self.p) ** (self.n - i))
            for i in range(k, self.n + 1)
        )

    def p_exactly_k(self, k: int) -> float:
        return math.comb(self.n, k) * (self.p ** k) * ((1 - self.p) ** (self.n - k))

    def p_quorum_lost(self, quorum: int | None = None) -> float:
        if quorum is None:
            quorum = self.n // 2 + 1
        return self.p_at_least_k(self.n - quorum + 1)

    def report(self) -> str:
        lines = [
            f"Failure Analysis: N={self.n} nodes, p={self.p}",
            "=" * 52,
            f"  P(single node fails)       = {self.p:.4f}",
            f"  P(no failures)             = {(1 - self.p) ** self.n:.6f}",
            f"  P(at least 1 failure)      = {self.p_at_least_k(1):.6f}",
            f"  P(at least 2 failures)     = {self.p_at_least_k(2):.6f}",
            f"  P(at least 3 failures)     = {self.p_at_least_k(3):.6f}",
        ]
        quorum = self.n // 2 + 1
        lines.append(f"  P(quorum lost, need {quorum})  = {self.p_quorum_lost(quorum):.6f}")
        return "\n".join(lines)


class NetworkPartition:
    def __init__(self, group_a: list[str], group_b: list[str]):
        self.group_a = group_a
        self.group_b = group_b
        self.partitions: list[dict] = []

    def simulate_partition(self, name: str = "partition"):
        writes_a = {node: f"write_{i}" for i, node in enumerate(self.group_a)}
        writes_b = {node: f"write_{i}" for i, node in enumerate(self.group_b)}
        self.partitions.append({
            "name": name,
            "writes_a": writes_a,
            "writes_b": writes_b,
            "conflict_nodes": [n for n in self.group_a[:1] if n in self.group_b[:1]],
        })

    def report(self) -> str:
        lines = [
            "Network Partition / Split-Brain Simulation",
            "=" * 52,
            f"  Group A: {', '.join(self.group_a)}",
            f"  Group B: {', '.join(self.group_b)}",
        ]
        if not self.partitions:
            lines.append("  (No partitions simulated yet.)")
            return "\n".join(lines)

        for part in self.partitions:
            lines.append(f"\n  --- {part['name']} ---")
            lines.append(f"  Group A writes: {part['writes_a']}")
            lines.append(f"  Group B writes: {part['writes_b']}")
            lines.append(f"  Split-brain: both groups accepted writes independently.")
            lines.append(f"  On heal: must reconcile conflicting state.")
        return "\n".join(lines)


def demo_latency_comparison():
    numbers = LatencyNumbers()
    numbers.display()
    sim = LatencySimulator(numbers)

    monolith_ops = ["l1", "l2", "ram", "ssd_read_4kb"]
    microservice_ops = monolith_ops + ["network_dc"]
    distributed_ops = microservice_ops + ["network_region"]

    monolith = sim.simulate(monolith_ops)
    microservice = sim.simulate(microservice_ops)
    distributed = sim.simulate(distributed_ops)

    print("Request Costs by Architecture")
    print("=" * 52)
    print("Monolith (local only):")
    print(sim.format_result(monolith))
    print()
    print("Microservice (local + 1 DC hop):")
    print(sim.format_result(microservice))
    print()
    print("Distributed (local + DC + region hop):")
    print(sim.format_result(distributed))
    print()

    print("Summary Ratios:")
    mono_ns = monolith["total_ns"]
    micro_ns = microservice["total_ns"]
    dist_ns = distributed["total_ns"]
    print(f"  Microservice / Monolith = {micro_ns / mono_ns:,.1f}×")
    print(f"  Distributed / Monolith  = {dist_ns / mono_ns:,.1f}×")
    print()


def demo_failure():
    scenarios = [
        (10, 0.01),
        (100, 0.01),
        (5, 0.10),
        (100, 0.05),
    ]
    for n, p in scenarios:
        sim = FailureSimulator(n, p)
        print(sim.report())
        print()


def demo_partition():
    partition = NetworkPartition(
        group_a=["node1", "node2", "node3"],
        group_b=["node4", "node5", "node6"],
    )
    partition.simulate_partition("inter-DC link failure")
    print(partition.report())
    print()


def demo_idempotence():
    print("Idempotence Demonstration")
    print("=" * 52)
    balance = 100
    print(f"  Starting balance: ${balance}")
    print()
    print("  ADD 10 (not idempotent):")
    for attempt in range(1, 4):
        balance += 10
        print(f"    Attempt {attempt}: balance = ${balance}")
    print(f"  ↑ Retrying added ${30 - 100 + 100 - 10} extra!")
    print()
    balance = 100
    print("  SET balance = 110 (idempotent):")
    for attempt in range(1, 4):
        balance = 110
        print(f"    Attempt {attempt}: balance = ${balance}")
    print("  ↑ Retrying is safe — same result every time.")
    print()
    balance = 100
    version = 0
    print("  CAS: SET balance = 110 IF version = 0 (idempotent):")
    for attempt in range(1, 4):
        if version == 0:
            balance = 110
            version = 1
            print(f"    Attempt {attempt}: applied, balance = ${balance}, version = {version}")
        else:
            print(f"    Attempt {attempt}: no-op (version already {version}), balance = ${balance}")
    print("  ↑ Conditional write: first succeeds, retries are no-ops.")
    print()


if __name__ == "__main__":
    print("What Distribution Costs You — Simulator")
    print("=" * 52)
    print()
    demo_latency_comparison()
    demo_failure()
    demo_partition()
    demo_idempotence()