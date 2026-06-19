# outputs/README.md — Atomics, CAS, ABA Problem

## Artifact: Lock-Free Stack with Tagged Pointer

This lesson produces a reusable lock-free stack (Treiber stack)
with an ABA-prevention tagged pointer, in both Rust and C++.

### Rust (`code/main.rs`)

| Item | Description |
|------|-------------|
| `AtomicCounter` | Minimal FAA counter using `AtomicUsize::fetch_add`. |
| `LockFreeStack<T>` | CAS-based stack using `AtomicPtr<Node<T>>`. Unsafe-free usage via `Box::into_raw` / `Box::from_raw`. |
| `TaggedStack<T>` | ABA-safe version: embeds 3-bit version tag in `AtomicUsize`, increments on every successful CAS. |
| Key pattern | `loop { load + CAS }` — the standard lock-free retry loop. |

### C++ (`code/main.cpp`)

| Item | Description |
|------|-------------|
| `LockFreeStack` | Raw `std::atomic<Node*>` with CAS retry loop. |
| ABA demonstration | Deterministic bug reproduction using a single-slot node recycler. Shows A→B→A address cycle defeating naive CAS. |
| `TaggedStack` | Uses `std::atomic<uintptr_t>` with 3-bit version tag in the lowest pointer bits. |

### Usage in later phases

The tagged-pointer stack is the foundation for:

- **Lock-free work-stealing deque** (Phase 13 capstone) — extend the stack to a
  deque with push/pop from one end and steal from the other.
- **Lock-free queue** (Michael-Scott) — CAS on both head and tail, with a
  dummy sentinel node.
- **Memory reclamation** — swap the tagged pointer for epoch-based reclamation
  (`crossbeam-epoch`) when nodes hold large allocations.

### Build

```bash
# Rust
rustc code/main.rs -o build/lockfree_atomics
./build/lockfree_atomics

# C++
clang++ -std=c++17 -O2 -pthread code/main.cpp -o build/lockfree_atomics_cpp
./build/lockfree_atomics_cpp
```

### Key bindings (mental model)

```
        CAS(&p, old, new)
           │
           ├── success → p = new, return true
           └── failure → old = p, return false (retry)
```

```
        Tagged pointer = (address & ptr_mask) | (counter & tag_mask)
           │
           ├── push → CAS(head, pack(A, t), pack(N, t+1))
           └── pop  → CAS(head, pack(A, t), pack(A->next, t+1))
```
