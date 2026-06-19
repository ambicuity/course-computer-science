# Space Complexity — L, NL, PSPACE, Savitch

> Time tells you how long a computation takes. Space tells you how much scratch paper it needs — and sometimes that distinction changes everything.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 05 lessons 01–16
**Time:** ~75 minutes

## Learning Objectives

- Define space complexity classes L (log-space), NL (nondeterministic log-space), and PSPACE (polynomial space).
- State and explain Savitch's theorem: NSPACE(f(n)) ⊆ SPACE(f(n)²), which implies NL ⊆ L² = O((log n)²).
- Identify NL-complete (Reachability) and PSPACE-complete (TQBF) problems and explain why completeness in space classes matters.
- Implement space-bounded Turing machine simulation and Savitch's reachability algorithm.

## The Problem

Lessons 15–16 classified problems by *time*. But time is not the only resource: some computations use very little memory even when they take a long time, and vice versa. Space complexity captures this:

- **Model checking** (does this protocol satisfy a temporal logic specification?) uses polynomial time but often only needs polynomial space — and it is PSPACE-complete.
- **Game solving** (chess on an n × n board, Go) is PSPACE-complete: the game tree has exponential depth but polynomial width.
- **Reachability in a graph** can be solved in O(n) space by a nondeterministic machine guessing a path — yet deterministic algorithms seem to need O(n²) space (Savitch's theorem shows this is essentially optimal).

Without space complexity you cannot reason about these, and you miss the surprising fact that *nondeterminism in space is far cheaper than nondeterminism in time.*

## The Concept

### Measuring space

A Turing machine's **space complexity** is the number of tape cells it uses (beyond the input). We measure this as a function f(n) of the input length n. The input tape is read-only; the work tape is where space is counted.

### The hierarchy

| Class | Definition | Intuition |
|-------|-----------|-----------|
| L | DSPACE(log n) | A few pointers' worth of memory |
| NL | NSPACE(log n) | Same, but the machine can guess |
| P | DTIME(poly n) | Polynomial time (uses at most poly space) |
| PSPACE | DSPACE(poly n) | Polynomial memory, any time |
| EXP | DTIME(2^poly n) | Exponential time |

Known inclusions: **L ⊆ NL ⊆ P ⊆ PSPACE ⊆ EXP**

The critical open question: is L = NL? (Analogous to P = NP for space.) Most believe L ≠ NL but no proof exists.

### Savitch's theorem

**Theorem (Savitch, 1970).** For any f(n) ≥ log n:

NSPACE(f(n)) ⊆ DSPACE(f(n)²)

The proof is beautiful: to decide reachability (the canonical NL problem) deterministically in O(log² n) space, use a divide-and-conquer strategy. To check if there's a path from s to t in ≤ n steps, guess a midpoint m and recursively check s → m and m → t in ≤ n/2 steps each. The recursion depth is O(log n), each frame stores O(log n) bits (a node ID), so total space is O(log² n).

This means: **NL ⊆ L²** — nondeterministic log-space problems can be solved deterministically in O((log n)²) space.

### NL-completeness: Reachability

**Reachability** (given a directed graph G and nodes s, t, is there a path from s to t?) is NL-complete. It's the "SAT of space complexity."

- It's in NL: guess a path node by node using log-space.
- Every NL problem reduces to it: the computation graph of a log-space NTM has polynomially many configurations; reachability in that graph is equivalent to the original problem.

### PSPACE-completeness: TQBF

A **quantified Boolean formula** (QBF) has quantifiers: ∀x₁∃x₂∀x₃...φ(x₁, x₂, x₃, ...). TQBF (True Quantified Boolean Formula) asks: is the formula true?

TQBF is PSPACE-complete. The quantifier alternation forces any algorithm to explore exponentially many branches but requires only polynomial space to remember the current path. This makes PSPACE the natural class for:

- **Game playing:** "Does white have a winning strategy?" is a ∀∃ formula.
- **Model checking:** temporal logic formulas over state spaces.
- **Planning:** "Does a sequence of actions exist to reach the goal?"

## Build It

### Step 1: Space-bounded TM simulator

Simulate a Turing machine while tracking the number of work-tape cells used.

```python
def logspace_tm_simulator(tm, input_str, space_bound):
    """Simulate TM with bounded work tape. Returns (accepted, max_cells_used)."""
    tape = list(input_str)
    work_tape = ["_"] * space_bound
    head = 0
    work_head = 0
    state = tm["start"]

    for _ in range(tm.get("max_steps", 10000)):
        if state == tm["accept"]:
            return True, work_head + 1
        if state == tm["reject"]:
            return False, work_head + 1

        read_input = tape[head] if head < len(tape) else "_"
        read_work = work_tape[work_head]
        key = (state, read_input, read_work)

        if key not in tm["transitions"]:
            return False, work_head + 1

        new_state, write_work, input_dir, work_dir = tm["transitions"][key]
        work_tape[work_head] = write_work

        if input_dir == "R":
            head = min(head + 1, len(tape))
        elif input_dir == "L":
            head = max(head - 1, 0)

        if work_dir == "R":
            work_head += 1
            if work_head >= space_bound:
                return False, space_bound
        elif work_dir == "L":
            work_head = max(work_head - 1, 0)

        state = new_state

    return False, work_head + 1
```

### Step 2: Savitch's reachability algorithm

```python
def reachability_savitch(graph, s, t, n):
    """Deterministic O(log² n) space reachability.

    Uses divide-and-conquer: is there a path s→t in ≤ 2^steps edges?
    Each recursive call stores O(log n) bits (a node ID).
    Recursion depth: O(log n). Total space: O(log² n).
    """
    def reachable(u, v, steps):
        if steps == 0:
            return u == v
        if steps == 1:
            return u == v or v in graph.get(u, [])
        mid = steps // 2
        # Try every possible midpoint — uses O(log n) space for m
        for m in range(n):
            if reachable(u, m, mid) and reachable(m, v, steps - mid):
                return True
        return False

    # Use n as upper bound on path length
    import math
    max_steps = math.ceil(math.log2(max(n, 2))) + 1
    # Actually: path length is at most n, so use n directly
    return reachable(s, t, n)
```

### Step 3: TQBF evaluator

```python
def tqbf_evaluator(formula):
    """Evaluate a quantified Boolean formula.

    formula = {"quantifiers": [("x1", "forall"), ("x2", "exists"), ...],
               "body": <propositional formula over x1, x2, ...>}
    """
    quantifiers = formula["quantifiers"]
    body = formula["body"]

    def eval_inner(remaining_q, env):
        if not remaining_q:
            return body(env)

        var, qtype = remaining_q[0]
        rest = remaining_q[1:]

        if qtype == "forall":
            return (eval_inner(rest, {**env, var: True}) and
                    eval_inner(rest, {**env, var: False}))
        else:  # exists
            return (eval_inner(rest, {**env, var: True}) or
                    eval_inner(rest, {**env, var: False}))

    return eval_inner(quantifiers, {})
```

Run `python3 code/main.py` to see space usage tracking and Savitch's algorithm in action.

## Use It

Space complexity classes matter in practice:

- **Model checking** (SPIN, TLA+, NuSMV): verifying that a concurrent protocol never reaches a bad state is PSPACE-complete. These tools work because the state space, while exponential, has structure that pruning can exploit.
- **Game solving:** Generalized chess (n × n) and Go are EXPTIME-complete or PSPACE-complete. The minimax tree has polynomial branching factor but exponential depth.
- **Reachability in static analysis:** Computing whether an error state is reachable in a program's control-flow graph is an NL-type problem. Tools like Facebook's Infer use log-space approximations.
- **Ladner's theorem:** If P ≠ NP, there exist problems in NP \ NP-complete (intermediate problems). The same holds for L and NL: if L ≠ NL, there are NL-intermediate problems.

## Read the Source

- Sipser, *Introduction to the Theory of Computation*, Chapter 8 — clean proofs of Savitch's theorem and PSPACE-completeness of TQBF.
- [Immerman–Szelepcsényi theorem](https://en.wikipedia.org/wiki/Immerman%E2%80%93Szelepcs%C3%A9nyi_theorem) — NL = coNL (nondeterministic log-space is closed under complement). A striking result with an elegant counting proof.

## Ship It

This lesson ships **`outputs/space_toolkit.py`** — a reference with Savitch's reachability, TQBF evaluation, and space-bounded simulation. Reuse when reasoning about memory-bounded computation, model checking encodings, or game-tree search.

## Exercises

1. **Easy.** Trace through `reachability_savitch` on the graph `{0: [1], 1: [2], 2: [3]}` with s=0, t=3. How many recursive calls are made? What is the maximum recursion depth?
2. **Medium.** Implement `is_nl_complete(problem_spec)` — given a problem described as a reachability instance on a configuration graph, show it encodes a log-space bounded NTM computation. Verify on the REACHABILITY problem itself.
3. **Hard.** Implement a game-tree evaluator that takes a game position (represented as a QBF: ∀ move by player 1, ∃ response by player 2, ...) and determines the outcome using polynomial space. Test on tic-tac-toe.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| L | "Log-space" | Problems solvable using O(log n) work-tape cells |
| NL | "Nondeterministic log-space" | Same, but the machine can guess (log-space verifier) |
| PSPACE | "Polynomial space" | Problems solvable using O(n^k) work-tape cells |
| Savitch's theorem | "NSPACE(f) ⊆ SPACE(f²)" | Nondeterminism in space costs at most a quadratic blowup |
| TQBF | "Quantified SAT" | Evaluating ∀x₁∃x₂...φ — the canonical PSPACE-complete problem |
| NL-complete | "Hardest NL problem" | In NL and every NL problem log-space reduces to it |

## Further Reading

- Sipser, *Introduction to the Theory of Computation*, Chapter 8.
- Papadimitriou, *Computational Complexity*, Chapters 7–8 — deeper treatment of space classes and the Immerman–Szelepcsényi theorem.
- [Arora & Barak, *Computational Complexity: A Modern Approach*](https://theory.cs.princeton.edu/complexity/), Chapter 4 — modern treatment with circuit complexity connections.
