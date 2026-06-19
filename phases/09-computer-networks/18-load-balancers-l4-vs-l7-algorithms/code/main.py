"""Load Balancer Simulation — L4 vs L7, Algorithms."""

import hashlib
import random
from collections import Counter
from dataclasses import dataclass, field
from typing import List, Optional, Tuple


@dataclass
class Backend:
    addr: str
    weight: int = 1
    active_connections: int = 0
    healthy: bool = True


class LoadBalancer:
    def __init__(self, backends: List[Backend], algorithm: str = "round_robin"):
        self.backends = backends
        self.algorithm = algorithm
        self._rr_index = 0
        self._wrr_state = [0] * len(backends)
        self._wrr_total = sum(b.weight for b in backends)
        self._consistent_ring = self._build_ring()

    def route(self, client_ip: str) -> Optional[Backend]:
        healthy = [b for b in self.backends if b.healthy]
        if not healthy:
            return None

        if self.algorithm == "round_robin":
            return self._round_robin(healthy)
        elif self.algorithm == "weighted_round_robin":
            return self._weighted_round_robin(healthy)
        elif self.algorithm == "least_connections":
            return min(healthy, key=lambda b: b.active_connections)
        elif self.algorithm == "ip_hash":
            idx = int(hashlib.md5(client_ip.encode()).hexdigest(), 16) % len(healthy)
            return healthy[idx]
        elif self.algorithm == "consistent_hash":
            return self._consistent_hash(client_ip, healthy)
        elif self.algorithm == "random_two_choices":
            a, b = random.sample(healthy, min(2, len(healthy)))
            return a if a.active_connections <= b.active_connections else b

    def _round_robin(self, healthy: List[Backend]) -> Backend:
        for _ in range(len(self.backends)):
            idx = self._rr_index % len(self.backends)
            self._rr_index += 1
            if self.backends[idx].healthy:
                return self.backends[idx]

    def _weighted_round_robin(self, healthy: List[Backend]) -> Backend:
        while True:
            for i, b in enumerate(self.backends):
                if not b.healthy:
                    continue
                self._wrr_state[i] += b.weight
                if self._wrr_state[i] >= self._wrr_total:
                    self._wrr_state[i] -= self._wrr_total
                    return b

    def _consistent_hash(self, client_ip: str, healthy: List[Backend]) -> Backend:
        h = int(hashlib.md5(client_ip.encode()).hexdigest(), 16)
        for ring_hash, idx in self._consistent_ring:
            if h <= ring_hash and self.backends[idx].healthy:
                return self.backends[idx]
        for _, idx in self._consistent_ring:
            if self.backends[idx].healthy:
                return self.backends[idx]

    def _build_ring(self) -> List[Tuple[int, int]]:
        ring = []
        for i, b in enumerate(self.backends):
            for vnode in range(150):
                key = f"{b.addr}:{vnode}"
                h = int(hashlib.md5(key.encode()).hexdigest(), 16)
                ring.append((h, i))
        ring.sort()
        return ring

    def simulate_requests(self, n: int, skew: bool = False) -> Counter:
        counts = Counter()
        for i in range(n):
            if skew:
                if random.random() < 0.8:
                    client = f"10.0.0.{random.randint(1, 20)}"
                else:
                    client = f"10.0.0.{random.randint(21, 200)}"
            else:
                client = f"10.0.0.{random.randint(1, 200)}"

            backend = self.route(client)
            if backend:
                counts[backend.addr] += 1
        return counts


def main() -> None:
    print("=" * 60)
    print("Load Balancers — L4 vs L7, Algorithms")
    print("=" * 60)

    def make_backends():
        return [
            Backend("10.0.0.1:8080"),
            Backend("10.0.0.2:8080"),
            Backend("10.0.0.3:8080"),
            Backend("10.0.0.4:8080"),
        ]

    algorithms = [
        "round_robin",
        "weighted_round_robin",
        "least_connections",
        "ip_hash",
        "consistent_hash",
        "random_two_choices",
    ]

    for algo in algorithms:
        backends = make_backends()
        lb = LoadBalancer(backends, algorithm=algo)
        counts = lb.simulate_requests(1000)

        print(f"\n--- {algo} (uniform traffic) ---")
        total = sum(counts.values())
        for addr in sorted(counts.keys()):
            pct = counts[addr] / total * 100
            print(f"  {addr}: {counts[addr]:>4} ({pct:.1f}%)")

    # Skewed traffic
    print("\n--- Round Robin with 80/20 skewed traffic ---")
    backends = make_backends()
    lb = LoadBalancer(backends, algorithm="round_robin")
    counts = lb.simulate_requests(1000, skew=True)
    total = sum(counts.values())
    for addr in sorted(counts.keys()):
        pct = counts[addr] / total * 100
        print(f"  {addr}: {counts[addr]:>4} ({pct:.1f}%)")

    # Health check failure
    print("\n--- Health Check Failure Simulation ---")
    backends = make_backends()
    backends[1].healthy = False
    lb = LoadBalancer(backends, algorithm="round_robin")
    counts = lb.simulate_requests(100)
    print(f"  Backend 2 marked DOWN. Distribution over 100 requests:")
    for addr in sorted(counts.keys()):
        print(f"  {addr}: {counts[addr]}")

    # Weighted round-robin
    print("\n--- Weighted Round Robin (weights: 5, 3, 2, 1) ---")
    backends = [
        Backend("10.0.0.1:8080", weight=5),
        Backend("10.0.0.2:8080", weight=3),
        Backend("10.0.0.3:8080", weight=2),
        Backend("10.0.0.4:8080", weight=1),
    ]
    lb = LoadBalancer(backends, algorithm="weighted_round_robin")
    counts = lb.simulate_requests(1100)
    total = sum(counts.values())
    for addr in sorted(counts.keys()):
        pct = counts[addr] / total * 100
        print(f"  {addr}: {counts[addr]:>4} ({pct:.1f}%)")


if __name__ == "__main__":
    main()
