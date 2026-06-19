// Reactor and Proactor Patterns — epoll, kqueue, io_uring
// Phase 13 — Concurrent & Parallel Computing
//
// Rust Tokio-based echo server demonstrating how Tokio wraps epoll/kqueue
// as a reactor under the hood.
//
// Tokio's I/O model:
//   Tokio's runtime contains a "reactor" (the I/O driver) that calls
//   epoll_wait (Linux) or kevent (macOS) in a loop. When a file descriptor
//   becomes ready, the reactor finds the associated Waker and schedules
//   the corresponding task. The task then performs the read/write.
//
//   This is a classic REACTOR pattern: readiness notification followed
//   by explicit I/O syscalls. Tokio does NOT use io_uring's proactor
//   model by default (though experimental support exists).
//
// Build & run:
//   cd code
//   # Create a minimal cargo project:
//   mkdir -p tokio_echo && cd tokio_echo
//   cargo init
//   echo 'tokio = { version = "1", features = ["full"] }' >> Cargo.toml
//   cp ../main.rs src/main.rs
//   cargo run --release
//
// Usage:
//   cargo run --release -- 8080 server
//   cargo run --release -- 8080 bench 10 100

use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

/// Global statistics collected across all connections.
#[derive(Default)]
struct Stats {
    connections: AtomicU64,
    bytes_echoed: AtomicU64,
    errors: AtomicU64,
    read_syscalls: AtomicU64,
    write_syscalls: AtomicU64,
}

/// Handle a single TCP connection: read from socket, write back the same data.
///
/// This is the echo handler. It demonstrates Tokio's async I/O:
///   socket.read(&mut buf).await  →  yields if no data ready yet
///   socket.write_all(&buf[..n]).await  →  yields if buffer full
///
/// Under the hood, Tokio registers the socket's fd with epoll. When the
/// reactor calls epoll_wait and gets a readiness notification, it wakes
/// the task that called read(). The task then issues the actual read()
/// syscall through the standard library.
async fn handle_connection(
    mut socket: TcpStream,
    peer: SocketAddr,
    stats: Arc<Stats>,
) -> io::Result<()> {
    stats.connections.fetch_add(1, Ordering::Relaxed);
    let mut buf = [0u8; 65536];

    loop {
        stats.read_syscalls.fetch_add(1, Ordering::Relaxed);
        match socket.read(&mut buf).await {
            Ok(0) => {
                return Ok(());
            }
            Ok(n) => {
                stats.bytes_echoed.fetch_add(n as u64, Ordering::Relaxed);
                stats.write_syscalls.fetch_add(1, Ordering::Relaxed);
                if socket.write_all(&buf[..n]).await.is_err() {
                    stats.errors.fetch_add(1, Ordering::Relaxed);
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionReset,
                        "write failed",
                    ));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Should not happen with Tokio's reactor model --
                // the reactor only wakes the task when data is ready.
                // If we get here, it's a kernel edge case.
                continue;
            }
            Err(_) => {
                stats.errors.fetch_add(1, Ordering::Relaxed);
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "read failed",
                ));
            }
        }
    }
}

/// Run the Tokio-based TCP echo server.
///
/// This binds a TcpListener, then loops calling accept(). Each accepted
/// connection is spawned as a separate Tokio task.
///
/// The accept + spawn pattern mimics the epoll single-threaded model but
/// with Tokio's work-stealing scheduler distributing tasks across cores:
///
///   Tokio worker threads:
///     Thread 0: epoll_wait → task1, task3, task5
///     Thread 1: epoll_wait → task2, task4
///
/// Each worker has its own epoll fd and reactor. Fds are distributed
/// across reactors using a load-balancing strategy.
async fn run_tokio_echo(port: u16, stats: Arc<Stats>) -> io::Result<()> {
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let listener = TcpListener::bind(addr).await?;
    println!(
        "[tokio] Echo server listening on {} (pid={})",
        addr,
        std::process::id()
    );

    let rt = tokio::runtime::Handle::current();
    println!(
        "[tokio] Runtime: {} worker threads, reactor={}",
        rt.metrics().num_workers(),
        if cfg!(target_os = "linux") {
            "epoll"
        } else if cfg!(target_os = "macos") {
            "kqueue"
        } else {
            "IOCP (Windows)"
        }
    );

    loop {
        let (socket, peer) = listener.accept().await?;
        let stats = Arc::clone(&stats);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, peer, stats).await {
                eprintln!("[tokio] connection error ({}): {}", peer, e);
            }
        });
    }
}

/// Print runtime metrics periodically.
async fn print_metrics_periodically(stats: Arc<Stats>, interval_secs: u64) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    loop {
        interval.tick().await;
        let conns = stats.connections.load(Ordering::Relaxed);
        let bytes = stats.bytes_echoed.load(Ordering::Relaxed);
        let errs = stats.errors.load(Ordering::Relaxed);
        let reads = stats.read_syscalls.load(Ordering::Relaxed);
        let writes = stats.write_syscalls.load(Ordering::Relaxed);
        println!(
            "[tokio] stats: conns={} bytes={} errors={} reads={} writes={}",
            conns, bytes, errs, reads, writes
        );
    }
}

/// Benchmark client: opens `nconnections` connections to the server and
/// sends `requests_per_conn` echo requests on each, measuring throughput.
async fn benchmark(
    port: u16,
    nconnections: usize,
    requests_per_conn: usize,
    timeout_secs: u64,
) -> io::Result<()> {
    let msg = b"hello";
    let mut handles = Vec::with_capacity(nconnections);
    let start = Instant::now();

    for conn_id in 0..nconnections {
        let handle = tokio::spawn(async move {
            let conn_start = Instant::now();
            let mut stream = match timeout(
                Duration::from_secs(5),
                TcpStream::connect(([127, 0, 0, 1], port)),
            )
            .await
            {
                Ok(Ok(s)) => s,
                _ => return (conn_id, Err(io::Error::new(io::ErrorKind::TimedOut, "connect timeout")), 0),
            };

            let mut buf = vec![0u8; msg.len()];
            let mut ops = 0;

            for _ in 0..requests_per_conn {
                if let Err(e) = stream.write_all(msg).await {
                    return (conn_id, Err(e), ops);
                }
                if let Err(e) = stream.read_exact(&mut buf).await {
                    return (conn_id, Err(e), ops);
                }
                if buf != msg {
                    return (
                        conn_id,
                        Err(io::Error::new(io::ErrorKind::InvalidData, "echo mismatch")),
                        ops,
                    );
                }
                ops += 1;
            }

            let _ = stream.shutdown().await;
            (conn_id, Ok(conn_start.elapsed()), ops)
        });
        handles.push(handle);
    }

    let mut total_ops = 0usize;
    let mut min_time = Duration::from_secs(3600);
    let mut max_time = Duration::ZERO;
    let mut errors = 0usize;

    for handle in handles {
        let (conn_id, result, ops) = handle.await.unwrap();
        total_ops += ops;
        match result {
            Ok(elapsed) => {
                if elapsed < min_time {
                    min_time = elapsed;
                }
                if elapsed > max_time {
                    max_time = elapsed;
                }
            }
            Err(e) => {
                errors += 1;
                eprintln!("[bench] connection {} error: {}", conn_id, e);
            }
        }
    }

    let elapsed = start.elapsed();
    let total_bytes = total_ops * msg.len() * 2;

    println!("\n=== Benchmark Results ===");
    println!("Connections:        {}", nconnections);
    println!("Requests/conn:      {}", requests_per_conn);
    println!("Total requests:     {}", total_ops);
    println!("Total bytes:        {} (read+write)", total_bytes);
    println!("Errors:             {}", errors);
    println!("Wall time:          {:?} ({:.3}s)", elapsed, elapsed.as_secs_f64());
    println!(
        "Throughput:         {:.0} req/s",
        total_ops as f64 / elapsed.as_secs_f64()
    );
    println!(
        "Bandwidth:          {:.1} MB/s",
        total_bytes as f64 / (1024.0 * 1024.0) / elapsed.as_secs_f64()
    );
    if errors == 0 {
        println!("Fastest conn:       {:?}", min_time);
        println!("Slowest conn:       {:?}", max_time);
        if nconnections > 1 {
            println!(
                "Avg conn time:      {:?}",
                (elapsed) / nconnections as u32
            );
        }
    }

    println!();

    // Comparison summary
    println!("=== Reactor vs Proactor Comparison ===");
    println!("Tokio (this benchmark) uses a reactor pattern:");
    println!("  epoll/kqueue → readiness notification → task wake → read/write");
    println!("Raw epoll (lesson Part 1):");
    println!("  epoll_wait → readiness → read/write (1 syscall + 2 syscalls)");
    println!("io_uring (lesson Part 2):");
    println!("  SQE submit → kernel does I/O → CQE completion (0-1 syscalls)");
    println!();
    println!("Key insight:");
    println!("  Reactor = 'ready' notification (you still do I/O)");
    println!("  Proactor = 'done' notification (kernel did I/O for you)");

    Ok(())
}

/// Entry point with subcommand dispatch.
///
/// Usage:
///   tokio_echo <port> server            — run echo server
///   tokio_echo <port> bench <n> <r>     — run benchmark
#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(8080);
    let mode = args.get(2).map(|s| s.as_str()).unwrap_or("server");

    match mode {
        "server" => {
            let stats = Arc::new(Stats::default());
            let stats_clone = Arc::clone(&stats);

            // Print metrics in the background
            let metrics_handle = tokio::spawn(print_metrics_periodically(Arc::clone(&stats), 5));

            // Run the echo server
            let server_handle = tokio::spawn(async move {
                run_tokio_echo(port, stats_clone).await
            });

            // Wait for Ctrl+C
            tokio::signal::ctrl_c().await?;
            println!("\n[tokio] Shutdown signal received.");

            server_handle.abort();
            metrics_handle.abort();

            let final_bytes = stats.bytes_echoed.load(Ordering::Relaxed);
            let final_errs = stats.errors.load(Ordering::Relaxed);
            println!(
                "[tokio] Final: {} bytes echoed, {} errors",
                final_bytes, final_errs
            );
            println!("[tokio] Shut down.");
        }
        "bench" | "client" => {
            let nconns: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10);
            let nreqs: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(100);
            let timeout_secs: u64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(30);
            benchmark(port, nconns, nreqs, timeout_secs).await?;
        }
        _ => {
            eprintln!("Usage: {} <port> <server|bench> [nconns [nreqs [timeout]]]", args[0]);
            eprintln!();
            eprintln!("  server                    Run echo server (default)");
            eprintln!("  bench <n> <r> [t]         Run benchmark: <n> connections,");
            eprintln!("                             <r> requests each, <t> sec timeout");
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo_single_request() {
        let stats = Arc::new(Stats::default());
        let stats_clone = Arc::clone(&stats);
        let server = tokio::spawn(async move {
            let _ = run_tokio_echo(9999, stats_clone).await;
        });
        tokio::time::sleep(Duration::from_millis(300)).await;

        let mut stream = TcpStream::connect("127.0.0.1:9999").await.unwrap();
        stream.write_all(b"hello").await.unwrap();
        let mut buf = vec![0u8; 5];
        stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"hello");

        server.abort();
    }

    #[tokio::test]
    async fn test_echo_multiple_messages() {
        let stats = Arc::new(Stats::default());
        let stats_clone = Arc::clone(&stats);
        let server = tokio::spawn(async move {
            let _ = run_tokio_echo(9998, stats_clone).await;
        });
        tokio::time::sleep(Duration::from_millis(300)).await;

        let mut stream = TcpStream::connect("127.0.0.1:9998").await.unwrap();
        let messages = vec!["a", "bb", "ccc", "hello world!"];

        for msg in &messages {
            stream.write_all(msg.as_bytes()).await.unwrap();
            let mut buf = vec![0u8; msg.len()];
            stream.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, msg.as_bytes());
        }

        server.abort();
    }

    #[tokio::test]
    async fn test_echo_concurrent_connections() {
        let stats = Arc::new(Stats::default());
        let stats_clone = Arc::clone(&stats);
        let server = tokio::spawn(async move {
            let _ = run_tokio_echo(9997, stats_clone).await;
        });
        tokio::time::sleep(Duration::from_millis(300)).await;

        let mut handles = vec![];
        for i in 0..5 {
            handles.push(tokio::spawn(async move {
                let mut stream =
                    TcpStream::connect("127.0.0.1:9997").await.unwrap();
                let msg = format!("hello from {}", i);
                stream.write_all(msg.as_bytes()).await.unwrap();
                let mut buf = vec![0u8; msg.len()];
                stream.read_exact(&mut buf).await.unwrap();
                assert_eq!(&buf, msg.as_bytes());
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        server.abort();
    }

    #[tokio::test]
    async fn test_echo_large_message() {
        let stats = Arc::new(Stats::default());
        let stats_clone = Arc::clone(&stats);
        let server = tokio::spawn(async move {
            let _ = run_tokio_echo(9996, stats_clone).await;
        });
        tokio::time::sleep(Duration::from_millis(300)).await;

        let large_msg = vec![0xABu8; 16384];
        let mut stream = TcpStream::connect("127.0.0.1:9996").await.unwrap();
        stream.write_all(&large_msg).await.unwrap();
        let mut buf = vec![0u8; 16384];
        stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(buf, large_msg);

        server.abort();
    }
}
