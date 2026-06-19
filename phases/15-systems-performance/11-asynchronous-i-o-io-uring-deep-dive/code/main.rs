//! Asynchronous I/O — io_uring Deep Dive
//! Phase 15 — Systems Programming & Performance
//!
//! Demonstrates io_uring file read with the `io_uring` crate and compares
//! against std::fs::read. Falls back to std-only on non-Linux platforms.
//!
//! Build:
//!   cargo build --release
//!
//! Run:
//!   ./target/release/iouring_demo [file_path]

#[cfg(target_os = "linux")]
use io_uring::{opcode, types, IoUring};

use std::env;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::time::Instant;

const BLOCK_SIZE: usize = 4096;
const QUEUE_DEPTH: u32 = 128;

fn sequential_read(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut f = fs::File::open(path)?;
    let meta = f.metadata()?;
    let file_size = meta.len() as usize;
    let blocks = file_size / BLOCK_SIZE;
    let blocks = if blocks == 0 { 1 } else { blocks };

    let mut buf = vec![0u8; BLOCK_SIZE];
    let start = Instant::now();

    for i in 0..blocks {
        f.seek(SeekFrom::Start((i * BLOCK_SIZE) as u64))?;
        f.read_exact(&mut buf)?;
    }

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    println!("  sequential: {:.3} ms for {} blocks", elapsed, blocks);
    Ok(())
}

#[cfg(target_os = "linux")]
fn iouring_read(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let meta = file.metadata()?;
    let file_size = meta.len() as usize;
    let blocks = file_size / BLOCK_SIZE;
    let blocks = if blocks == 0 { 1 } else { blocks };

    let mut ring = IoUring::new(QUEUE_DEPTH)?;
    let fd = types::Fd(file.as_raw_fd());

    let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(QUEUE_DEPTH as usize);
    for _ in 0..QUEUE_DEPTH {
        bufs.push(vec![0u8; BLOCK_SIZE]);
    }

    let start = Instant::now();
    let mut submitted: usize = 0;
    let mut completed: usize = 0;

    while completed < blocks {
        let mut batch: usize = 0;
        let submit_end = std::cmp::min(submitted + QUEUE_DEPTH as usize, blocks);

        while submitted < submit_end {
            let buf_idx = submitted % (QUEUE_DEPTH as usize);
            let offset = (submitted * BLOCK_SIZE) as u64;

            let read_e = opcode::Read::new(fd, bufs[buf_idx].as_mut_ptr(), BLOCK_SIZE as u32)
                .offset(offset)
                .build()
                .user_data(submitted as u64);

            unsafe {
                ring.submission().push(&read_e).map_err(|e| {
                    format!("push failed: {:?}", e)
                })?;
            }
            submitted += 1;
            batch += 1;
        }

        ring.submit()?;
        ring.completion().drain().for_each(|_| { completed += 1; });

        if batch == 0 {
            break;
        }
    }

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    println!("  io_uring:   {:.3} ms for {} blocks", elapsed, blocks);
    drop(file);
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn iouring_read(_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("  (io_uring not available on this platform — skipped)");
    Ok(())
}

#[cfg(target_os = "linux")]
fn iouring_write_demo(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let out_path = format!("{}.iouring_copy", path);
    let src = fs::File::open(path)?;
    let dst = fs::File::create(&out_path)?;
    let meta = src.metadata()?;
    let file_size = meta.len() as usize;

    let mut ring = io_uring::IoUring::new(QUEUE_DEPTH)?;
    let src_fd = types::Fd(src.as_raw_fd());
    let dst_fd = types::Fd(dst.as_raw_fd());

    let buf = vec![0u8; BLOCK_SIZE];
    let blocks = file_size / BLOCK_SIZE;
    let blocks = if blocks == 0 { 1 } else { blocks };

    println!("\nDemo: io_uring copy {} → {}", path, out_path);

    let start = Instant::now();

    for block_idx in 0..blocks {
        let offset = (block_idx * BLOCK_SIZE) as u64;

        let read_e = opcode::Read::new(src_fd, buf.as_ptr() as *mut u8, BLOCK_SIZE as u32)
            .offset(offset)
            .build()
            .user_data(block_idx as u64 * 2);

        unsafe {
            ring.submission().push(&read_e)?;
        }
        ring.submit()?;

        let cqe = ring.completion().next().expect("no completion");
        let bytes_read = cqe.result() as usize;
        if bytes_read < 0 {
            return Err(format!("read error at block {}", block_idx).into());
        }

        let write_e = opcode::Write::new(dst_fd, buf.as_ptr(), bytes_read as u32)
            .offset(offset)
            .build()
            .user_data(block_idx as u64 * 2 + 1);

        unsafe {
            ring.submission().push(&write_e)?;
        }
        ring.submit()?;

        let _cqe = ring.completion().next().expect("no completion");
    }

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    println!("  io_uring copy: {:.3} ms for {} blocks", elapsed, blocks);

    drop(src);
    drop(dst);
    fs::remove_file(&out_path).ok();
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn iouring_write_demo(_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n(io_uring not available on this platform — skipping write demo)");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).unwrap_or_else(|| "/etc/hostname".into());

    let meta = fs::metadata(&path)?;
    let file_size = meta.len() as usize;
    println!("File: {} ({} bytes, {} blocks)\n",
             path, file_size, file_size / BLOCK_SIZE);

    println!("Benchmark 1: sequential std::fs read");
    sequential_read(&path)?;

    println!("\nBenchmark 2: io_uring batched read");
    iouring_read(&path)?;

    iouring_write_demo(&path)?;

    Ok(())
}

#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;