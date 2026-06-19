//! echo_server.rs — Multi-client TCP echo server (thread-per-connection)
//! Phase 09 — Computer Networks, Lesson 10
//!
//! Listens on port 8080, accepts connections, echoes data back.
//! Each client gets its own thread. Ctrl-C for graceful shutdown.
//!
//! Build:  rustc echo_server.rs -o echo_server
//!         (or use cargo with a Cargo.toml)
//! Run:    ./echo_server
//! Test:   echo "hello" | nc localhost 8080

use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

fn handle_client(stream: TcpStream) {
    let addr = match stream.peer_addr() {
        Ok(a) => a.to_string(),
        Err(_) => "unknown".to_string(),
    };
    println!("  [+] Client connected: {}", addr);

    let mut reader = stream.try_clone().expect("clone stream");
    let mut writer = stream;

    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => {
                println!("  [-] Client disconnected: {}", addr);
                break;
            }
            Ok(n) => {
                if let Err(e) = writer.write_all(&buf[..n]) {
                    eprintln!("  write error to {}: {}", addr, e);
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                eprintln!("  read error from {}: {}", addr, e);
                break;
            }
        }
    }
}

fn main() -> io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        eprintln!("\nShutting down.");
        std::process::exit(0);
    })
    .expect("failed to set Ctrl-C handler");

    let listener = TcpListener::bind("0.0.0.0:8080")?;
    listener.set_nonblocking(false)?;
    println!("Echo server listening on 0.0.0.0:8080");

    let mut handles = Vec::new();

    for stream in listener.incoming() {
        if !running.load(Ordering::SeqCst) {
            break;
        }
        match stream {
            Ok(stream) => {
                let handle = thread::spawn(|| handle_client(stream));
                handles.push(handle);
            }
            Err(e) => {
                eprintln!("accept error: {}", e);
            }
        }
    }

    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}

// Minimal version without ctrlc crate (for standalone compilation with rustc):
//
// use std::io::{self, Read, Write};
// use std::net::{TcpListener, TcpStream};
// use std::thread;
//
// fn handle_client(mut stream: TcpStream) {
//     let addr = stream.peer_addr().unwrap();
//     let mut buf = [0u8; 4096];
//     loop {
//         match stream.read(&mut buf) {
//             Ok(0) | Err(_) => break,
//             Ok(n) => { let _ = stream.write_all(&buf[..n]); }
//         }
//     }
// }
//
// fn main() -> io::Result<()> {
//     let listener = TcpListener::bind("0.0.0.0:8080")?;
//     println!("Listening on 8080");
//     for stream in listener.incoming() {
//         if let Ok(stream) = stream {
//             thread::spawn(|| handle_client(stream));
//         }
//     }
//     Ok(())
// }
