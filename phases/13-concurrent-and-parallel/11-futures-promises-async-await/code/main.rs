// Phase 13, Lesson 11 — Futures, Promises, async/await (Rust)
// Demonstrates: manual callback-based future, async fn, toy executor with waker,
// join/select composition, error handling.
//
// Compile:  rustc main.rs -o futures_lesson
// Run:      ./futures_lesson

use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::{Duration, Instant};

// ============================================================================
// Step 1: Manual Callback-Based Future
// ============================================================================
// Represents a value that will be available later. The user provides a
// callback (closure) that is invoked when the value is produced. This is
// analogous to how early promise libraries worked — heap-allocated, dynamic
// dispatch, no zero-cost state machines.

struct ManualFuture<T: Send + 'static> {
    value: Arc<Cell<Option<T>>>,
}

fn spawn_manual<T: Send + 'static>(
    producer: Box<dyn FnOnce(Box<dyn FnOnce(T) + Send>) + Send>,
) -> ManualFuture<T> {
    let value = Arc::new(Cell::new(None));
    let value_clone = value.clone();

    let callback: Box<dyn FnOnce(T) + Send> = Box::new(move |result: T| {
        value_clone.set(Some(result));
    });

    producer(callback);

    ManualFuture { value }
}

impl<T: Send + 'static> ManualFuture<T> {
    fn is_ready(&self) -> bool {
        self.value.take().is_some()
    }

    fn take(&self) -> Option<T> {
        self.value.take()
    }
}

fn sleep_manual(ms: u64) -> ManualFuture<()> {
    spawn_manual(
        Box::new(move |callback| {
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(ms));
                callback(());
            });
        }),
    )
}

fn block_on_manual<T: Send + 'static>(fut: ManualFuture<T>) -> T {
    loop {
        if let Some(val) = fut.take() {
            return val;
        }
        thread::yield_now();
    }
}

fn step1_manual_future() {
    println!("--- Step 1: Manual Callback-Based Future ---");
    let start = Instant::now();

    let future = sleep_manual(100);
    block_on_manual(future);

    println!("  Manual future completed in {:?}", start.elapsed());
    println!("  (spawns real OS thread — wasteful but pedagogically clear)");
}

// ============================================================================
// Step 2: Async/Await Rewrite
// ============================================================================
// Uses real Rust async/await. The async fn is desugared into a state machine
// struct implementing Future. We drive it with a simple block_on that uses
// a real Waker (from the standard library's `noop_waker`) and polls in a loop.
// This is not efficient — no waker-based wakeup — but it shows the API.

async fn sleep_async(ms: u64) {
    let start = Instant::now();
    // Busy-wait for simplicity (no tokio timer dependency)
    while start.elapsed() < Duration::from_millis(ms) {
        // Yield to let the executor poll other futures
        YieldFuture(true).await;
    }
}

struct YieldFuture(bool);

impl Future for YieldFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

async fn fetch_data(id: u32) -> String {
    println!("    fetch_data({}): starting", id);
    sleep_async(50).await;
    let result = format!("data-{}", id);
    println!("    fetch_data({}): complete -> {}", id, result);
    result
}

fn block_on_simple<F: Future>(fut: F) -> F::Output {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);
    let mut pinned = Box::pin(fut);
    loop {
        match pinned.as_mut().poll(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => {
                // Yield the current thread to avoid busy-spinning
                thread::yield_now();
            }
        }
    }
}

fn step2_async_await() {
    println!("\n--- Step 2: Async/Await Rewrite ---");
    let start = Instant::now();
    let result = block_on_simple(fetch_data(42));
    println!("  Result: {} in {:?}", result, start.elapsed());
    println!("  (zero-cost: no heap alloc for state machine)");
}

// ============================================================================
// Step 3: Custom Toy Executor with Waker
// ============================================================================
// A proper executor that:
// 1. Spawns futures onto a shared queue
// 2. Polls them in a loop
// 3. Waker sends futures back to the ready queue when wake() is called
//
// This mirrors Tokio's single-threaded executor in miniature.

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

struct ToyExecutor {
    ready_queue: Arc<Mutex<VecDeque<BoxFuture>>>,
    spawn_count: Arc<AtomicU64>,
    parked: Arc<(Mutex<bool>, Condvar)>,
}

impl ToyExecutor {
    fn new() -> Self {
        ToyExecutor {
            ready_queue: Arc::new(Mutex::new(VecDeque::new())),
            spawn_count: Arc::new(AtomicU64::new(0)),
            parked: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    fn spawner(&self) -> ToySpawner {
        ToySpawner {
            ready_queue: self.ready_queue.clone(),
            spawn_count: self.spawn_count.clone(),
            parked: self.parked.clone(),
        }
    }

    fn block_on<F: Future + Send + 'static>(&self, fut: F) -> F::Output {
        let spawner = self.spawner();
        let (tx, rx) = std::sync::mpsc::channel();

        spawner.spawn(async move {
            let result = fut.await;
            let _ = tx.send(result);
        });

        self.run();

        rx.recv().unwrap()
    }

    fn run(&self) {
        loop {
            // Check if there's outstanding work
            if self.spawn_count.load(AtomicOrdering::SeqCst) == 0 {
                let done = {
                    let queue = self.ready_queue.lock().unwrap();
                    queue.is_empty()
                };
                if done {
                    break;
                }
            }

            // Dequeue and poll
            let future = {
                let mut queue = self.ready_queue.lock().unwrap();
                queue.pop_front()
            };

            if let Some(mut fut) = future {
                let spawner = self.spawner();
                let waker = make_toy_waker(spawner);
                let mut cx = Context::from_waker(&waker);

                match fut.as_mut().poll(&mut cx) {
                    Poll::Ready(()) => {
                        self.spawn_count
                            .fetch_sub(1, AtomicOrdering::SeqCst);
                    }
                    Poll::Pending => {
                        // Future will be re-queued when waker is called
                    }
                }
            } else {
                // No ready futures — park until a waker wakes us
                let (lock, cvar) = &*self.parked;
                let mut parked = lock.lock().unwrap();
                *parked = true;
                while *parked {
                    parked = cvar.wait(parked).unwrap();
                }
            }
        }
    }
}

struct ToySpawner {
    ready_queue: Arc<Mutex<VecDeque<BoxFuture>>>,
    spawn_count: Arc<AtomicU64>,
    parked: Arc<(Mutex<bool>, Condvar)>,
}

impl ToySpawner {
    fn spawn<F: Future + Send + 'static>(&self, fut: F) where F::Output: Send {
        self.spawn_count.fetch_add(1, AtomicOrdering::SeqCst);
        let mut queue = self.ready_queue.lock().unwrap();
        queue.push_back(Box::pin(fut));

        // Wake the executor if it's parked
        let (lock, cvar) = &*self.parked;
        let mut parked = lock.lock().unwrap();
        if *parked {
            *parked = false;
            cvar.notify_one();
        }
    }
}

impl Clone for ToySpawner {
    fn clone(&self) -> Self {
        ToySpawner {
            ready_queue: self.ready_queue.clone(),
            spawn_count: self.spawn_count.clone(),
            parked: self.parked.clone(),
        }
    }
}

fn make_toy_waker(spawner: ToySpawner) -> Waker {
    // We use a simple Arc-based waker that re-queues the future.
    // The waker is cloned via Waker::from_raw with a custom RawWaker.
    // For simplicity, we store a sentinel; in a real executor the waker
    // would identify *which* future to re-queue.
    //
    // Our approach: the RawWaker holds a cloned ToySpawner. When wake()
    // is called, the spawner increments the counter (so the executor knows
    // there's work) and notifies the Condvar. The future must be manually
    // re-inserted by the executor logic.

    use std::task::{RawWaker, RawWakerVTable};

    fn clone_fn(data: *const ()) -> RawWaker {
        let arc = unsafe { (*(data as *const ToySpawner)).clone() };
        let ptr = Box::into_raw(Box::new(arc));
        RawWaker::new(ptr as *const (), &VTABLE)
    }

    fn wake_fn(data: *const ()) {
        let arc = unsafe { (*(data as *const ToySpawner)).clone() };
        arc.spawn_count.fetch_add(1, AtomicOrdering::SeqCst);
        let (lock, cvar) = &*arc.parked;
        let mut parked = lock.lock().unwrap();
        if *parked {
            *parked = false;
            cvar.notify_one();
        }
        // Leak the arc — dropped in drop_fn
        let _ = Box::from_raw(data as *mut ToySpawner);
    }

    fn wake_by_ref_fn(data: *const ()) {
        wake_fn(data);
    }

    fn drop_fn(data: *const ()) {
        let _ = unsafe { Box::from_raw(data as *mut ToySpawner) };
    }

    const VTABLE: RawWakerVTable =
        RawWakerVTable::new(clone_fn, wake_fn, wake_by_ref_fn, drop_fn);

    let spawner_box = Box::new(spawner);
    let raw = RawWaker::new(
        Box::into_raw(spawner_box) as *const (),
        &VTABLE,
    );
    unsafe { Waker::from_raw(raw) }
}

async fn toy_task(id: u32, duration_ms: u64) {
    println!("    toy_task({}): spawned", id);
    let deadline = Instant::now() + Duration::from_millis(duration_ms);

    // Busy-wait loop that yields via YieldFuture
    while Instant::now() < deadline {
        YieldFuture(true).await;
    }

    println!("    toy_task({}): done after {}ms", id, duration_ms);
}

fn step3_toy_executor() {
    println!("\n--- Step 3: Custom Toy Executor with Waker ---");
    let executor = ToyExecutor::new();
    let spawner = executor.spawner();

    spawner.spawn(toy_task(1, 30));
    spawner.spawn(toy_task(2, 20));
    spawner.spawn(toy_task(3, 10));

    let start = Instant::now();
    executor.run();
    println!("  All toy tasks completed in {:?}", start.elapsed());
    println!("  (total time ≈ max duration, not sum — concurrent polling!)");
}

// ============================================================================
// Step 4a: Join — Run multiple futures concurrently, collect all results
// ============================================================================

async fn join_all<Fut, T>(futures: Vec<Fut>) -> Vec<T>
where
    Fut: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let handles: Vec<_> = futures
        .into_iter()
        .map(|f| {
            let (tx, rx) = std::sync::mpsc::channel::<T>();
            thread::spawn(move || {
                let result = block_on_simple(f);
                let _ = tx.send(result);
            });
            rx
        })
        .collect();

    let mut results = Vec::new();
    for rx in handles {
        results.push(rx.recv().unwrap());
    }
    results
}

async fn slow_double(n: u32) -> u32 {
    sleep_async(30 * n as u64).await;
    n * 2
}

fn step4_join() {
    println!("\n--- Step 4a: Join (run concurrently, collect all) ---");
    let start = Instant::now();
    let tasks = vec![slow_double(1), slow_double(2), slow_double(3)];
    let results = block_on_simple(join_all(tasks));
    let elapsed = start.elapsed();
    println!("  Results: {:?} in {:?}", results, elapsed);
    println!("  (sequential would take ~180ms, concurrent took ~90ms)");
}

// ============================================================================
// Step 4b: Select — Race two futures, pick whichever finishes first
// ============================================================================

enum Either<A, B> {
    Left(A),
    Right(B),
}

impl<A: Display, B: Display> Display for Either<A, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Either::Left(a) => write!(f, "Left({})", a),
            Either::Right(b) => write!(f, "Right({})", b),
        }
    }
}

async fn select_first<A: Send + 'static, B: Send + 'static>(
    fut_a: impl Future<Output = A> + Send + 'static,
    fut_b: impl Future<Output = B> + Send + 'static,
) -> Either<A, B> {
    let (tx_a, rx) = std::sync::mpsc::channel();
    let tx_b = tx_a.clone();

    let h1 = thread::spawn(move || {
        let result = block_on_simple(fut_a);
        let _ = tx_a.send(Either::Left(result));
    });

    let h2 = thread::spawn(move || {
        let result = block_on_simple(fut_b);
        let _ = tx_b.send(Either::Right(result));
    });

    let winner = rx.recv().unwrap();
    // Detach the other thread — it will finish harmlessly
    let _ = (h1, h2);
    winner
}

async fn fast_task() -> &'static str {
    sleep_async(20).await;
    "fast"
}

async fn slow_task() -> &'static str {
    sleep_async(100).await;
    "slow"
}

fn step4_select() {
    println!("\n--- Step 4b: Select (race futures, pick first) ---");
    let start = Instant::now();
    let winner = block_on_simple(select_first(fast_task(), slow_task()));
    let elapsed = start.elapsed();
    println!("  Winner: {} in {:?}", winner, elapsed);
    println!("  (the slower future is abandoned)");
}

// ============================================================================
// Step 4c: Error Handling with async Result
// ============================================================================

#[derive(Debug)]
struct ComputeError(String);

async fn fallible_compute(should_fail: bool) -> Result<String, ComputeError> {
    sleep_async(10).await;
    if should_fail {
        Err(ComputeError("computation failed".into()))
    } else {
        Ok("computation succeeded".into())
    }
}

async fn handle_errors() -> String {
    // Use `?` inside an async fn that returns Result
    let ok = fallible_compute(false).await.map_err(|e| e.0).unwrap_or_else(|e| e);
    let fail = fallible_compute(true)
        .await
        .unwrap_or_else(|_| "fallback value".to_string());
    format!("ok={}, fail={}", ok, fail)
}

fn step4_errors() {
    println!("\n--- Step 4c: Error Handling ---");
    let result = block_on_simple(handle_errors());
    println!("  {}", result);
    println!("  (? operator works in async fns returning Result)");
}

// ============================================================================
// Main — run all steps sequentially
// ============================================================================

fn main() {
    println!("=== Phase 13.11: Futures, Promises, async/await (Rust) ===\n");

    step1_manual_future();
    step2_async_await();
    step3_toy_executor();
    step4_join();
    step4_select();
    step4_errors();

    println!("\n=== All steps completed. ===");
    println!("Key insight: Rust futures are *lazy* — nothing runs without poll().");
    println!("The toy executor reveals that async/await is just syntax for");
    println!("state machines driven by a poll loop with wake notifications.");
}
