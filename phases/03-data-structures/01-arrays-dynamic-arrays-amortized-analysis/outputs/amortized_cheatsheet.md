# Amortized analysis cheatsheet

| Operation | Amortized | Why |
|-----------|-----------|-----|
| Dynamic array `push` (2× growth) | O(1) | Doubling absorbs n copies across n pushes ≤ 2n. |
| Dynamic array `push` (+k growth) | O(n) | Each k pushes copy the entire array → ≈n²/2k. |
| Dynamic array `pop` w/ shrink-at-1/4 | O(1) | Shrink threshold prevents push-pop oscillation. |
| Stack-via-array `push`/`pop` | O(1) | Same as above. |
| Union-Find with path compression | α(n) ≈ O(1) | Inverse Ackermann. |
| Splay tree access | O(log n) | Amortized over any sequence. |
| Incremental GC mark | O(1) per write | Spread the marking cost across writes. |

## Three techniques

| Technique | Mental model | When to reach for it |
|-----------|--------------|----------------------|
| **Aggregate** | "Total cost across n ops" | Simple, when costs sum cleanly |
| **Accounting** | "Charge each op a constant; bank the surplus" | Different ops with different actual costs |
| **Potential (Φ)** | "Stored-up work as a function of state" | Complex data-structure invariants (splay, Fib-heap) |

All three are equivalent — they prove the same theorem. Pick whichever fits the structure.

## Three myths

1. **"Amortized = average".** No. Amortized is **worst-case average across any sequence** — much stronger than "expected" (probability).
2. **"Amortized analysis lets you ignore individual slow ops".** No. The worst single op is still O(n). If you need bounded latency (real-time), amortized isn't enough — you need worst-case.
3. **"Doubling is wasteful".** Memory peak is 2n; allocator usually reclaims fragments. Push-only loops are 2-3× faster than `push_back`-on-reserved (cache-friendly). Trust the standard.
