# Side-Channel Analysis Toolkit

**Author:** Phase 12 — Cryptography & Security, Lesson 19

## What It Is

A C-based side-channel analysis toolkit demonstrating four attack and defense techniques:

1. **Timing Attack on Variable-Time Memcmp** — Recover a 16-byte secret byte-by-byte using only execution time measurements on a naive comparison function.
2. **Constant-Time Comparison** — XOR-OR based comparison whose timing is independent of mismatch position.
3. **Flush+Reload Cache Attack** — Detect which memory address a victim accessed by measuring cache line reload times after flushing.
4. **Spectre v1 Gadget** — Demonstrates the bounds-check bypass structure; shows how speculative execution can leak out-of-bounds data through cache state.

## How to Compile & Run

Requires an x86-64 CPU (RDTSC and CLFLUSH instructions are mandatory).

```bash
cd code
gcc -O2 -o sidechannel main.c
./sidechannel
```

**Compiler flags:** `-O2` is important — lower optimization levels may produce code where the timing difference is dominated by function-call overhead rather than comparison logic. Higher levels (`-O3`) are fine but may unroll loops and change the timing profile.

**Turbo boost:** Disable CPU frequency scaling and turbo boost for reproducible results:

```bash
# Linux — set performance governor (requires root)
sudo cpupower frequency-set -g performance
```

On macOS, the default performance governor is usually adequate.

## What It Ships

- `code/main.c` — Complete C source for all four demonstrations.
- Usage:
  - **Demo 1** prints the secret, shows the timing gradient, then recovers each byte with accuracy reporting.
  - **Demo 2** verifies constant-time correctness and shows flat timing independent of mismatch position.
  - **Demo 3** calibrates cache hit/miss timing, then runs 25 Flush+Reload attack trials with accuracy.
  - **Demo 4** sets up the Spectre v1 gadget and probes for leakage.

## Expected Output Highlights

```
Cache miss:   276 cycles
Cache hit:    52 cycles
Differential: 224 cycles (5.3x)

Flush+Reload attack (25 trials):
  trial  0: victim=142  spy=142  OK
  trial  1: victim= 31  spy= 31  OK
  ...
Accuracy: 24/25 (96%)
```

## Connection to Capstone

The Phase 12 capstone requires building a TLS 1.3 library and a mini-CTF toolkit. Side-channel knowledge is essential because:

- CTF challenges frequently include timing oracles, padding oracles, and cache attacks — you need to recognize and exploit these.
- Your TLS library's cryptographic primitives must be constant-time; the timing attack demo shows you how to verify this.
- The mini-CTF's "break the server" challenges often involve cache side-channels on shared hardware.

This toolkit serves as a reference for building and testing constant-time code throughout the capstone.

## Limitations

- **Spectre demo** may not leak on modern CPUs with IBRS, STIBP, and microcode mitigations (post-2018). The code structure is correct and matches the original POC; the educational value is in understanding the gadget pattern rather than reliable exploitation.
- Timing measurements depend on CPU frequency, cache state, and OS scheduling. Run multiple times for best results.
- Single-threaded — no cross-core/HT attack demonstration.
