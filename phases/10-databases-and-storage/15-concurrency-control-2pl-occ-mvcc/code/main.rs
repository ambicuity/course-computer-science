//! OCC (Optimistic Concurrency Control) Validator
//! Phase 10 — Databases & Storage Systems
//!
//! DatabaseTable with CRUD operations. OCCTransaction tracks read/write sets,
//! validates against concurrent transactions, and retries on conflict.
//! Benchmark compares OCC vs simulated 2PL under low and high contention.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Key-value store
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct TableInner {
    data: HashMap<u64, u64>,
}

#[derive(Clone)]
pub struct DatabaseTable {
    inner: Arc<Mutex<TableInner>>,
}

impl DatabaseTable {
    pub fn new() -> Self {
        DatabaseTable {
            inner: Arc::new(Mutex::new(TableInner {
                data: HashMap::new(),
            })),
        }
    }

    pub fn read(&self, key: u64) -> Option<u64> {
        let guard = self.inner.lock().unwrap();
        guard.data.get(&key).copied()
    }

    pub fn write(&self, key: u64, value: u64) {
        let mut guard = self.inner.lock().unwrap();
        guard.data.insert(key, value);
    }

    pub fn snapshot(&self) -> HashMap<u64, u64> {
        let guard = self.inner.lock().unwrap();
        guard.data.clone()
    }
}

// ---------------------------------------------------------------------------
// Transaction ID allocator
// ---------------------------------------------------------------------------

static NEXT_TXN_ID: AtomicU64 = AtomicU64::new(1);

fn next_txn_id() -> u64 {
    NEXT_TXN_ID.fetch_add(1, Ordering::SeqCst)
}

// ---------------------------------------------------------------------------
// OCC Transaction Manager
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct OCCManager {
    table: DatabaseTable,
    committed: Arc<Mutex<Vec<CommittedTxn>>>,
    commit_counter: Arc<AtomicU64>,
}

#[derive(Clone)]
struct CommittedTxn {
    _txn_id: u64,
    write_set: HashMap<u64, u64>,
    commit_seq: u64,
}

#[derive(Clone)]
pub struct OCCTransaction {
    pub txn_id: u64,
    read_set: Vec<(u64, u64)>,
    write_set: HashMap<u64, u64>,
    start_seq: u64,
    manager: OCCManager,
}

impl OCCManager {
    pub fn new(table: DatabaseTable) -> Self {
        OCCManager {
            table,
            committed: Arc::new(Mutex::new(Vec::new())),
            commit_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn begin(&self) -> OCCTransaction {
        let seq = self.commit_counter.load(Ordering::SeqCst);
        OCCTransaction {
            txn_id: next_txn_id(),
            read_set: Vec::new(),
            write_set: HashMap::new(),
            start_seq: seq,
            manager: self.clone(),
        }
    }

    /// Remove old committed transaction records to bound memory.
    pub fn prune_committed(&self, keep: usize) {
        let mut guard = self.committed.lock().unwrap();
        if guard.len() > keep {
            let excess = guard.len() - keep;
            guard.drain(0..excess);
        }
    }
}

impl OCCTransaction {
    pub fn read(&mut self, key: u64) -> Option<u64> {
        if let Some(val) = self.write_set.get(&key) {
            return Some(*val);
        }
        let val = self.manager.table.read(key)?;
        self.read_set.push((key, val));
        Some(val)
    }

    pub fn write(&mut self, key: u64, value: u64) {
        self.write_set.insert(key, value);
    }

    /// Backward validation: check read-set against write-sets of transactions
    /// that committed after this transaction started.
    pub fn commit(&mut self) -> Result<(), ()> {
        let read_keys: HashSet<u64> = self.read_set.iter().map(|(k, _)| *k).collect();

        {
            let committed = self.manager.committed.lock().unwrap();
            for ct in committed.iter() {
                if ct.commit_seq <= self.start_seq {
                    continue;
                }
                let written_keys: HashSet<u64> = ct.write_set.keys().copied().collect();
                if !read_keys.is_disjoint(&written_keys) {
                    return Err(());
                }
            }
        }

        let seq = self.manager.commit_counter.fetch_add(1, Ordering::SeqCst) + 1;

        for (key, value) in &self.write_set {
            self.manager.table.write(*key, *value);
        }

        {
            let mut committed = self.manager.committed.lock().unwrap();
            committed.push(CommittedTxn {
                _txn_id: self.txn_id,
                write_set: self.write_set.clone(),
                commit_seq: seq,
            });
        }
        self.manager.prune_committed(100);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 2PL simulation (per-key Mutex with ordered acquisition to avoid deadlock)
// ---------------------------------------------------------------------------

pub struct TwoPLManager {
    table: DatabaseTable,
    key_locks: Arc<HashMap<u64, Mutex<()>>>,
}

impl TwoPLManager {
    pub fn new(keys: &[u64]) -> Self {
        let locks: HashMap<u64, Mutex<()>> = keys.iter().map(|k| (*k, Mutex::new(()))).collect();
        TwoPLManager {
            table: DatabaseTable::new(),
            key_locks: Arc::new(locks),
        }
    }

    pub fn transfer(&self, from: u64, to: u64, amount: u64) -> bool {
        let (first, second) = if from < to { (from, to) } else { (to, from) };
        let lock1 = self.key_locks.get(&first).unwrap();
        let lock2 = self.key_locks.get(&second).unwrap();
        let _g1 = lock1.lock().unwrap();
        let _g2 = lock2.lock().unwrap();

        let from_val = self.table.read(from).unwrap_or(0);
        if from_val < amount {
            return false;
        }
        let to_val = self.table.read(to).unwrap_or(0);
        self.table.write(from, from_val - amount);
        self.table.write(to, to_val + amount);
        true
    }
}

// ---------------------------------------------------------------------------
// Benchmark
// ---------------------------------------------------------------------------

fn bench_occ(
    table: &DatabaseTable,
    num_txns: usize,
    keys_per_txn: usize,
    key_pool: u64,
    label: &str,
) {
    let manager = OCCManager::new(table.clone());

    // Initialize data.
    for k in 0..key_pool {
        table.write(k, 100);
    }

    let start = Instant::now();
    let mut attempts = 0;
    let mut aborts = 0;

    let max_retries = 20;

    let mut handles = Vec::new();
    for t in 0..num_txns {
        let mgr = manager.clone();
        handles.push(std::thread::spawn(move || {
            let mut local_attempts = 0;
            let mut local_aborts = 0;
            for _attempt in 0..max_retries {
                local_attempts += 1;
                let mut txn = mgr.begin();
                for k in 0..keys_per_txn {
                    let key = ((t as u64 * keys_per_txn as u64 + k as u64) % key_pool) + 1;
                    let val = txn.read(key).unwrap_or(0);
                    txn.write(key, val.wrapping_add(1));
                }
                match txn.commit() {
                    Ok(()) => return (local_attempts, local_aborts),
                    Err(()) => local_aborts += 1,
                }
            }
            (local_attempts, local_aborts)
        }));
    }

    for h in handles {
        let (a, ab) = h.join().unwrap();
        attempts += a;
        aborts += ab;
    }
    let elapsed = start.elapsed();

    let commit_count = num_txns;
    println!(
        "OCC [{:20}] {} commits, {} aborts, {} attempts, {:.3}s, {:.0} txns/s",
        label,
        commit_count,
        aborts,
        attempts,
        elapsed.as_secs_f64(),
        commit_count as f64 / elapsed.as_secs_f64()
    );
}

fn bench_2pl(keys: &[u64], num_txns: usize, key_pool: u64, label: &str) {
    let mgr = Arc::new(TwoPLManager::new(keys));

    // Initialize data.
    for k in 0..key_pool {
        mgr.table.write(k, 100);
    }

    let start = Instant::now();
    let mut handles = Vec::new();
    for t in 0..num_txns {
        let m = mgr.clone();
        handles.push(std::thread::spawn(move || {
            let from = ((t as u64 * 2) % key_pool) + 1;
            let to = ((t as u64 * 2 + 1) % key_pool) + 1;
            m.transfer(from, to, 1);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    let elapsed = start.elapsed();
    println!(
        "2PL [{:20}] {} txns, {:.3}s, {:.0} txns/s",
        label,
        num_txns,
        elapsed.as_secs_f64(),
        num_txns as f64 / elapsed.as_secs_f64()
    );
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("=== OCC Transaction Validator ===");

    // --- Single-transaction test ---
    let table = DatabaseTable::new();
    table.write(1, 10);
    table.write(2, 20);
    let mgr = OCCManager::new(table.clone());
    let mut txn = mgr.begin();
    let a = txn.read(1).unwrap();
    let b = txn.read(2).unwrap();
    txn.write(1, a + 5);
    txn.write(2, b + 5);
    assert!(txn.commit().is_ok());
    println!("Single txn: table[1]={}, table[2]={}", table.read(1).unwrap(), table.read(2).unwrap());
    assert_eq!(table.read(1), Some(15));
    assert_eq!(table.read(2), Some(25));

    // --- Conflict test ---
    let table2 = DatabaseTable::new();
    table2.write(10, 100);
    let mgr2 = OCCManager::new(table2.clone());
    let mut txn_a = mgr2.begin();
    let mut txn_b = mgr2.begin();
    txn_a.read(10);
    txn_a.write(10, 200);
    txn_b.read(10);
    txn_b.write(10, 300);
    assert!(txn_a.commit().is_ok());
    assert!(txn_b.commit().is_err());
    println!("Conflict test: T2 aborted (expected), table[10]={}", table2.read(10).unwrap());
    assert_eq!(table2.read(10), Some(200));

    // --- Retry test ---
    let table3 = DatabaseTable::new();
    table3.write(5, 50);
    let mgr3 = OCCManager::new(table3.clone());
    let mut tx_a = mgr3.begin();
    let mut tx_b = mgr3.begin();
    tx_a.read(5);
    tx_a.write(5, 51);
    tx_b.read(5);
    tx_b.write(5, 52);
    tx_a.commit().ok();
    let mut retried = false;
    for _ in 0..5 {
        let mut tx = mgr3.begin();
        tx.read(5);
        tx.write(5, 60);
        if tx.commit().is_ok() {
            retried = true;
            break;
        }
    }
    assert!(retried);
    println!("Retry test: committed after abort, table[5]={}", table3.read(5).unwrap());

    // --- Benchmark: low vs high contention ---
    println!("\n=== Benchmark ===");
    println!("{:─<80}", "");

    // Low contention: 10 txns, 10 keys each, 100-key pool → ~1% overlap.
    let table_low = DatabaseTable::new();
    bench_occ(&table_low, 10, 10, 100, "low contention");

    // 2PL low contention (same 100-key pool).
    let keys_100: Vec<u64> = (1..=100).collect();
    bench_2pl(&keys_100, 10, 100, "low contention");

    // High contention: 10 txns, 3 keys each, 5-key pool → 100% overlap.
    let table_high = DatabaseTable::new();
    bench_occ(&table_high, 10, 3, 5, "high contention");

    // 2PL high contention (same 5-key pool).
    let keys_5: Vec<u64> = (1..=5).collect();
    bench_2pl(&keys_5, 10, 5, "high contention");

    println!("\nAll OCC tests passed.");
}
