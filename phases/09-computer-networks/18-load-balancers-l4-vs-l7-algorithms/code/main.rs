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
    consistent_ring: Vec<(u64, usize)>,
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
    println!("=== Load Balancer -- L4 vs L7, Algorithms ===\n");

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
                let ip_str = backend.addr.ip().to_string();
                let last = ip_str.split('.').last().unwrap();
                let idx: usize = last.parse::<usize>().unwrap() - 1;
                if idx < 4 {
                    counts[idx] += 1;
                }
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
    lb.set_unhealthy(1);
    let mut counts: [u32; 4] = [0; 4];
    for i in 0..12 {
        let client = format!("10.0.0.{}", i);
        if let Some(backend) = lb.route(&client) {
            let ip_str = backend.addr.ip().to_string();
            let last = ip_str.split('.').last().unwrap();
            let idx: usize = last.parse::<usize>().unwrap() - 1;
            if idx < 4 {
                counts[idx] += 1;
            }
        }
    }
    println!("With backend 2 down:");
    for (i, count) in counts.iter().enumerate() {
        let status = if i == 1 { "DOWN" } else { "healthy" };
        println!("  Backend {} ({}) : {} requests", i + 1, status, count);
    }
}
