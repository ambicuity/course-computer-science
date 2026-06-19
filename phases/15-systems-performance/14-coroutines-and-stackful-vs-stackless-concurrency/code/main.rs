//! Coroutines and Stackful vs Stackless Concurrency
//! Phase 15 — Systems Programming & Performance
//!
//! Demonstrates Rust async/await, Pin, Future, Waker,
//! benchmarks async vs thread for I/O-bound work,
//! and implements the generator pattern.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker, RawWaker, RawWakerVTable};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

// ── Minimal Future implementation ──

struct Delay {
    duration: Duration,
    start: Option<Instant>,
    waker_set: bool,
}

impl Delay {
    fn new(duration: Duration) -> Self {
        Delay {
            duration,
            start: None,
            waker_set: false,
        }
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.start.is_none() {
            self.start = Some(Instant::now());
            let waker = cx.waker().clone();
            let duration = self.duration;
            thread::spawn(move || {
                thread::sleep(duration);
                waker.wake();
            });
        }
        if self.start.unwrap().elapsed() >= self.duration {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

// ── A simple block_on executor ──

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = future;
    let mut future = unsafe { Pin::new_unchecked(&mut future) };

    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => thread::sleep(Duration::from_millis(1)),
        }
    }
}

fn noop_waker() -> Waker {
    unsafe fn noop_clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    unsafe fn noop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop, noop, noop);
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

// ── Waking executor that actually handles async I/O ──

struct WakingExecutor {
    waker: Arc<Mutex<Option<Waker>>>,
}

impl WakingExecutor {
    fn new() -> Self {
        WakingExecutor {
            waker: Arc::new(Mutex::new(None)),
        }
    }

    fn block_on<F: Future>(&self, future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let waker = self.create_waker();
        let mut cx = Context::from_waker(&waker);

        loop {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(val) => return val,
                Poll::Pending => {
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    fn create_waker(&self) -> Waker {
        let waker_clone = self.waker.clone();
        unsafe fn clone(ptr: *const ()) -> RawWaker {
            let arc = Arc::from_raw(ptr as *const Mutex<Option<Waker>>);
            let cloned = arc.clone();
            std::mem::forget(arc);
            RawWaker::new(Arc::into_raw(cloned) as *const (), &VTABLE)
        }
        unsafe fn wake(ptr: *const ()) {
            let arc = Arc::from_raw(ptr as *const Mutex<Option<Waker>>);
            if let Some(w) = arc.lock().unwrap().take() {
                w.wake();
            }
            std::mem::forget(arc);
        }
        unsafe fn wake_by_ref(ptr: *const ()) {
            let arc = ManuallyDrop::new(Arc::from_raw(ptr as *const Mutex<Option<Waker>>));
            if let Some(w) = arc.lock().unwrap().as_ref() {
                w.wake_by_ref();
            }
        }
        unsafe fn drop(ptr: *const ()) {
            drop(Arc::from_raw(ptr as *const Mutex<Option<Waker>>));
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
        unsafe { Waker::from_raw(RawWaker::new(Arc::into_raw(waker_clone) as *const (), &VTABLE)) }
    }
}

// ── Generator pattern using async stream ──

async fn fibonacci_stream() -> i32 {
    let mut a = 0i32;
    let mut b = 1i32;
    for _ in 0..10 {
        let val = a;
        let tmp = a;
        a = b;
        b = tmp + b;
        Delay::new(Duration::from_millis(10)).await;
        return val; // simplified: real generators use yield
    }
    0
}

// Manual generator using a state machine
struct FibGenerator {
    a: i32,
    b: i32,
    count: i32,
}

impl FibGenerator {
    fn new() -> Self {
        FibGenerator { a: 0, b: 1, count: 0 }
    }
    fn next(&mut self) -> Option<i32> {
        if self.count >= 10 { return None; }
        let val = self.a;
        let tmp = self.a;
        self.a = self.b;
        self.b = tmp + self.b;
        self.count += 1;
        Some(val)
    }
}

struct PrimeGenerator {
    current: i32,
}

impl PrimeGenerator {
    fn new() -> Self {
        PrimeGenerator { current: 2 }
    }
    fn next(&mut self) -> Option<i32> {
        if self.current == 2 {
            self.current = 3;
            return Some(2);
        }
        loop {
            let n = self.current;
            self.current += 2;
            let mut is_prime = true;
            let mut d = 3;
            while d * d <= n {
                if n % d == 0 { is_prime = false; break; }
                d += 2;
            }
            if is_prime { return Some(n); }
        }
    }
}

// ── Demonstrate Pin necessity ──

async fn demonstrate_pin() {
    let data = String::from("hello");
    let reference = &data;
    Delay::new(Duration::from_millis(10)).await;
    println!("  After await, reference is still valid: {}", reference);
}

// ── Benchmarks ──

fn benchmark_threads(count: usize, delay_ms: u64) -> Duration {
    let start = Instant::now();
    let mut handles = Vec::with_capacity(count);
    for _ in 0..count {
        handles.push(thread::spawn(move || {
            thread::sleep(Duration::from_millis(delay_ms));
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    start.elapsed()
}

fn benchmark_async(count: usize, delay_ms: u64) -> Duration {
    let start = Instant::now();
    let mut handles = Vec::with_capacity(count);
    for _ in 0..count {
        handles.push(thread::spawn(move || {
            let executor = WakingExecutor::new();
            executor.block_on(async {
                Delay::new(Duration::from_millis(delay_ms)).await;
            })
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    start.elapsed()
}

fn show_stack_usage() {
    println!("\n=== Stack / Frame Size Comparison ===");
    println!("OS thread default stack:     8,388,608 bytes (8 MB, Linux default)");
    println!("OS thread minimal stack:      131,072 bytes (128 KB, PTHREAD_STACK_MIN)");
    println!("goroutine initial stack:         4,096 bytes (4 KB, Go runtime)");
    println!("C++20 coroutine frame:             ~48 bytes (varies by captured locals)");
    println!("Rust async state machine:          ~64 bytes (varies by captured locals)");
    println!("\nScaling to 100,000 concurrent tasks:");
    println!("  OS threads:    100,000 * 8 MB  = ~800 GB  (impossible)");
    println!("  goroutines:   100,000 * 4 KB  = ~400 MB  (feasible)");
    println!("  Rust async:   100,000 * 64 B  = ~6.4 MB  (trivial)");
}

fn main() {
    println!("=== Rust Async/Await Demo ===\n");

    println!("--- Fibonacci Generator ---");
    let mut fib = FibGenerator::new();
    print!("First 10 Fibonacci numbers: ");
    while let Some(val) = fib.next() {
        print!("{} ", val);
    }
    println!();

    println!("\n--- Prime Generator ---");
    let mut primes = PrimeGenerator::new();
    print!("First 10 primes: ");
    for _ in 0..10 {
        if let Some(p) = primes.next() {
            print!("{} ", p);
        }
    }
    println!();

    println!("\n--- Async Future with Delay ---");
    let executor = WakingExecutor::new();
    let result = executor.block_on(async {
        println!("  Starting async operation...");
        Delay::new(Duration::from_millis(50)).await;
        println!("  Async operation completed!");
        42i32
    });
    println!("  Result: {}", result);

    println!("\n--- Pin Demonstration ---");
    let executor = WakingExecutor::new();
    executor.block_on(demonstrate_pin());

    println!("\n--- Multiple async operations ---");
    let executor = WakingExecutor::new();
    let result = executor.block_on(async {
        let (a, b, c) = tokio_like_join!(
            async { Delay::new(Duration::from_millis(30)).await; 1i32 },
            async { Delay::new(Duration::from_millis(50)).await; 2i32 },
            async { Delay::new(Duration::from_millis(10)).await; 3i32 }
        );
        println!("  Results: a={}, b={}, c={}", a, b, c);
    });
}

// Manual "join" implementation since we avoid external crates
macro_rules! tokio_like_join {
    ($($fut:expr),* $(,)?) => {{
        let start = Instant::now();
        $(block_on($fut);)*
    }};
}

use std::mem::ManuallyDrop;

fn _multiple_async_demo() {
    println!("\n=== Async vs Thread Benchmark ===");

    for &count in &[100usize, 500] {
        let delay_ms: u64 = 1;
        println!("\n{} tasks x {}ms I/O each:", count, delay_ms);

        let thread_time = benchmark_threads(count, delay_ms);
        println!("  Threads:   {:?}", thread_time);

        let async_time = benchmark_async(count, delay_ms);
        println!("  Async:     {:?}", async_time);

        let ratio = thread_time.as_secs_f64() / async_time.as_secs_f64();
        println!("  Ratio:     {:.2}x", ratio);
    }

    show_stack_usage();

    println!("\n=== Key Takeaways ===");
    println!("1. Stackless coroutines (C++20, Rust async) use ~100x less memory than goroutines.");
    println!("2. Goroutines use ~2000x less memory than OS threads.");
    println!("3. Coroutine context switch: ~10 ns (function call).");
    println!("4. Thread context switch:    ~1-10 us (kernel syscall).");
    println!("5. Use async for I/O-bound, threads for CPU-bound.");
}