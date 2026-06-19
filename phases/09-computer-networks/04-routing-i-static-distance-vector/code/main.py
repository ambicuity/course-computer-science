"""
Routing I — Static, Distance Vector
Phase 09 — Computer Networks

Distance-vector routing simulator with split horizon support.
Demonstrates Bellman-Ford convergence, count-to-infinity problem,
and loop-prevention techniques.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List, Optional

INF = 999  # represents infinity


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------

@dataclass
class Router:
    """A single router in the network."""
    rid: str
    neighbors: Dict[str, int] = field(default_factory=dict)   # neighbor_id -> link cost
    dv: Dict[str, int] = field(default_factory=dict)           # destination -> cost
    next_hop: Dict[str, str] = field(default_factory=dict)     # destination -> next_hop

    def __post_init__(self):
        self.dv[self.rid] = 0
        self.next_hop[self.rid] = self.rid

    def add_static_route(self, dest: str, next_hop_id: str, cost: int = 1):
        """Manually configure a static route."""
        self.dv[dest] = cost
        self.next_hop[dest] = next_hop_id

    def remove_route(self, dest: str):
        """Remove a route entry."""
        if dest != self.rid:
            self.dv.pop(dest, None)
            self.next_hop.pop(dest, None)

    def set_unreachable(self, dest: str):
        """Mark a destination as unreachable."""
        if dest != self.rid:
            self.dv[dest] = INF
            self.next_hop[dest] = None


@dataclass
class Network:
    """Collection of routers with links."""
    routers: Dict[str, Router] = field(default_factory=dict)
    split_horizon: bool = False

    def add_router(self, rid: str) -> Router:
        if rid in self.routers:
            return self.routers[rid]
        r = Router(rid=rid)
        self.routers[rid] = r
        return r

    def add_link(self, a: str, b: str, cost: int = 1):
        self.routers[a].neighbors[b] = cost
        self.routers[b].neighbors[a] = cost

    def remove_link(self, a: str, b: str):
        self.routers[a].neighbors.pop(b, None)
        self.routers[b].neighbors.pop(a, None)

    def all_destinations(self) -> List[str]:
        return sorted(self.routers.keys())


# ---------------------------------------------------------------------------
# Static routing helper
# ---------------------------------------------------------------------------

def static_route(router: Router, dest: str, next_hop: str, cost: int = 1):
    """Add a static route to a router."""
    router.add_static_route(dest, next_hop, cost)


# ---------------------------------------------------------------------------
# Distance vector algorithm
# ---------------------------------------------------------------------------

def distance_vector_update(router: Router, network: Network) -> bool:
    """Run one round of Bellman-Ford relaxation. Returns True if table changed."""
    changed = False
    destinations = network.all_destinations()

    for dest in destinations:
        if dest == router.rid:
            continue

        best_cost = router.dv.get(dest, INF)
        best_next = router.next_hop.get(dest, None)

        for nbr_id, link_cost in router.neighbors.items():
            nbr = network.routers[nbr_id]
            nbr_cost = nbr.dv.get(dest, INF)

            # Split horizon: skip routes learned from this neighbor
            if network.split_horizon and best_next == nbr_id and nbr_cost == INF:
                # Poison reverse: neighbor advertises infinity back
                continue

            candidate = link_cost + nbr_cost
            if candidate < best_cost:
                best_cost = candidate
                best_next = nbr_id

        old_cost = router.dv.get(dest)
        old_next = router.next_hop.get(dest)

        if best_cost < INF:
            if old_cost != best_cost or old_next != best_next:
                router.dv[dest] = best_cost
                router.next_hop[dest] = best_next
                changed = True
        elif dest in router.dv and router.dv[dest] < INF:
            # Route became unreachable
            router.dv[dest] = INF
            router.next_hop[dest] = None
            changed = True

    return changed


# ---------------------------------------------------------------------------
# Simulation
# ---------------------------------------------------------------------------

def simulate(network: Network, max_rounds: int = 20, verbose: bool = True) -> int:
    """Run DV protocol. Returns number of rounds to converge (-1 if not converged)."""
    for rnd in range(max_rounds):
        any_changed = False
        for r in network.routers.values():
            if distance_vector_update(r, network):
                any_changed = True

        if verbose:
            print(f"--- Round {rnd + 1} ---")
            print_tables(network)

        if not any_changed:
            if verbose:
                print(f"[Converged after {rnd + 1} round(s)]\n")
            return rnd + 1

    if verbose:
        print(f"[Did not converge within {max_rounds} rounds]\n")
    return -1


def print_tables(network: Network):
    for rid in network.all_destinations():
        r = network.routers[rid]
        lines = []
        for dest in sorted(r.dv):
            cost = r.dv[dest]
            nxt = r.next_hop[dest]
            cost_str = "INF" if cost >= INF else str(cost)
            nxt_str = nxt if nxt else "-"
            lines.append(f"    {dest}  cost={cost_str:>3}  via={nxt_str}")
        print(f"  Router {rid}:")
        for line in lines:
            print(line)
    print()


# ---------------------------------------------------------------------------
# Demos
# ---------------------------------------------------------------------------

def demo_static_routes():
    """Demonstrate static route configuration."""
    print("=" * 60)
    print("DEMO 1: Static routing")
    print("=" * 60)
    net = Network()
    for r in ["R1", "R2", "R3"]:
        net.add_router(r)
    net.add_link("R1", "R2", 1)
    net.add_link("R2", "R3", 1)

    # Manually configure static routes
    static_route(net.routers["R1"], "10.0.3.0", "R2", cost=2)
    static_route(net.routers["R2"], "10.0.3.0", "R3", cost=1)
    static_route(net.routers["R2"], "10.0.1.0", "R1", cost=1)

    for rid in ["R1", "R2", "R3"]:
        r = net.routers[rid]
        print(f"  Router {rid} static routes:")
        for dest, cost in r.dv.items():
            print(f"    {dest} -> cost {cost}")
    print()


def demo_basic_dv():
    """Demonstrate basic DV convergence."""
    print("=" * 60)
    print("DEMO 2: Distance-vector convergence (4 nodes)")
    print("=" * 60)
    net = Network()
    for r in ["A", "B", "C", "D"]:
        net.add_router(r)
    net.add_link("A", "B", 1)
    net.add_link("B", "C", 1)
    net.add_link("C", "D", 2)
    net.add_link("A", "D", 3)
    simulate(net)


def demo_count_to_infinity():
    """Demonstrate count-to-infinity problem."""
    print("=" * 60)
    print("DEMO 3: Count-to-infinity (NO split horizon)")
    print("=" * 60)
    net = Network(split_horizon=False)
    for r in ["A", "B", "C"]:
        net.add_router(r)
    net.add_link("A", "B", 1)
    net.add_link("B", "C", 1)
    simulate(net, max_rounds=10)

    print(">>> Breaking link B-C...")
    net.remove_link("B", "C")
    simulate(net, max_rounds=10)


def demo_split_horizon():
    """Demonstrate split horizon preventing loops."""
    print("=" * 60)
    print("DEMO 4: Split horizon prevents count-to-infinity")
    print("=" * 60)
    net = Network(split_horizon=True)
    for r in ["A", "B", "C"]:
        net.add_router(r)
    net.add_link("A", "B", 1)
    net.add_link("B", "C", 1)
    simulate(net, max_rounds=10)

    print(">>> Breaking link B-C...")
    net.remove_link("B", "C")
    simulate(net, max_rounds=10)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    demo_static_routes()
    demo_basic_dv()
    demo_count_to_infinity()
    demo_split_horizon()


if __name__ == "__main__":
    main()
