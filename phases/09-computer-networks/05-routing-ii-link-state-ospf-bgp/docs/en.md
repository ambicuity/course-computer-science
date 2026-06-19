# Routing II — Link State (OSPF), BGP

> Every router knows the full map — Dijkstra finds the shortest path — and BGP stitches the Internet together.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 09 lessons 01–04
**Time:** ~75 minutes

## Learning Objectives

- Explain how link-state routing differs from distance vector and why it converges faster.
- Implement Dijkstra's algorithm on a link-state database.
- Describe OSPF areas, LSA flooding, and SPF computation.
- Explain BGP's role as an inter-AS path-vector protocol and its policy-driven route selection.

## The Problem

Distance-vector protocols have a fundamental limitation: each router only knows what its neighbors tell it, so it cannot distinguish between a true shortest path and a loop. Link-state routing solves this by giving every router the **complete network topology**. With the full map, each router independently computes shortest paths — no loops, faster convergence.

Between autonomous systems (ASes), routing isn't about shortest paths — it's about **policy**. BGP handles inter-AS routing where commercial agreements, peering relationships, and political boundaries matter more than link cost.

## The Concept

### Link-State Routing

Each router:

1. **Discovers neighbors** and measures the cost of each link.
2. **Floods a Link-State Advertisement (LSA)** to every router in the network — describing its directly connected links and their costs.
3. **Builds a complete topology graph** from received LSAs.
4. **Runs Dijkstra's algorithm** (Shortest Path First) to compute the shortest path to every other router.

**Why it's better than DV:**
- No count-to-infinity: every router has the full picture, so there's no false information to propagate.
- Faster convergence: once all LSAs arrive, computation is local and instant.
- **Trade-off:** more memory (stores the full topology) and more CPU (runs Dijkstra).

**Dijkstra's algorithm recap:**

```
Initialize: dist[self] = 0, dist[all others] = INF, unvisited = all nodes
While unvisited is not empty:
    Pick node u in unvisited with smallest dist[u]
    Remove u from unvisited
    For each neighbor v of u with link cost c:
        if dist[u] + c < dist[v]:
            dist[v] = dist[u] + c
            prev[v] = u
```

### OSPF — Open Shortest Path First

OSPF is the standard link-state Interior Gateway Protocol (IGP). Key features:

- **Areas** for hierarchy: area 0 (backbone) connects all other areas. Reduces LSA flooding scope.
- **LSA types**: Router LSA, Network LSA, Summary LSA, External LSA — each carries different topology information.
- **SPF computation**: Each router runs Dijkstra on its link-state database (LSDB) to build a shortest-path tree rooted at itself.
- **Convergence**: Typically sub-second on modern hardware. Triggered updates when topology changes.

### BGP — Border Gateway Protocol

BGP is a **path-vector** Exterior Gateway Protocol (EGP). Each BGP route includes an **AS path** — the sequence of AS numbers the route has traversed.

Route selection prefers shortest AS path (fewest AS hops), but real selection is driven by **policy**:
- **Peering**: two ISPs exchange customer routes for free.
- **Transit**: one ISP pays another to carry its traffic.
- Communities, local preference, MED attributes let operators encode business relationships.

## Build It

### Step 1: Link-State Router

```python
import heapq
from dataclasses import dataclass, field
from typing import Dict, Set, Tuple, List

INF = float("inf")


@dataclass
class LinkStateRouter:
    rid: str
    neighbors: Dict[str, int] = field(default_factory=dict)   # direct neighbors + link cost
    lsdb: Dict[str, Dict[str, int]] = field(default_factory=dict)  # router_id -> {neighbor: cost}
    spf_table: Dict[str, Tuple[int, str]] = field(default_factory=dict)  # dest -> (cost, next_hop)

    def __post_init__(self):
        self.lsdb[self.rid] = dict(self.neighbors)
```

### Step 2: LSA Flooding

```python
def flood_lsa(source: str, routers: Dict[str, "LinkStateRouter"]):
    """Flood source router's LSA to all other routers."""
    lsa = routers[source].neighbors.copy()
    for rid, router in routers.items():
        if rid != source:
            router.lsdb[source] = dict(lsa)
```

### Step 3: Dijkstra's SPF

```python
def compute_spf(router: "LinkStateRouter") -> Dict[str, Tuple[float, str]]:
    """Run Dijkstra from this router using its link-state database."""
    dist = {router.rid: 0}
    prev = {}
    visited = set()
    heap = [(0, router.rid)]

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

    # Build forwarding table: for each dest, find first hop
    result = {}
    for dest in dist:
        if dest == router.rid:
            continue
        # Trace back to find next hop
        node = dest
        while prev.get(node) != router.rid and node in prev:
            node = prev[node]
        next_hop = node if node != dest else None
        result[dest] = (dist[dest], next_hop)
    router.spf_table = result
    return result
```

### Step 4: BGP Route Selection

```python
@dataclass
class BGPRoute:
    prefix: str
    as_path: List[int]
    next_hop: str
    local_pref: int = 100

    @property
    def as_path_length(self):
        return len(self.as_path)


def bgp_select_best(routes: List[BGPRoute]) -> BGPRoute:
    """Select best BGP route: highest local_pref, then shortest AS path."""
    return min(routes, key=lambda r: (-r.local_pref, r.as_path_length))
```

## Use It

In production:
- **OSPF** (RFC 2328): Linux's FRRouting (`ospfd/ospf_spf.c`) runs Dijkstra on the LSDB. Look at `ospf_spf_calculate()` for the core SPF loop.
- **BGP** (RFC 4271): FRRouting's `bgpd/bgp_route.c` — `bgp_best_selection()` implements the full BGP decision process (local_pref > AS path length > MED > IGP cost > router ID tiebreaker).

OSPF convergence is typically under 1 second; BGP convergence can take minutes due to policy processing and route dampening.

## Read the Source

- `frr/ospfd/ospf_spf.c` — OSPF's Dijkstra implementation with LSA handling.
- `frr/bgpd/bgp_route.c` — BGP route selection and best-path computation.
- Linux kernel `net/ipv4/fib_trie.c` — The forwarding information base (FIB) where computed routes end up.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A link-state routing simulator with Dijkstra SPF computation** you can use to model network topologies.

## Exercises

1. **Easy** — Build a 5-node topology and verify that the SPF table matches your manual Dijkstra trace.
2. **Medium** — Add a `link_down(a, b)` function that removes the link, re-floods LSAs, and re-runs SPF. Compare the new routing tables with the old ones.
3. **Hard** — Implement OSPF area support: partition the topology into two areas connected by a backbone (area 0). ABRs (area border routers) generate summary LSAs. Verify that inter-area traffic always transits the backbone.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Link state | "Every router knows the whole map" | Each router floods its link costs; all routers build identical topology maps |
| LSA | "Link-state advertisement" | A message describing a router's directly connected links and their costs |
| SPF | "Shortest Path First" | Dijkstra's algorithm run on the LSDB to compute shortest paths |
| OSPF | "Open Shortest Path First" | The standard link-state IGP; uses areas for hierarchy |
| BGP | "Border Gateway Protocol" | Path-vector EGP for inter-AS routing driven by policy |
| AS path | "The list of ASes a route crossed" | Ordered sequence of autonomous system numbers a BGP route has traversed |
| Area 0 | "The backbone" | OSPF's mandatory transit area that connects all other areas |

## Further Reading

- RFC 2328 — OSPF Version 2
- RFC 4271 — BGP-4
- RFC 5340 — OSPF for IPv6
- Halabi, *Internet Routing Architectures* — practical BGP design
