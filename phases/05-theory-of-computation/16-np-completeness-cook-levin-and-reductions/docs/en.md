# NP-Completeness — Cook-Levin and Reductions

> If your optimization problem turns out to be NP-complete, stop dreaming about a fast algorithm — and start dreaming about a clever reduction.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 05 lessons 01–15
**Time:** ~90 minutes

## Learning Objectives

- Define *NP-completeness* precisely: a problem is NP-complete if it is in NP and every problem in NP reduces to it in polynomial time.
- Explain the Cook-Levin theorem: SAT is NP-complete — sketch the tableau construction that encodes any nondeterministic polynomial-time computation as a Boolean formula.
- Perform polynomial-time reductions by hand and verify their correctness.
- Recognize classic NP-complete problems (3-SAT, Vertex Cover, Hamiltonian Cycle, CLIQUE, Subset Sum) and understand reduction chains connecting them.

## The Problem

After Lesson 15 you know the classes P and NP. But knowing the names does not help when you stare at a scheduling problem, a graph partition, or a packing instance and wonder: *does a fast algorithm exist?* NP-completeness answers that question with a sharp knife: either the problem is in P (unlikely) or it is as hard as the hardest problems in NP. Concretely, without reductions you cannot:

- **Prove your problem is hard.** When a PM asks "can we ship this feature by Friday?", you need to know whether the underlying optimization is NP-complete so you can pick the right strategy (approximation, heuristic, SAT solver).
- **Reuse existing solvers.** Every NP-complete problem reduces to every other one. Encode your problem as SAT and hand it to a production solver (Z3, Glucose) — you get a near-optimal answer for free.
- **Understand the landscape.** The reduction chains (SAT → 3-SAT → Vertex Cover, etc.) show that hundreds of real problems are inter-convertible. One hardness result, thousands of consequences.

## The Concept

### What is NP-completeness?

Recall from Lesson 15: **NP** is the class of decision problems whose "yes" answers have polynomial-time verifiable witnesses. A problem L is **NP-complete** if:

1. L ∈ NP (it has verifiable witnesses).
2. Every problem in NP polynomial-time reduces to L (L is NP-*hard*).

"A reduces to B" (written A ≤_p B) means: there is a polynomial-time function f that transforms any instance x of A into an instance f(x) of B, such that x ∈ A ⟺ f(x) ∈ B.

**Key consequence:** if *any* NP-complete problem is in P, then P = NP. We don't know if P = NP — it's the biggest open question in CS.

### Cook-Levin: SAT is NP-complete

Stephen Cook (1971) and Leonid Levin (1973, independently) proved:

> **Theorem (Cook-Levin).** Boolean Satisfiability (SAT) is NP-complete.

The proof takes an arbitrary NP problem L and shows how to construct, in polynomial time, a Boolean formula that is satisfiable iff the input is in L. The construction uses a **tableau**: imagine running the nondeterministic TM for L on input w for at most p(|w|) steps (p is a polynomial). The tableau is a p(n) × p(n) grid where each cell holds a tape symbol, state, and head position. The formula encodes:

- **Cell constraints:** each cell contains exactly one symbol.
- **Transition constraints:** each 2×2 window of cells is consistent with the TM's transition function.
- **Start constraints:** row 0 encodes the initial configuration.
- **Accept constraints:** at least one cell is the accept state.

If the formula is satisfiable, the satisfying assignment *is* a valid accepting computation. If no accepting computation exists, no assignment works. The formula has O(p(n)²) variables and clauses — polynomial in n.

### Reduction chains

Once SAT is known NP-complete, we prove more problems NP-complete by reducing *from* SAT (or from any already-known NP-complete problem):

| Reduction | Direction | Intuition |
|-----------|-----------|-----------|
| SAT → 3-SAT | Replace long clauses with auxiliary variables | Every SAT instance becomes a 3-CNF with each clause having exactly 3 literals |
| 3-SAT → Vertex Cover | Build a gadget graph per clause | A satisfying assignment selects exactly k vertices to cover all edges |
| 3-SAT → CLIQUE | Literals as nodes, compatible pairs as edges | A satisfying assignment forms a clique of size = number of clauses |
| 3-SAT → Subset Sum | Encode literals as base-10 numbers | A solution picks numbers summing to a target iff the assignment satisfies |
| 3-SAT → Hamiltonian Cycle | Build a traversal gadget per variable and clause | A cycle exists iff the assignment satisfies |

### Practical meaning

If you prove your problem is NP-complete:
- Don't search for a polynomial algorithm (unless you solve P = NP).
- Use a SAT/SMT solver (Z3, Glucose) by encoding your problem.
- Use approximation algorithms (Vertex Cover has a 2-approximation).
- Use heuristics (simulated annealing, genetic algorithms) for practice.
- Restrict the problem structure (trees, planar graphs) to escape NP-completeness.

## Build It

### Step 1: Brute-force SAT solver

A formula is a list of clauses; each clause is a set of (variable, is_negated) pairs. Brute-force: try all 2^n assignments.

```python
def brute_force_sat(formula):
    """formula = list of clauses. Each clause = frozenset of (var_name, is_negated)."""
    variables = set()
    for clause in formula:
        for var, _ in clause:
            variables.add(var)
    variables = sorted(variables)
    for bits in itertools.product([False, True], repeat=len(variables)):
        assignment = dict(zip(variables, bits))
        if all(any(assignment[v] != neg for v, neg in clause) for clause in formula):
            return assignment
    return None  # UNSAT
```

Each clause is satisfied if at least one of its literals is true. A literal `(v, False)` is true when `v` is True; `(v, True)` (negated) is true when `v` is False.

### Step 2: SAT → 3-SAT reduction

A clause with more than 3 literals gets split using auxiliary variables:

```python
def sat_to_3sat(formula):
    """Reduce arbitrary CNF to 3-CNF. Each clause with k>3 literals becomes k-2 clauses."""
    new_clauses = []
    aux_counter = 0
    for clause in formula:
        literals = list(clause)
        if len(literals) <= 3:
            new_clauses.append(frozenset(literals))
        else:
            # Split: (l1 ∨ l2 ∨ ... ∨ lk) becomes:
            # (l1 ∨ l2 ∨ z1) ∧ (¬z1 ∨ l3 ∨ z2) ∧ ... ∧ (¬z_{k-3} ∨ l_{k-1} ∨ lk)
            prev_aux = None
            for i in range(len(literals) - 3):
                aux = f"_aux{aux_counter}"
                aux_counter += 1
                if i == 0:
                    new_clauses.append(frozenset([literals[0], literals[1], (aux, False)]))
                else:
                    new_clauses.append(frozenset([(prev_aux, True), literals[i + 1], (aux, False)]))
                prev_aux = aux
            new_clauses.append(frozenset([(prev_aux, True), literals[-2], literals[-1]]))
    return new_clauses
```

### Step 3: 3-SAT → Vertex Cover reduction

For each clause `(a ∨ b ∨ c)` create a triangle gadget. For each variable, add edges linking negated and non-negated appearances. The minimum vertex cover has size k = n (variables) + 2m (clauses).

```python
def three_sat_to_vc(clauses):
    """Reduce 3-SAT to Vertex Cover. Returns (graph_edges, k)."""
    edges = []
    var_positions = {}  # var -> list of node ids for its appearances

    for ci, clause in enumerate(clauses):
        literals = list(clause)
        # Triangle for this clause
        nodes = [(ci, i) for i in range(3)]
        edges.append((nodes[0], nodes[1]))
        edges.append((nodes[1], nodes[2]))
        edges.append((nodes[2], nodes[0]))
        for j, (v, neg) in enumerate(literals):
            key = (v, neg)
            var_positions.setdefault(key, []).append(nodes[j])

    # Cross-clause edges: (v, pos) <-> (v, neg) for each variable
    vars_set = set(v for clause in clauses for v, _ in clause)
    for v in vars_set:
        pos_nodes = var_positions.get((v, False), [])
        neg_nodes = var_positions.get((v, True), [])
        for pn in pos_nodes:
            for nn in neg_nodes:
                edges.append((pn, nn))

    n_vars = len(vars_set)
    k = n_vars + 2 * len(clauses)
    return edges, k
```

### Step 4: Verify reduction correctness

```python
def verify_reduction(sat_formula, vc_edges, vc_k, assignment):
    """If assignment satisfies the SAT formula, the selected vertices cover all edges."""
    selected = set()
    for ci, clause in enumerate(sat_formula):
        literals = list(clause)
        satisfied_idx = None
        for j, (v, neg) in enumerate(literals):
            if assignment[v] != neg:
                satisfied_idx = j
                break
        # Pick 2 non-satisfied nodes from triangle + all satisfied nodes
        for j in range(3):
            if j == satisfied_idx:
                selected.add((ci, j))
            else:
                pass  # need 2 out of 3
        # Always pick 2 out of 3 to cover triangle
        picked = 0
        for j in range(3):
            if j == satisfied_idx:
                selected.add((ci, j))
                picked += 1
                break
        # Pick remaining 2
        for j in range(3):
            if j != satisfied_idx and picked < 2:
                selected.add((ci, j))
                picked += 1

    covered = all(u in selected or v in selected for u, v in vc_edges)
    return len(selected) <= vc_k and covered
```

Run `python3 code/main.py` to see all reduction demonstrations with trace output.

## Use It

NP-completeness is not just theory — it drives engineering decisions daily:

- **SAT solvers:** Z3 (Microsoft Research), Glucose, CaDiCaL — accept CNF, return satisfying assignments or proofs of UNSAT. Used in software verification, scheduling, hardware model checking.
- **SMT solvers:** Z3 extends SAT with theories (integers, arrays, bitvectors). You encode your domain problem; Z3 decides it.
- **Approximation:** Vertex Cover has a simple 2-approximation (pick both endpoints of an uncovered edge, repeat). TSP on metric spaces has a 1.5-approximation via Christofides.
- **Parameterized complexity:** Vertex Cover is FPT — solvable in O(2^k · n) where k is the cover size. For small k, this is fast.

## Read the Source

- [SAT Competition benchmarks](https://satcompetition.github.io/) — real-world CNF instances used to benchmark SAT solvers.
- [Z3 tutorial](https://theory.stanford.edu/~nikolaj/programmingz3.html) — encode graph coloring, scheduling, and arithmetic constraints as SMT.
- Sipser, *Introduction to the Theory of Computation*, Chapter 7 — the cleanest textbook proof of Cook-Levin.

## Ship It

This lesson ships **`outputs/np_reduction_toolkit.py`** — a self-contained toolkit with a brute-force SAT solver, SAT → 3-SAT reducer, 3-SAT → Vertex Cover reducer, and a reduction verifier. Reuse in any context where you need to encode a problem as SAT or demonstrate NP-completeness.

## Exercises

1. **Easy.** Use `brute_force_sat` to solve the formula `[(A, False), (B, True)] ∧ [(A, True), (C, False)]` — write out the formula in CNF, try all 8 assignments by hand, then verify with code.
2. **Medium.** Implement `three_sat_to_clique(clauses)` — the standard reduction from 3-SAT to CLIQUE. Each clause becomes a group of 3 nodes; add edges between nodes in different groups iff the literals are compatible. Verify with a known satisfiable instance.
3. **Hard.** Implement `subset_sum_from_3sat(clauses)` — reduce 3-SAT to Subset Sum by encoding each literal as a decimal number in a position corresponding to its clause and variable. Prove your reduction correct by showing the correspondence between satisfying assignments and subset sums.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| NP-complete | "Hardest problems in NP" | In NP AND NP-hard (every NP problem reduces to it in poly time) |
| NP-hard | "At least as hard as NP" | Every NP problem reduces to it, but it may not itself be in NP |
| Reduction (≤_p) | "A transforms into B" | A poly-time function f where x ∈ A ⟺ f(x) ∈ B |
| Cook-Levin | "SAT is NP-complete" | The first NP-completeness proof; uses a tableau encoding of an NTM |
| Tableau | "Computation grid" | A p(n) × p(n) table encoding an accepting computation of an NTM |
| Witness | "Certificate" | A string that, combined with the input, lets a poly-time verifier accept |

## Further Reading

- Sipser, *Introduction to the Theory of Computation*, Chapters 7–8 — rigorous proofs of Cook-Levin and classic reductions.
- Garey & Johnson, *Computers and Intractability* — the original NP-completeness catalogue (212 problems, 1979).
- [William Cook, *In Pursuit of the Traveling Salesman*](http://press.princeton.edu/titles/9531.html) — a book-length tour of what happens when your problem is NP-hard and you still need to solve it.
