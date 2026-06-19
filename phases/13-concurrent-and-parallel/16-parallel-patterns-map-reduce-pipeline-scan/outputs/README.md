# Lesson 13.16 — Outputs

## Artifact: Parallel Patterns Reference Suite

This directory contains the compiled binaries and measurement data produced by the lesson code.

### Files

| File | Source | Description |
|------|--------|-------------|
| `parallel-patterns` | `code/` (Rust, cargo) | Rust binary with all four patterns + benchmark (compile with `cargo build --release` from `code/`) |
| `scan_bench.txt` | Rust benchmark output | Timing table for map, reduce, and scan at SMALL (100K) and MEDIUM (1M) sizes |
| `pipeline_verify.txt` | Rust + Python pipeline | Verification of pipeline correctness (expected sum matches actual) |

### How to Generate

```bash
# Rust (requires cargo + rayon)
cd ../code
cargo build --release
./target/release/parallel-patterns

# Python
python3 ../code/main.py
```

### Expected Output

The Rust binary prints four demo sections (map, reduce, pipeline, scan) followed by a benchmark table comparing sequential vs parallel times with speedup factors for each pattern at two data sizes. The Python script produces equivalent output using multiprocessing.Pool (including Hillis-Steele scan).

### Reuse

Use the map–reduce pattern as a template for data-parallel processing pipelines in later lessons. The pipeline pattern (channels + threads) is reusable for any producer–consumer or staged processing workflow. The scan algorithms are building blocks for parallel sort (radix sort), stream compaction, and worklist scheduling — all of which appear in later Phase 13 lessons (GPU computing, MPI).
