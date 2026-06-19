use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

const NUM_THREADS: usize = 4;
const INCREMENTS: u64 = 50_000_000;

struct PackedCounters {
    counters: [AtomicU64; NUM_THREADS],
}

#[repr(C, align(64))]
struct CachePadded<T> {
    value: T,
    _pad: [u8; 64 - std::mem::size_of::<AtomicU64>()],
}

struct PaddedCounters {
    counters: [CachePadded<AtomicU64>; NUM_THREADS],
}

fn run_packed_demo() {
    let counters = Arc::new(PackedCounters {
        counters: std::array::from_fn(|_| AtomicU64::new(0)),
    });

    let start = Instant::now();

    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|i| {
            let c = Arc::clone(&counters);
            thread::spawn(move || {
                let counter = &c.counters[i];
                for _ in 0..INCREMENTS {
                    counter.store(counter.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    let elapsed = start.elapsed();
    println!("=== Packed (false sharing) ===");
    println!("Total time: {:.2} ms", elapsed.as_secs_f64() * 1000.0);
    for i in 0..NUM_THREADS {
        println!("  Counter {}: {}", i, counters.counters[i].load(Ordering::Relaxed));
    }
    println!("  sizeof(PackedCounters): {} bytes", std::mem::size_of::<PackedCounters>());
    println!();
}

fn run_padded_demo() {
    let counters = Arc::new(PaddedCounters {
        counters: std::array::from_fn(|_| CachePadded {
            value: AtomicU64::new(0),
            _pad: [0u8; 64 - std::mem::size_of::<AtomicU64>()],
        }),
    });

    let start = Instant::now();

    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|i| {
            let c = Arc::clone(&counters);
            thread::spawn(move || {
                let counter = &c.counters[i].value;
                for _ in 0..INCREMENTS {
                    counter.store(counter.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    let elapsed = start.elapsed();
    println!("=== Padded (#[repr(C, align(64))], no false sharing) ===");
    println!("Total time: {:.2} ms", elapsed.as_secs_f64() * 1000.0);
    for i in 0..NUM_THREADS {
        println!("  Counter {}: {}", i, counters.counters[i].value.load(Ordering::Relaxed));
    }
    println!("  sizeof(PaddedCounters): {} bytes", std::mem::size_of::<PaddedCounters>());
    println!("  sizeof(CachePadded<AtomicU64>): {} bytes", std::mem::size_of::<CachePadded<AtomicU64>>());
    println!();
}

fn run_per_thread_timing_demo() {
    println!("=== Per-thread timing (padded) ===");

    let counters = Arc::new(PaddedCounters {
        counters: std::array::from_fn(|_| CachePadded {
            value: AtomicU64::new(0),
            _pad: [0u8; 64 - std::mem::size_of::<AtomicU64>()],
        }),
    });

    let go = Arc::new(AtomicU64::new(0));
    let results: Vec<_> = (0..NUM_THREADS)
        .map(|i| {
            let c = Arc::clone(&counters);
            let g = Arc::clone(&go);
            thread::spawn(move || {
                while g.load(Ordering::Acquire) == 0 {
                    std::hint::spin_loop();
                }
                let t0 = Instant::now();
                let counter = &c.counters[i].value;
                for _ in 0..INCREMENTS {
                    counter.store(counter.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
                }
                let elapsed = t0.elapsed();
                (i, elapsed.as_secs_f64() * 1000.0)
            })
        })
        .collect();

    go.store(1, Ordering::Release);

    let mut timings = Vec::new();
    for h in results {
        timings.push(h.join().unwrap());
    }
    timings.sort_by_key(|t| t.0);

    for (i, ms) in &timings {
        println!(
            "  Thread {}: {:.2} ms, counter = {}",
            i,
            ms,
            counters.counters[*i].value.load(Ordering::Relaxed)
        );
    }
    println!();
}

fn print_cache_line_info() {
    println!("=== Cache line diagnostics ===");
    println!("  sizeof(AtomicU64):              {} bytes", std::mem::size_of::<AtomicU64>());
    println!("  sizeof(PackedCounters):          {} bytes", std::mem::size_of::<PackedCounters>());
    println!("  sizeof(CachePadded<AtomicU64>):  {} bytes", std::mem::size_of::<CachePadded<AtomicU64>>());
    println!("  sizeof(PaddedCounters):          {} bytes", std::mem::size_of::<PaddedCounters>());
    println!("  align_of(CachePadded<AtomicU64>): {}", std::mem::align_of::<CachePadded<AtomicU64>>());

    println!("\n  Packed counter offsets:");
    for i in 0..NUM_THREADS {
        println!("    counters[{}] at offset {} (8 bytes apart, sharing cache lines)", i, i * 8);
    }

    println!("\n  Padded counter offsets:");
    for i in 0..NUM_THREADS {
        println!("    counters[{}] at offset {} (64 bytes apart, each on own cache line)", i, i * 64);
    }
    println!();
}

fn print_numa_hints() {
    println!("=== NUMA detection hints ===");
    println!("  Run: numactl --hardware");
    println!("  Run: numastat -m");
    println!("  Run: perf stat -e L1-dcache-load-misses,cache-misses <program>");
    println!();
}

fn main() {
    println!("False Sharing and NUMA Demo");
    println!("============================");
    println!("Threads: {}", NUM_THREADS);
    println!("Increments per thread: {}", INCREMENTS);
    println!();

    print_cache_line_info();
    run_packed_demo();
    run_padded_demo();
    run_per_thread_timing_demo();
    print_numa_hints();
}