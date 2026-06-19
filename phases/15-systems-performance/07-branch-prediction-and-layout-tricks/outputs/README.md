# Outputs — Branch Prediction & Layout Tricks

This directory contains the reusable artifact for Lesson 07.

## Artifact: `branch_layout_reference.md`

A quick-reference card covering:
- **Branchless patterns** — cmov equivalents, lookup tables, arithmetic, partitioning
- **likely/unlikely syntax** — C++20 `[[likely]]`/`[[unlikely]]`, `__builtin_expect`, Linux kernel macros
- **Struct layout tips** — hot/cold splitting, cache-line alignment, field ordering
- **Decision flowchart** — when to go branchless vs. keep branches
- **Compilation commands** — how to verify cmov emission and profile branch-misses

Keep this reference alongside your performance toolbox for every optimization pass.