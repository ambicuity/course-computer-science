# Memory Model Litmus Test Suite

## What It Is

A portable litmus test suite that exercises hardware and compiler memory ordering
behavior. Implemented in both C++ and Rust.

## Files

| File | Language | What it demonstrates |
|------|----------|---------------------|
| `../code/main.cpp` | C++20 | SC Dekker, relaxed reordering, acquire/release message passing, fence-based ordering, IRIW |
| `../code/main.rs` | Rust 2021 | Same patterns using Rust's `Ordering` enum and `std::sync::atomic::fence` |

## How to Run

```bash
# C++
cd ../code
g++ -O2 -pthread -std=c++20 -o memmodel main.cpp && ./memmodel

# Rust
cd ../code
rustc -O main.rs && ./main
```

## What to Look For

1. **SC Dekker**: `(0,0)` should never occur — if it does, your hardware is not SC.
2. **Relaxed reordering**: Expect `(a=1,b=0)` on ARM. On x86 it may be absent or rare.
3. **Message passing**: Acquire/release should always succeed. Relaxed may fail on ARM.
4. **IRIW**: Disagreements between readers indicate hardware weaker than SC.

## Reuse in Later Phases

Use this suite in Phase 13 lessons 07–09 (atomics, lock-free data structures)
to verify that your lock-free implementations are correct on the target hardware.
Run it on any new architecture before deploying concurrent code.
