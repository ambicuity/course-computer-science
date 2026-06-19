# Graph Theory II — Coloring, Matching, Planarity

> Two adversarial classes of problems: "use as few resources as possible" (coloring) and "match as many things as possible" (matching). Plus the puzzle of which graphs you can draw without crossings.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lesson 17
**Time:** ~75 minutes

## Learning Objectives

- Define proper graph coloring; compute or bound the chromatic number χ(G) for small graphs.
- Distinguish bipartite graphs (2-colorable) from non-bipartite (need ≥ 3 colors), with a BFS test.
- Find a maximum bipartite matching via augmenting paths in O(V · E); reduce real problems (job assignment, register allocation) to bipartite matching.
- State the Four Color Theorem; understand why planarity is the distinguishing graph property and how Euler's formula `V - E + F = 2` constrains it.

## The Problem

The seemingly unrelated questions:

- "How many time slots do I need to schedule exams so no student has two at once?" — graph coloring.
- "How many CPU registers does this function need?" — graph coloring (Phase 08).
- "Match interns to projects so every intern has one project and every project gets one intern." — bipartite matching.
- "Can I lay out this circuit on a single-layer PCB without trace crossings?" — planarity.

Each is a classical graph problem with rich algorithms. This lesson walks the three.

## The Concept

### Graph coloring

A **proper k-coloring** of G = (V, E) assigns a color in {1, …, k} to each vertex so that adjacent vertices get different colors. The **chromatic number** χ(G) is the smallest such k.

Key facts:
- χ(complete graph K_n) = n.
- χ(any tree) ≤ 2.
- χ(any cycle of even length) = 2; odd length = 3.
- χ(G) ≤ Δ(G) + 1, where Δ is the maximum degree (Brooks' theorem).
- Determining χ(G) exactly is NP-hard.

The **greedy coloring** algorithm: order the vertices, assign each the smallest color not used by an already-colored neighbor. Result is ≤ Δ + 1 colors. Sometimes χ, but order matters.

### Bipartite graphs

G is **bipartite** iff its vertices can be partitioned into two sets A, B with every edge going between A and B. Equivalent: G has no odd cycle. Equivalent: χ(G) ≤ 2.

Test via BFS: assign start to side 0, neighbors to side 1, their neighbors to side 0, … If any edge violates, G is not bipartite.

### Matchings

A **matching** in a graph is a set of edges with no shared vertices. A **maximum matching** is the largest such set.

For **bipartite** graphs, finding a maximum matching is polynomial-time via augmenting paths (Hopcroft-Karp's O(E · √V), simpler O(V · E)). The classic algorithm:

```
While an augmenting path exists (an alternating path from unmatched to unmatched):
    flip every edge along it (matched ↔ unmatched).
```

For **general** (non-bipartite) graphs, Edmonds' blossom algorithm solves it in O(V³). Out of scope here.

### Planarity

A graph is **planar** iff it can be drawn in the plane with no edge crossings. Beautiful constraint:

> **Euler's formula** (for connected planar graphs): `V - E + F = 2`,
> where F is the number of faces (regions, including the outer one).

Two famous non-planar graphs:
- **K₅** (complete graph on 5 vertices): 10 edges, 5 vertices. Planar would need F = 2 + E - V = 7 faces; each face borders ≥ 3 edges, so 2E ≥ 3F, contradiction.
- **K₃,₃** (complete bipartite 3+3): similar argument with girth 4.

**Kuratowski's theorem** says a graph is non-planar iff it contains a subdivision of K₅ or K₃,₃.

### The Four Color Theorem

Every planar graph is 4-colorable (Appel-Haken 1976, computer-assisted proof). Hence: every map can be colored with 4 colors so no two adjacent regions share a color.

## Build It

Open `code/main.py`.

### Step 1: Greedy coloring on canonical small graphs

K₄ (needs 4), C₅ — cycle of length 5 (needs 3), C₆ (needs 2).

### Step 2: Bipartiteness via BFS

C₆ is bipartite, C₅ is not, complete bipartite K₃,₃ is.

### Step 3: Maximum bipartite matching via augmenting paths

Kuhn's algorithm: for each left vertex try a DFS that finds an augmenting path.

### Step 4: Euler's formula

A cube graph: V=8, E=12, F=6. Verify V - E + F = 2.

### Step 5: K₅ counting argument

Show that any planar drawing of K₅ would require F = 7, but each face borders ≥ 3 edges so 2E ≥ 3F gives 20 ≥ 21 — contradiction. Hence K₅ is non-planar.

## Use It

- **Register allocation** (Phase 08): treat variables as vertices, edges as interference (live at the same time). χ ≤ k means k registers suffice.
- **Exam scheduling**: minimize time slots = minimize colors.
- **Frequency assignment**: cell towers as vertices, edges if too close — coloring → least frequency band.
- **Job assignment**: bipartite matching.
- **Web layout / typesetting**: planar embedding for tree-of-content rendering.
- **Computational topology**: Euler characteristic generalizes to surfaces.

## Read the Source

- *Graph Theory* by Diestel — chapters on coloring and planarity, free PDF.
- [Hopcroft-Karp 1973 paper](https://epubs.siam.org/doi/10.1137/0202019) — O(E√V) bipartite matching.
- *Compilers: Principles, Techniques, and Tools* (Dragon Book) — Chapter 8 on register allocation via graph coloring.

## Ship It

This lesson ships **`outputs/coloring.py`** (greedy + bipartite check) and **`outputs/matching.py`** (bipartite maximum matching via Kuhn's algorithm).

## Exercises

1. **Easy.** Build the Petersen graph (10 vertices, 15 edges). Compute its chromatic number (should be 3) and verify it's not bipartite.
2. **Medium.** Show that greedy coloring with the wrong vertex order can use χ + Ω(n) colors on a star-like graph. Find an ordering that achieves χ.
3. **Hard.** Implement Hopcroft-Karp's O(E√V) bipartite matching from scratch; benchmark against the simpler Kuhn's algorithm on graphs of varying sparsity.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Chromatic number | "How many colors" | Smallest k such that G has a proper k-coloring |
| Bipartite | "Two-colorable" | Vertices partition into two sets with no edge inside either set; ⇔ no odd cycle |
| Matching | "Edge set with no shared endpoints" | Maximum matching is polynomial in bipartite graphs, O(V³) in general |
| Planar | "Drawable without crossings" | Equivalent: no K₅ or K₃,₃ subdivision (Kuratowski) |
| Augmenting path | "Alternating path between unmatched ends" | Flipping its edges grows the matching by one |

## Further Reading

- [Visualization of bipartite matching with augmenting paths](https://visualgo.net/en/matching) — animated.
- *Combinatorial Optimization* by Schrijver — graduate-level, full coverage of matching algorithms.
- *Algorithm Design* by Kleinberg & Tardos — gentle chapter on graph coloring + bipartite matching.
