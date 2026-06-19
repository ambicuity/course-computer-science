# Compute Patterns Reference Card

This directory contains `compute_patterns.md` — a reusable reference card for GPU compute
shader patterns (reduction, prefix sum, sorting) in both CUDA and WGSL.

## Files

- **compute_patterns.md** — Quick-reference card with pseudocode for the three fundamental
  GPU compute patterns: parallel reduction, Blelloch prefix sum, and bitonic merge sort.
  Each pattern is shown in both CUDA C++ and WGSL with key annotations.

## How to Use

1. Keep `compute_patterns.md` open when implementing GPU algorithms in Phase 14 capstones.
2. Start from the reduction pattern for any "combine all values" problem (sum, max, min, any, all).
3. Start from the prefix sum pattern for stream compaction, sorting, or running totals.
4. Start from the bitonic sort pattern for GPU-side sorting of moderate-sized arrays.
5. Translate between CUDA and WGSL using the mapping table in the reference card.

## Why This Exists

These three patterns (reduce, scan, sort) form the foundation of GPU compute programming.
Nearly every non-trivial GPU algorithm uses one or more of them. This card gives you the
minimal correct implementation so you can focus on your algorithm, not on re-deriving
the thread indexing arithmetic.