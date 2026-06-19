# Software Transactional Memory

> Software Transactional Memory (STM) brings database-style transactions to memory — composable, deadlock-free
> concurrent operations without explicit locks.

**Type:** Build
**Languages:** Haskell, Rust
**Prerequisites:** Phase 13 lessons 01–09 (threads, locks, condition variables, atomics)
**Time:** ~60 minutes

## Learning Objectives

- Understand the composability problem with locks and how STM solves it.
- Implement composable concurrent operations using `atomically`, `retry`, and `orElse`.
- Build a bank account transfer system that is deadlock-free by construction.
- Translate STM patterns between Haskell (`Control.Concurrent.STM`) and Rust (`stm` crate).
- Recognize when STM is appropriate versus lower-level atomics or locks.

## The Problem

Imagine you need to transfer money between two bank accounts. With locks, you write:

```python
lock_a.acquire()
lock_b.acquire()
a.balance -= 100
b.balance += 100
lock_b.release()
lock_a.release()
```

This works until you need to *compose* transfers. Suppose `transfer(a, b, 100)` calls `withdraw(a, 100)` then `deposit(b, 100)`. If `withdraw` and `deposit` each acquire and release locks internally, the composite operation is **not atomic** — another thread can see the money leave `a` before it arrives in `b`. If you try to fix this by acquiring locks externally, you get **lock ordering deadlocks**: thread 1 does `transfer(a, b)` while thread 2 does `transfer(b, a)`, and each holds one lock waiting for the other.

This is the **composability problem**: lock-based synchronization does not compose. You cannot take two correct concurrent operations and combine them into a larger correct concurrent operation without knowing their internal locking details.

Software Transactional Memory solves this. STM transactions compose: any two correct transactions, when combined, form a larger correct transaction.

## The Concept

### Transactional Memory

STM applies the core idea of database transactions to in-memory data:

- **Atomicity**: A transaction either commits all its writes or none of them.
- **Consistency**: The transaction sees a consistent snapshot of memory.
- **Isolation**: Partial effects of a concurrent transaction are never visible.

STM does **not** provide durability (that would require a disk). This is ACI, not ACID.

### Optimistic Concurrency

STM uses **optimistic concurrency**: a transaction reads values, builds a write set, and then **validates** at commit time. If another thread committed conflicting changes in the meantime, the transaction **aborts and retries**. This is the opposite of pessimistic locking (lock first, then read/write).

### Transaction Log

Each thread maintains a **transaction log** (also called a "transactional record" or "redo log") during a transaction:

- **Read log**: set of `TVar` addresses and their observed versions/values.
- **Write log**: set of `TVar` addresses and the new values to write on commit.

When `atomically` executes, reads check the write log first (read-your-writes), then the read log (avoid re-reading), then the global `TVar`. Writes go into the write log. On commit, the runtime validates all read entries still have their observed versions. If valid, the write log is flushed atomically to global memory.

### Conflict Detection

Two strategies exist, and Haskell's GHC implementation uses **commit-time** detection (optimistic):

| Strategy | When detected | Retry cost |
|---|---|---|
| **Encounter-time** (eager) | At each read/write | Low (fail fast) |
| **Commit-time** (lazy/optimistic) | At commit | High (wasted work) |

GHC STM uses commit-time: transactions run unhindered, validating only at commit. If validation fails, the entire transaction re-runs from scratch. This works well when conflicts are rare (the common case).

### The Three Core Operations

| Operation | Effect |
|---|---|
| `atomically` | Execute a transaction block; retry on conflict |
| `retry` | Abort the current transaction and block until a `TVar` touched so far changes |
| `orElse` | Try the first transaction; if it calls `retry`, run the second instead |

### retry and orElse

`retry` is what makes STM *compositional* for blocking. If a transaction reads a `TVar` and finds the value unsuitable (e.g., account balance too low), it calls `retry`. This *aborts* the transaction and blocks the calling thread until *any* `TVar` that was read during the transaction is written by another thread. At that point, the transaction re-runs automatically.

`orElse` provides an alternative: "try transaction A; if it calls `retry`, run transaction B instead."

```haskell
-- Try to withdraw 100; if insufficient funds, do nothing (return False)
withdrawOrSkip acc 100 `orElse` return False
```

This is impossible to express correctly with locks — adding timeout/fallback logic to a locked critical section requires restructuring the entire locking protocol.

## Build It

### Step 1: Haskell STM Basics

We begin with `TVar` (transactional variable), the core STM data type in Haskell.

```haskell
import Control.Concurrent.STM
import Control.Concurrent (forkIO, threadDelay)
import Control.Monad (forever)

main :: IO ()
main = do
  -- Create a TVar initialized to 0
  counter <- newTVarIO 0

  -- Spawn 10 threads, each incrementing the counter 1000 times
  let workers = 10
      increments = 1000
  threads <- replicateM workers . forkIO $
    replicateM_ increments $ atomically $ modifyTVar' counter (+1)

  -- Wait for all threads (simplified; real code uses MVars)
  threadDelay 1000000

  result <- atomically $ readTVar counter
  putStrLn $ "Counter: " ++ show result
  -- Output: Counter: 10000
```

Key points:
- `newTVarIO` creates a `TVar` in `IO` (outside a transaction).
- `atomically` wraps an `STM` action, runs it as a transaction.
- `modifyTVar'` applies a function to a `TVar` within `STM`.
- The runtime handles conflict detection and retry transparently.

#### retry in Action

```haskell
-- Block until the counter reaches at least 10
waitForCount :: TVar Int -> STM ()
waitForCount counter = do
  val <- readTVar counter
  when (val < 10) retry
```

`retry` does not spin. It blocks the thread efficiently until `counter` is written by another thread.

#### orElse in Action

```haskell
-- Read from left channel; if empty, read from right
readEither :: TChan a -> TChan a -> STM a
readEither left right = readTChan left `orElse` readTChan right
```

### Step 2: Bank Account Transfer (Composable with STM)

This is the canonical STM example. With locks, a transfer between two accounts requires careful lock ordering and is **not composable**. With STM, it's trivial and composes perfectly.

```haskell
module Main where

import Control.Concurrent.STM
import Control.Concurrent (forkIO, threadDelay)
import Control.Monad (replicateM_)

type Account = TVar Int

createAccount :: Int -> IO Account
createAccount balance = newTVarIO balance

-- Withdraw: composable STM operation
withdraw :: Account -> Int -> STM ()
withdraw acc amount = do
  bal <- readTVar acc
  when (bal < amount) retry  -- block until sufficient funds
  writeTVar acc (bal - amount)

-- Deposit: composable STM operation
deposit :: Account -> Int -> STM ()
deposit acc amount = modifyTVar' acc (+ amount)

-- Transfer: COMPOSES withdraw and deposit atomically!
transfer :: Account -> Account -> Int -> STM ()
transfer from to amount = do
  withdraw from amount
  deposit to amount

-- Balance check (non-blocking)
balance :: Account -> STM Int
balance = readTVar

main :: IO ()
main = do
  alice <- createAccount 1000
  bob   <- createAccount 500

  -- Spawn 5 concurrent transfers of 100 each
  replicateM_ 5 . forkIO $ atomically $ transfer alice bob 100

  -- Give threads time to finish
  threadDelay 1000000

  -- Check final balances atomically
  (aBal, bBal) <- atomically $ do
    a <- balance alice
    b <- balance bob
    return (a, b)

  putStrLn $ "Alice: " ++ show aBal  -- 500
  putStrLn $ "Bob:   " ++ show bBal  -- 1000
```

**Why this is impossible with locks:**
- `withdraw` and `deposit` are individually correct STM transactions.
- `transfer` composes them into a larger transaction — no lock ordering, no deadlock, no exposed intermediate state.
- `retry` blocks efficiently when funds are insufficient, and the block is *composable* (callers can use `orElse`).
- The runtime handles all conflict detection. If `transfer` conflicts with another transfer, it retries automatically.

**What happens if we try this with locks?**

```python
def transfer(from_acct, to_acct, amount):
    with from_acct.lock:    # lock ordering deadlock possible
        with to_acct.lock:
            from_acct.withdraw(amount)
            to_acct.deposit(amount)
```

If thread 1 calls `transfer(a, b, 100)` and thread 2 calls `transfer(b, a, 50)`, each holds one lock waiting for the other. Deadlock. You can fix this with global lock ordering (e.g., always lock by account ID), but that requires **external knowledge** and breaks composition.

#### Adding orElse for Fallback

```haskell
-- Try to pay; if insufficient funds, use credit card instead
tryPay :: Account -> Account -> Int -> STM Bool
tryPay checking credit amount = do
  withdraw checking amount
  return True
  `orElse` do
    withdraw credit amount
    return True
  `orElse` return False
```

Each `orElse` branch is a full transaction. If the first calls `retry` (insufficient funds in checking), the second runs (try credit card). If that also calls `retry`, the third returns `False`. The caller can compose this further:

```haskell
atomically $ do
  paid <- tryPay checking credit 50
  when paid $ logPayment 50
```

### Step 3: Rust STM (stm crate)

The Rust ecosystem provides the `stm` crate with semantics closely matching Haskell's STM.

Add to `Cargo.toml`:
```toml
[dependencies]
stm = "0.3"
```

```rust
use stm::{
    atomically, TVar,
    stm::{Transaction, TransactionResult},
};
use std::thread;
use std::time::Duration;

type Account = TVar<i64>;

fn create_account(balance: i64) -> Account {
    TVar::new(balance)
}

fn withdraw(acc: &Account, amount: i64) -> Transaction<()> {
    stm::stm(move || {
        let bal = acc.read()?;
        if bal < amount {
            return TransactionResult::Retry;
        }
        acc.write(bal - amount)
    })
}

fn deposit(acc: &Account, amount: i64) -> Transaction<()> {
    stm::stm(move || {
        let bal = acc.read()?;
        acc.write(bal + amount)
    })
}

fn transfer(from: &Account, to: &Account, amount: i64) -> Transaction<()> {
    stm::stm(move || {
        withdraw(from, amount).and_then(|_| deposit(to, amount))
    })
}

fn main() {
    let alice = create_account(1000);
    let bob = create_account(500);

    let mut handles = vec![];
    for _ in 0..5 {
        let alice_ref = alice.clone();
        let bob_ref = bob.clone();
        handles.push(thread::spawn(move || {
            atomically(|| transfer(&alice_ref, &bob_ref, 100));
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let a_bal = atomically(|| alice.read()).unwrap();
    let b_bal = atomically(|| bob.read()).unwrap();
    println!("Alice: {a_bal}"); // 500
    println!("Bob:   {b_bal}"); // 1000
}
```

**Note on Rust STM differences:**
- Rust's type system requires explicit `clone()` for `TVar` sharing across threads (unlike Haskell's garbage collector).
- `stm::stm()` creates a `Transaction` closure; `atomically()` runs it.
- `TransactionResult::Retry` in Rust corresponds to `retry` in Haskell.
- The Rust `stm` crate is less mature than GHC's STM; performance characteristics differ, but the semantics are equivalent.

## Use It

### Haskell's GHC STM

GHC's STM implementation is the gold standard. It is used in production at companies like Galois, Standard Chartered, and FP Complete. Key implementation details:

- **Runtime** (RTS): The GHC runtime handles STM at the scheduler level.
- **TVar representation**: Each `TVar` has a current value, a "modification" log entry, and a list of blocked transactions waiting on `retry`.
- **Commit protocol**: On commit, all read `TVar`s are validated. If any changed, abort and retry. Otherwise, atomically swap all write-log entries using hardware CAS on the TVar's "current value" field.
- **Wake-up**: When a `TVar` is committed, the runtime wakes all transactions blocked on that `TVar` via `retry`.

### Rust's stm crate

The `stm` crate (by Nikolai K. and contributors) implements the same semantics in pure Rust:
- No runtime required; it uses `std::sync::atomic` operations.
- `TVar` internally uses an `AtomicPtr` with a versioned value.
- Validates at commit time using compare-and-swap on version counters.
- The `retry` mechanism uses `std::sync::Condvar` for blocking (parking_thread).

**Comparison:**

| Aspect | Haskell GHC STM | Rust stm crate |
|---|---|---|
| Runtime support | Yes (RTS manages retry/blocking) | No (uses std primitives) |
| Performance | Mature, highly optimized | Younger, adequate for moderate use |
| retry blocking | Scheduler-level (efficient) | Condvar-based |
| Integration | Built into language | External crate |
| Safety type system | Runtime | Compile-time (Send + Sync) |

## Read the Source

- **GHC STM runtime**: [`rts/STM.c`](https://gitlab.haskell.org/ghc/ghc/-/blob/master/rts/STM.c) in the GHC source tree. This is the C implementation of STM commit logic, TVar invalidation, and blocking.
- **Haskell `Control.Concurrent.STM`**: [`libraries/base/Control/Concurrent/STM.hs`](https://gitlab.haskell.org/ghc/ghc/-/blob/master/libraries/base/Control/Concurrent/STM.hs) — the Haskell API surface.
- **Rust `stm` crate**: [`src/transaction.rs`](https://github.com/ntrrgc/stm-rs/blob/main/src/transaction.rs) — the transaction log and commit validation logic.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`outputs/README.md`** — Standalone annotated snippet demonstrating STM bank transfer in Haskell and Rust, with explanations of `atomically`, `retry`, and `orElse`. Drop this into any project that needs composable concurrent operations.

## Exercises

1. **Easy** — Write an STM-based counter that 20 threads increment concurrently 5000 times each. Verify the result is 100000. Now write the same with a `MVar`-based counter and compare code complexity.

2. **Medium** — Build a bounded buffer (producer-consumer queue) using `TChan`. Producers block via `retry` when the channel is full, consumers block when empty. Hint: `TChan` does not have a length limit natively — wrap it with a `TVar Int` for the count and use `retry` to enforce bounds.

3. **Hard** — Implement a concurrent hash map using STM where each bucket is a `TVar` holding a list of key-value pairs. Provide `insert`, `lookup`, and `delete` operations. Then extend it with an atomic `transferKey` operation that moves a key from one map to another — show that this composes without deadlock, which is impossible with per-bucket locks.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| **STM** | "Transactions for memory" | A concurrency control mechanism that groups reads and writes into atomic, isolated, consistent units — like database transactions but for RAM. No durability guarantee. |
| **TVar** | "Transactional variable" | A mutable memory cell that can only be read/written inside an `atomically` block. The runtime tracks reads and writes to detect conflicts at commit time. |
| **TMVar** | "Transactional MVar" | A transactional version of `MVar` — a single-slot channel that can be empty or full. `putTMVar` blocks (via `retry`) if full; `takeTMVar` blocks if empty. |
| **TChan** | "Transactional channel" | An unbounded, multi-producer multi-consumer channel for use inside `STM`. `writeTChan` appends; `readTChan` removes the head (blocking via `retry` if empty). |
| **atomically** | "Run this as a transaction" | The function that executes an `STM` action as a single atomic transaction. On conflict, it transparently retries from the beginning. |
| **retry** | "Abort and block until something changes" | Aborts the current transaction and blocks the calling thread until any previously read `TVar` is modified. On wake-up, the transaction re-runs. |
| **orElse** | "Try this, fallback to that" | Composes two transactions: executes the first; if it calls `retry`, discards its effects and runs the second. If both `retry`, the combined transaction calls `retry`. |
| **Transaction log** | "Scratch pad for the transaction" | Per-thread data structure tracking all reads (with expected versions) and pending writes for the current `atomically` block. Used to detect conflicts and commit atomically. |
| **Conflict** | "Two transactions touched the same TVar" | Occurs when two transactions concurrently read and write overlapping sets of `TVar`s, and at least one write conflicts. Resolved by aborting one transaction and retrying. |
| **Composability** | "Small correct pieces build larger correct pieces" | The property that combining two correct concurrent operations yields a correct concurrent operation. STM achieves this because transactions are isolated and retry transparently. |
| **Optimistic concurrency** | "Assume no conflict, detect on commit" | Strategy where transactions proceed without locking, checking for conflicts only at commit time. Contrast with pessimistic (lock-based) approaches. |

## Further Reading

- Tim Harris, Simon Marlow, Simon Peyton Jones, and Maurice Herlihy. *"Composable Memory Transactions"*. PPoPP 2005. The seminal paper describing Haskell's STM.
- Simon Peyton Jones. *"Beautiful Concurrency"*. In *Beautiful Code*, O'Reilly 2007. Agentle introduction to STM with Haskell.
- `stm` crate documentation: <https://docs.rs/stm/>
- `Control.Concurrent.STM` on Hackage: <https://hackage.haskell.org/package/stm>
- Maurice Herlihy and J. Eliot B. Moss. *"Transactional Memory: Architectural Support for Lock-Free Data Structures"*. ISCA 1993. The original hardware TM paper that inspired STM.
- Simon Marlow. *"Parallel and Concurrent Programming in Haskell"*. O'Reilly 2013. Chapters on STM.
