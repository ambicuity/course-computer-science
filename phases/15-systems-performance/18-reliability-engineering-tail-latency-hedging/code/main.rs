// Reliability Engineering — Tail Latency, Hedging
// Phase 15 — Systems Programming & Performance
//
// Implements: hedged requests, circuit breaker, histogram-based latency analysis, percentile calculation.

use rand::Rng;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// --- Latency Histogram & Percentile Calculator ---

struct LatencyHistogram {
    buckets: BTreeMap<u64, u64>, // bucket boundary (µs) -> count
    total: u64,
    sum_us: u64,
    min_us: u64,
    max_us: u64,
}

impl LatencyHistogram {
    fn new() -> Self {
        LatencyHistogram {
            buckets: BTreeMap::new(),
            total: 0,
            sum_us: 0,
            min_us: u64::MAX,
            max_us: 0,
        }
    }

    fn record(&mut self, duration: Duration) {
        let us = duration.as_micros() as u64;
        // Bucket into powers of 2 for histogram
        let bucket = if us == 0 { 1 } else { us.next_power_of_two() };
        *self.buckets.entry(bucket).or_insert(0) += 1;
        self.total += 1;
        self.sum_us += us;
        if us < self.min_us {
            self.min_us = us;
        }
        if us > self.max_us {
            self.max_us = us;
        }
    }

    fn percentile(&self, p: f64) -> Duration {
        if self.total == 0 {
            return Duration::ZERO;
        }
        let target = (self.total as f64 * p / 100.0).ceil() as u64;
        let mut cumulative: u64 = 0;
        for (&bucket, &count) in &self.buckets {
            cumulative += count;
            if cumulative >= target {
                return Duration::from_micros(bucket / 2); // midpoint of bucket
            }
        }
        Duration::from_micros(self.max_us)
    }

    fn stats(&self) -> (Duration, Duration, Duration, Duration, Duration, usize) {
        if self.total == 0 {
            return (Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO, 0);
        }
        let p50 = self.percentile(50.0);
        let p90 = self.percentile(90.0);
        let p95 = self.percentile(95.0);
        let p99 = self.percentile(99.0);
        let max = Duration::from_micros(self.max_us);
        (p50, p90, p95, p99, max, self.total as usize)
    }

    fn print_histogram(&self) {
        println!("  Latency histogram (bucket → count):");
        let max_count = self.buckets.values().cloned().max().unwrap_or(1);
        for (&bucket, &count) in &self.buckets {
            let bar_len = (count as f64 / max_count as f64 * 40.0) as usize;
            let bar: String = "█".repeat(bar_len);
            println!("    {:>8} µs: {} ({})", bucket, bar, count);
        }
    }
}

// --- Circuit Breaker ---

#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

struct CircuitBreaker {
    state: Mutex<CircuitState>,
    failures: Mutex<u32>,
    threshold: u32,
    open_timeout: Duration,
    last_state_change: Mutex<Instant>,
    successes_in_half_open: Mutex<u32>,
    half_open_max: u32,
}

impl CircuitBreaker {
    fn new(threshold: u32, open_timeout: Duration) -> Self {
        CircuitBreaker {
            state: Mutex::new(CircuitState::Closed),
            failures: Mutex::new(0),
            threshold,
            open_timeout,
            last_state_change: Mutex::new(Instant::now()),
            successes_in_half_open: Mutex::new(0),
            half_open_max: 1,
        }
    }

    fn allow(&self) -> bool {
        let state = *self.state.lock().unwrap();
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let elapsed = self.last_state_change.lock().unwrap().elapsed();
                if elapsed > self.open_timeout {
                    *self.state.lock().unwrap() = CircuitState::HalfOpen;
                    *self.successes_in_half_open.lock().unwrap() = 0;
                    *self.last_state_change.lock().unwrap() = Instant::now();
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                let successes = *self.successes_in_half_open.lock().unwrap();
                successes < self.half_open_max
            }
        }
    }

    fn record_success(&self) {
        *self.failures.lock().unwrap() = 0;
        let state = *self.state.lock().unwrap();
        if state == CircuitState::HalfOpen {
            let mut successes = self.successes_in_half_open.lock().unwrap();
            *successes += 1;
            if *successes >= self.half_open_max {
                *self.state.lock().unwrap() = CircuitState::Closed;
                *self.last_state_change.lock().unwrap() = Instant::now();
            }
        }
    }

    fn record_failure(&self) {
        let mut failures = self.failures.lock().unwrap();
        *failures += 1;
        let state = *self.state.lock().unwrap();
        if state == CircuitState::HalfOpen {
            *self.state.lock().unwrap() = CircuitState::Open;
            *self.last_state_change.lock().unwrap() = Instant::now();
        } else if *failures >= self.threshold {
            *self.state.lock().unwrap() = CircuitState::Open;
            *self.last_state_change.lock().unwrap() = Instant::now();
        }
    }

    fn get_state(&self) -> CircuitState {
        *self.state.lock().unwrap()
    }
}

// --- Simulated Backend ---

struct Backend {
    name: String,
    min_latency_us: u64,
    max_latency_us: u64,
    fail_rate: f64,
}

impl Backend {
    fn call(&self, deadline: Instant) -> (Duration, Result<(), String>) {
        let mut rng = rand::thread_rng();
        let latency_range = self.max_latency_us - self.min_latency_us;

        let u: f64 = rng.r#gen();
        let latency_us = if u < 0.9 {
            // 90% fast path
            self.min_latency_us + (rng.r#gen::<f64>() * latency_range as f64 * 0.3) as u64
        } else if u < 0.99 {
            // 9% medium slow
            let base = latency_range as f64 * 0.3;
            self.min_latency_us + (base + rng.r#gen::<f64>() * latency_range as f64 * 0.5) as u64
        } else {
            // 1% very slow (tail)
            let base = latency_range as f64 * 0.8;
            self.min_latency_us + (base + rng.r#gen::<f64>() * latency_range as f64 * 0.2) as u64
        };

        let latency = Duration::from_micros(latency_us);

        if Instant::now() > deadline {
            return (latency, Err("deadline exceeded".into()));
        }

        // Simulate wait (shortened for demo — scale down by 1000x)
        let sim_latency = Duration::from_micros(latency_us / 1000);
        thread::sleep(sim_latency);

        if rng.r#gen::<f64>() < self.fail_rate {
            return (latency, Err(format!("backend {}: random failure", self.name)));
        }

        (latency, Ok(()))
    }
}

// --- Hedged Request ---

struct HedgedResult {
    response_time: Duration,
    used_hedge: bool,
    backend: String,
    error: Option<String>,
}

fn hedged_request(backends: &[&Backend], hedge_delay: Duration, deadline: Instant) -> HedgedResult {
    let (tx, rx) = std::sync::mpsc::channel();
    let hedge_delay_actual = hedge_delay / 1000; // scale for demo

    // Primary request
    let primary = backends[0];
    let primary_name = primary.name.clone();
    let tx_primary = tx.clone();
    let deadline_clone = deadline;
    thread::spawn(move || {
        let (d, r) = primary.call(deadline_clone);
        let _ = tx_primary.send((d, primary_name, r.map_err(|e| e.to_string())));
    });

    // Wait hedge delay, then send hedge if primary hasn't responded
    let start = Instant::now();
    let mut hedge_sent = false;
    if backends.len() > 1 {
        thread::sleep(hedge_delay_actual);
        if rx.try_recv().is_err() {
            // Primary hasn't responded yet, send hedge
            let hedge = backends[1];
            let hedge_name = hedge.name.clone();
            let tx_hedge = tx;
            hedge_sent = true;
            thread::spawn(move || {
                let (d, r) = hedge.call(deadline);
                let _ = tx_hedge.send((d, hedge_name, r.map_err(|e| e.to_string())));
            });
        }
    }

    // Take first response
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok((latency, name, result)) => HedgedResult {
            response_time: latency,
            used_hedge: hedge_sent,
            backend: name,
            error: result.err(),
        },
        Err(_) => HedgedResult {
            response_time: start.elapsed(),
            used_hedge: hedge_sent,
            backend: "timeout".into(),
            error: Some("all backends timed out".into()),
        },
    }
}

fn single_request(backend: &Backend, deadline: Instant) -> HedgedResult {
    let (d, r) = backend.call(deadline);
    HedgedResult {
        response_time: d,
        used_hedge: false,
        backend: backend.name.clone(),
        error: r.err(),
    }
}

// --- Simulation ---

fn simulate(
    num_requests: usize,
    backends: &[&Backend],
    use_hedging: bool,
    hedge_delay: Duration,
    cb: &CircuitBreaker,
) -> LatencyHistogram {
    let histogram = Arc::new(Mutex::new(LatencyHistogram::new()));
    let mut handles = Vec::new();

    for _ in 0..num_requests {
        let hist = Arc::clone(&histogram);
        let cb_state = cb; // shared ref
        let backends_owned: Vec<&Backend> = backends.to_vec();

        let handle = thread::spawn(move || {
            if !cb_state.allow() {
                hist.lock().unwrap().record(Duration::from_millis(1)); // fail-fast
                return;
            }

            let deadline = Instant::now() + Duration::from_secs(2);

            let result = if use_hedging {
                hedged_request(&backends_owned, hedge_delay, deadline)
            } else {
                single_request(backends_owned[0], deadline)
            };

            if result.error.is_some() {
                cb_state.record_failure();
            } else {
                cb_state.record_success();
            }

            hist.lock().unwrap().record(result.response_time);
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.join();
    }

    let hist = Arc::try_unwrap(histogram).unwrap().into_inner().unwrap();
    hist
}

fn main() {
    println!("=== Tail Latency & Hedging Demo (Rust) ===\n");

    let b1 = Backend {
        name: "A".into(),
        min_latency_us: 5_000,
        max_latency_us: 500_000,
        fail_rate: 0.02,
    };
    let b2 = Backend {
        name: "B".into(),
        min_latency_us: 5_000,
        max_latency_us: 500_000,
        fail_rate: 0.02,
    };
    let b3 = Backend {
        name: "C".into(),
        min_latency_us: 5_000,
        max_latency_us: 600_000,
        fail_rate: 0.03,
    };

    let backends: Vec<&Backend> = vec![&b1, &b2, &b3];
    let hedge_delay = Duration::from_millis(20);
    let num_requests = 500;

    // Without hedging
    let cb1 = CircuitBreaker::new(5, Duration::from_millis(30_000));
    let hist1 = simulate(num_requests, &backends, false, hedge_delay, &cb1);
    let (p50, p90, p95, p99, max, count) = hist1.stats();
    println!("--- WITHOUT Hedging ---");
    println!("  Requests: {}", count);
    println!("  p50:  {:?}", p50);
    println!("  p90:  {:?}", p90);
    println!("  p95:  {:?}", p95);
    println!("  p99:  {:?}", p99);
    println!("  max:  {:?}", max);
    println!("  Circuit breaker state: {:?}", cb1.get_state());
    hist1.print_histogram();
    println!();

    // With hedging
    let cb2 = CircuitBreaker::new(5, Duration::from_millis(30_000));
    let hist2 = simulate(num_requests, &backends, true, hedge_delay, &cb2);
    let (p50, p90, p95, p99, max, count) = hist2.stats();
    println!("--- WITH Hedging (delay=20ms) ---");
    println!("  Requests: {}", count);
    println!("  p50:  {:?}", p50);
    println!("  p90:  {:?}", p90);
    println!("  p95:  {:?}", p95);
    println!("  p99:  {:?}", p99);
    println!("  max:  {:?}", max);
    println!("  Circuit breaker state: {:?}", cb2.get_state());
    hist2.print_histogram();
    println!();

    // Latency lottery
    println!("--- Latency Lottery (fan-out=100) ---");
    let p99us = p99.as_micros() as f64;
    println!("  Single-call p99:  {:.1} ms", p99us / 1000.0);
    let prob_slow = 1.0 - 0.99_f64.powi(100);
    println!("  P(at least one slow call in 100): {:.1}%", prob_slow * 100.0);
    println!(
        "  With hedging, P(both slow):       {:.4}%",
        0.01_f64.powi(2) * 100.0
    );
    println!();

    // Circuit breaker demo
    println!("--- Circuit Breaker Demo ---");
    let cb = CircuitBreaker::new(3, Duration::from_millis(100));
    println!("  Initial state:  {:?}", cb.get_state());
    for i in 0..5 {
        cb.record_failure();
        println!("  After failure {}: state={:?}", i + 1, cb.get_state());
    }
    println!("  Request allowed: {}", cb.allow());
    thread::sleep(Duration::from_millis(150));
    println!("  After timeout:  state={:?}", cb.get_state());
    println!("  Request allowed: {}", cb.allow());
    cb.record_success();
    println!("  After success:  state={:?}", cb.get_state());
}