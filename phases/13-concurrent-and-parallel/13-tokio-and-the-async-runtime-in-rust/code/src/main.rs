// Tokio and the Async Runtime in Rust
// Phase 13 — Concurrent & Parallel Computing
//
// Build steps:
//   1. tokio::spawn — basic async tasks with JoinHandle
//   2. TCP echo server with TcpListener and per-connection tasks
//   3. Shared state with tokio::sync (Mutex, RwLock, mpsc, broadcast)
//   4. join! and select! — concurrent execution
//   5. Custom runtime builder (worker threads, event interval)

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tokio::time::{self, Duration};
use std::sync::Arc;

// ============================================================
// Step 1: tokio::spawn — Basic Async Tasks
// ============================================================
// Key concepts:
//   - tokio::spawn queues a future on the runtime's thread pool
//   - Returns JoinHandle<T> which you .await for the result
//   - Spawned tasks are NOT OS threads — stackless coroutines
//   - Tasks cooperate by yielding at .await points
//   - Panics in spawned tasks are caught and returned as JoinError
// ============================================================
async fn step1_basic_spawn() {
    println!("=== Step 1: tokio::spawn ===\n");

    // Spawn a computation task that yields periodically.
    let compute = tokio::spawn(async {
        let mut sum = 0u64;
        for i in 1..=100 {
            sum += i;
            if i % 50 == 0 {
                tokio::task::yield_now().await;
            }
        }
        sum
    });
    println!("  Sum 1..100 = {} (expect 5050)",
             compute.await.expect("join failed"));

    // Spawn multiple tasks concurrently. They run on the shared thread pool.
    let t1 = tokio::spawn(async {
        time::sleep(Duration::from_millis(20)).await;
        "task-1"
    });
    let t2 = tokio::spawn(async {
        time::sleep(Duration::from_millis(30)).await;
        "task-2"
    });
    let t3 = tokio::spawn(async {
        time::sleep(Duration::from_millis(10)).await;
        "task-3"
    });

    let start = std::time::Instant::now();
    let results = tokio::join!(t1, t2, t3);
    println!("  Concurrent: {}, {}, {} (took ~{}ms not 60ms sum)",
             results.0.as_ref().unwrap(),
             results.1.as_ref().unwrap(),
             results.2.as_ref().unwrap(),
             start.elapsed().as_millis());

    // Panic isolation: a panicking task does not crash the process.
    let panicky = tokio::spawn(async { panic!("intentional") });
    match panicky.await {
        Err(e) => println!("  Caught JoinError from panic: {}\n", e),
        _ => unreachable!(),
    }
}

// ============================================================
// Step 2: TCP Echo Server with TcpListener
// ============================================================
// Key concepts:
//   - TcpListener accepts connections asynchronously
//   - Task-per-connection model: each client gets a lightweight task
//   - AsyncReadExt / AsyncWriteExt provide async read/write
// ============================================================
async fn step2_tcp_echo_server() {
    println!("=== Step 2: TCP Echo Server ===\n");

    // Bind to port 0 so the OS assigns a free port (no conflicts).
    let listener = TcpListener::bind("127.0.0.1:0").await
        .expect("failed to bind");
    let addr = listener.local_addr().expect("failed to get addr");
    println!("  Listening on {}", addr);

    // Accept loop in its own task.
    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _peer) = listener.accept().await
                .expect("accept failed");
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                loop {
                    let n = stream.read(&mut buf).await
                        .expect("read error");
                    if n == 0 { return; }
                    stream.write_all(&buf[..n]).await
                        .expect("write error");
                }
            });
        }
    });

    time::sleep(Duration::from_millis(20)).await;

    // Single client: connect, send, receive, verify.
    println!("  --- Client tests ---");
    let mut c1 = TcpStream::connect(addr).await.expect("connect failed");
    let msg = b"Hello, Tokio!";
    c1.write_all(msg).await.expect("write failed");
    let mut buf = vec![0u8; 1024];
    let n = c1.read(&mut buf).await.expect("read failed");
    assert_eq!(&buf[..n], msg);
    println!("    Single: {} bytes echoed OK", n);

    // Concurrent clients to test the task-per-connection model.
    let mut handles = Vec::new();
    for i in 0..3 {
        let mut stream = TcpStream::connect(addr).await
            .expect("connect failed");
        handles.push(tokio::spawn(async move {
            let payload = format!("client-{}", i);
            stream.write_all(payload.as_bytes()).await.unwrap();
            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).await.unwrap();
            let reply = std::str::from_utf8(&buf[..n]).unwrap().to_string();
            assert_eq!(reply, payload);
            reply
        }));
    }
    for h in handles {
        println!("    Concurrent: \"{}\" echoed OK", h.await.unwrap());
    }

    server.abort();
    let _ = server.await;
    println!();
}

// ============================================================
// Step 3: Shared State with tokio::sync
// ============================================================
// Key concepts:
//   - Mutex: async-aware lock (hold across .await points)
//   - RwLock: concurrent reads, exclusive writes
//   - mpsc: multi-producer, single-consumer channel
//   - broadcast: fan-out channel (every receiver gets every message)
// ============================================================
async fn step3_shared_state() {
    println!("=== Step 3: Shared State with tokio::sync ===\n");

    // --- 3a: Mutex ---
    println!("  --- 3a: tokio::sync::Mutex ---");
    let counter = Arc::new(Mutex::new(0u64));
    let mut handles = Vec::new();
    for i in 0..10 {
        let c = Arc::clone(&counter);
        handles.push(tokio::spawn(async move {
            for _ in 0..100 {
                *c.lock().await += 1;
            }
            println!("    Worker {} done", i);
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    println!("    Counter = {} (expect 1000)\n", *counter.lock().await);

    // --- 3b: RwLock ---
    println!("  --- 3b: tokio::sync::RwLock ---");
    let data = Arc::new(RwLock::new(vec![0u64; 5]));

    // Writer runs first, acquires exclusive access.
    let w_data = Arc::clone(&data);
    let writer = tokio::spawn(async move {
        let mut guard = w_data.write().await;
        for v in guard.iter_mut() {
            *v += 1;
        }
        println!("    Writer updated data");
    });
    writer.await.unwrap();

    // Multiple concurrent readers — they all share the read lock.
    let mut readers = Vec::new();
    for i in 0..3 {
        let d = Arc::clone(&data);
        readers.push(tokio::spawn(async move {
            let guard = d.read().await;
            println!("    Reader {}: {:?}", i, *guard);
        }));
    }
    for h in readers {
        h.await.unwrap();
    }
    println!();

    // --- 3c: mpsc channel ---
    println!("  --- 3c: mpsc channel ---");
    let (tx, mut rx) = mpsc::channel::<u64>(32);
    let producer = tokio::spawn(async move {
        for i in 0..10 {
            tx.send(i).await.unwrap();
        }
    });
    let consumer = tokio::spawn(async move {
        let mut sum = 0u64;
        while let Some(v) = rx.recv().await {
            sum += v;
        }
        sum
    });
    producer.await.unwrap();
    let sum = consumer.await.unwrap();
    println!("    Sum via mpsc = {} (expect 45)\n", sum);

    // --- 3d: broadcast channel ---
    println!("  --- 3d: broadcast channel ---");
    let (tx, mut rx_a) = broadcast::channel::<i32>(16);
    let mut rx_b = tx.subscribe();

    tokio::spawn(async move {
        for i in 1..=3 {
            tx.send(i).unwrap();
            time::sleep(Duration::from_millis(5)).await;
        }
    });

    for _ in 0..3 {
        let va = rx_a.recv().await.expect("rx_a recv");
        let vb = rx_b.recv().await.expect("rx_b recv");
        println!("    rx_a={} rx_b={} (broadcast)", va, vb);
    }
    println!();
}

// ============================================================
// Step 4: join! and select! — Concurrent Execution
// ============================================================
// Key concepts:
//   - join!: run multiple futures, return when ALL complete
//   - select!: run multiple futures, return when FIRST completes (others cancelled)
// ============================================================
async fn step4_join_select() {
    println!("=== Step 4: join! and select! ===\n");

    // --- 4a: join! ---
    println!("  --- 4a: join! (all complete) ---");
    let start = std::time::Instant::now();
    let (a, b) = tokio::join!(
        async { time::sleep(Duration::from_millis(20)).await; "alpha" },
        async { time::sleep(Duration::from_millis(30)).await; "beta" },
    );
    println!("    join! = ({}, {}) in ~{}ms",
             a, b, start.elapsed().as_millis());

    // --- 4b: select! (first wins) ---
    println!("\n  --- 4b: select! (first wins, others cancelled) ---");
    let start = std::time::Instant::now();
    let winner = tokio::select! {
        v = async { time::sleep(Duration::from_millis(30)).await; "slow" } => v,
        v = async { time::sleep(Duration::from_millis(10)).await; "fast" } => v,
    };
    println!("    select! winner = \"{}\" in ~{}ms",
             winner, start.elapsed().as_millis());

    // --- 4c: select! as timeout ---
    println!("\n  --- 4c: select! as timeout ---");
    let start = std::time::Instant::now();
    let result = tokio::select! {
        v = async { time::sleep(Duration::from_millis(100)).await; "done" } => v,
        _ = time::sleep(Duration::from_millis(30)) => "TIMEOUT",
    };
    println!("    timeout = \"{}\" in ~{}ms\n",
             result, start.elapsed().as_millis());
}

// ============================================================
// Step 5: Custom Runtime Builder
// ============================================================
// Key concepts:
//   - tokio::runtime::Builder creates runtimes with explicit config
//   - new_multi_thread(): work-stealing thread pool
//   - new_current_thread(): single-threaded executor
//   - Parameters: worker_threads, thread_name, enable_io,
//     enable_time, event_interval, max_io_events_per_tick
// ============================================================
fn step5_custom_runtime() {
    println!("=== Step 5: Custom Runtime Builder ===\n");

    use tokio::runtime::Builder;

    // Multi-threaded runtime with explicit configuration.
    println!("  --- 5a: Multi-threaded runtime ---");
    println!("    4 workers, event_interval=61, thread_name=\"my-rt\"");
    let rt = Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("my-rt")
        .enable_io()
        .enable_time()
        .event_interval(61)
        .build()
        .expect("failed to build multi-threaded runtime");

    rt.block_on(async {
        let mut handles = Vec::new();
        for i in 0..8 {
            handles.push(tokio::spawn(async move {
                let tid = std::thread::current().id();
                println!("      Task {} on {:?}", i, tid);
                i * 2
            }));
        }
        let mut sum = 0u64;
        for h in handles {
            sum += h.await.unwrap() as u64;
        }
        println!("    Sum = {} (expect 56)", sum);
        // 8 tasks: i*2 for i=0..7 → 0+2+4+6+8+10+12+14 = 56
    });

    // Single-threaded runtime for comparison.
    println!("\n  --- 5b: Single-threaded runtime ---");
    let rt_st = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build single-threaded runtime");

    rt_st.block_on(async {
        println!("    Running on single worker thread");
        let a = tokio::spawn(async { 100 });
        let b = tokio::spawn(async { 200 });
        let va = a.await.unwrap();
        let vb = b.await.unwrap();
        println!("    {} + {} = {}", va, vb, va + vb);
    });

    println!("\n  (runtimes dropped — thread pools shut down)\n");
}

// ============================================================
// Entry Point
// ============================================================
// Note: We build the runtime manually instead of using #[tokio::main]
// so that Step 5 can create its own custom runtime without nesting.
fn main() {
    let start = std::time::Instant::now();

    // Default multi-threaded runtime for steps 1-4.
    let rt = tokio::runtime::Runtime::new().expect("failed to create default runtime");
    rt.block_on(async {
        step1_basic_spawn().await;
        step2_tcp_echo_server().await;
        step3_shared_state().await;
        step4_join_select().await;
    });
    // rt is dropped here — runtime context is cleared.

    // Step 5 builds its own runtime from scratch on this thread.
    step5_custom_runtime();

    println!("All steps completed in {:.2?}", start.elapsed());
}
