# Lesson 20: JIT Compilation — V8, JVM, LuaJIT Principles

## Overview

Interpreted code is portable but slow. Ahead-of-time (AOT) compiled code is fast but requires compile time and loses runtime information. **Just-in-time (JIT) compilation** bridges the gap by compiling hot code paths to native machine code at runtime, guided by profiling data collected during execution.

## The JIT Pipeline

A JIT compiler operates in multiple stages during program execution:

```
Source Code
    ↓
Bytecode (portable IR)
    ↓
Interpreter — executes bytecode, collects profiling data
    ↓ (function called N times)
Baseline JIT — quick compilation to native, cached
    ↓ (function called N×10 times, hot path identified)
Optimizing JIT — aggressive optimization using type profiles
    ↓ (type assumption violated)
Deoptimization — bail back to interpreter, discard compiled code
```

## Tiered Compilation

Modern JIT systems use **multiple tiers** of compilation, trading compile speed for execution speed:

| Tier | Compile Speed | Code Quality | Used When |
|------|-------------|-------------|-----------|
| Interpreter | None | Slowest | Initial execution |
| Baseline JIT | Very fast | Moderate | Function seen 100+ times |
| Optimizing JIT | Slow | Near-AOT quality | Function seen 10,000+ times |

The JVM's **Tiered Compilation** starts with the C1 compiler (fast, minimal optimization) and promotes to C2 (slow, aggressive optimization) when profiling confirms a method is hot. V8 follows a similar pattern with Ignition (interpreter) → Sparkplug (baseline) → Maglev (mid-tier optimizing) → TurboFan (full optimizing).

The intuition: most functions run once or twice. Spending time optimizing them is wasted. Only hot functions justify the cost of aggressive optimization.

## Profiling and Hot Spot Detection

The JIT must answer: **which code is worth compiling?**

Common approaches:
- **Invocation counting:** Count how many times a function is called. Compile when a threshold is exceeded.
- **Back-edge counting:** Count how many times a loop iterates. A hot inner loop may be worth compiling even if its containing function is called only once.
- **Sampling:** Periodically interrupt execution and record which instruction is running. Statistical but low overhead.

```
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

// After 1,000 calls → baseline JIT compiles
// After 100,000 calls → optimizing JIT recompiles with type info
```

## Inline Caching (IC)

When accessing a property or calling a method on an object, the runtime must look it up. Without caching, every access performs a full hash table search.

**Inline caching** stores the result of a lookup directly at the call site:

```javascript
// First call: obj.x → looks up x on obj's shape, caches result
// Subsequent calls with same shape: direct memory read (fast)

class Point { constructor(x, y) { this.x = x; this.y = y; } }
let p = new Point(3, 4);
let a = p.x;  // miss — lookup x, cache (Point, offset 0)
let b = p.x;  // hit — read memory directly
```

### Monomorphic vs Polymorphic IC

- **Monomorphic IC:** Only one type seen at this site. Fastest — single comparison and jump.
- **Polymorphic IC:** A few types (2–4) seen. Maintains a small table of shape → offset mappings.
- **Megamorphic IC:** Many types seen. Falls back to hash table lookup. Deoptimization signal.

```
Monomorphic:  if (shape == Point) → offset 0 → fast read
Polymorphic:  if (shape == Point) → offset 0
              if (shape == Rect)  → offset 2 → fast read
Megamorphic:  hashLookup(property) → slow
```

## Hidden Classes (Shapes/VMaps)

JavaScript objects are dynamic — properties can be added or removed at any time. Naive representation (hash maps) is slow.

V8 introduces **hidden classes** (also called **shapes** or **transitions**):

```javascript
// All Point objects created the same way share one hidden class
let p1 = new Point(1, 2);  // hidden class C0: {x at offset 0}
let p2 = new Point(3, 4);  // same hidden class C0
let p3 = new Point(5, 6);  // same hidden class C0

// Adding properties creates transition chains
let obj = {};
obj.x = 1;      // C0 → C1 (x at offset 0)
obj.y = 2;      // C1 → C2 (y at offset 1)
obj.z = 3;      // C2 → C3 (z at offset 2)
```

Objects with the same hidden class store properties at the same offsets, enabling inline caching to work by comparing a single hidden class pointer rather than inspecting the object's structure.

## On-Stack Replacement (OSR)

A hot loop inside a rarely-called function presents a problem: the JIT normally compiles at function entry. But if the loop is executing and the invocation count is low, the JIT won't trigger.

**On-Stack Replacement** allows the JIT to replace an interpreted stack frame with a compiled one **while the loop is running**:

```
Stack during execution:
┌──────────────────────┐
│  main()              │
│  process() [interp]  │  ← hot loop running here
└──────────────────────┘

After OSR:
┌──────────────────────┐
│  main()              │
│  process() [native]  │  ← replaced mid-execution
└──────────────────────┘
```

This requires mapping interpreter state (local variables, stack) to compiled code state — a complex but worthwhile optimization for benchmark-heavy workloads.

## Deoptimization

JIT optimizations are **speculative**. The compiler assumes types won't change, but JavaScript is dynamic. When assumptions break:

```javascript
function add(a, b) { return a + b; }
// JIT assumes: both arguments are integers, emits integer add
add(1, 2);    // compiled code runs — fast
add("1", 2);  // assumption violated! deoptimize
```

**Deoptimization** patches the compiled code to jump back to the interpreter, discards the optimized version, and re-profiles. The JIT may later recompile with weaker assumptions.

## Stack Maps and GC Integration

JIT-compiled code must interoperate with the garbage collector. The compiler emits **stack maps** — tables recording which stack slots and registers contain object pointers at each GC-safe point. When a GC occurs in compiled code, the collector consults the stack map to find roots.

## JIT in Practice

| System | Runtime | Tiers | Notable Features |
|--------|---------|-------|-----------------|
| V8 | JavaScript/Node.js | Ignition → Sparkplug → Maglev → TurboFan | Hidden classes, concurrent compilation |
| JVM | Java/Kotlin/Scala | C1 (client) → C2 (server) | Tiered compilation, GraalVM as alternative |
| LuaJIT | Lua | Interpreter → trace JIT | Records hot traces, not functions; extremely fast |
| PyPy | Python | Interpreter → tracing JIT | Meta-tracing, much faster than CPython |
| .NET CLR | C#/F# | Interpreter → Tier0 → Tier1 | Dynamic PGO, OSR in .NET 9+ |

## Build It

In the companion code, we build a simplified JIT system with bytecode interpretation, hot spot detection, a basic code generator, and inline caching. Benchmarks compare interpreted vs JIT-compiled execution.

## Use It

**V8 flags:** `--trace-opt` shows optimized functions. `--trace-deopt` shows deoptimizations. `--print-code` dumps generated machine code.

**JVM flags:** `-XX:+PrintCompilation` prints JIT compilation events. `-XX:+UnlockDiagnosticVMOptions -XX:+PrintInlining` shows inlining decisions.

**LuaJIT:** `jit.on()` / `jit.off()`. `jit.trace` displays compiled traces.

## Ship It

The JIT demo shows how profiling-guided compilation can speed up hot code by 10–100× over interpretation, while keeping cold code cheap to interpret.

## Exercises

**Level 1:** Add invocation counting to the bytecode interpreter. Identify the top 5 hottest functions after running a benchmark.

**Level 2:** Implement a monomorphic inline cache for property access. Measure the speedup over uncached lookup for a loop that reads the same property 1,000,000 times.

**Level 3:** Add deoptimization support to the JIT compiler. When compiled code encounters an unexpected type, patch the call site to jump back to the interpreter and record the failure.
