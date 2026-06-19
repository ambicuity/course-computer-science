// Software Transactional Memory — Rust Examples
// ==============================================
// Uses the `stm` crate (https://crates.io/crates/stm).
// Build & run: cargo run
//
// Covers:
//   - TVar basics with atomically()
//   - Transaction::retry() for blocking
//   - orElse via Transaction::or_else() combinator
//   - Bank account transfer (composable)
//   - TChan-like channel using TVar + Vec
//   - Concurrent counter

use stm::{
    atomically, stm, TVar,
    transaction::{Transaction, TransactionResult},
};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// ===================================================================
// Helper: convenience alias
// ===================================================================

type Account = TVar<i64>;

// ===================================================================
// Example 1: Concurrent Counter (TVar basics)
// ===================================================================

fn counter_example() {
    println!("\n=== Example 1: Concurrent Counter ===");
    let counter = Arc::new(TVar::new(0i64));
    let workers = 10;
    let iterations = 1000;

    let mut handles = vec![];
    for _ in 0..workers {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..iterations {
                atomically(|| {
                    let val = c.read()?;
                    c.write(val + 1)
                })
                .unwrap();
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let result = atomically(|| counter.read()).unwrap();
    println!("  Final counter value: {}", result);
    println!("  Expected:            {}", workers * iterations);
}

// ===================================================================
// Example 2: retry — Blocking until condition is met
// ===================================================================

fn retry_example() {
    println!("\n=== Example 2: retry — Blocking ===");
    let flag = Arc::new(TVar::new(false));

    // Spawn a thread that sets flag to true after 1 second
    let f = Arc::clone(&flag);
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(1));
        atomically(|| f.write(true)).unwrap();
        println!("  [worker] Flag set to true");
    });

    // Wait for flag to become true using retry
    println!("  [main] Waiting for flag (via retry)...");
    atomically(|| -> Transaction<()> {
        stm::stm(move || {
            let val = flag.read()?;
            if val {
                TransactionResult::Ok(())
            } else {
                TransactionResult::Retry
            }
        })
    })
    .unwrap();
    println!("  [main] Done waiting!");
}

// ===================================================================
// Example 3: orElse — Fallback on retry
// ===================================================================

fn or_else_example() {
    println!("\n=== Example 3: orElse — Fallback ===");

    // Simulate TChan with TVar<Option<String>>
    let empty = Arc::new(TVar::new(None::<String>));
    let full = Arc::new(TVar::new(Some("hello from full".to_string())));

    let e = Arc::clone(&empty);
    let f = Arc::clone(&full);

    // Try to read from empty first; fall back to full
    let result: String = atomically(|| {
        // Attempt 1: read empty
        let tx1: Transaction<String> = stm::stm(move || match e.read()? {
            Some(val) => TransactionResult::Ok(val),
            None => TransactionResult::Retry,
        });

        // Attempt 2: read full (fallback)
        let tx2: Transaction<String> = stm::stm(move || match f.read()? {
            Some(val) => TransactionResult::Ok(val),
            None => TransactionResult::Retry,
        });

        tx1.or_else(tx2)
    })
    .unwrap();

    println!("  Read: {:?}", result);
}

// ===================================================================
// Example 4: Bank Account Transfer (Composable STM)
// ===================================================================

fn create_account(balance: i64) -> Account {
    TVar::new(balance)
}

/// Withdraw `amount` from account. Retries if insufficient funds.
fn withdraw(acc: &Account, amount: i64) -> Transaction<()> {
    let acc = acc.clone();
    stm::stm(move || {
        let bal = acc.read()?;
        if bal < amount {
            return TransactionResult::Retry;
        }
        acc.write(bal - amount)
    })
}

/// Deposit `amount` into account.
fn deposit(acc: &Account, amount: i64) -> Transaction<()> {
    let acc = acc.clone();
    stm::stm(move || {
        let bal = acc.read()?;
        acc.write(bal + amount)
    })
}

/// Transfer `amount` from one account to another.
/// Composes withdraw and deposit into a single transaction — deadlock-free!
fn transfer(from: &Account, to: &Account, amount: i64) -> Transaction<()> {
    let from = from.clone();
    let to = to.clone();
    stm::stm(move || {
        withdraw(&from, amount)
            .and_then(move |_| deposit(&to, amount))
            .exec()
    })
}

fn bank_transfer_example() {
    println!("\n=== Example 4: Bank Account Transfer ===");

    let alice = Arc::new(create_account(1000));
    let bob = Arc::new(create_account(500));

    println!("  Initial balances:");
    println!("    Alice: {}", atomically(|| alice.read()).unwrap());
    println!("    Bob:   {}", atomically(|| bob.read()).unwrap());

    // Spawn 5 concurrent transfers of 100 each from Alice to Bob
    let mut handles = vec![];
    for _ in 0..5 {
        let a = Arc::clone(&alice);
        let b = Arc::clone(&bob);
        handles.push(thread::spawn(move || {
            atomically(|| transfer(&a, &b, 100)).unwrap();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("\n  After 5x $100 transfers (Alice -> Bob):");
    println!("    Alice: {} (expected: 500)", atomically(|| alice.read()).unwrap());
    println!("    Bob:   {} (expected: 1000)", atomically(|| bob.read()).unwrap());
}

// ===================================================================
// Example 5: tryPay with orElse fallback
// ===================================================================

/// Try to pay from checking first; if insufficient, try credit.
/// If both fail, return false.
fn try_pay(
    checking: &Account,
    credit: &Account,
    amount: i64,
) -> Transaction<bool> {
    let checking = checking.clone();
    let credit = credit.clone();

    let tx1: Transaction<bool> = stm::stm(move || {
        let bal = checking.read()?;
        if bal < amount {
            return TransactionResult::Retry;
        }
        checking.write(bal - amount)?;
        TransactionResult::Ok(true)
    });

    let tx2: Transaction<bool> = stm::stm(move || {
        let bal = credit.read()?;
        if bal < amount {
            return TransactionResult::Retry;
        }
        credit.write(bal - amount)?;
        TransactionResult::Ok(true)
    });

    let tx_fallback: Transaction<bool> = stm::stm(|| TransactionResult::Ok(false));

    tx1.or_else(tx2).or_else(tx_fallback)
}

fn or_else_pay_example() {
    println!("\n=== Example 5: orElse Payment Fallback ===");

    let checking = Arc::new(create_account(50));
    let credit = Arc::new(create_account(5000));

    println!("  Initial:");
    println!("    Checking: {}", atomically(|| checking.read()).unwrap());
    println!("    Credit:   {}", atomically(|| credit.read()).unwrap());

    // Try to pay $100 — insufficient in checking, falls back to credit
    let paid = atomically(|| try_pay(&checking, &credit, 100)).unwrap();
    println!("  Payment of $100 succeeded? {}", paid);

    println!("  After payment:");
    println!("    Checking: {}", atomically(|| checking.read()).unwrap());
    println!("    Credit:   {}", atomically(|| credit.read()).unwrap());
}

// ===================================================================
// Example 6: STM-based channel (simplified TChan)
// ===================================================================

struct StmChan<T> {
    items: TVar<Vec<T>>,
}

impl<T: Send + 'static> StmChan<T> {
    fn new() -> Self {
        StmChan {
            items: TVar::new(Vec::new()),
        }
    }

    fn write(&self, item: T) -> Transaction<()> {
        let items = self.items.clone();
        stm::stm(move || {
            let mut vec = items.read()?;
            vec.push(item);
            items.write(vec)
        })
    }

    fn read(&self) -> Transaction<T> {
        let items = self.items.clone();
        stm::stm(move || {
            let mut vec = items.read()?;
            if vec.is_empty() {
                return TransactionResult::Retry;
            }
            let item = vec.remove(0);
            items.write(vec)?;
            TransactionResult::Ok(item)
        })
    }
}

fn stm_chan_example() {
    println!("\n=== Example 6: STM Channel (Producer-Consumer) ===");
    let chan = Arc::new(StmChan::new());
    let num_items = 10;

    // Producer
    let c1 = Arc::clone(&chan);
    let p = thread::spawn(move || {
        for i in 0..num_items {
            atomically(|| c1.write(format!("msg-{}", i))).unwrap();
            thread::sleep(Duration::from_millis(50));
        }
        println!("  [producer] Done producing");
    });

    // Consumer
    let c2 = Arc::clone(&chan);
    let con = thread::spawn(move || {
        let mut count = 0;
        for _ in 0..num_items {
            let msg = atomically(|| c2.read()).unwrap();
            count += 1;
            thread::sleep(Duration::from_millis(100));
        }
        println!("  [consumer] Done consuming. total: {}", count);
    });

    p.join().unwrap();
    con.join().unwrap();
}

// ===================================================================
// Example 7: Concurrent increment with contention
// ===================================================================

fn contention_example() {
    println!("\n=== Example 7: High-Contention Counter ===");
    let counter = Arc::new(TVar::new(0i64));
    let threads = 20;
    let increments = 5000;

    let mut handles = vec![];
    for _ in 0..threads {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..increments {
                atomically(|| -> Transaction<()> {
                    let val = c.read()?;
                    c.write(val + 1)
                })
                .unwrap();
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let result = atomically(|| counter.read()).unwrap();
    println!("  Threads: {}, increments/thread: {}", threads, increments);
    println!("  Result: {} (expected: {})", result, threads * increments);
}

// ===================================================================
// Main
// ===================================================================

fn main() {
    println!("Software Transactional Memory — Rust Examples");
    println!("==============================================");

    counter_example();
    retry_example();
    or_else_example();
    bank_transfer_example();
    or_else_pay_example();
    stm_chan_example();
    contention_example();

    println!("\nAll examples completed.");
}
