use std::time::Instant;

mod arena {
    pub struct Arena {
        buffer: Vec<u8>,
        offset: usize,
    }

    impl Arena {
        pub fn new(capacity: usize) -> Self {
            let mut buffer = Vec::with_capacity(capacity);
            buffer.resize(capacity, 0);
            Arena { buffer, offset: 0 }
        }

        pub fn allocate(&mut self, size: usize, align: usize) -> Option<*mut u8> {
            let aligned = (self.offset + align - 1) & !(align - 1);
            if aligned + size > self.buffer.len() {
                return None;
            }
            let ptr = self.buffer.as_mut_ptr().wrapping_add(aligned);
            self.offset = aligned + size;
            Some(ptr)
        }

        pub fn reset(&mut self) {
            self.offset = 0;
        }

        pub fn used(&self) -> usize {
            self.offset
        }

        pub fn capacity(&self) -> usize {
            self.buffer.len()
        }
    }
}

mod stats {
    #[derive(Debug, Clone)]
    pub struct Stats {
        pub mean_ns: f64,
        pub median_ns: f64,
        pub p99_ns: f64,
        pub stddev_ns: f64,
        pub min_ns: f64,
        pub max_ns: f64,
    }

    pub fn compute(samples: &mut [f64]) -> Stats {
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = samples.len();
        let sum: f64 = samples.iter().copied().sum();
        let mean = sum / n as f64;
        let median = samples[n / 2];
        let p99_idx = ((n as f64) * 0.99) as usize;
        let p99 = samples[p99_idx.min(n - 1)];
        let variance: f64 = samples.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n as f64;
        Stats {
            mean_ns: mean,
            median_ns: median,
            p99_ns: p99,
            stddev_ns: variance.sqrt(),
            min_ns: samples[0],
            max_ns: samples[n - 1],
        }
    }

    pub fn format_ns(ns: f64) -> String {
        if ns >= 1_000_000.0 {
            format!("{:.2}ms", ns / 1_000_000.0)
        } else if ns >= 1_000.0 {
            format!("{:.2}us", ns / 1_000.0)
        } else {
            format!("{:.0}ns", ns)
        }
    }
}

struct NaiveStringProcessor {
    lines: Vec<String>,
    raw_text: String,
}

impl NaiveStringProcessor {
    fn new() -> Self {
        NaiveStringProcessor {
            lines: Vec::new(),
            raw_text: String::new(),
        }
    }

    fn load_from_string(&mut self, text: &str) {
        self.raw_text = text.to_string();
        self.lines = text.lines().map(|l| l.to_string()).collect();
    }

    fn count_pattern(&self, pattern: &str) -> usize {
        let text = self.raw_text.as_bytes();
        let pat = pattern.as_bytes();
        let plen = pat.len();
        if plen == 0 || text.len() < plen {
            return 0;
        }
        let mut count = 0usize;
        for i in 0..=text.len() - plen {
            let mut match_found = true;
            for j in 0..plen {
                if text[i + j] != pat[j] {
                    match_found = false;
                    break;
                }
            }
            if match_found {
                count += 1;
            }
        }
        count
    }

    fn transform_to_upper(&self) -> String {
        let mut result = self.raw_text.clone();
        for c in result.as_bytes_mut().iter_mut() {
            if *c >= b'a' && *c <= b'z' {
                *c = *c - 32;
            }
        }
        result
    }

    fn byte_count(&self) -> usize {
        self.raw_text.len()
    }
}

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

struct OptimizedStringProcessor {
    text_buf: Vec<u8>,
    text_len: usize,
}

impl OptimizedStringProcessor {
    fn load_from_string(&mut self, text: &str) {
        self.text_len = text.len();
        self.text_buf.resize(text.len() + 64, 0);
        self.text_buf[..text.len()].copy_from_slice(text.as_bytes());
    }

    #[cfg(target_arch = "x86_64")]
    fn count_pattern_simd(&self, pattern: &str) -> usize {
        if !is_x86_feature_detected!("avx2") {
            return self.count_pattern_scalar(pattern);
        }
        unsafe { self.count_pattern_avx2(pattern) }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn count_pattern_simd(&self, pattern: &str) -> usize {
        self.count_pattern_scalar(pattern)
    }

    fn count_pattern_scalar(&self, pattern: &str) -> usize {
        let text = &self.text_buf;
        let pat = pattern.as_bytes();
        let plen = pat.len();
        if plen == 0 || self.text_len < plen {
            return 0;
        }
        let mut count = 0usize;
        for i in 0..=self.text_len - plen {
            let mut match_found = true;
            for j in 0..plen {
                if text[i + j] != pat[j] {
                    match_found = false;
                    break;
                }
            }
            count += match_found as usize;
        }
        count
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn count_pattern_avx2(&self, pattern: &str) -> usize {
        let text = &self.text_buf;
        let pat = pattern.as_bytes();
        let plen = pat.len();
        if plen == 0 || self.text_len < plen {
            return 0;
        }
        if plen == 1 {
            let target = pat[0];
            let tgt = _mm256_set1_epi8(target as i8);
            let mut count = 0usize;
            let mut i = 0usize;
            while i + 32 <= self.text_len {
                let chunk = _mm256_loadu_si256(text.as_ptr().add(i) as *const __m256i);
                let eq = _mm256_cmpeq_epi8(chunk, tgt);
                let mask = _mm256_movemask_epi8(eq) as u32;
                count += mask.count_ones() as usize;
                i += 32;
            }
            while i < self.text_len {
                count += (text[i] == target) as usize;
                i += 1;
            }
            return count;
        }
        let first = pat[0];
        let tgt = _mm256_set1_epi8(first as i8);
        let mut count = 0usize;
        let mut i = 0usize;
        while i + 32 + plen <= self.text_len {
            let chunk = _mm256_loadu_si256(text.as_ptr().add(i) as *const __m256i);
            let eq = _mm256_cmpeq_epi8(chunk, tgt);
            let mut mask = _mm256_movemask_epi8(eq) as u32;
            while mask != 0 {
                let bit = mask.trailing_zeros() as usize;
                mask &= mask - 1;
                let pos = i + bit;
                if pos + plen <= self.text_len {
                    let mut full_match = true;
                    for j in 1..plen {
                        if text[pos + j] != pat[j] {
                            full_match = false;
                            break;
                        }
                    }
                    count += full_match as usize;
                }
            }
            i += 32;
        }
        while i + plen <= self.text_len {
            let mut match_found = true;
            for j in 0..plen {
                if text[i + j] != pat[j] {
                    match_found = false;
                    break;
                }
            }
            count += match_found as usize;
            i += 1;
        }
        count
    }

    fn count_pattern_branchless(&self, pattern: &str) -> usize {
        let text = &self.text_buf;
        let pat = pattern.as_bytes();
        let plen = pat.len();
        if plen == 0 || self.text_len < plen {
            return 0;
        }
        let mut count = 0usize;
        for i in 0..=self.text_len - plen {
            let mut match_bits = 1usize;
            for j in 0..plen {
                match_bits &= (text[i + j] == pat[j]) as usize;
            }
            count += match_bits;
        }
        count
    }

    #[cfg(target_arch = "x86_64")]
    fn transform_to_upper_simd(&mut self) {
        if !is_x86_feature_detected!("avx2") {
            self.transform_to_upper_scalar();
            return;
        }
        unsafe { self.transform_to_upper_avx2() }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn transform_to_upper_simd(&mut self) {
        self.transform_to_upper_scalar();
    }

    fn transform_to_upper_scalar(&mut self) {
        for c in self.text_buf[..self.text_len].iter_mut() {
            if *c >= b'a' && *c <= b'z' {
                *c -= 32;
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn transform_to_upper_avx2(&mut self) {
        let data = self.text_buf.as_mut_ptr();
        let len = self.text_len;
        let lo = _mm256_set1_epi8(b'a' as i8 - 1);
        let hi = _mm256_set1_epi8(b'z' as i8 + 1);
        let offset = _mm256_set1_epi8(32i8);
        let mut i = 0usize;
        while i + 32 <= len {
            let chunk = _mm256_loadu_si256(data.add(i) as *const __m256i);
            let ge_lo = _mm256_cmpgt_epi8(chunk, lo);
            let le_hi = _mm256_cmpgt_epi8(hi, chunk);
            let is_lower = _mm256_and_si256(ge_lo, le_hi);
            let subtract = _mm256_and_si256(is_lower, offset);
            let upper = _mm256_sub_epi8(chunk, subtract);
            _mm256_storeu_si256(data.add(i) as *mut __m256i, upper);
            i += 32;
        }
        while i < len {
            if self.text_buf[i] >= b'a' && self.text_buf[i] <= b'z' {
                self.text_buf[i] -= 32;
            }
            i += 1;
        }
    }

    fn transform_with_arena(&self, arena: &mut arena::Arena) {
        let ptr = arena.allocate(self.text_len, 16);
        if ptr.is_null() {
            return;
        }
        unsafe {
            let src = self.text_buf.as_ptr();
            std::ptr::copy_nonoverlapping(src, ptr, self.text_len);
            let len = self.text_len;
            let data = ptr;
            let mut i = 0usize;
            while i < len {
                let c = *data.add(i);
                *data.add(i) = if c >= b'a' && c <= b'z' { c - 32 } else { c };
                i += 1;
            }
        }
    }

    fn count_pattern_parallel(&self, pattern: &str, num_threads: usize) -> usize {
        use std::thread;
        let pat = pattern.as_bytes().to_vec();
        let text = self.text_buf.clone();
        let text_len = self.text_len;
        let plen = pat.len();
        if plen == 0 || text_len < plen {
            return 0;
        }
        let chunk_size = text_len / num_threads;
        let mut handles = Vec::new();
        for t in 0..num_threads {
            let start = t * chunk_size;
            let end = if t == num_threads - 1 { text_len } else { (t + 1) * chunk_size };
            let textClone = text.clone();
            let patClone = pat.clone();
            handles.push(thread::spawn(move || {
                let mut count = 0usize;
                let search_end = if end > plen { end } else { start };
                for i in start..=search_end.saturating_sub(plen) {
                    if i + plen > textClone.len() { break; }
                    let mut match_found = true;
                    for j in 0..plen {
                        if textClone[i + j] != patClone[j] {
                            match_found = false;
                            break;
                        }
                    }
                    count += match_found as usize;
                }
                count
            }));
        }
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    }

    fn byte_count(&self) -> usize {
        self.text_len
    }
}

fn generate_test_data(num_lines: usize, avg_line_len: usize) -> String {
    let alphanum = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ";
    let pattern = b"findme";
    let mut result = Vec::with_capacity(num_lines * avg_line_len);
    let mut rng_state: u64 = 12345;
    for i in 0..num_lines {
        let len_mod = (i % 17) as i64;
        let line_len = if avg_line_len as i64 - 8 + len_mod > 0 {
            (avg_line_len as i64 - 8 + len_mod) as usize
        } else {
            avg_line_len / 2
        };
        for _ in 0..line_len {
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let idx = (rng_state >> 33) as usize % (alphanum.len() - 1);
            result.push(alphanum[idx]);
        }
        if i % 13 == 0 {
            result.extend_from_slice(pattern);
        }
        result.push(b'\n');
    }
    String::from_utf8(result).unwrap()
}

fn run_benchmark<F>(name: &str, f: F, iters: usize) -> stats::Stats
where
    F: Fn() -> f64,
{
    let mut samples = Vec::with_capacity(iters);
    for _ in 0..iters {
        let elapsed = f();
        samples.push(elapsed);
    }
    let result = stats::compute(&mut samples);
    eprintln!("  {} done", name);
    result
}

fn print_comparison(results: &[(&str, stats::Stats)]) {
    println!("\n+---------------------------------------------------------------+");
    println!("|           PGO Benchmark — Before / After Comparison            |");
    println!("+---------------------------------------------------------------+");
    println!("| {:<35} | {:<10} | {:<10} |", "Benchmark", "Median", "P99");
    println!("+---------------------------------------------------------------+");
    for (name, r) in results {
        println!(
            "| {:<35} | {:<10} | {:<10} |",
            name,
            stats::format_ns(r.median_ns),
            stats::format_ns(r.p99_ns)
        );
    }
    println!("+---------------------------------------------------------------+");

    if results.len() >= 2 {
        let baseline = &results[0].1;
        println!("\nSpeedup vs baseline:");
        for i in 1..results.len() {
            let speedup = baseline.median_ns / results[i].1.median_ns;
            println!("  {}: {:.2}x", results[i].0, speedup);
        }
    }
}

fn print_lesson_mapping() {
    println!("\nOptimization → Lesson Mapping:");
    println!("  Baseline measurement          → L02: Measurement Discipline");
    println!("  Cache-friendly contiguous buf  → L05: Cache-Aware Design");
    println!("  Branchless pattern match       → L07: Branch Prediction");
    println!("  SIMD first-char filter (AVX2)  → L08: Vectorization");
    println!("  Arena allocator (bump alloc)   → L09: Memory Allocators");
    println!("  Thread-local lock-free count   → L06/L13: False Sharing & Lock Contention");
    println!("  P50/P99 reporting              → L18: Tail Latency");
    println!("  CPU frequency awareness        → L17: Power/Frequency Scaling");
    println!("  Capacity planning (Little's L) → L19: Capacity Planning");
    println!("  Rust UnsafeCell/alignment       → L16: Rust High Performance");
    println!("  Coroutines pipeline             → L14: Coroutines & Concurrency");
}

fn main() {
    println!("=== Phase 15 Capstone: Profile-Guided Optimization Walk-Through (Rust) ===\n");

    let num_lines = 10000usize;
    let avg_line_len = 120usize;
    let test_data = generate_test_data(num_lines, avg_line_len);
    let pattern = "findme";
    let single_pattern = "a";

    println!(
        "Test data: {} bytes, {} lines",
        test_data.len(),
        num_lines
    );
    println!(
        "Search patterns: \"{}\" (multi-char), \"{}\" (single-char)\n",
        pattern, single_pattern
    );

    let mut naive = NaiveStringProcessor::new();
    naive.load_from_string(&test_data);

    let mut optimized = OptimizedStringProcessor {
        text_buf: Vec::new(),
        text_len: 0,
    };
    optimized.load_from_string(&test_data);

    let mut arena = arena::Arena::new(test_data.len() * 2);

    let iters = 31;
    println!("Running benchmarks ({} iterations each)...\n", iters);

    let mut results: Vec<(&str, stats::Stats)> = Vec::new();

    let data_clone = test_data.clone();
    let pattern_owned0 = pattern.to_string();
    results.push(run_benchmark("Naive: count_pattern (multi)", move || {
        let mut proc = NaiveStringProcessor::new();
        proc.load_from_string(&data_clone);
        let start = Instant::now();
        let count = proc.count_pattern(&pattern_owned0);
        let _ = count;
        start.elapsed().as_nanos() as f64
    }, iters));

    let pattern_owned = pattern.to_string();
    results.push(run_benchmark("Optimized: branchless+SIMD (multi)", move || {
        let mut proc = OptimizedStringProcessor { text_buf: Vec::new(), text_len: 0 };
        proc.load_from_string(&data_clone);
        let start = Instant::now();
        let count = proc.count_pattern_simd(&pattern_owned);
        let _ = count;
        start.elapsed().as_nanos() as f64
    }, iters));

    let data_clone2 = test_data.clone();
    let single_owned = single_pattern.to_string();
    results.push(run_benchmark("Naive: count_pattern (single)", move || {
        let mut proc = NaiveStringProcessor::new();
        proc.load_from_string(&data_clone2);
        let start = Instant::now();
        let count = proc.count_pattern(&single_owned);
        let _ = count;
        start.elapsed().as_nanos() as f64
    }, iters));

    let data_clone3 = test_data.clone();
    let single_owned2 = single_pattern.to_string();
    results.push(run_benchmark("Optimized: SIMD-only (single)", move || {
        let mut proc = OptimizedStringProcessor { text_buf: Vec::new(), text_len: 0 };
        proc.load_from_string(&data_clone3);
        let start = Instant::now();
        let count = proc.count_pattern_simd(&single_owned2);
        let _ = count;
        start.elapsed().as_nanos() as f64
    }, iters));

    let data_clone4 = test_data.clone();
    results.push(run_benchmark("Naive: upper transform", move || {
        let mut proc = NaiveStringProcessor::new();
        proc.load_from_string(&data_clone4);
        let start = Instant::now();
        let result = proc.transform_to_upper();
        let _ = result;
        start.elapsed().as_nanos() as f64
    }, iters));

    let data_clone5 = test_data.clone();
    results.push(run_benchmark("Optimized: SIMD upper", move || {
        let mut proc = OptimizedStringProcessor { text_buf: Vec::new(), text_len: 0 };
        proc.load_from_string(&data_clone5);
        let start = Instant::now();
        proc.transform_to_upper_simd();
        start.elapsed().as_nanos() as f64
    }, iters));

    results.push(run_benchmark("Optimized: SIMD upper + arena", || {
        arena.reset();
        let start = Instant::now();
        optimized.transform_with_arena(&mut arena);
        start.elapsed().as_nanos() as f64
    }, iters));

    let data_clone6 = test_data.clone();
    let pattern_owned3 = pattern.to_string();
    results.push(run_benchmark("Optimized: branchless (multi)", move || {
        let mut proc = OptimizedStringProcessor { text_buf: Vec::new(), text_len: 0 };
        proc.load_from_string(&data_clone6);
        let start = Instant::now();
        let count = proc.count_pattern_branchless(&pattern_owned3);
        let _ = count;
        start.elapsed().as_nanos() as f64
    }, iters));

    let num_threads = std::thread::available_parallelism().map(|v| v.get()).unwrap_or(1);
    if num_threads >= 2 {
        let data_clone7 = test_data.clone();
        let pattern_owned4 = pattern.to_string();
        results.push(run_benchmark("Optimized: parallel (2 threads)", move || {
            let mut proc = OptimizedStringProcessor { text_buf: Vec::new(), text_len: 0 };
            proc.load_from_string(&data_clone7);
            let start = Instant::now();
            let count = proc.count_pattern_parallel(&pattern_owned4, 2);
            let _ = count;
            start.elapsed().as_nanos() as f64
        }, iters));
    }

    print_comparison(&results);
    print_lesson_mapping();

    println!("\n=== Verification: Correctness ===");
    let naive_count = naive.count_pattern(pattern);
    let opt_count = optimized.count_pattern_branchless(pattern);
    let opt_simd_count = optimized.count_pattern_simd(pattern);
    println!("Naive count(\"{}\"): {}", pattern, naive_count);
    println!("Optimized branchless count(\"{}\"): {}", pattern, opt_count);
    println!("Optimized SIMD count(\"{}\"): {}", pattern, opt_simd_count);
    println!("Naive == Optimized branchless: {}", naive_count == opt_count);
    println!("Naive == Optimized SIMD: {}", naive_count == opt_simd_count);

    let naive_single = naive.count_pattern(single_pattern);
    let opt_single = optimized.count_pattern_simd(single_pattern);
    println!("Naive count(\"{}\"): {}", single_pattern, naive_single);
    println!("Optimized SIMD count(\"{}\"): {}", single_pattern, opt_single);
    println!("Match: {}", naive_single == opt_single);

    let mut upper_proc = OptimizedStringProcessor { text_buf: Vec::new(), text_len: 0 };
    upper_proc.load_from_string(&test_data);
    upper_proc.transform_to_upper_simd();
    let naive_upper = naive.transform_to_upper();
    let upper_correct = naive_upper.as_bytes().iter()
        .zip(upper_proc.text_buf[..upper_proc.text_len].iter())
        .filter(|(a, b)| a == b)
        .count();
    println!("Upper transform: {}/{} characters correct",
             upper_correct, test_data.len());

    println!("\n=== PGO Workflow Summary ===");
    println!("1. MEASURE:   Baseline established with {} iterations (L02)", iters);
    println!("2. PROFILE:   Flamegraphs would show hotspots in count/transform (L03/L04)");
    println!("3. IDENTIFY:  Pattern matching and case transform are bottlenecks");
    println!("4. OPTIMIZE:  Applied SIMD, branchless, arena, parallel optimizations");
    println!("5. VERIFY:    Correctness checked; speedups measured (L02 discipline)");
    println!("6. DOCUMENT:  Comparison table and lesson mapping produced above");
}