"""
Routing II — Link State (OSPF), BGP
Phase 09 — Computer Networks

Link-state routing simulator with Dijkstra SPF computation
and basic BGP path-vector route selection.
"""

from __future__ import annotations

import heapq
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple

INF = float("inf")


# ---------------------------------------------------------------------------
# Link-State Router
# ---------------------------------------------------------------------------

@dataclass
class LinkStateRouter:
    """A router that maintains a link-state database and runs Dijkstra."""
    rid: str
    neighbors: Dict[str, int] = field(default_factory=dict)   # direct neighbor -> link cost
    lsdb: Dict[str, Dict[str, int]] = field(default_factory=dict)  # router_id -> {neighbor: cost}
    spf_table: Dict[str, Tuple[float, Optional[str]]] = field(default_factory=dict)  # dest -> (cost, next_hop)

    def __post_init__(self):
        self.lsdb[self.rid] = dict(self.neighbors)


@dataclass
class LinkStateNetwork:
    """A network of link-state routers."""
    routers: Dict[str, LinkStateRouter] = field(default_factory=dict)

    def add_router(self, rid: str) -> LinkStateRouter:
        if rid in self.routers:
            return self.routers[rid]
        r = LinkStateRouter(rid=rid)
        self.routers[rid] = r
        return r

    def add_link(self, a: str, b: str, cost: int = 1):
        self.routers[a].neighbors[b] = cost
        self.routers[b].neighbors[a] = cost
        self.routers[a].lsdb[a] = dict(self.routers[a].neighbors)
        self.routers[b].lsdb[b] = dict(self.routers[b].neighbors)

    def remove_link(self, a: str, b: str):
        self.routers[a].neighbors.pop(b, None)
        self.routers[b].neighbors.pop(a, None)
        self.routers[a].lsdb[a] = dict(self.routers[a].neighbors)
        self.routers[b].lsdb[b] = dict(self.routers[b].neighbors)

    def all_rids(self) -> List[str]:
        return sorted(self.routers.keys())


# ---------------------------------------------------------------------------
# LSA Flooding
# ---------------------------------------------------------------------------

def flood_lsa(source: str, network: LinkStateNetwork):
    """Flood source router's link-state info to all routers."""
    source_router = network.routers[source]
    lsa = dict(source_router.neighbors)
    for rid, router in network.routers.items():
        if rid != source:
            router.lsdb[source] = dict(lsa)


def flood_all(network: LinkStateNetwork):
    """Flood LSAs from every router (initial convergence)."""
    for rid in network.all_rids():
        flood_lsa(rid, network)


# ---------------------------------------------------------------------------
# Dijkstra SPF
# ---------------------------------------------------------------------------

def compute_spf(router: LinkStateRouter) -> Dict[str, Tuple[float, Optional[str]]]:
    """Run Dijkstra's algorithm from this router using its LSDB.

    Returns: {destination: (cost, next_hop)}
    """
    dist: Dict[str, float] = {router.rid: 0}
    prev: Dict[str, str] = {}
    visited: set = set()
    heap: List[Tuple[float, str]] = [(0, router.rid)]

    while heap:
        cost, u = heapq.heappop(heap)
        if u in visited:
            continue
        visited.add(u)

        neighbors = router.lsdb.get(u, {})
        for v, link_cost in neighbors.items():
            if v in visited:
                continue
            new_cost = cost + link_cost
            if new_cost < dist.get(v, INF):
                dist[v] = new_cost
                prev[v] = u
                heapq.heappush(heap, (new_cost, v))

    # Build forwarding table
    result: Dict[str, Tuple[float, Optional[str]]] = {}
    for dest in sorted(dist.keys()):
        if dest == router.rid:
            continue
        # Trace back from dest to find the first hop from router.rid
        node = dest
        while prev.get(node) != router.rid and node in prev:
            node = prev[node]
        next_hop = node if node != dest and prev.get(node) == router.rid else (
            node if dest in prev else None
        )
        # Handle direct neighbor case
        if dest in prev and prev[dest] == router.rid:
            next_hop = dest
        result[dest] = (dist[dest], next_hop)

    router.spf_table = result
    return result


# ---------------------------------------------------------------------------
# Link-state simulation
# ---------------------------------------------------------------------------

def run_ls(network: LinkStateNetwork, verbose: bool = True):
    """Flood LSAs and run SPF on all routers."""
    flood_all(network)
    for rid in network.all_rids():
        compute_spf(network.routers[rid])
    if verbose:
        print_ls_tables(network)


def print_ls_tables(network: LinkStateNetwork):
    for rid in network.all_rids():
        r = network.routers[rid]
        print(f"  Router {rid} SPF table:")
        for dest in sorted(r.spf_table):
            cost, nh = r.spf_table[dest]
            cost_str = "INF" if cost >= INF else str(cost)
            print(f"    -> {dest}: cost={cost_str}, next_hop={nh}")
    print()


# ---------------------------------------------------------------------------
# BGP Router
# ---------------------------------------------------------------------------

@dataclass
class BGPRoute:
    """A BGP route advertisement."""
    prefix: str
    as_path: List[int]
    next_hop: str
    local_pref: int = 100
    med: int = 0

    @property
    def as_path_length(self) -> int:
        return len(self.as_path)

    def __repr__(self):
        path_str = " -> ".join(str(a) for a in self.as_path)
        return f"BGPRoute({self.prefix}, path=[{path_str}], via={self.next_hop}, lp={self.local_pref})"


@dataclass
class BGPRouter:
    """A BGP-speaking router (simulates one AS)."""
    asn: int
    prefixes: List[str] = field(default_factory=list)
    rib: Dict[str, List[BGPRoute]] = field(default_factory=dict)  # prefix -> routes
    best_routes: Dict[str, BGPRoute] = field(default_factory=dict)

    def receive_route(self, route: BGPRoute):
        """Process an incoming BGP route."""
        prefix = route.prefix
        if prefix not in self.rib:
            self.rib[prefix] = []
        # Prepend own AS to path
        new_path = [self.asn] + route.as_path
        new_route = BGPRoute(
            prefix=prefix,
            as_path=new_path,
            next_hop=route.next_hop,
            local_pref=route.local_pref,
            med=route.med,
        )
        self.rib[prefix].append(new_route)

    def select_best(self):
        """BGP best-path selection: highest local_pref, then shortest AS path."""
        for prefix, routes in self.rib.items():
            if routes:
                self.best_routes[prefix] = min(
                    routes,
                    key=lambda r: (-r.local_pref, r.as_path_length, r.med),
                )


def bgp_advertise(router: BGPRouter) -> List[BGPRoute]:
    """Generate BGP advertisements for this router's best routes."""
    routes = []
    for prefix, route in router.best_routes.items():
        advertised = BGPRoute(
            prefix=prefix,
            as_path=[router.asn] + route.as_path,
            next_hop=route.next_hop,
            local_pref=route.local_pref,
            med=route.med,
        )
        routes.append(advertised)
    # Also advertise directly owned prefixes
    for pfx in router.prefixes:
        if pfx not in router.best_routes:
            routes.append(BGPRoute(prefix=pfx, as_path=[router.asn], next_hop="self"))
    return routes


# ---------------------------------------------------------------------------
# Demos
# ---------------------------------------------------------------------------

def demo_ls_convergence():
    """Demonstrate link-state routing convergence."""
    print("=" * 60)
    print("DEMO 1: Link-state routing (5 nodes)")
    print("=" * 60)
    net = LinkStateNetwork()
    for r in ["R1", "R2", "R3", "R4", "R5"]:
        net.add_router(r)
    net.add_link("R1", "R2", 2)
    net.add_link("R1", "R3", 5)
    net.add_link("R2", "R3", 1)
    net.add_link("R2", "R4", 3)
    net.add_link("R3", "R5", 2)
    net.add_link("R4", "R5", 1)

    print("Initial topology:")
    run_ls(net)

    print(">>> Removing link R2-R4...")
    net.remove_link("R2", "R4")
    run_ls(net)


def demo_ls_vs_dv_convergence():
    """Compare LS convergence (one round after flood) vs DV (multiple rounds)."""
    print("=" * 60)
    print("DEMO 2: Link-state convergence speed")
    print("=" * 60)
    print("Link-state: flood all LSAs (one round), then each router runs Dijkstra locally.")
    print("Result: immediate optimal paths after one flood round.\n")
    print("Distance vector: each round relaxes one hop. For a network with diameter D,")
    print("DV needs at least D rounds to converge. LS needs 1 flood round + local compute.\n")

    net = LinkStateNetwork()
    for r in ["A", "B", "C", "D"]:
        net.add_router(r)
    net.add_link("A", "B", 1)
    net.add_link("B", "C", 1)
    net.add_link("C", "D", 1)

    run_ls(net)
    print("Network diameter = 3. LS converged in 1 flood round.\n")


def demo_bgp():
    """Demonstrate BGP path-vector route selection."""
    print("=" * 60)
    print("DEMO 3: BGP path-vector route selection")
    print("=" * 60)

    as1 = BGPRouter(asn=100, prefixes=["10.0.0.0/8"])
    as2 = BGPRouter(asn=200)
    as3 = BGPRouter(asn=300)
    as4 = BGPRouter(asn=400)

    # AS 100 originates 10.0.0.0/8, advertises to AS 200 and AS 300
    origin_route = BGPRoute(prefix="10.0.0.0/8", as_path=[100], next_hop="AS100")

    # AS 200 receives from AS 100, re-advertises to AS 400
    as2.receive_route(origin_route)
    as2.select_best()

    # AS 300 receives from AS 100, re-advertises to AS 400
    as3.receive_route(origin_route)
    as3.select_best()

    # AS 400 receives from both AS 200 and AS 300
    for route in bgp_advertise(as2):
        as4.receive_route(route)
    for route in bgp_advertise(as3):
        as4.receive_route(route)
    as4.select_best()

    print("  AS400's routes to 10.0.0.0/8:")
    for route in as4.rib.get("10.0.0.0/8", []):
        path_str = " -> ".join(str(a) for a in route.as_path)
        print(f"    path=[{path_str}], local_pref={route.local_pref}")

    best = as4.best_routes.get("10.0.0.0/8")
    if best:
        path_str = " -> ".join(str(a) for a in best.as_path)
        print(f"  Best route: path=[{path_str}] (AS path length = {best.as_path_length})")

    # Now add a shorter path via AS 300 with higher local_pref
    print("\n  >>> Setting local_pref=200 on AS300 path...")
    as4.rib["10.0.0.0/8"] = []
    route_via_200 = BGPRoute(prefix="10.0.0.0/8", as_path=[200, 100], next_hop="AS200", local_pref=100)
    route_via_300 = BGPRoute(prefix="10.0.0.0/8", as_path=[300, 100], next_hop="AS300", local_pref=200)
    as4.rib["10.0.0.0/8"] = [route_via_200, route_via_300]
    as4.select_best()

    best = as4.best_routes.get("10.0.0.0/8")
    if best:
        path_str = " -> ".join(str(a) for a in best.as_path)
        print(f"  Best route: path=[{path_str}] (local_pref={best.local_pref})")
    print()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    demo_ls_convergence()
    demo_ls_vs_dv_convergence()
    demo_bgp()


if __name__ == "__main__":
    main()
