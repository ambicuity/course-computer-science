//! The Actor Model — Ping-Pong, Stateful Counter, Supervisor
//! Phase 13, Lesson 15
//!
//! Implements actors using `std::sync::mpsc` channels (no external crates).
//! Each actor runs in its own OS thread and owns its state.
//!
//! Run:  cargo run --release
//! Or:   rustc -O main.rs && ./main

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// ============================================================================
// Message types
// ============================================================================

/// Messages understood by the ping-pong actor.
#[derive(Debug, Clone)]
enum PingMsg {
    Ping { reply_to: Sender<PingReply> },
    Stop,
}

#[derive(Debug, Clone, Copy)]
enum PingReply {
    Pong,
}

/// Messages understood by the counter actor.
#[derive(Debug, Clone)]
enum CounterMsg {
    Increment(i64),
    Decrement(i64),
    Multiply(i64),
    Divide(i64),
    Set(i64),
    Get(Sender<i64>),
    Status,
    Stop,
}

/// Messages understood by the supervisor.
#[derive(Debug, Clone)]
enum SupMsg {
    Spawn,
    Stop,
}

/// A generic actor handle that can send any cloneable message.
struct ActorHandle<M: Clone + Send> {
    tx: Sender<M>,
}

impl<M: Clone + Send> ActorHandle<M> {
    fn send(&self, msg: M) -> Result<(), mpsc::SendError<M>> {
        self.tx.send(msg)
    }
}

impl<M: Clone + Send> Clone for ActorHandle<M> {
    fn clone(&self) -> Self {
        ActorHandle {
            tx: self.tx.clone(),
        }
    }
}

// ============================================================================
// Section 1 — Ping-Pong Actor
// ============================================================================

/// Creates a pong actor. Returns a handle.
/// The pong actor waits for Ping messages and replies with Pong.
fn spawn_pong() -> ActorHandle<PingMsg> {
    let (tx, rx) = mpsc::channel::<PingMsg>();
    thread::spawn(move || {
        for msg in rx {
            match msg {
                PingMsg::Ping { reply_to } => {
                    println!("[pong]  received Ping, sending Pong");
                    if reply_to.send(PingReply::Pong).is_err() {
                        break;
                    }
                }
                PingMsg::Stop => {
                    println!("[pong]  received Stop, exiting");
                    break;
                }
            }
        }
    });
    ActorHandle { tx }
}

/// Runs a ping actor in the current thread.
/// Sends `count` Ping messages to `pong`, expecting a reply each time.
fn run_ping(count: usize, pong: &ActorHandle<PingMsg>) {
    let (reply_tx, reply_rx) = mpsc::channel::<PingReply>();
    for i in 0..count {
        pong.send(PingMsg::Ping {
            reply_to: reply_tx.clone(),
        })
        .expect("pong actor died");
        match reply_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(PingReply::Pong) => {
                println!("[ping]  received Pong ({} left)", count - i - 1);
            }
            Err(e) => {
                eprintln!("[ping]  error waiting for pong: {:?}", e);
                break;
            }
        }
    }
    pong.send(PingMsg::Stop).ok();
}

// ============================================================================
// Section 2 — Stateful Counter Actor
// ============================================================================

/// Spawns a counter actor with initial value `init`.
/// Returns a handle that can send `CounterMsg` values.
fn spawn_counter(init: i64) -> ActorHandle<CounterMsg> {
    let (tx, rx) = mpsc::channel::<CounterMsg>();
    thread::spawn(move || {
        let mut count = init;
        for msg in rx {
            match msg {
                CounterMsg::Increment(n) => count += n,
                CounterMsg::Decrement(n) => count -= n,
                CounterMsg::Multiply(n) => count *= n,
                CounterMsg::Divide(n) => {
                    if n == 0 {
                        eprintln!("[counter] division by zero — crashing!");
                        panic!("division by zero");
                    }
                    count /= n;
                }
                CounterMsg::Set(n) => count = n,
                CounterMsg::Get(reply) => {
                    let _ = reply.send(count);
                }
                CounterMsg::Status => {
                    println!("[counter] current count = {}", count);
                }
                CounterMsg::Stop => {
                    println!("[counter] final count = {}", count);
                    break;
                }
            }
        }
    });
    ActorHandle { tx }
}

/// Same as `spawn_counter` but uses `catch_unwind` so the supervisor
/// can detect panics by checking if the channel receiver drops.
fn spawn_supervised_counter(init: i64) -> (
    ActorHandle<CounterMsg>,
    Arc<Mutex<Option<thread::JoinHandle<()>>>>,
) {
    let (tx, rx) = mpsc::channel::<CounterMsg>();
    let join_handle = Arc::new(Mutex::new(None));
    let handle_clone = join_handle.clone();
    let jh = thread::spawn(move || {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut count = init;
            for msg in rx {
                match msg {
                    CounterMsg::Increment(n) => count += n,
                    CounterMsg::Decrement(n) => count -= n,
                    CounterMsg::Multiply(n) => count *= n,
                    CounterMsg::Divide(n) => {
                        if n == 0 {
                            eprintln!("[supervised] division by zero — crashing!");
                            panic!("division by zero");
                        }
                        count /= n;
                    }
                    CounterMsg::Set(n) => count = n,
                    CounterMsg::Get(reply) => {
                        let _ = reply.send(count);
                    }
                    CounterMsg::Status => {
                        println!("[supervised] count = {}", count);
                    }
                    CounterMsg::Stop => {
                        println!("[supervised] final count = {}", count);
                        break;
                    }
                }
            }
        }));
    });
    *handle_clone.lock().unwrap() = Some(jh);
    (ActorHandle { tx }, join_handle)
}

// ============================================================================
// Section 3 — Supervisor
// ============================================================================

/// Spawns a supervisor that monitors a supervised counter and restarts it
/// on panic (detected when the channel closes).
fn spawn_supervisor() -> ActorHandle<SupMsg> {
    let (tx, rx) = mpsc::channel::<SupMsg>();
    thread::spawn(move || {
        let mut child: Option<ActorHandle<CounterMsg>> = None;
        let mut child_join: Option<Arc<Mutex<Option<thread::JoinHandle<()>>>>> = None;

        fn start_child(
        ) -> (
            ActorHandle<CounterMsg>,
            Arc<Mutex<Option<thread::JoinHandle<()>>>>,
        ) {
            println!("[supervisor] starting new supervised counter");
            spawn_supervised_counter(0)
        }

        // Start initial child
        {
            let (h, j) = start_child();
            child = Some(h);
            child_join = Some(j);
        }

        // In a real production supervisor, we'd monitor via linked processes
        // or JoinHandle::is_finished(). Here we periodically check the child
        // channel by sending a Status message — if the channel is broken,
        // the child panicked and we restart it.
        for msg in rx {
            match msg {
                SupMsg::Spawn => {
                    println!("[supervisor] manual restart requested");
                    let (h, j) = start_child();
                    child = Some(h);
                    child_join = Some(j);
                }
                SupMsg::Stop => {
                    if let Some(ref c) = child {
                        let _ = c.send(CounterMsg::Stop);
                    }
                    println!("[supervisor] stopping");
                    break;
                }
            }

            // Check if child is still alive (channel not dropped)
            if let Some(ref c) = child {
                if c.send(CounterMsg::Status).is_err() {
                    println!("[supervisor] child channel broken — restarting");
                    let (h, j) = start_child();
                    child = Some(h);
                    child_join = Some(j);
                }
            }
        }
    });
    ActorHandle { tx }
}

// ============================================================================
// Section 4 — Batch Operation on Counter
// ============================================================================

fn counter_send_batch(counter: &ActorHandle<CounterMsg>, ops: &[CounterMsg]) {
    for op in ops {
        counter.send(op.clone()).expect("counter died during batch");
    }
}

// ============================================================================
// Section 5 — Name Registry (Concurrent HashMap with actor semantics)
// ============================================================================

use std::collections::HashMap;

#[derive(Debug, Clone)]
enum RegistryMsg {
    Register {
        name: String,
        sender: Sender<Result<(), String>>,
    },
    Lookup {
        name: String,
        sender: Sender<Option<String>>,
    },
    Unregister {
        name: String,
    },
    List,
    Stop,
}

fn spawn_registry() -> ActorHandle<RegistryMsg> {
    let (tx, rx) = mpsc::channel::<RegistryMsg>();
    thread::spawn(move || {
        let mut map: HashMap<String, String> = HashMap::new();
        for msg in rx {
            match msg {
                RegistryMsg::Register { name, sender } => {
                    if map.contains_key(&name) {
                        let _ = sender.send(Err(format!("'{}' already registered", name)));
                    } else {
                        map.insert(name.clone(), format!("actor:{}", name));
                        let _ = sender.send(Ok(()));
                    }
                }
                RegistryMsg::Lookup { name, sender } => {
                    let result = map.get(&name).cloned();
                    let _ = sender.send(result);
                }
                RegistryMsg::Unregister { name } => {
                    map.remove(&name);
                }
                RegistryMsg::List => {
                    println!("[registry] entries:");
                    for (k, v) in &map {
                        println!("  {} => {}", k, v);
                    }
                }
                RegistryMsg::Stop => {
                    println!("[registry] stopping");
                    break;
                }
            }
        }
    });
    ActorHandle { tx }
}

// ============================================================================
// Section 6 — Multi-Actor Interaction
// ============================================================================

/// Demonstrates multiple actors coordinating via message passing.
fn demo_multi_actor() {
    println!();
    println!("----- 6. Multi-Actor Coordination -----");

    // Create two counters and a registry
    let counter_a = spawn_counter(0);
    let counter_b = spawn_counter(100);
    let registry = spawn_registry();

    // Register both counters
    let (reg_tx1, reg_rx1) = mpsc::channel();
    registry
        .send(RegistryMsg::Register {
            name: "counter-a".into(),
            sender: reg_tx1,
        })
        .ok();
    let _ = reg_rx1.recv();

    let (reg_tx2, reg_rx2) = mpsc::channel();
    registry
        .send(RegistryMsg::Register {
            name: "counter-b".into(),
            sender: reg_tx2,
        })
        .ok();
    let _ = reg_rx2.recv();

    // Send operations to both
    counter_a.send(CounterMsg::Increment(42)).ok();
    counter_b.send(CounterMsg::Decrement(20)).ok();

    // Query
    let (get_tx1, get_rx1) = mpsc::channel();
    counter_a.send(CounterMsg::Get(get_tx1)).ok();
    println!("[multi] counter-a = {}", get_rx1.recv().unwrap_or(-1));

    let (get_tx2, get_rx2) = mpsc::channel();
    counter_b.send(CounterMsg::Get(get_tx2)).ok();
    println!("[multi] counter-b = {}", get_rx2.recv().unwrap_or(-1));

    registry.send(RegistryMsg::List).ok();
    registry.send(RegistryMsg::Stop).ok();
    counter_a.send(CounterMsg::Stop).ok();
    counter_b.send(CounterMsg::Stop).ok();

    thread::sleep(Duration::from_millis(50));
}

// ============================================================================
// Section 7 — Message Passing Benchmark
// ============================================================================

fn demo_benchmark() {
    println!();
    println!("----- 7. Message Passing Benchmark -----");

    let (tx, rx) = mpsc::channel::<u64>();
    let counter = thread::spawn(move || {
        let mut total = 0u64;
        for n in rx {
            total += n;
        }
        println!("[bench] actor received total = {}", total);
    });

    let batch_size = 100_000;
    let start = std::time::Instant::now();
    for i in 0..batch_size {
        tx.send(i).unwrap();
    }
    drop(tx); // close channel so receiver exits
    let elapsed = start.elapsed();
    counter.join().ok();
    println!(
        "[bench] sent {} messages in {:?} ({:.0} msg/s)",
        batch_size,
        elapsed,
        batch_size as f64 / elapsed.as_secs_f64()
    );
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    println!("===== Actor Model Demo (Rust) =====");
    println!();

    // --- 1. Ping-Pong ---
    println!("----- 1. Ping-Pong Actors -----");
    let pong = spawn_pong();
    run_ping(4, &pong);
    thread::sleep(Duration::from_millis(100));
    println!();

    // --- 2. Stateful Counter ---
    println!("----- 2. Stateful Counter Actor -----");
    let counter = spawn_counter(0);
    counter.send(CounterMsg::Increment(10)).ok();
    counter.send(CounterMsg::Decrement(3)).ok();
    counter.send(CounterMsg::Multiply(2)).ok();
    counter.send(CounterMsg::Status).ok();
    let (get_tx, get_rx) = mpsc::channel();
    counter.send(CounterMsg::Get(get_tx)).ok();
    match get_rx.recv() {
        Ok(v) => println!("[main]    counter = {}", v),
        Err(e) => eprintln!("[main]    get failed: {:?}", e),
    }
    // Batch operations
    let ops = vec![
        CounterMsg::Increment(100),
        CounterMsg::Divide(7),
        CounterMsg::Decrement(1),
    ];
    counter_send_batch(&counter, &ops);
    let (get_tx2, get_rx2) = mpsc::channel();
    counter.send(CounterMsg::Get(get_tx2)).ok();
    match get_rx2.recv() {
        Ok(v) => println!("[main]    counter after batch = {}", v),
        Err(e) => eprintln!("[main]    batch get failed: {:?}", e),
    }
    counter.send(CounterMsg::Stop).ok();
    thread::sleep(Duration::from_millis(100));
    println!();

    // --- 3. Let It Crash (Supervisor) ---
    println!("----- 3. Let-It-Crash Supervisor Demo -----");
    let sup = spawn_supervisor();
    // The supervisor can't forward arbitrary CounterMsg directly
    // because it reads from a SupMsg channel. We demonstrate
    // the restart logic by dropping the child reference.
    // In practice, the supervisor would own the child handle
    // and detect panics via JoinHandle or channel drops.
    //
    // For a more direct demo, we spawn a counter, crash it,
    // and show the supervisor recovers:
    let (sc_handle, sc_join) = spawn_supervised_counter(0);
    sc_handle.send(CounterMsg::Increment(42)).ok();
    sc_handle.send(CounterMsg::Status).ok();
    println!("[main]    sending divide-by-zero to cause crash...");
    sc_handle.send(CounterMsg::Divide(0)).ok();
    // Give time for crash
    thread::sleep(Duration::from_millis(50));
    // The handle's channel is now broken because the thread panicked.
    // A real supervisor would detect this and restart.
    if sc_handle.send(CounterMsg::Increment(100)).is_err() {
        println!("[main]    child dead (channel closed) — supervisor would restart");

        // Simulate supervisor restart
        let (sc2_handle, _sc2_join) = spawn_supervised_counter(0);
        sc2_handle.send(CounterMsg::Increment(100)).ok();
        sc2_handle.send(CounterMsg::Status).ok();
        sc2_handle.send(CounterMsg::Stop).ok();
    }
    thread::sleep(Duration::from_millis(100));
    sup.send(SupMsg::Stop).ok();
    println!();

    // --- 4. Registry ---
    println!("----- 4. Name Registry Actor -----");
    let registry = spawn_registry();
    let (reg_tx, reg_rx) = mpsc::channel();
    registry
        .send(RegistryMsg::Register {
            name: "actor-alpha".into(),
            sender: reg_tx,
        })
        .ok();
    match reg_rx.recv() {
        Ok(Ok(())) => println!("[main]    registered actor-alpha"),
        _ => {}
    }
    // Try duplicate
    let (reg_tx2, reg_rx2) = mpsc::channel();
    registry
        .send(RegistryMsg::Register {
            name: "actor-alpha".into(),
            sender: reg_tx2,
        })
        .ok();
    match reg_rx2.recv() {
        Ok(Err(e)) => println!("[main]    duplicate rejected: {}", e),
        _ => {}
    }
    let (look_tx, look_rx) = mpsc::channel();
    registry
        .send(RegistryMsg::Lookup {
            name: "actor-alpha".into(),
            sender: look_tx,
        })
        .ok();
    match look_rx.recv() {
        Ok(Some(id)) => println!("[main]    found actor-alpha: {}", id),
        _ => {}
    }
    registry.send(RegistryMsg::List).ok();
    registry.send(RegistryMsg::Stop).ok();
    thread::sleep(Duration::from_millis(50));
    println!();

    // --- 5. Benchmark ---
    demo_benchmark();
    println!();

    // --- 6. Multi-Actor ---
    demo_multi_actor();
    println!();

    println!("===== Demo Complete =====");
}
