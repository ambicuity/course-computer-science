# Load Balancers — L4 vs L7, Algorithms

> The traffic cop that keeps your server fleet from drowning — and the algorithms that decide which server handles which request.

**Type:** Learn
**Languages:** Rust, Python
**Prerequisites:** Phase 09 lessons 01–17
**Time:** ~60 minutes

## Learning Objectives

- Distinguish L4 (transport) from L7 (application) load balancing and know when to use each.
- Implement six routing algorithms: round-robin, weighted round-robin, least connections, IP hash, consistent hash, random-two-choices.
- Simulate health checks and session affinity.
- Benchmark algorithm distribution quality with 1000 simulated requests.

## The Problem

You have four application servers. Without a load balancer, users connect directly to one — and if that server dies, they're stuck. Worse, Server 1 handles 80% of traffic while Server 3 sits idle.

A **load balancer** sits in front of the fleet, distributes incoming requests across healthy backends, detects failures, and (optionally) keeps a user pinned to the same server. It's the single most common infrastructure component in production systems.

## The Concept

### L4 vs L7

**Layer 4 (Transport)** — Routes by IP address and port. No content inspection.

```
Client → LB: "Connect to 10.0.0.1:80"
LB looks up (dst_ip=10.0.0.1, dst_port=80) in its rules
LB picks backend → forwards raw TCP/UDP packets
LB rewrites dst_ip to backend IP, src_ip to its own (NAT mode)
```

Properties:
- Fast: no parsing, just packet forwarding
- Simple: works with any protocol (TCP, UDP, QUIC)
- Stateless per-connection: pick backend at connection time, stick with it
- No content awareness: can't route by URL path or HTTP header

Examples: HAProxy (TCP mode), Linux IPVS, AWS NLB, iptables DNAT

**Layer 7 (Application)** — Routes by HTTP content: URL, headers, cookies, body.

```
Client → LB: "GET /api/users HTTP/1.1"
LB parses HTTP request
LB sees path="/api/users" → route to API backend pool
LB sees path="/static/img.png" → route to static backend pool
```

Properties:
- Slower: must parse HTTP (TLS termination, header parsing)
- Flexible: route by URL, header, cookie, method, body
- Per-request: different requests on same connection can go to different backends
- Protocol-specific: only works with HTTP (or gRPC, WebSocket, etc.)

Examples: Nginx, HAProxy (HTTP mode), AWS ALB, Envoy, Traefik

| Property | L4 | L7 |
|----------|----|----|
| Speed | Very fast | Slower |
| Content awareness | None | Full |
| TLS termination | No (pass-through) | Yes |
| Per-request routing | No | Yes |
| Use case | TCP/UDP, gaming, MQTT | HTTP, gRPC, WebSocket |

### Algorithms

**1. Round Robin** — Cycle through backends sequentially.

```
Request 1 → Backend A
Request 2 → Backend B
Request 3 → Backend C
Request 4 → Backend A  (wraps around)
```

Simple, uniform. Ignores backend capacity and current load.

**2. Weighted Round Robin** — Backends with higher capacity get proportionally more requests.

```
Weights: A=5, B=3, C=2
Sequence: A A A A A B B B C C, A A A A A B B B C C, ...
```

**3. Least Connections** — Route to the backend with the fewest active connections.

```
Active: A=12, B=8, C=15 → pick B
```

Adapts to varying request durations. Good for long-lived connections.

**4. IP Hash** — Hash client IP to pick a backend. Same client always hits the same server.

```
backend = backends[hash(client_ip) % len(backends)]
```

Provides session affinity without cookies. Unbalanced when one IP sends far more traffic.

**5. Consistent Hashing** — Hash both backends and requests onto a ring. Walk clockwise to find the backend. Adding/removing a backend only remaps ~1/N of requests.

```
Ring: 0 ──────────────────────────── 2^32
  Backend A at hash 1000
  Backend B at hash 2000
  Backend C at hash 3000
  Request hash 2500 → walk clockwise → Backend C
  Request hash 500  → walk clockwise → Backend A
```

Used by: Memcached clients, Cassandra, DynamoDB partitioning.

**6. Random Two Choices** — Pick two random backends, route to the one with fewer connections. Nearly optimal distribution with O(1) cost.

```
Pick random A, C → A has 10 connections, C has 5 → route to C
```

### Health Checks

**Active**: Load balancer periodically probes backends (HTTP GET /health, TCP connect). If probe fails N times, mark backend as down.

```
Every 5s: GET /healthz → 200 OK → healthy
          GET /healthz → timeout  → fail_count++
          fail_count >= 3 → mark DOWN, remove from pool
```

**Passive**: Monitor real responses. If backend returns 5xx or times out, mark it as degraded.

### Session Affinity (Sticky Sessions)

Keep a client pinned to the same backend. Methods:

- **Cookie-based**: LB inserts a `SERVERID=backend-A` cookie.
- **IP-based**: Hash client IP (IP Hash algorithm above).
- **Application**: App stores session in-memory; affinity is required.

## Build It

### Rust: Load Balancer Library

```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Backend {
    pub addr: SocketAddr,
    pub weight: u32,
    pub active_connections: u32,
    pub healthy: bool,
}

impl Backend {
    pub fn new(addr: &str, weight: u32) -> Self {
        Backend {
            addr: addr.parse().expect("invalid address"),
            weight,
            active_connections: 0,
            healthy: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Algorithm {
    RoundRobin,
    WeightedRoundRobin,
    LeastConnections,
    IpHash,
    ConsistentHash,
    RandomTwoChoices,
}

pub struct LoadBalancer {
    backends: Vec<Backend>,
    algorithm: Algorithm,
    rr_index: usize,
    wrr_current_weights: Vec<u32>,
    wrr_total_weight: u32,
    consistent_ring: Vec<(u64, usize)>, // (hash, backend_index)
}

impl LoadBalancer {
    pub fn new(backends: Vec<Backend>, algorithm: Algorithm) -> Self {
        let wrr_total: u32 = backends.iter().map(|b| b.weight).sum();
        let n = backends.len();
        let mut lb = LoadBalancer {
            backends,
            algorithm,
            rr_index: 0,
            wrr_current_weights: vec![0; n],
            wrr_total_weight: wrr_total,
            consistent_ring: Vec::new(),
        };
        lb.build_consistent_ring();
        lb
    }

    pub fn route(&mut self, client_ip: &str) -> Option<&Backend> {
        let healthy_count = self.backends.iter().filter(|b| b.healthy).count();
        if healthy_count == 0 {
            return None;
        }

        match self.algorithm {
            Algorithm::RoundRobin => self.round_robin(),
            Algorithm::WeightedRoundRobin => self.weighted_round_robin(),
            Algorithm::LeastConnections => self.least_connections(),
            Algorithm::IpHash => self.ip_hash(client_ip),
            Algorithm::ConsistentHash => self.consistent_hash(client_ip),
            Algorithm::RandomTwoChoices => self.random_two_choices(),
        }
    }

    fn round_robin(&mut self) -> Option<&Backend> {
        let n = self.backends.len();
        for _ in 0..n {
            let idx = self.rr_index % n;
            self.rr_index += 1;
            if self.backends[idx].healthy {
                return Some(&self.backends[idx]);
            }
        }
        None
    }

    fn weighted_round_robin(&mut self) -> Option<&Backend> {
        loop {
            for i in 0..self.backends.len() {
                if !self.backends[i].healthy {
                    continue;
                }
                self.wrr_current_weights[i] += self.backends[i].weight;
                if self.wrr_current_weights[i] >= self.wrr_total_weight {
                    self.wrr_current_weights[i] -= self.wrr_total_weight;
                    return Some(&self.backends[i]);
                }
            }
        }
    }

    fn least_connections(&self) -> Option<&Backend> {
        self.backends.iter()
            .filter(|b| b.healthy)
            .min_by_key(|b| b.active_connections)
    }

    fn ip_hash(&self, client_ip: &str) -> Option<&Backend> {
        let mut hasher = DefaultHasher::new();
        client_ip.hash(&mut hasher);
        let hash = hasher.finish() as usize;
        let healthy: Vec<&Backend> = self.backends.iter().filter(|b| b.healthy).collect();
        if healthy.is_empty() {
            return None;
        }
        Some(healthy[hash % healthy.len()])
    }

    fn consistent_hash(&self, client_ip: &str) -> Option<&Backend> {
        let mut hasher = DefaultHasher::new();
        client_ip.hash(&mut hasher);
        let hash = hasher.finish();

        for (ring_hash, idx) in &self.consistent_ring {
            if hash <= *ring_hash && self.backends[*idx].healthy {
                return Some(&self.backends[*idx]);
            }
        }
        // Wrap around
        for (_, idx) in &self.consistent_ring {
            if self.backends[*idx].healthy {
                return Some(&self.backends[*idx]);
            }
        }
        None
    }

    fn random_two_choices(&self) -> Option<&Backend> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let healthy: Vec<(usize, &Backend)> = self.backends.iter()
            .enumerate()
            .filter(|(_, b)| b.healthy)
            .collect();
        if healthy.is_empty() {
            return None;
        }
        let n = healthy.len();
        let a = (t as usize) % n;
        let b = ((t >> 32) as usize) % n;
        let pick = if healthy[a].1.active_connections <= healthy[b].1.active_connections {
            a
        } else {
            b
        };
        Some(healthy[pick].1)
    }

    fn build_consistent_ring(&mut self) {
        self.consistent_ring.clear();
        for (i, backend) in self.backends.iter().enumerate() {
            for vnode in 0..150 {
                let mut hasher = DefaultHasher::new();
                format!("{}:{}", backend.addr, vnode).hash(&mut hasher);
                self.consistent_ring.push((hasher.finish(), i));
            }
        }
        self.consistent_ring.sort_by_key(|(h, _)| *h);
    }

    pub fn health_check(&mut self) {
        for backend in &mut self.backends {
            // Simulated: in production, attempt TCP connect or HTTP GET
            backend.healthy = true;
        }
    }

    pub fn set_unhealthy(&mut self, idx: usize) {
        if idx < self.backends.len() {
            self.backends[idx].healthy = false;
        }
    }
}

fn main() {
    println!("=== Load Balancer — L4 vs L7, Algorithms ===\n");

    let backends = vec![
        Backend::new("10.0.0.1:8080", 1),
        Backend::new("10.0.0.2:8080", 1),
        Backend::new("10.0.0.3:8080", 1),
        Backend::new("10.0.0.4:8080", 1),
    ];

    let algorithms = [
        Algorithm::RoundRobin,
        Algorithm::WeightedRoundRobin,
        Algorithm::LeastConnections,
        Algorithm::IpHash,
        Algorithm::ConsistentHash,
        Algorithm::RandomTwoChoices,
    ];

    for algo in algorithms {
        let mut lb = LoadBalancer::new(backends.clone(), algo);
        let mut counts: [u32; 4] = [0; 4];
        let num_requests = 1000;

        for i in 0..num_requests {
            let client = format!("192.168.1.{}", i % 200);
            if let Some(backend) = lb.route(&client) {
                let idx = match backend.addr.port() {
                    8080 => {
                        match backend.addr.ip().to_string().as_str() {
                            "10.0.0.1" => 0,
                            "10.0.0.2" => 1,
                            "10.0.0.3" => 2,
                            "10.0.0.4" => 3,
                            _ => 0,
                        }
                    }
                    _ => 0,
                };
                counts[idx] += 1;
            }
        }

        println!("{:?}:", algo);
        for (i, count) in counts.iter().enumerate() {
            let pct = (*count as f64 / num_requests as f64) * 100.0;
            println!("  Backend {} : {:>4} ({:.1}%)", i + 1, count, pct);
        }
        println!();
    }

    // Health check failure demo
    println!("--- Health Check Failure ---");
    let mut lb = LoadBalancer::new(backends.clone(), Algorithm::RoundRobin);
    lb.set_unhealthy(1); // Mark backend 2 as down
    let mut counts: [u32; 4] = [0; 4];
    for i in 0..12 {
        let client = format!("10.0.0.{}", i);
        if let Some(backend) = lb.route(&client) {
            let idx: usize = backend.addr.ip().to_string()
                .split('.').last().unwrap().parse::<usize>().unwrap() - 1;
            counts[idx] += 1;
        }
    }
    println!("With backend 2 down:");
    for (i, count) in counts.iter().enumerate() {
        let status = if i == 1 { "DOWN" } else { "healthy" };
        println!("  Backend {} ({}) : {} requests", i + 1, status, count);
    }
}
```

### Python: Load Balancer Simulation

```python
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
        """Simulate n requests, return distribution per backend addr."""
        counts = Counter()
        for i in range(n):
            if skew:
                # 80% of traffic from 20% of IPs (Zipf-like)
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
```

## Use It

Production load balancers:

- **HAProxy**: L4 + L7. The industry standard for TCP and HTTP load balancing. Configuration is declarative (`backend`/`frontend` blocks). See `src/cfgparse.c` for config parsing.
- **Nginx**: Primarily L7. `upstream` blocks define backends with `least_conn`, `ip_hash`, or `hash` directives. Also supports L4 via `stream` module.
- **Envoy**: L4 + L7, designed for service mesh. xDS API for dynamic configuration. Written in C++.
- **AWS ELB family**: ALB (L7), NLB (L4), CLB (legacy). Managed — you don't configure algorithms directly.
- **IPVS**: Linux kernel L4 load balancer. Part of `net/netfilter/ipvs/`. Faster than userspace solutions for raw TCP/UDP.

## Read the Source

- `haproxy/src/backend.c` — `process_srv_conn()`: how HAProxy selects a backend server for a new connection. Look at the `algo` dispatch.
- `nginx/src/http/ngx_http_upstream_round_robin.c` — Nginx round-robin implementation with weighted support.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A load balancer library** — pluggable algorithms, health checking, and session affinity for simulating and benchmarking traffic distribution.

## Exercises

1. **Easy** — Run the Python simulation with 10,000 requests. Calculate the standard deviation of requests per backend for each algorithm. Which algorithm produces the most uniform distribution?

2. **Medium** — Implement session affinity using cookie-based sticky sessions. The LB inserts a `SERVERID` cookie on the first request; subsequent requests from the same client route to the same backend. Test by simulating a client that makes 10 requests.

3. **Hard** — Implement consistent hashing with virtual nodes. Add a backend, remove a backend, and measure what fraction of requests get remapped. Compare 50, 150, and 300 virtual nodes. Plot the results.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| L4 load balancer | "Transport LB" | Routes by IP/port; forwards raw TCP/UDP packets without inspecting content |
| L7 load balancer | "Application LB" | Routes by HTTP content (URL, headers, cookies); parses the application protocol |
| Round robin | "Equal distribution" | Cycle through backends sequentially; simple, assumes equal capacity |
| Weighted round robin | "Proportional" | Backends with higher weights receive proportionally more requests |
| Least connections | "Least busy" | Route to the backend with the fewest active connections |
| Consistent hashing | "Minimal remapping" | Hash ring technique; adding/removing a backend only redistributes ~1/N of keys |
| Sticky session | "Session affinity" | Pin a client to the same backend for the duration of a session |
| Health check | "Liveness probe" | Periodic probe (TCP connect or HTTP GET) to determine if a backend is functional |
| Backend pool | "Upstream group" | The set of servers behind a load balancer that handle actual requests |

## Further Reading

- [HAProxy documentation](https://www.haproxy.org/documentation/) — Comprehensive reference for L4/L7 configuration
- [Envoy architecture overview](https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/upstream/load_balancing/load_balancing) — All supported LB algorithms explained
- [Consistent hashing paper (Karger et al., 1997)](https://dl.acm.org/doi/10.1145/258533.258660) — The original consistent hashing algorithm
- [Google Maglev](https://research.google/pubs/pub44824/) — Production L4 load balancer using consistent hashing with connection tracking
