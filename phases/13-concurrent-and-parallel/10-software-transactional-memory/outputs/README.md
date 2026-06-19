# Software Transactional Memory — Artifacts

## Overview

This directory contains the artifacts produced by Phase 13, Lesson 10 (Software Transactional Memory).
These are self-contained reference snippets demonstrating composable concurrent operations using STM.

## Files

| File | Language | Description |
|------|----------|-------------|
| `haskell-transfer.hs` | Haskell | Composable bank account transfer using `Control.Concurrent.STM`. Demonstrates `TVar`, `atomically`, `retry`, and `orElse` patterns. |
| `rust-transfer.rs` | Rust | Equivalent bank transfer using the `stm` crate (`TVar`, `atomically`, `Transaction::or_else`). |

## Usage

### Haskell
```bash
runghc haskell-transfer.hs
```

### Rust
```bash
rustc rust-transfer.rs && ./rust-transfer
```

## Key Pattern

Both snippets demonstrate the core STM insight: small atomic operations compose.
`withdraw` and `deposit` are individually correct transactions. `transfer` composes
them into a larger transaction — no lock ordering, no deadlock, no intermediate state exposure.

```haskell
transfer from to amount = do
    withdraw from amount
    deposit to amount
```

This pattern — composing transactional operations into larger atomic units — is the
defining advantage of STM over lock-based synchronization.
