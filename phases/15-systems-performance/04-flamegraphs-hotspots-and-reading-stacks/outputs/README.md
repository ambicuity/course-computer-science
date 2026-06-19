# Outputs — Flamegraphs, Hotspots, and Reading Stacks

This directory contains the reusable artifact for Lesson 04.

## Artifact: `flamegraph_guide.md`

A quick-reference card for flamegraph generation, reading, and troubleshooting. Covers:

- Complete generation pipeline for CPU, off-CPU, differential, and memory flamegraphs
- Visual guide for reading flamegraphs (axes, width, color, hotspots)
- Flamegraph type comparison table
- Troubleshooting missing frames (frame pointers, JIT symbols, kernel symbols)
- Where to get Brendan Gregg's FlameGraph tools
- Common `perf` command quick reference

Use this card alongside the `run.sh` scripts in this lesson's `code/` directory.