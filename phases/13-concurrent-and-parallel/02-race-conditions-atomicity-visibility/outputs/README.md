# Race Condition Demo Suite

## Overview

This directory contains the outputs from the "Race Conditions, Atomicity, Visibility" lesson. The compiled binaries demonstrate the three fundamental hazards of shared-memory concurrency: data races, non-atomic operations, and visibility failures.

## Contents

- **C binary** (`race` after compilation): Demonstrates counter race, mutex fix, atomic fix, broken visibility, and fixed visibility.
- **Rust binary** (`main` after compilation): Demonstrates `Arc<Mutex<>>`, atomic counters with different orderings, and acquire/release visibility.
- **TSan logs** (if run with `-fsanitize=thread`): ThreadSanitizer's report of detected data races.

## Usage

```bash
# Build and run C version
gcc -O2 -pthread -std=c11 -o race ../code/main.c && ./race

# Build and run Rust version
rustc ../code/main.rs && ./main

# C version with ThreadSanitizer (race detection)
gcc -fsanitize=thread -O1 -g -pthread -std=c11 -o race_tsan ../code/main.c && ./race_tsan
```

## Expected Results

| Demo | Expected Output | Notes |
|------|----------------|-------|
| Counter Race | value < 2,000,000 | Varies per run, non-deterministic |
| Mutex Fix | 2,000,000 | Always correct |
| Atomic Fix | 2,000,000 | Always correct, faster than mutex |
| Visibility Broken | May hang or show incorrect data | Architecture-dependent |
| Visibility Fixed | Shows correct data (42) | Always correct |

## Reuse

The demo patterns in this suite can be used as a reference when debugging race conditions in later phases. The key patterns to remember:
- Unsynchronized shared `int` → data race (counter)
- Volatile does NOT fix races (only prevents compiler optimization)
- C11 `_Atomic` / Rust `AtomicUsize` → atomic RMW operations
- `memory_order_release` / `memory_order_acquire` → establishes happens-before
- Rust's type system prevents data races at compile time
