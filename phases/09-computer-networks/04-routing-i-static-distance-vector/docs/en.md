# Routing I — Static, Distance Vector

> How routers decide where to send each packet — and why distributed algorithms sometimes disagree.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 09 lessons 01–03
**Time:** ~60 minutes

## Learning Objectives

- Explain the difference between static and dynamic routing and when each is appropriate.
- Implement the Bellman-Ford distance vector algorithm from scratch.
- Identify the count-to-infinity problem and explain how split horizon / poison reverse fix it.
- Run a working distance vector simulator and inspect routing tables at convergence.

## The Problem

A network with 5 routers and 8 links has 5! possible paths between any two nodes. Without a routing protocol, every router would need a manually configured entry for every possible destination on the network. When a link fails, someone has to update those entries by hand. This doesn't scale.

Routing protocols solve this automatically: each router builds a **routing table** (destination → next hop + cost) and updates it as the network changes.

## The Concept

### Static Routing

A **static route** is a manually configured entry in a router's forwarding table:

| Destination | Mask | Next Hop | Interface | Cost |
|-------------|------|----------|-----------|------|
| 10.0.1.0 | /24 | 192.168.1.2 | eth1 | 1 |
| 10.0.2.0 | /24 | 192.168.1.6 | eth2 | 1 |
| 0.0.0.0 | /0 | 192.168.1.1 | eth0 | 1 |

The **default route** (`0.0.0.0/0`) matches anything not covered by a more specific entry — the "if nothing else matches, send it here" rule.

Static routes work for small, stable networks. They break when links fail or new routers appear because no one updates them automatically.

### Distance Vector (Bellman-Ford)

Distance vector is a **distributed** algorithm. Each router:

1. Maintains a **distance vector** — the cost to reach every known destination.
2. Shares its distance vector with **direct neighbors** periodically.
3. On receiving a neighbor's vector, applies the **Bellman-Ford equation**:

```
D(x) = min over all neighbors v { cost(self, v) + Dv(x) }
```

Where `D(x)` is the best-known cost to destination `x`, and `Dv(x)` is neighbor `v`'s advertised cost to `x`.

**Worked example — 4-node network:**

```
A --1-- B --1-- C
|               |
3               2
|               |
+-------1-------D
```

- Round 0: A knows {A:0}, B knows {B:0}, C knows {C:0}, D knows {D:0}.
- Round 1: A receives B={B:0} → A knows {A:0, B:1}. D receives C={C:0} → D knows {D:0, C:2}.
- Round 2: A receives D={D:0,C:2} via the A–D link (cost 1) → A knows {A:0, B:1, D:1, C:3}.
- Eventually A discovers it can reach C via D with cost 3 (A→D→C = 1+2) or via B→C = 1+1 = 2, so it picks B.

### Count-to-Infinity

If link B–C breaks:
- Round 0: B knows C is unreachable (cost ∞). But D still advertises C:2.
- Round 1: B hears D advertise C:2, so B sets C:1+2=3 (through A→D→C... wait, B's neighbor A might still advertise).
- The problem: **false routing information propagates in loops**. Costs climb 1, 2, 3... until they reach the "infinity" threshold.

**Fixes:**
- **Split horizon**: Don't advertise a route back to the neighbor you learned it from.
- **Poison reverse**: Advertise the route back with cost ∞ (explicitly telling the neighbor "don't use me to reach this").

## Build It

### Step 1: Router and Network Data Structures

```python
from __future__ import annotations
from dataclasses import dataclass, field
from typing import Dict, Tuple

INF = 999  # represents infinity


@dataclass
class Router:
    rid: str
    neighbors: Dict[str, int] = field(default_factory=dict)  # neighbor_id -> link cost
    dv: Dict[str, int] = field(default_factory=dict)          # destination -> cost
    next_hop: Dict[str, str] = field(default_factory=dict)    # destination -> next_hop

    def __post_init__(self):
        self.dv[self.rid] = 0
        self.next_hop[self.rid] = self.rid


@dataclass
class Network:
    routers: Dict[str, Router] = field(default_factory=dict)
    split_horizon: bool = False

    def add_router(self, rid: str) -> Router:
        r = Router(rid=rid)
        self.routers[rid] = r
        return r

    def add_link(self, a: str, b: str, cost: int = 1):
        self.routers[a].neighbors[b] = cost
        self.routers[b].neighbors[a] = cost
```

### Step 2: Bellman-Ford Update

```python
def dv_update(router: Router, network: Network) -> bool:
    """Run one Bellman-Ford relaxation. Returns True if table changed."""
    changed = False
    for dest in list(network.routers.keys()):
        if dest == router.rid:
            continue
        best_cost = router.dv.get(dest, INF)
        best_next = router.next_hop.get(dest, None)
        for nbr_id, link_cost in router.neighbors.items():
            nbr = network.routers[nbr_id]
            nbr_cost = nbr.dv.get(dest, INF)
            if network.split_horizon and best_next == nbr_id:
                continue  # split horizon: don't use the path learned from this neighbor
            candidate = link_cost + nbr_cost
            if candidate < best_cost:
                best_cost = candidate
                best_next = nbr_id
        if best_cost != router.dv.get(dest, INF) or best_next != router.next_hop.get(dest):
            router.dv[dest] = best_cost
            router.next_hop[dest] = best_next
            changed = True
        elif dest not in router.dv and best_cost < INF:
            router.dv[dest] = best_cost
            router.next_hop[dest] = best_next
            changed = True
    return changed
```

### Step 3: Simulation Loop

```python
def simulate(network: Network, max_rounds: int = 20):
    for rnd in range(max_rounds):
        any_changed = False
        for r in network.routers.values():
            if dv_update(r, network):
                any_changed = True
        print(f"--- Round {rnd + 1} ---")
        print_tables(network)
        if not any_changed:
            print(f"Converged after {rnd + 1} rounds.\n")
            return
    print(f"Did not converge within {max_rounds} rounds.\n")


def print_tables(network: Network):
    for rid in sorted(network.routers):
        r = network.routers[rid]
        entries = []
        for dest in sorted(r.dv):
            entries.append(f"  {dest}: cost={r.dv[dest]}, next={r.next_hop[dest]}")
        print(f"Router {rid}:\n" + "\n".join(entries))
    print()
```

### Step 4: Demonstration

```python
def demo_basic():
    print("=" * 60)
    print("DEMO 1: Basic distance-vector convergence (4 nodes)")
    print("=" * 60)
    net = Network()
    for r in ["A", "B", "C", "D"]:
        net.add_router(r)
    net.add_link("A", "B", 1)
    net.add_link("B", "C", 1)
    net.add_link("C", "D", 2)
    net.add_link("A", "D", 3)
    simulate(net)


def demo_loop():
    print("=" * 60)
    print("DEMO 2: Count-to-infinity WITHOUT split horizon")
    print("=" * 60)
    net = Network(split_horizon=False)
    for r in ["A", "B", "C"]:
        net.add_router(r)
    net.add_link("A", "B", 1)
    net.add_link("B", "C", 1)
    simulate(net, max_rounds=10)
    # Now break the B-C link
    print(">>> Breaking link B-C...")
    del net.routers["B"].neighbors["C"]
    del net.routers["C"].neighbors["B"]
    simulate(net, max_rounds=10)


def demo_split_horizon():
    print("=" * 60)
    print("DEMO 3: Split horizon prevents count-to-infinity")
    print("=" * 60)
    net = Network(split_horizon=True)
    for r in ["A", "B", "C"]:
        net.add_router(r)
    net.add_link("A", "B", 1)
    net.add_link("B", "C", 1)
    simulate(net, max_rounds=10)
    print(">>> Breaking link B-C...")
    del net.routers["B"].neighbors["C"]
    del net.routers["C"].neighbors["B"]
    simulate(net, max_rounds=10)
```

## Use It

RIP (Routing Information Protocol, [RFC 2453](https://www.rfc-editor.org/rfc/rfc2453)) is the classic distance-vector protocol. Key differences from our simulator:

- RIP uses **hop count** (each link costs 1) rather than arbitrary link metrics.
- RIP sends updates every **30 seconds**; our simulator sends every round.
- RIP marks routes unreachable at **16 hops** (our `INF = 999` is more generous).
- RIP implements split horizon with poison reverse by default.

In practice, RIP is considered too slow to converge for modern networks. OSPF (next lesson) replaced it in most enterprise deployments.

## Read the Source

- `bird/proto/rip/rip.c` — The BIRD routing daemon's RIP implementation shows the real packet format and timer-driven update loop.
- `quagga/ripd/ripd.c` — Quagga/FRR's RIP daemon; look at `rip_update_process()` for the periodic advertisement logic.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A self-contained distance-vector routing simulator** you can embed in network simulation projects.

## Exercises

1. **Easy** — Build the 4-node network from the worked example and verify that router A picks the correct next hop to reach C.
2. **Medium** — Add a `link_down(a, b)` method to `Network` that removes the link from both routers' neighbor tables, then re-run convergence. Observe the count-to-infinity behavior and verify that split horizon stops it.
3. **Hard** — Implement **hold-down timers**: when a route's cost increases, the router should suppress updates for that destination for N rounds. Compare convergence behavior with and without hold-down.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Distance vector | "Each router tells its neighbors what it knows" | A routing algorithm where each node shares its entire cost-to-destination table with neighbors |
| Bellman-Ford | "The DV equation" | `D(x) = min_v { c(v) + D_v(x) }` — relax through neighbors to find shortest paths |
| Split horizon | "Don't advertise back" | Don't send a route back to the neighbor you learned it from |
| Poison reverse | "Advertise back as infinite" | Advertise learned routes back to source neighbor with cost ∞ |
| Count-to-infinity | "Routing loop slow-death" | False info loops cause costs to climb incrementally toward the infinity threshold |
| Default route | "Gateway of last resort" | `0.0.0.0/0` — the catch-all entry when no specific route matches |

## Further Reading

- RFC 2453 — RIP Version 2
- RFC 1058 — RIP Version 1 (original specification)
- Peterson & Davie, *Computer Networks: A Systems Approach*, Ch. 4.3 — Distance-Vector Routing
