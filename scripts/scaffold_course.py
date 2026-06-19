#!/usr/bin/env python3
"""Scaffold the full course-computer-science tree.

Materializes:
  - phases/<NN>-<phase-slug>/README.md
  - phases/<NN>-<phase-slug>/<NN>-<lesson-slug>/
        docs/en.md       (six-beat skeleton)
        code/*           (language stubs, 2-3 per lesson)
        quiz.json        (valid schema, stage="todo")
        outputs/.gitkeep

Idempotent: re-running won't clobber files that already have content beyond the stub
sentinel. To force overwrite use --force.

Run from repo root:
    python3 scripts/scaffold_course.py
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Tuple

REPO_ROOT = Path(__file__).resolve().parent.parent
PHASES_DIR = REPO_ROOT / "phases"

STUB_SENTINEL = "<!-- scaffold-stub: safe to overwrite until removed -->"


# ─────────────────────────── Curriculum manifest ───────────────────────────

@dataclass
class Lesson:
    n: int
    title: str
    minutes: int
    languages: Tuple[str, ...]
    kind: str = "Learn"  # "Learn" or "Build"
    motto: str = ""


@dataclass
class Phase:
    n: int
    slug: str
    name: str
    desc: str
    artifact: str
    lessons: List[Lesson] = field(default_factory=list)


def L(n, title, minutes, langs, kind="Learn", motto=""):
    return Lesson(n=n, title=title, minutes=minutes, languages=langs, kind=kind, motto=motto)


# Each phase's lesson catalog — exactly as in the approved plan.
PHASES: List[Phase] = [
    Phase(0, "setup-and-tooling", "Setup & Tooling",
          "Stand up a reproducible C/C++/Rust/Go/Haskell environment so every later phase just works.",
          "A reproducible polyglot toolchain.", [
        L(1, "The CS Toolchain — What You'll Install and Why", 45, ("sh",)),
        L(2, "Terminal, Shell, Pipes, Job Control", 60, ("sh",)),
        L(3, "Git Deep — Internals, Refs, Rebase, Bisect", 90, ("sh",)),
        L(4, "C/C++ Toolchain — gcc, clang, ld, ar, make", 75, ("c", "make")),
        L(5, "Rust Toolchain — cargo, rustup, build profiles", 60, ("rs",)),
        L(6, "Build Systems — Make, CMake, Bazel, Cargo", 75, ("make", "sh")),
        L(7, "Debuggers — gdb, lldb, rust-gdb, core dumps", 75, ("c", "rs")),
        L(8, "Profilers — perf, valgrind, instruments, flamegraphs", 75, ("sh", "c")),
        L(9, "Editor Setup — Neovim/VS Code with LSP, DAP", 60, ("sh",)),
        L(10, "Linux for Builders — proc, sys, cgroups, namespaces", 75, ("sh", "c")),
        L(11, "Docker & Devcontainers for CS Work", 60, ("dockerfile", "sh")),
        L(12, "Documentation & Diagrams — Markdown, Mermaid, plantuml", 45, ("md",)),
    ]),
    Phase(1, "discrete-math-and-logic", "Discrete Math & Logic Foundations",
          "Build the proof, counting, and graph machinery every later phase quietly depends on.",
          "A proof companion CLI + combinatorics library.", [
        L(1, "Propositional Logic & Truth Tables", 45, ("py",)),
        L(2, "Predicate Logic & Quantifiers", 60, ("py",)),
        L(3, "Proof Techniques — Direct, Contradiction, Induction", 75, ("py",)),
        L(4, "Sets, Relations, Functions", 60, ("py",)),
        L(5, "Equivalence Relations & Partitions", 45, ("py",)),
        L(6, "Partial Orders, Lattices, Hasse Diagrams", 60, ("py",)),
        L(7, "Cardinality, Countability, Diagonalization", 60, ("py",)),
        L(8, "Combinatorics — Counting, Permutations, Combinations", 75, ("py", "rs")),
        L(9, "Pigeonhole, Inclusion-Exclusion, Catalan", 60, ("py",)),
        L(10, "Generating Functions", 75, ("py",)),
        L(11, "Recurrence Relations & the Master Theorem", 75, ("py",)),
        L(12, "Asymptotic Notation — Big-O, Θ, Ω, o, ω", 60, ("py",)),
        L(13, "Number Theory — Divisibility, GCD, Bezout", 60, ("py", "rs")),
        L(14, "Modular Arithmetic & Fermat / Euler", 60, ("py",)),
        L(15, "Primes, Sieves, Primality Tests", 75, ("py", "rs")),
        L(16, "Boolean Algebra & Karnaugh Maps", 60, ("py",)),
        L(17, "Graph Theory I — Basics, Traversals, Trees", 75, ("py",)),
        L(18, "Graph Theory II — Coloring, Matching, Planarity", 75, ("py",)),
        L(19, "Discrete Probability & Expectation", 60, ("py",)),
        L(20, "Markov Chains & Random Walks (Discrete)", 60, ("py",)),
        L(21, "Information & Coding Theory (Discrete)", 60, ("py",)),
        L(22, "Phase Capstone — A Proof Companion CLI", 90, ("py", "rs"), "Build"),
    ]),
    Phase(2, "programming-foundations-and-memory-model", "Programming Foundations & Memory Model",
          "Internalize the machine model: what a pointer really is, what the stack does, how Rust ownership works.",
          "A tiny manual-memory library plus ownership demos.", [
        L(1, "What Is a Program, Really (Compilation, Linking, Loading)", 60, ("c", "sh")),
        L(2, "Values, Types, Variables, Scope", 45, ("c", "rs")),
        L(3, "Control Flow — Branches, Loops, Recursion", 60, ("c", "rs")),
        L(4, "Functions, the Stack, and Calling Conventions", 75, ("c", "s")),
        L(5, "Pointers, Addresses, and Indirection (in C)", 75, ("c",)),
        L(6, "The Heap — malloc, free, fragmentation", 75, ("c",)),
        L(7, "Arrays, Strings, and Bounds", 60, ("c", "rs")),
        L(8, "Structs, Unions, Bitfields, Alignment", 60, ("c",)),
        L(9, "Errors — Returns, errno, exceptions, Result types", 60, ("c", "rs")),
        L(10, "Ownership and Borrowing — the Rust model", 75, ("rs",)),
        L(11, "Lifetimes, References, RAII", 75, ("rs", "cpp")),
        L(12, "Generics, Traits, Polymorphism", 75, ("rs",)),
        L(13, "Modules, Packages, Linkage", 45, ("c", "rs")),
        L(14, "The Preprocessor and Macros (C, Rust)", 45, ("c", "rs")),
        L(15, "Build a Pool Allocator (from scratch in C)", 90, ("c",), "Build"),
        L(16, "Build a Bump and Arena Allocator (in Rust)", 90, ("rs",), "Build"),
        L(17, "Defensive Programming — Asserts, Invariants, ASAN/UBSAN", 60, ("c", "rs")),
        L(18, "Phase Capstone — A Tiny Manual-Memory Library", 75, ("c", "rs"), "Build"),
    ]),
    Phase(3, "data-structures", "Data Structures",
          "Build every workhorse data structure from scratch and prove its invariants.",
          "A generic data-structure library in Rust with invariant checks.", [
        L(1, "Arrays & Dynamic Arrays (amortized analysis)", 60, ("rs", "py")),
        L(2, "Singly and Doubly Linked Lists", 60, ("c", "rs")),
        L(3, "Stacks & Queues (array and list backings)", 45, ("rs", "py")),
        L(4, "Deques, Ring Buffers, Circular Queues", 60, ("rs", "c")),
        L(5, "Hash Tables — Open Addressing vs Chaining", 90, ("rs", "py")),
        L(6, "Hash Function Design — Universal, Tabulation, SipHash", 75, ("rs", "py")),
        L(7, "Binary Trees — Traversal and Recursion Patterns", 60, ("rs", "py")),
        L(8, "Binary Search Trees & Rotations", 75, ("rs", "py")),
        L(9, "AVL Trees", 75, ("rs", "py")),
        L(10, "Red-Black Trees", 90, ("rs", "py")),
        L(11, "Splay Trees & Treaps", 75, ("rs", "py")),
        L(12, "B-Trees and B+-Trees", 90, ("rs", "py")),
        L(13, "Heaps & Priority Queues (binary, Fibonacci, pairing)", 75, ("rs", "py")),
        L(14, "Tries and Radix Trees", 75, ("rs", "py")),
        L(15, "Suffix Trees and Suffix Arrays", 90, ("rs", "py")),
        L(16, "Disjoint Set Union (Union-Find)", 60, ("rs", "py")),
        L(17, "Segment Trees & Fenwick (BIT)", 90, ("rs", "py")),
        L(18, "Sparse Tables & RMQ", 60, ("rs", "py")),
        L(19, "Skip Lists", 60, ("rs",)),
        L(20, "Bloom Filters, Cuckoo Filters, Count-Min Sketch", 75, ("rs", "py")),
        L(21, "LSM Trees and Write-Optimized Structures", 75, ("rs",)),
        L(22, "Persistent / Immutable Data Structures", 75, ("rs", "hs")),
        L(23, "Graphs — Representations and APIs", 60, ("rs", "py")),
        L(24, "Concurrent Data Structures Preview (Treiber stack, MS queue)", 75, ("rs",)),
        L(25, "Phase Capstone — A Generic DS Library in Rust with Invariants", 120, ("rs",), "Build"),
    ]),
    Phase(4, "algorithms-and-complexity", "Algorithms & Complexity Analysis",
          "Master the canon — sorting, DP, graphs, strings, geometry, randomization — and the analysis tools that bound them.",
          "An algorithms cookbook plus a benchmark harness.", [
        L(1, "Analyzing Algorithms — Recurrences, Master Theorem in Action", 60, ("py",)),
        L(2, "Sorting I — Insertion, Selection, Bubble, and Why They Lose", 45, ("py", "rs")),
        L(3, "Sorting II — Merge, Quick (and Quickselect)", 75, ("py", "rs")),
        L(4, "Sorting III — Heap, Intro, Tim", 75, ("py", "rs")),
        L(5, "Sorting IV — Linear-Time: Counting, Radix, Bucket", 60, ("py", "rs")),
        L(6, "Searching — Binary, Exponential, Ternary, Interpolation", 60, ("py", "rs")),
        L(7, "Divide & Conquer Patterns", 60, ("py",)),
        L(8, "Dynamic Programming I — 1D, Memoization, Tabulation", 75, ("py",)),
        L(9, "Dynamic Programming II — 2D and Beyond", 75, ("py",)),
        L(10, "DP III — Bitmask, Digit, Tree, DP on DAGs", 90, ("py", "cpp")),
        L(11, "Greedy Algorithms & Matroids", 75, ("py",)),
        L(12, "Backtracking, Branch & Bound", 75, ("py", "cpp")),
        L(13, "Graph Algorithms I — BFS, DFS, Topo, SCC", 75, ("py", "rs")),
        L(14, "Graph Algorithms II — Dijkstra, Bellman-Ford, A*", 75, ("py", "rs")),
        L(15, "Graph Algorithms III — Floyd-Warshall, Johnson, APSP", 60, ("py",)),
        L(16, "Minimum Spanning Trees — Prim, Kruskal, Borůvka", 60, ("py", "rs")),
        L(17, "Network Flow — Ford-Fulkerson, Edmonds-Karp, Dinic", 90, ("py", "cpp")),
        L(18, "Matching — Bipartite, Hopcroft-Karp, Hungarian", 75, ("py", "cpp")),
        L(19, "String Matching — KMP, Z, Boyer-Moore", 75, ("py", "rs")),
        L(20, "Suffix Structures in Action — Aho-Corasick, LCP", 75, ("py", "rs")),
        L(21, "Hashing in Algorithms — Rabin-Karp, Rolling Hashes", 60, ("py", "rs")),
        L(22, "Computational Geometry I — Convex Hull, Sweep Line", 75, ("py", "cpp")),
        L(23, "Computational Geometry II — kd-Tree, R-Tree, Range Query", 75, ("py", "rs")),
        L(24, "Randomized Algorithms — Las Vegas vs Monte Carlo", 60, ("py",)),
        L(25, "Approximation Algorithms — Vertex Cover, TSP, Set Cover", 75, ("py",)),
        L(26, "Online Algorithms & Competitive Analysis", 60, ("py",)),
        L(27, "Streaming Algorithms — Frequency, Quantiles, HyperLogLog", 75, ("py", "rs")),
        L(28, "Parallel Algorithms — PRAM, Brent, Map-Reduce style", 75, ("py", "rs")),
        L(29, "Amortized Analysis Deep — Aggregate, Accounting, Potential", 60, ("py",)),
        L(30, "Phase Capstone — Algorithm Cookbook + Benchmark Harness", 120, ("rs", "py"), "Build"),
    ]),
    Phase(5, "theory-of-computation", "Theory of Computation",
          "From regular languages to undecidability — know what computers can't do and why.",
          "A regex engine plus a Turing-machine simulator.", [
        L(1, "What Counts as Computation?", 45, ("py",)),
        L(2, "Finite Automata — DFAs", 60, ("py",)),
        L(3, "NFAs and Subset Construction", 60, ("py",)),
        L(4, "Regular Expressions ↔ Automata", 75, ("py",)),
        L(5, "Build a Regex Engine (Thompson construction)", 90, ("py", "rs"), "Build"),
        L(6, "Pumping Lemma for Regular Languages", 45, ("py",)),
        L(7, "Context-Free Grammars", 60, ("py",)),
        L(8, "Pushdown Automata & CFG Equivalence", 75, ("py",)),
        L(9, "Chomsky and Greibach Normal Forms", 60, ("py",)),
        L(10, "Parsing Theory — CYK and Earley", 75, ("py",)),
        L(11, "Turing Machines & Variants", 75, ("py",)),
        L(12, "Build a Turing Machine Simulator", 75, ("py", "rs"), "Build"),
        L(13, "Decidability and the Halting Problem", 60, ("py",)),
        L(14, "Rice's Theorem & Undecidable Properties", 60, ("py",)),
        L(15, "Time Complexity Classes — P, NP, EXP", 60, ("py",)),
        L(16, "NP-Completeness — Cook-Levin and Reductions", 90, ("py",)),
        L(17, "Space Complexity — L, NL, PSPACE, Savitch", 75, ("py",)),
        L(18, "Phase Capstone — A Toy Proof Assistant for Reductions", 90, ("py",), "Build"),
    ]),
    Phase(6, "digital-logic-and-architecture", "Digital Logic & Computer Architecture",
          "Walk down from instruction to transistor, then back up: ALU, pipeline, cache, MMU.",
          "A 5-stage pipelined RISC-V CPU in HDL with assembler.", [
        L(1, "Bits, Bytes, Two's Complement, IEEE 754", 60, ("c", "py")),
        L(2, "Transistors → Logic Gates", 60, ("v",)),
        L(3, "Combinational Logic — Adders, Mux, Decoders", 75, ("v",)),
        L(4, "Sequential Logic — Latches, Flip-Flops, FSMs", 75, ("v",)),
        L(5, "Build an ALU in HDL", 90, ("v",), "Build"),
        L(6, "Registers, Register Files, Memory Banks", 60, ("v",)),
        L(7, "The Datapath — Single-Cycle CPU", 90, ("v",)),
        L(8, "Control Unit — Microcoded vs Hardwired", 75, ("v",)),
        L(9, "ISA Design — RISC vs CISC, RISC-V Tour", 60, ("md", "s")),
        L(10, "RISC-V Assembly — Hands-On", 75, ("s",)),
        L(11, "Pipelining — 5-Stage, Hazards, Forwarding", 90, ("v",)),
        L(12, "Branch Prediction — Static, Dynamic, Tournament", 75, ("v", "c")),
        L(13, "Out-of-Order Execution & Tomasulo", 90, ("py",)),
        L(14, "Memory Hierarchy — Cache Mapping & Coherence", 90, ("c",)),
        L(15, "Cache Performance — Locality, Blocking, Prefetch", 75, ("c", "cpp")),
        L(16, "Virtual Memory — TLB, Page Tables, MMU", 75, ("c",)),
        L(17, "I/O — DMA, MMIO, Interrupts", 60, ("c",)),
        L(18, "SIMD & Vector ISAs — AVX, SVE, RVV", 75, ("c", "cpp")),
        L(19, "GPU Architecture — SIMT, Warps, Memory Hierarchy", 75, ("cu",)),
        L(20, "Modern Microarchitecture Tour (Apple Silicon, AMD Zen)", 60, ("md",)),
        L(21, "Power, Heat, Reliability — Why Cores Stopped Scaling", 45, ("md",)),
        L(22, "Phase Capstone — A 5-Stage Pipelined RISC-V CPU in HDL", 150, ("v", "s"), "Build"),
    ]),
    Phase(7, "operating-systems", "Operating Systems",
          "Write the abstractions you've always used: process, page, file, syscall.",
          "“nanos”: a bootable mini-kernel.", [
        L(1, "What an OS Actually Does (and Doesn't)", 45, ("md",)),
        L(2, "The Boot Process — BIOS, UEFI, GRUB", 60, ("sh",)),
        L(3, "Hello World as a Bootloader (in asm + C)", 90, ("s", "c"), "Build"),
        L(4, "Privilege Modes, Traps, and System Calls", 75, ("c", "s")),
        L(5, "Processes — fork, exec, wait", 75, ("c",)),
        L(6, "Threads, TLS, Context Switching", 75, ("c",)),
        L(7, "Scheduling — FCFS, RR, MLFQ, CFS, EDF", 90, ("c", "rs")),
        L(8, "Virtual Memory in the OS — Demand Paging", 75, ("c",)),
        L(9, "Page Replacement — LRU, Clock, ARC", 60, ("c", "rs")),
        L(10, "The Kernel Heap — slab and slub allocators", 75, ("c",)),
        L(11, "File Systems I — VFS, inodes, journaling", 90, ("c",)),
        L(12, "File Systems II — ext4, btrfs, ZFS deep cuts", 75, ("md",)),
        L(13, "I/O Architecture — Block, Char, syscalls, vfs", 60, ("c",)),
        L(14, "Synchronization in the Kernel — Spinlocks, RCU", 75, ("c",)),
        L(15, "Deadlock — Detection, Prevention, Avoidance", 60, ("c", "py")),
        L(16, "IPC — Pipes, FIFOs, Shared Memory, sockets", 75, ("c",)),
        L(17, "Signals — Delivery, Handling, Pitfalls", 60, ("c",)),
        L(18, "Devices and Drivers — Char, Block, Net", 75, ("c",)),
        L(19, "Containers — namespaces, cgroups, seccomp", 75, ("c", "sh")),
        L(20, "Virtualization — Type 1/2 Hypervisors, KVM", 75, ("c",)),
        L(21, "Microkernels and Unikernels", 60, ("md",)),
        L(22, "Real-Time and Embedded OS", 60, ("c",)),
        L(23, "Linux Internals Tour — The Source Tree", 75, ("md", "sh")),
        L(24, "Phase Capstone — 'nanos': A Bootable Mini-Kernel", 180, ("c", "s"), "Build"),
    ]),
    Phase(8, "compilers-and-language-design", "Compilers & Programming Language Design",
          "Lex, parse, type-check, optimize, codegen — and then bootstrap.",
          "A self-hosting compiler for a Pascal-ish language.", [
        L(1, "The Compilation Pipeline — End to End", 45, ("md",)),
        L(2, "Lexing I — Regex → DFA → Lexer", 75, ("rs", "py")),
        L(3, "Lexing II — Hand-Written Scanners", 60, ("rs", "c")),
        L(4, "Parsing I — Recursive Descent", 75, ("rs",)),
        L(5, "Parsing II — LL(1), Predictive Tables", 75, ("py",)),
        L(6, "Parsing III — LR, SLR, LALR, GLR", 90, ("py",)),
        L(7, "PEG Parsers and Packrat", 60, ("rs",)),
        L(8, "Parser Generators (yacc/bison/lalrpop/tree-sitter)", 60, ("rs",)),
        L(9, "AST Design and Visitor Patterns", 60, ("rs",)),
        L(10, "Semantic Analysis — Symbol Tables, Scopes", 75, ("rs",)),
        L(11, "Type Checking — Mono, Sub, Inference (HM)", 90, ("hs", "rs")),
        L(12, "Intermediate Representation — Three-Address Code", 75, ("rs",)),
        L(13, "SSA Form — Construction and Dominance", 90, ("rs",)),
        L(14, "Classic Optimizations — DCE, CSE, Inlining, LICM", 90, ("rs",)),
        L(15, "Loop Optimization & Vectorization", 75, ("rs", "c")),
        L(16, "Register Allocation — Linear Scan vs Graph Coloring", 90, ("rs",)),
        L(17, "Code Generation — Instruction Selection, Scheduling", 75, ("rs", "s")),
        L(18, "Linkers and Loaders", 60, ("c", "sh")),
        L(19, "Garbage Collection — Mark-Sweep, Copying, Generational", 90, ("rs",)),
        L(20, "JIT Compilation — V8, JVM, LuaJIT principles", 75, ("rs",)),
        L(21, "LLVM in Practice — IR, Passes, Backends", 75, ("cpp", "sh")),
        L(22, "Phase Capstone — A Self-Hosting Compiler", 180, ("rs",), "Build"),
    ]),
    Phase(9, "computer-networks", "Computer Networks",
          "Build the stack: Ethernet, IP, TCP, TLS, HTTP — by hand.",
          "An HTTP/2 server on a custom userspace TCP/IP stack.", [
        L(1, "The Stack — Why Layers (OSI vs TCP/IP)", 45, ("md",)),
        L(2, "Physical & Link Layers — Ethernet, MAC, ARP", 60, ("c", "py")),
        L(3, "Network Layer — IPv4, IPv6, Subnetting, CIDR", 75, ("py", "c")),
        L(4, "Routing I — Static, Distance Vector", 60, ("py",)),
        L(5, "Routing II — Link State (OSPF), BGP", 75, ("py",)),
        L(6, "NAT, ICMP, DHCP, IPAM", 60, ("py",)),
        L(7, "Transport — UDP, TCP State Machine", 90, ("c", "rs")),
        L(8, "TCP Congestion Control — Reno, CUBIC, BBR", 75, ("py", "c")),
        L(9, "QUIC and HTTP/3", 75, ("rs",)),
        L(10, "Sockets API — Build a TCP Echo Server", 75, ("c", "rs")),
        L(11, "Build a userspace TCP/IP stack (toy)", 120, ("rs", "c"), "Build"),
        L(12, "DNS — Resolvers, Records, DNSSEC", 75, ("py", "c")),
        L(13, "HTTP/1.1, HTTP/2 — Wire Format and Multiplexing", 75, ("rs", "py")),
        L(14, "TLS in the Network Course (Handshake Overview)", 60, ("py",)),
        L(15, "WebSockets, SSE, gRPC", 60, ("rs", "ts")),
        L(16, "Firewalls, NAT Traversal, STUN/TURN/ICE", 60, ("py",)),
        L(17, "CDNs and Anycast", 60, ("md",)),
        L(18, "Load Balancers — L4 vs L7, Algorithms", 60, ("rs", "py")),
        L(19, "P2P Networks — Kademlia, BitTorrent", 75, ("rs", "py")),
        L(20, "Software-Defined Networking & eBPF", 75, ("c", "py")),
        L(21, "Network Programming in Rust (async + tokio)", 75, ("rs",)),
        L(22, "Phase Capstone — An HTTP/2 Server on a Custom TCP Stack", 150, ("rs",), "Build"),
    ]),
    Phase(10, "databases-and-storage", "Databases & Storage Systems",
          "From relational algebra to MVCC: write the storage engine, write the planner.",
          "An MVCC KV store with a SQL frontend.", [
        L(1, "What a Database Actually Is", 45, ("md",)),
        L(2, "The Relational Model & Relational Algebra", 75, ("py", "sql")),
        L(3, "SQL — DDL, DML, Joins, Subqueries", 75, ("sql",)),
        L(4, "Normalization 1NF → BCNF (and 4NF)", 75, ("sql", "py")),
        L(5, "Physical Storage — Pages, Slotted Pages", 60, ("rs", "c")),
        L(6, "Buffer Pool Management & Replacement", 75, ("rs",)),
        L(7, "Indexing — B+ Trees in DBs", 90, ("rs",)),
        L(8, "Indexing — Hash, Bitmap, GiST", 60, ("rs", "py")),
        L(9, "LSM-Tree Storage Engines (LevelDB/RocksDB style)", 90, ("rs",)),
        L(10, "Query Execution — Iterator vs Vectorized", 75, ("rs", "py")),
        L(11, "Join Algorithms — Nested Loop, Hash, Sort-Merge", 75, ("rs", "py")),
        L(12, "Query Optimization — Plans, Cost, Cardinality", 90, ("py", "sql")),
        L(13, "Transactions — ACID, Anomalies", 75, ("sql", "py")),
        L(14, "Isolation Levels — Read Committed → Serializable", 75, ("sql", "py")),
        L(15, "Concurrency Control — 2PL, OCC, MVCC", 90, ("rs", "py")),
        L(16, "Recovery — WAL, ARIES", 90, ("rs",)),
        L(17, "NoSQL — KV, Document, Wide-Column, Graph", 60, ("py",)),
        L(18, "Distributed DBs — Sharding, Replication, Spanner", 75, ("md", "go")),
        L(19, "Columnar Storage & OLAP — Parquet, DuckDB", 75, ("py", "sql")),
        L(20, "Vector Databases — HNSW, IVF, PQ", 75, ("rs", "py")),
        L(21, "Time-Series and Event-Sourced Stores", 60, ("py", "sql")),
        L(22, "Phase Capstone — Build an MVCC KV Store with a SQL Frontend", 180, ("rs", "sql"), "Build"),
    ]),
    Phase(11, "distributed-systems", "Distributed Systems",
          "Clocks, consensus, replication, CRDTs — and a Raft you can break and watch heal.",
          "A Raft-replicated KV store with snapshotting.", [
        L(1, "What Distribution Costs You", 45, ("md",)),
        L(2, "Failure Models — Crash, Omission, Byzantine", 60, ("md",)),
        L(3, "Time — Physical, Logical, Lamport Clocks", 75, ("go", "py")),
        L(4, "Vector Clocks and Causal Order", 75, ("go", "py")),
        L(5, "The FLP Impossibility Result", 60, ("md", "py")),
        L(6, "CAP and PACELC — Read Honestly", 60, ("md",)),
        L(7, "Consensus I — Paxos (Single-Decree)", 90, ("go", "tla")),
        L(8, "Consensus II — Multi-Paxos and Variants", 75, ("go",)),
        L(9, "Consensus III — Raft (with a working implementation)", 120, ("go", "rs"), "Build"),
        L(10, "Replication — Leader/Follower, Quorum", 75, ("go",)),
        L(11, "Eventual Consistency & Read-Repair", 60, ("go", "py")),
        L(12, "CRDTs — Counters, Sets, Sequences", 90, ("rs", "ts")),
        L(13, "Gossip Protocols & SWIM", 60, ("go",)),
        L(14, "Distributed Transactions — 2PC, 3PC, Sagas", 90, ("go", "py")),
        L(15, "Distributed File Systems — GFS, HDFS", 75, ("md", "py")),
        L(16, "MapReduce, Spark, Dataflow", 75, ("py", "scala")),
        L(17, "Service Discovery, Membership, Leader Election", 60, ("go",)),
        L(18, "Distributed Caches — Memcached, Redis, consistent hashing", 60, ("go", "py")),
        L(19, "Message Queues & Streams — Kafka, NATS", 75, ("go", "py")),
        L(20, "Microservices vs Monolith — Real Trade-offs", 60, ("md",)),
        L(21, "Observability — Metrics, Traces, Logs in Distributed Systems", 60, ("go", "ts")),
        L(22, "Phase Capstone — A Raft-Replicated KV Store with Snapshotting", 180, ("go", "rs"), "Build"),
    ]),
    Phase(12, "cryptography-and-security", "Cryptography & Security",
          "Build the primitives, then the protocols, then the attacks that bypass both.",
          "A TLS 1.3 implementation plus a mini-CTF toolkit.", [
        L(1, "What Cryptography Actually Promises", 45, ("md",)),
        L(2, "Classical Ciphers and Why They Fail", 45, ("py",)),
        L(3, "Symmetric I — Stream Ciphers and One-Time Pad", 60, ("py", "rs")),
        L(4, "Symmetric II — Block Ciphers, AES Internals", 90, ("c", "rs")),
        L(5, "Modes of Operation — ECB, CBC, CTR, GCM", 75, ("py", "rs")),
        L(6, "Hash Functions — SHA-2, SHA-3, BLAKE", 75, ("c", "rs")),
        L(7, "MACs and HMAC", 60, ("py", "rs")),
        L(8, "Authenticated Encryption (AEAD)", 60, ("rs",)),
        L(9, "Public Key I — Diffie-Hellman", 75, ("py", "rs")),
        L(10, "Public Key II — RSA Internals & Padding", 90, ("py", "rs")),
        L(11, "Public Key III — Elliptic Curves & Ed25519", 90, ("rs", "py")),
        L(12, "Digital Signatures — ECDSA, EdDSA, BLS", 75, ("rs",)),
        L(13, "KDFs, PBKDF2, scrypt, Argon2", 60, ("rs", "py")),
        L(14, "TLS 1.3 — Handshake, Records, 0-RTT", 90, ("rs",)),
        L(15, "Build a Toy TLS 1.3 Client", 120, ("rs",), "Build"),
        L(16, "PKI, Certs, Transparency", 60, ("md", "py")),
        L(17, "Zero-Knowledge Proofs — Sigma, zk-SNARK overview", 75, ("py", "rs")),
        L(18, "Post-Quantum — Kyber, Dilithium, SPHINCS+", 60, ("rs",)),
        L(19, "Side-Channels — Timing, Cache, Spectre/Meltdown", 75, ("c",)),
        L(20, "Memory-Safety Attacks — Stack Smash, ROP, ASLR", 90, ("c", "s")),
        L(21, "Web Security — XSS, CSRF, SQLi, SSRF, deserialization", 90, ("ts", "py")),
        L(22, "Threat Modeling — STRIDE, DREAD, attack trees", 60, ("md",)),
        L(23, "CTF Toolkit — pwntools, GDB, Ghidra", 75, ("py", "sh")),
        L(24, "Phase Capstone — A TLS 1.3 Library + a Mini-CTF", 150, ("rs", "py"), "Build"),
    ]),
    Phase(13, "concurrent-and-parallel", "Concurrent & Parallel Computing",
          "Get atomic, lock-free, async, and GPU all right — with a memory model in your head.",
          "A work-stealing scheduler plus a lock-free queue.", [
        L(1, "Concurrency vs Parallelism — Get This Right", 45, ("md",)),
        L(2, "Race Conditions, Atomicity, Visibility", 60, ("c", "rs")),
        L(3, "Memory Models — Sequential Consistency vs Relaxed", 90, ("cpp", "rs")),
        L(4, "Locks — Mutex, RW Lock, Spinlock, Ticket Lock", 75, ("c", "rs")),
        L(5, "Condition Variables and Monitors", 75, ("c", "rs")),
        L(6, "Semaphores and the Classics (Producer/Consumer, Dining)", 60, ("c", "go")),
        L(7, "Atomics, CAS, ABA Problem", 75, ("rs", "cpp")),
        L(8, "Lock-Free Data Structures — Treiber Stack, MS Queue", 90, ("rs", "cpp")),
        L(9, "Wait-Free Algorithms and Their Limits", 60, ("rs",)),
        L(10, "Software Transactional Memory", 60, ("hs", "rs")),
        L(11, "Futures, Promises, async/await", 75, ("rs", "ts")),
        L(12, "Reactor and Proactor Patterns — epoll, kqueue, io_uring", 90, ("c", "rs")),
        L(13, "Tokio and the Async Runtime in Rust", 75, ("rs",)),
        L(14, "CSP and Go Channels", 60, ("go",)),
        L(15, "The Actor Model — Erlang and Akka principles", 60, ("erl", "rs")),
        L(16, "Parallel Patterns — Map, Reduce, Pipeline, Scan", 75, ("rs", "py")),
        L(17, "Work-Stealing Schedulers", 90, ("rs",)),
        L(18, "SIMD Programming in Practice", 75, ("cpp", "rs")),
        L(19, "GPU Programming — CUDA Basics", 90, ("cu",)),
        L(20, "GPU Programming — WebGPU / Compute Shaders", 75, ("wgsl", "ts")),
        L(21, "MPI and Distributed-Memory Parallelism", 75, ("c", "py")),
        L(22, "Phase Capstone — A Work-Stealing Scheduler + Lock-Free Queue", 150, ("rs",), "Build"),
    ]),
    Phase(14, "graphics-and-visualization", "Computer Graphics & Visualization",
          "Rasterize. Ray-trace. Path-trace. Make pixels meaningful.",
          "A path tracer plus a triangle rasterizer.", [
        L(1, "Pixels, Colors, Gamma", 45, ("py",)),
        L(2, "The Graphics Pipeline at 30,000 ft", 45, ("md",)),
        L(3, "Linear Algebra for Graphics — Transforms, Projections", 75, ("py", "rs")),
        L(4, "Rasterization I — Lines and Triangles", 75, ("rs", "cpp")),
        L(5, "Rasterization II — Z-buffer, Clipping, Culling", 75, ("rs", "cpp")),
        L(6, "Shading Models — Lambert, Phong, Blinn-Phong", 60, ("glsl", "rs")),
        L(7, "Physically Based Rendering — BRDF, Microfacet", 90, ("glsl", "rs")),
        L(8, "Shaders 101 — Vertex and Fragment", 75, ("glsl", "wgsl")),
        L(9, "Build a Software Rasterizer (in Rust)", 120, ("rs",), "Build"),
        L(10, "Ray Tracing I — Whitted Style", 90, ("rs", "cpp")),
        L(11, "Ray Tracing II — Path Tracing, Monte Carlo", 90, ("rs",)),
        L(12, "Acceleration Structures — BVH, kd-Tree", 75, ("rs",)),
        L(13, "Real-Time Techniques — Deferred, Tiled, Cluster", 75, ("glsl", "rs")),
        L(14, "Modern APIs — Vulkan, Metal, WebGPU compared", 60, ("md",)),
        L(15, "Compute Shaders for Non-Graphics Work", 60, ("wgsl", "cu")),
        L(16, "Animation, Skinning, IK", 60, ("rs",)),
        L(17, "Visualization — D3, deck.gl, scientific plotting", 45, ("ts", "py")),
        L(18, "Phase Capstone — A Path Tracer + a Triangle Rasterizer", 180, ("rs",), "Build"),
    ]),
    Phase(15, "systems-performance", "Systems Programming & Performance",
          "Measure honestly. Tune cache, branches, IO. Win 10x by knowing the machine.",
          "A profile-guided optimization walk-through.", [
        L(1, "How to Think About Performance", 45, ("md",)),
        L(2, "Measurement Discipline — Benchmarks That Don't Lie", 75, ("rs", "cpp")),
        L(3, "Profiling — perf, dtrace, Instruments, eBPF", 75, ("sh", "c")),
        L(4, "Flamegraphs, Hotspots, and Reading Stacks", 60, ("sh",)),
        L(5, "Cache-Aware Algorithm Design", 75, ("cpp", "rs")),
        L(6, "False Sharing and NUMA", 75, ("cpp", "rs")),
        L(7, "Branch Prediction and Layout Tricks", 60, ("cpp",)),
        L(8, "Vectorization in Practice (auto and intrinsics)", 75, ("cpp", "rs")),
        L(9, "Memory Allocators in Production — jemalloc, mimalloc", 75, ("c",)),
        L(10, "Zero-Copy and mmap", 60, ("c",)),
        L(11, "Asynchronous I/O — io_uring Deep Dive", 90, ("c", "rs")),
        L(12, "Kernel Bypass — DPDK, SPDK, AF_XDP", 75, ("c",)),
        L(13, "Lock Contention Patterns and Cures", 75, ("rs", "cpp")),
        L(14, "Coroutines and Stackful vs Stackless Concurrency", 75, ("cpp", "rs")),
        L(15, "C++ Low-Latency Idioms", 75, ("cpp",)),
        L(16, "Rust for High Performance — UnsafeCell, MaybeUninit, alignment", 75, ("rs",)),
        L(17, "Power, Frequency Scaling, Thermal Throttling", 45, ("md",)),
        L(18, "Reliability Engineering — Tail Latency, Hedging", 60, ("go", "rs")),
        L(19, "Capacity Planning and Little's Law", 60, ("py",)),
        L(20, "Phase Capstone — A Profile-Guided Optimization Walk-Through", 150, ("rs", "cpp"), "Build"),
    ]),
    Phase(16, "software-engineering-and-architecture", "Software Engineering & Architecture",
          "Make code other people can read, change, and ship — at scale, over years.",
          "A refactored real-world OSS repo with ADRs.", [
        L(1, "What Makes Software 'Engineered'", 45, ("md",)),
        L(2, "Naming, Cohesion, Coupling", 45, ("ts", "py")),
        L(3, "SOLID Principles — Demystified", 60, ("ts", "py")),
        L(4, "GoF Patterns That Still Matter", 75, ("ts", "py")),
        L(5, "Modern Patterns — Functional Core / Imperative Shell", 60, ("ts", "rs")),
        L(6, "Refactoring Catalogue and Mechanics", 75, ("ts", "py")),
        L(7, "Code Review Practice", 45, ("md",)),
        L(8, "Domain-Driven Design — Bounded Contexts, Aggregates", 90, ("ts", "py")),
        L(9, "Hexagonal / Clean Architecture", 60, ("ts",)),
        L(10, "Event-Driven Architectures", 60, ("ts", "go")),
        L(11, "CQRS and Event Sourcing", 75, ("ts", "rs")),
        L(12, "Microservices — When and When Not", 60, ("md",)),
        L(13, "API Design — REST, GraphQL, gRPC Trade-offs", 75, ("ts", "proto")),
        L(14, "Versioning, Deprecation, Compatibility", 60, ("md",)),
        L(15, "Monorepos vs Polyrepos", 45, ("md",)),
        L(16, "Dependency Management & SemVer", 45, ("md", "sh")),
        L(17, "Build & CI/CD — Pipelines That Don't Suck", 60, ("yaml", "sh")),
        L(18, "Observability as a Design Concern", 60, ("ts", "go")),
        L(19, "Technical Debt — Measure, Pay Down, Negotiate", 60, ("md",)),
        L(20, "Architecture Decision Records (ADRs)", 45, ("md",)),
        L(21, "Reading Large Codebases", 60, ("md", "sh")),
        L(22, "Phase Capstone — Refactor a Real OSS Repo + ADR Bundle", 150, ("ts", "md"), "Build"),
    ]),
    Phase(17, "testing-and-verification", "Testing, Verification & Formal Methods",
          "Move from unit tests to fuzzers to TLA+ to Coq. Know what each one actually proves.",
          "TLA+ models plus a fuzzer plus a property suite.", [
        L(1, "Why We Test (and what tests don't prove)", 45, ("md",)),
        L(2, "Unit, Integration, E2E — Pyramid vs Trophy", 60, ("ts", "py")),
        L(3, "Test Doubles — Stubs, Mocks, Fakes, Spies", 60, ("ts", "py")),
        L(4, "Property-Based Testing — QuickCheck, Hypothesis", 75, ("hs", "py")),
        L(5, "Fuzz Testing — libFuzzer, AFL++, structured fuzzing", 90, ("c", "rs")),
        L(6, "Mutation Testing", 60, ("py", "ts")),
        L(7, "Coverage — What It Tells You and What It Doesn't", 45, ("py",)),
        L(8, "Contracts and Design by Contract", 60, ("py", "rs")),
        L(9, "Hoare Logic — Pre/Post/Invariant", 75, ("py",)),
        L(10, "Separation Logic Primer", 75, ("md",)),
        L(11, "Model Checking — TLA+ Hands-On", 120, ("tla",)),
        L(12, "TLA+ for Distributed Protocols", 90, ("tla",)),
        L(13, "Alloy and Lightweight Modeling", 75, ("als",)),
        L(14, "SMT Solvers — Z3 in Practice", 75, ("py",)),
        L(15, "Symbolic Execution — KLEE, angr", 75, ("py", "c")),
        L(16, "Proof Assistants — Coq/Lean/Isabelle Primer", 90, ("coq",)),
        L(17, "Verified Software — seL4, CompCert, etc.", 60, ("md",)),
        L(18, "Phase Capstone — Specify, Model-Check, and Prove a Lock-Free Algorithm", 150, ("tla", "rs"), "Build"),
    ]),
    Phase(18, "language-paradigms-and-type-theory", "Programming Language Paradigms & Type Theory",
          "Lambda calculus to dependent types to algebraic effects — see why your favorite language behaves the way it does.",
          "A bidirectional type-checker for STLC with extensions.", [
        L(1, "What 'Paradigm' Means (and Doesn't)", 45, ("md",)),
        L(2, "Lambda Calculus — Reduction, Encodings", 90, ("hs", "py")),
        L(3, "Simply Typed Lambda Calculus", 75, ("hs",)),
        L(4, "Polymorphism — System F, ML, Bounded Quantification", 75, ("hs",)),
        L(5, "Type Inference — Hindley-Milner Reconstruction", 90, ("hs", "rs")),
        L(6, "Subtyping, Variance, Higher-Kinded Types", 75, ("hs", "ts")),
        L(7, "Dependent Types — A Tour", 75, ("idr", "hs")),
        L(8, "Linear and Affine Types — Rust's Lineage", 60, ("rs", "hs")),
        L(9, "Algebraic Effects vs Monads", 75, ("hs",)),
        L(10, "Functional Programming Deep — Haskell Idioms", 75, ("hs",)),
        L(11, "OOP Deep — Inheritance, Composition, Mixins, Traits", 60, ("ts", "rs")),
        L(12, "Logic Programming — Prolog and Unification", 75, ("pl",)),
        L(13, "Dataflow and Reactive — RxJS, FRP", 60, ("ts",)),
        L(14, "DSL Design — Internal and External", 60, ("rs", "ts")),
        L(15, "Phase Capstone — A Bidirectional Type-Checker for STLC + Extensions", 150, ("hs", "rs"), "Build"),
    ]),
    Phase(19, "capstone-projects", "Capstone Projects",
          "Multi-week builds. Each ships a real working artifact. Pick a track.",
          "A learner-chosen ambitious build.", [
        L(1, "Build a Compiler for a Pascal-like Language", 720, ("rs",), "Build"),
        L(2, "Build a Kernel That Boots, Schedules, Pages", 960, ("c", "s"), "Build"),
        L(3, "Build a Distributed KV Store (Raft + MVCC)", 840, ("go", "rs"), "Build"),
        L(4, "Build a SQL Database with B+-Tree Index", 840, ("rs", "sql"), "Build"),
        L(5, "Build a User-Space TCP/IP Stack", 720, ("rs", "c"), "Build"),
        L(6, "Build a Path Tracer with BVH", 600, ("rs",), "Build"),
        L(7, "Build a Chess Engine with Search & Eval", 720, ("rs", "cpp"), "Build"),
        L(8, "Build a Search Engine (Crawler + Index + Ranker)", 840, ("rs", "py"), "Build"),
        L(9, "Build a Container Runtime", 600, ("go", "c"), "Build"),
        L(10, "Build a Toy ML Framework (autodiff, optimizers)", 720, ("py", "rs"), "Build"),
        L(11, "Build a TLS 1.3 Client + Server", 720, ("rs",), "Build"),
        L(12, "Build a Distributed Build System (Bazel-style cache)", 720, ("go", "rs"), "Build"),
        L(13, "Build a Toy Browser (HTML/CSS layout + render)", 840, ("rs", "ts"), "Build"),
        L(14, "Build a Static Analyzer for C with Abstract Interpretation", 720, ("rs", "ocaml"), "Build"),
        L(15, "Build a Verified Sorting Library in Coq/Lean", 600, ("coq",), "Build"),
        L(16, "Build a Quantum Circuit Simulator", 480, ("py", "rs"), "Build"),
        L(17, "Build a GPU Rasterizer in WebGPU", 600, ("wgsl", "ts"), "Build"),
        L(18, "Open Capstone — Learner-Defined, Mentor-Reviewed", 900, ("md",), "Build"),
    ]),
]


# ─────────────────────────── Helpers ───────────────────────────

LANG_INFO = {
    # ext -> (display, code body kind)
    "py":  ("Python", "py"),
    "rs":  ("Rust", "rs"),
    "c":   ("C", "c"),
    "cpp": ("C++", "cpp"),
    "ts":  ("TypeScript", "ts"),
    "go":  ("Go", "go"),
    "hs":  ("Haskell", "hs"),
    "s":   ("RISC-V Assembly", "s"),
    "v":   ("SystemVerilog (HDL)", "v"),
    "sql": ("SQL", "sql"),
    "tla": ("TLA+", "tla"),
    "als": ("Alloy", "als"),
    "coq": ("Coq", "coq"),
    "idr": ("Idris", "idr"),
    "pl":  ("Prolog", "pl"),
    "erl": ("Erlang", "erl"),
    "ocaml": ("OCaml", "ocaml"),
    "scala": ("Scala", "scala"),
    "cu":  ("CUDA C++", "cu"),
    "glsl": ("GLSL", "glsl"),
    "wgsl": ("WGSL", "wgsl"),
    "ts":  ("TypeScript", "ts"),
    "sh":  ("Shell", "sh"),
    "make": ("Makefile", "make"),
    "md":  ("Markdown", "md"),
    "dockerfile": ("Dockerfile", "dockerfile"),
    "proto": ("Protobuf", "proto"),
    "yaml": ("YAML", "yaml"),
}

COMMENT_PREFIX = {
    "py": "#", "rs": "//", "c": "//", "cpp": "//", "ts": "//", "go": "//",
    "hs": "--", "s": "#", "v": "//", "sql": "--", "tla": "\\*", "als": "//",
    "coq": "(*", "idr": "--", "pl": "%", "erl": "%", "ocaml": "(*",
    "scala": "//", "cu": "//", "glsl": "//", "wgsl": "//",
    "sh": "#", "make": "#", "md": "<!--", "dockerfile": "#", "proto": "//",
    "yaml": "#",
}

CODE_FILENAME = {
    "py": "main.py", "rs": "main.rs", "c": "main.c", "cpp": "main.cpp",
    "ts": "main.ts", "go": "main.go", "hs": "Main.hs", "s": "main.s",
    "v": "main.v", "sql": "main.sql", "tla": "Main.tla", "als": "main.als",
    "coq": "Main.v", "idr": "Main.idr", "pl": "main.pl", "erl": "main.erl",
    "ocaml": "main.ml", "scala": "Main.scala", "cu": "main.cu",
    "glsl": "main.glsl", "wgsl": "main.wgsl", "sh": "run.sh",
    "make": "Makefile", "md": "notes.md", "dockerfile": "Dockerfile",
    "proto": "main.proto", "yaml": "ci.yaml",
}


def slugify(text: str) -> str:
    """Match the AI course's slug style: lowercase, hyphens, no punctuation."""
    text = text.lower()
    # Replace common separators / punctuation with hyphens
    text = re.sub(r"[—–—–:]+", " ", text)  # em/en dashes, colons -> space
    text = re.sub(r"[^a-z0-9]+", "-", text)
    text = re.sub(r"-+", "-", text).strip("-")
    return text


def write_if_stub(path: Path, content: str, force: bool) -> bool:
    """Write content if file is missing, force, or still has the stub sentinel."""
    if path.exists() and not force:
        try:
            existing = path.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            existing = ""
        if STUB_SENTINEL not in existing:
            return False  # preserve user content
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    return True


# ─────────────────────────── Skeleton builders ───────────────────────────

def lesson_doc_skeleton(phase: Phase, lesson: Lesson, prereqs: str) -> str:
    langs = ", ".join(LANG_INFO[l][0] for l in lesson.languages if l in LANG_INFO)
    motto = lesson.motto or f"{lesson.title} — the part of CS you can't skip."
    return f"""{STUB_SENTINEL}
# {lesson.title}

> {motto}

**Type:** {lesson.kind}
**Languages:** {langs}
**Prerequisites:** {prereqs}
**Time:** ~{lesson.minutes} minutes

## Learning Objectives

- Understand the core concept introduced in this lesson and why it matters.
- Implement the lesson's "Build It" artifact from scratch in one of: {langs}.
- Compare your from-scratch implementation against the production tool used in industry.
- Ship the reusable artifact (see "Ship It") and add it to your toolbox.

## The Problem

This lesson sits in **Phase {phase.n:02d} — {phase.name}**. Without the concept it teaches, you cannot
build the phase's capstone ({phase.artifact}). Concretely, *not* knowing this means you get stuck the
moment you try to {phase.desc.lower()}

The next few sections walk through the smallest concrete scenario where this gap hurts, then build
the mental model, then the code, then the production equivalent.

## The Concept

[Concept overview — diagrams and intuition, no code yet. The lesson body will:
- Sketch the data structures / state machines / equations involved.
- Use a worked example with concrete numbers.
- Link to the relevant entry in `glossary/terms.md`.]

## Build It

[Step-by-step implementation in {langs}. Build the smallest correct version first,
then add the realistic refinements (performance, error handling, edge cases).]

### Step 1: Minimal Version

[Code block — runnable.]

### Step 2: Realistic Version

[Code block — handles the edge cases that the minimal version skips.]

## Use It

[How the production tool / library / system actually does this:
- For systems lessons: point to the relevant file in the Linux/PostgreSQL/etcd/LLVM/etc. source tree.
- For algorithm lessons: show the equivalent in the language's standard library.
- For protocol lessons: read the RFC sections that pin this down.
Compare your hand-built version against the production one — what does the production version do
that yours doesn't, and why?]

## Read the Source

- [Production codebase pointer — file path + a 1–2 line note on what to look at.]

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **{phase.artifact if lesson.kind == "Build" else "A self-contained reference snippet you can reuse in later phases."}**

## Exercises

1. **Easy** — Reproduce the from-scratch implementation without looking at the lesson code.
2. **Medium** — Apply the concept to a variation (different input shape, different constraint).
3. **Hard** — Extend the implementation in a way the lesson didn't cover (e.g., concurrency safety,
   alternative representation, formal proof of an invariant).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| _(filled in when lesson body is written)_ |  |  |

## Further Reading

- _(filled in when lesson body is written)_
"""


def code_stub(ext: str, phase: Phase, lesson: Lesson) -> str:
    prefix = COMMENT_PREFIX.get(ext, "#")
    if ext in ("md",):
        return f"{STUB_SENTINEL}\n# Notes — {lesson.title}\n\nDrop in design notes / ASCII diagrams here while the lesson body is being written.\n"
    if ext == "make":
        return f"# {STUB_SENTINEL}\n# Makefile stub for: {lesson.title}\n# TODO: build rules.\n\n.PHONY: all\nall:\n\t@echo \"TODO: implement build for lesson {phase.n:02d}.{lesson.n:02d}\"\n"
    if ext == "dockerfile":
        return f"# {STUB_SENTINEL}\n# Dockerfile stub for: {lesson.title}\nFROM debian:stable-slim\nRUN echo \"TODO: implement {lesson.title}\"\n"
    if ext == "yaml":
        return f"# {STUB_SENTINEL}\n# CI stub for: {lesson.title}\nname: lesson-{phase.n:02d}-{lesson.n:02d}\non: [push]\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - run: echo \"TODO\"\n"
    if ext == "proto":
        return f"// {STUB_SENTINEL}\nsyntax = \"proto3\";\npackage lesson{phase.n:02d}_{lesson.n:02d};\n// TODO: define messages for {lesson.title}\n"
    if ext == "sh":
        return f"#!/usr/bin/env bash\n# {STUB_SENTINEL}\n# Runner stub for: {lesson.title}\nset -euo pipefail\necho \"TODO: implement {lesson.title}\"\n"
    if ext == "tla":
        return f"\\* {STUB_SENTINEL}\n---- MODULE Main ----\nEXTENDS Naturals, Sequences\n\\* TODO: specify {lesson.title}\n===="
    if ext == "coq":
        return f"(* {STUB_SENTINEL} *)\n(* TODO: prove the property introduced in {lesson.title} *)\nDefinition todo : nat := 0.\n"
    if ext == "v":
        return f"// {STUB_SENTINEL}\n// HDL stub for: {lesson.title}\nmodule main;\n  initial $display(\"TODO: {lesson.title}\");\nendmodule\n"
    if ext == "sql":
        return f"-- {STUB_SENTINEL}\n-- SQL stub for: {lesson.title}\n-- TODO: schema + queries.\nSELECT 'TODO' AS status;\n"
    if ext == "py":
        return (f'"""\n{STUB_SENTINEL}\n{lesson.title}\nPhase {phase.n:02d} — {phase.name}\n\nTODO: implement from-scratch version per docs/en.md "Build It".\n"""\n\n\n'
                f'def main() -> None:\n    raise NotImplementedError("Lesson {phase.n:02d}.{lesson.n:02d} body not written yet")\n\n\nif __name__ == "__main__":\n    main()\n')
    if ext == "rs":
        return (f"// {STUB_SENTINEL}\n//! {lesson.title}\n//! Phase {phase.n:02d} — {phase.name}\n//!\n//! TODO: implement from-scratch version per docs/en.md \"Build It\".\n\n"
                f"fn main() {{\n    unimplemented!(\"Lesson {phase.n:02d}.{lesson.n:02d} body not written yet\");\n}}\n")
    if ext == "c":
        return (f"/* {STUB_SENTINEL}\n * {lesson.title}\n * Phase {phase.n:02d} — {phase.name}\n *\n * TODO: implement per docs/en.md \"Build It\".\n */\n#include <stdio.h>\n\n"
                f"int main(void) {{\n    fprintf(stderr, \"TODO: lesson {phase.n:02d}.{lesson.n:02d}\\n\");\n    return 1;\n}}\n")
    if ext == "cpp":
        return (f"// {STUB_SENTINEL}\n// {lesson.title}\n// Phase {phase.n:02d} — {phase.name}\n#include <iostream>\n\nint main() {{\n    std::cerr << \"TODO: lesson {phase.n:02d}.{lesson.n:02d}\\n\";\n    return 1;\n}}\n")
    if ext == "go":
        return (f"// {STUB_SENTINEL}\n// {lesson.title}\npackage main\n\nimport \"fmt\"\n\nfunc main() {{\n    fmt.Println(\"TODO: lesson {phase.n:02d}.{lesson.n:02d}\")\n}}\n")
    if ext == "ts":
        return (f"// {STUB_SENTINEL}\n// {lesson.title}\nfunction main(): void {{\n  console.error(\"TODO: lesson {phase.n:02d}.{lesson.n:02d}\");\n}}\nmain();\n")
    if ext == "hs":
        return (f"-- {STUB_SENTINEL}\n-- {lesson.title}\nmodule Main where\n\nmain :: IO ()\nmain = error \"TODO: lesson {phase.n:02d}.{lesson.n:02d}\"\n")
    if ext == "s":
        return (f"# {STUB_SENTINEL}\n# {lesson.title}\n# RISC-V assembly stub.\n.section .text\n.globl _start\n_start:\n  # TODO: implement\n  li a7, 93\n  li a0, 0\n  ecall\n")
    if ext == "als":
        return f"// {STUB_SENTINEL}\nmodule main\n// TODO: model {lesson.title}\n"
    if ext == "idr":
        return f"-- {STUB_SENTINEL}\nmodule Main\nmain : IO ()\nmain = putStrLn \"TODO\"\n"
    if ext == "pl":
        return f"% {STUB_SENTINEL}\n:- initialization(main).\nmain :- write('TODO'), nl.\n"
    if ext == "erl":
        return f"%% {STUB_SENTINEL}\n-module(main).\n-export([main/0]).\nmain() -> io:format(\"TODO~n\").\n"
    if ext == "ocaml":
        return f"(* {STUB_SENTINEL} *)\nlet () = prerr_endline \"TODO\"\n"
    if ext == "scala":
        return f"// {STUB_SENTINEL}\nobject Main extends App {{ Console.err.println(\"TODO\") }}\n"
    if ext == "cu":
        return (f"// {STUB_SENTINEL}\n// {lesson.title}\n#include <cstdio>\n__global__ void k() {{}}\nint main() {{ k<<<1,1>>>(); cudaDeviceSynchronize(); return 0; }}\n")
    if ext == "glsl":
        return f"// {STUB_SENTINEL}\n// {lesson.title}\nvoid main() {{ /* TODO */ }}\n"
    if ext == "wgsl":
        return f"// {STUB_SENTINEL}\n// {lesson.title}\n@vertex fn vs() -> @builtin(position) vec4f {{ return vec4f(0.0); }}\n"
    return f"{prefix} {STUB_SENTINEL}\n{prefix} TODO: {lesson.title}\n"


def quiz_stub(phase: Phase, lesson: Lesson) -> str:
    payload = {
        "_meta": {
            "scaffold_stub": True,
            "phase": phase.n,
            "lesson": lesson.n,
            "title": lesson.title,
            "note": "Pre/post MCQs will be filled in when the lesson body is written.",
        },
        "questions": [
            {
                "stage": "todo",
                "question": f"(stub) Pre-lesson question for: {lesson.title}",
                "options": ["TODO A", "TODO B", "TODO C", "TODO D"],
                "correct": 0,
                "explanation": "TODO: explain after lesson body is written.",
            },
            {
                "stage": "todo",
                "question": f"(stub) Post-lesson question for: {lesson.title}",
                "options": ["TODO A", "TODO B", "TODO C", "TODO D"],
                "correct": 0,
                "explanation": "TODO: explain after lesson body is written.",
            },
        ],
    }
    return json.dumps(payload, indent=2) + "\n"


def phase_readme(phase: Phase) -> str:
    rows = "\n".join(
        f"| {l.n:02d} | [{l.title}](./{l.n:02d}-{slugify(l.title)}/) | ⬚ | ~{l.minutes} min |"
        for l in phase.lessons
    )
    total = sum(l.minutes for l in phase.lessons)
    return f"""{STUB_SENTINEL}
# Phase {phase.n:02d} — {phase.name}

> {phase.desc}

**Lessons:** {len(phase.lessons)} &nbsp;·&nbsp; **Estimated time:** ~{total // 60} h {total % 60} min
**Phase capstone artifact:** {phase.artifact}

## Lessons

| # | Lesson | Status | Time |
|---|--------|--------|------|
{rows}

**Legend:** ✅ Complete &nbsp;·&nbsp; 🚧 In Progress &nbsp;·&nbsp; ⬚ Planned

## How this phase fits

See [`../../ROADMAP.md`](../../ROADMAP.md) for the full curriculum and prerequisites.
See [`../../LESSON_TEMPLATE.md`](../../LESSON_TEMPLATE.md) for the shape every lesson follows.
"""


def prereqs_for(phase: Phase, lesson: Lesson) -> str:
    if phase.n == 0 and lesson.n == 1:
        return "None — start here."
    if lesson.n == 1:
        return f"Phase {phase.n - 1:02d}"
    return f"Phase {phase.n:02d} lessons 01–{lesson.n - 1:02d}"


# ─────────────────────────── Materialize ───────────────────────────

README_BEGIN = "<!-- BEGIN AUTO-GENERATED PHASES -->"
README_END = "<!-- END AUTO-GENERATED PHASES -->"


def readme_phase_block() -> str:
    """Build the parseable per-phase contents block expected by site/build.js."""
    parts: List[str] = [README_BEGIN, ""]
    for p in PHASES:
        total_min = sum(l.minutes for l in p.lessons)
        parts.append(f"### Phase {p.n}: {p.name} `{len(p.lessons)} lessons`")
        parts.append("")
        parts.append(f"> {p.desc}")
        parts.append("")
        parts.append("| # | Lesson | Type | Lang |")
        parts.append("|---|--------|------|------|")
        for l in p.lessons:
            slug = f"{l.n:02d}-{slugify(l.title)}"
            link = f"phases/{p.n:02d}-{p.slug}/{slug}/"
            lang_display = ", ".join(LANG_INFO[ext][0] for ext in l.languages if ext in LANG_INFO) or "—"
            parts.append(f"| {l.n:02d} | [{l.title}]({link}) | {l.kind} | {lang_display} |")
        parts.append("")
        parts.append(f"_Phase capstone artifact: {p.artifact} &nbsp;·&nbsp; ~{total_min // 60} h {total_min % 60} min total._")
        parts.append("")
    parts.append(README_END)
    return "\n".join(parts)


def inject_readme_block() -> bool:
    """Replace the marker block in README.md with the auto-generated phase contents."""
    path = REPO_ROOT / "README.md"
    if not path.exists():
        return False
    src = path.read_text(encoding="utf-8")
    block = readme_phase_block()
    if README_BEGIN in src and README_END in src:
        new = re.sub(
            re.escape(README_BEGIN) + r".*?" + re.escape(README_END),
            block,
            src,
            count=1,
            flags=re.DOTALL,
        )
    else:
        # Insert before the final license section if present; otherwise append.
        marker = "\n## License"
        if marker in src:
            new = src.replace(marker, "\n" + block + "\n" + marker, 1)
        else:
            new = src + "\n\n" + block + "\n"
    if new != src:
        path.write_text(new, encoding="utf-8")
        return True
    return False


def write_roadmap(force: bool) -> bool:
    """Emit ROADMAP.md at repo root, matching the AI course's status-glyph format."""
    lines = [
        "# Roadmap",
        "",
        "Status tracker for every phase and lesson. The status glyphs in this file feed",
        "the website (`site/build.js` parses them into `site/data.js`); do not change",
        "their shape.",
        "",
    ]
    total_minutes = sum(l.minutes for p in PHASES for l in p.lessons)
    total_lessons = sum(len(p.lessons) for p in PHASES)
    lines.append(f"Total: **{total_lessons} lessons** across **{len(PHASES)} phases**, "
                 f"~{total_minutes // 60} hours at your own pace.")
    lines.append("")
    lines.append("**Legend:** ✅ Complete &nbsp;·&nbsp; 🚧 In Progress &nbsp;·&nbsp; ⬚ Planned")
    lines.append("")
    for p in PHASES:
        ph = sum(l.minutes for l in p.lessons)
        lines.append(f"## Phase {p.n}: {p.name} — ⬚ (~{ph // 60} hours)")
        lines.append("")
        lines.append("| # | Lesson | Status | Est. |")
        lines.append("|---|--------|--------|------|")
        for l in p.lessons:
            slug = f"{l.n:02d}-{slugify(l.title)}"
            link = f"phases/{p.n:02d}-{p.slug}/{slug}/"
            lines.append(f"| {l.n:02d} | [{l.title}]({link}) | ⬚ | ~{l.minutes} min |")
        lines.append("")
    path = REPO_ROOT / "ROADMAP.md"
    if path.exists() and not force:
        existing = path.read_text(encoding="utf-8", errors="ignore")
        if STUB_SENTINEL not in existing and "Status tracker for every phase" not in existing:
            return False
    path.write_text("\n".join(lines), encoding="utf-8")
    return True


def materialize(force: bool) -> dict:
    counts = {"phases": 0, "lessons": 0, "files": 0, "skipped": 0}
    for phase in PHASES:
        pdir = PHASES_DIR / f"{phase.n:02d}-{phase.slug}"
        pdir.mkdir(parents=True, exist_ok=True)
        readme = pdir / "README.md"
        if write_if_stub(readme, phase_readme(phase), force):
            counts["files"] += 1
        else:
            counts["skipped"] += 1
        counts["phases"] += 1
        for lesson in phase.lessons:
            ldir = pdir / f"{lesson.n:02d}-{slugify(lesson.title)}"
            (ldir / "docs").mkdir(parents=True, exist_ok=True)
            (ldir / "code").mkdir(parents=True, exist_ok=True)
            (ldir / "outputs").mkdir(parents=True, exist_ok=True)
            # docs/en.md
            doc = ldir / "docs" / "en.md"
            if write_if_stub(doc, lesson_doc_skeleton(phase, lesson, prereqs_for(phase, lesson)), force):
                counts["files"] += 1
            else:
                counts["skipped"] += 1
            # code stubs
            for ext in lesson.languages:
                fname = CODE_FILENAME.get(ext, f"main.{ext}")
                f = ldir / "code" / fname
                if write_if_stub(f, code_stub(ext, phase, lesson), force):
                    counts["files"] += 1
                else:
                    counts["skipped"] += 1
            # quiz.json
            q = ldir / "quiz.json"
            if write_if_stub(q, quiz_stub(phase, lesson), force):
                counts["files"] += 1
            else:
                counts["skipped"] += 1
            # outputs/.gitkeep
            gk = ldir / "outputs" / ".gitkeep"
            if not gk.exists():
                gk.write_text("", encoding="utf-8")
                counts["files"] += 1
            counts["lessons"] += 1
    return counts


def main() -> int:
    ap = argparse.ArgumentParser(description="Scaffold course-computer-science")
    ap.add_argument("--force", action="store_true", help="Overwrite even non-stub files")
    args = ap.parse_args()
    if write_roadmap(args.force):
        print("ROADMAP.md: written")
    else:
        print("ROADMAP.md: preserved (existing content)")
    if inject_readme_block():
        print("README.md: phases block injected/refreshed")
    else:
        print("README.md: phases block unchanged")
    counts = materialize(args.force)
    print(f"Phases: {counts['phases']}")
    print(f"Lessons: {counts['lessons']}")
    print(f"Files written: {counts['files']}")
    print(f"Files preserved (already had content): {counts['skipped']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
