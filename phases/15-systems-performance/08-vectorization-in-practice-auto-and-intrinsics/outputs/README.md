# Outputs — Vectorization in Practice

This directory contains the reusable artifact for Lesson 08: Vectorization in Practice.

## Artifact

**`simd_reference.md`** — A quick-reference card for SIMD programming, covering:

- x86 SIMD register summary (SSE, AVX, AVX2, AVX-512 widths and lane counts)
- Common intrinsics cheat sheet (load/store, arithmetic, compare/mask, gather/scatter)
- Auto-vectorization checklist (conditions, diagnostics flags)
- Alignment rules (required alignment per register width, best practices)
- Reduction patterns (horizontal sum for SSE, AVX, AVX-512)
- Rust SIMD quick reference (feature detection, intrinsics, safe SIMD)

Use this reference when writing or debugging vectorized code in C++ or Rust.