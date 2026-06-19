//! Network Programming in Rust (async + tokio)
//! Phase 09 — Computer Networks
//!
//! Demonstrates:
//!   1. Async TCP echo server (Step 1)
//!   2. Timeouts with tokio::time::timeout (Step 2)
//!   3. Async TCP proxy with copy_bidirectional (Step 3)
//!
//! Run with: cargo run -- <mode>
//!   mode = "echo"    — start the echo server on port 8080
//!   mode = "timeout" — start the timeout demo (connects to echo server)
//!   mode = "proxy"   — start the TCP proxy on port 8081 → 8080
//!   mode = "client"  — start a test client that sends messages

use std::env;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

// ── Step 1: Async TCP Echo Server ──────────────────────────────────────────

async fn run_echo_server() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("[echo] Listening on 127.0.0.1:8080");

    loop {
        let (mut socket, addr) = listener.accept().await?;
        println!("[echo] Accepted connection from: {}", addr);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                match socket.read(&mut buf).await {
                    Ok(0) => {
                        println!("[echo] Connection closed by client: {}", addr);
                        break;
                    }
                    Ok(n) => {
                        println!(
                            "[echo] Received {} bytes from {}: {:?}",
                            n,
                            addr,
                            String::from_utf8_lossy(&buf[..n]).trim()
                        );
                        if socket.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[echo] Read error from {}: {}", addr, e);
                        break;
                    }
                }
            }
        });
    }
}

// ── Step 2: Timeout Demo ───────────────────────────────────────────────────

async fn run_timeout_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("[timeout] Connecting to echo server at 127.0.0.1:8080 ...");
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    println!("[timeout] Connected.");

    let hello = b"hello from timeout demo\n";
    stream.write_all(hello).await?;
    println!("[timeout] Sent: {}", String::from_utf8_lossy(hello).trim());

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    println!(
        "[timeout] Echo: {}",
        String::from_utf8_lossy(&buf[..n]).trim()
    );

    // Demonstrate timeout on a slow read:
    println!("[timeout] Waiting 3 seconds before sending more data...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    let second_msg = b"this should arrive quickly\n";
    stream.write_all(second_msg).await?;

    let n = stream.read(&mut buf).await?;
    println!(
        "[timeout] Echo: {}",
        String::from_utf8_lossy(&buf[..n]).trim()
    );

    // Demonstrate a timeout that fires: try reading when nothing is coming
    println!("[timeout] Now waiting 2 seconds for data that will never come...");
    let result = timeout(Duration::from_secs(2), async {
        let mut buf = vec![0u8; 4096];
        loop {
            stream.read(&mut buf).await?;
        }
        #[allow(unreachable_code)]
        Ok::<_, std::io::Error>(())
    })
    .await;

    match result {
        Ok(_) => println!("[timeout] (unexpected)"),
        Err(_) => println!("[timeout] Timeout fired after 2 seconds as expected!"),
    }

    Ok(())
}

// ── Step 3: Async TCP Proxy ────────────────────────────────────────────────

async fn run_proxy() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8081").await?;
    println!("[proxy] Listening on 127.0.0.1:8081, forwarding to 127.0.0.1:8080");

    let upstream = "127.0.0.1:8080".to_string();

    loop {
        let (mut downstream, addr) = listener.accept().await?;
        let upstream = upstream.clone();

        tokio::spawn(async move {
            println!("[proxy] Accepted connection from {}", addr);

            match TcpStream::connect(&upstream).await {
                Ok(mut upstream_stream) => {
                    println!("[proxy] Connected {} → upstream {}", addr, upstream);

                    match tokio::io::copy_bidirectional(&mut downstream, &mut upstream_stream)
                        .await
                    {
                        Ok((to_upstream, to_downstream)) => {
                            println!(
                                "[proxy] {} closed: {} bytes → upstream, {} bytes ← downstream",
                                addr, to_upstream, to_downstream
                            );
                        }
                        Err(e) => {
                            eprintln!("[proxy] Copy error for {}: {}", addr, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[proxy] Failed to connect to upstream {}: {}",
                        upstream, e
                    );
                }
            }
        });
    }
}

// ── Test Client ────────────────────────────────────────────────────────────

async fn run_client() -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::var("TARGET").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let mut stream = TcpStream::connect(&addr).await?;
    println!("[client] Connected to {}", addr);

    let messages = vec![
        "hello from async rust!",
        "tokio is great",
        "one more message for good measure",
    ];

    for msg in &messages {
        stream.write_all(msg.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        println!("[client] Sent: {}", msg);

        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).await?;
        let response = String::from_utf8_lossy(&buf[..n]).trim().to_string();
        println!("[client] Received: {}", response);

        if response != *msg {
            eprintln!("[client] Mismatch! Expected '{}', got '{}'", msg, response);
        }
    }

    println!("[client] All messages echoed successfully!");
    println!("[client] Now testing timeout: waiting for data that won't arrive...");

    // Demonstrate idle timeout: the server keeps the connection open, so read
    // would block forever. We use timeout to detect this.
    let result = timeout(Duration::from_secs(2), async {
        let mut buf = vec![0u8; 4096];
        loop {
            stream.read(&mut buf).await?;
        }
        #[allow(unreachable_code)]
        Ok::<_, std::io::Error>(())
    })
    .await;

    match result {
        Ok(_) => println!("[client] (unexpected — got data)"),
        Err(_) => println!("[client] Timeout: no data arrived in 2 seconds (expected)."),
    }

    Ok(())
}

// ── Entry Point ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let mode = if args.len() > 1 {
        args[1].as_str()
    } else {
        "echo"
    };

    match mode {
        "echo" => run_echo_server().await,
        "timeout" => run_timeout_demo().await,
        "proxy" => run_proxy().await,
        "client" => run_client().await,
        other => {
            eprintln!(
                "Unknown mode: {}. Use: echo, timeout, proxy, or client",
                other
            );
            Ok(())
        }
    }
}
