# Analyzing Algorithms — Recurrences, Master Theorem in Action

> A recurrence relation is the fingerprint of a divide-and-conquer algorithm — read it, and you know its running time.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 01 (asymptotic notation, recurrence relations), Phase 04 (lesson prior knowledge)
**Time:** ~60 minutes

## Learning Objectives

- Solve recurrence relations using the recursion tree method and verify results with the Master Theorem
- Apply all three cases of the Master Theorem to classify and solve common recurrences
- Construct substitution-method proof skeletons to verify guessed asymptotic bounds

## The Problem

You implement merge sort. The recursive call halves the input, then a linear-time merge stitches the halves back together. You want to know the total running time, but the recursion keeps splitting — how do you sum up work across all levels?

A recurrence relation expresses the running time in terms of the running time of smaller inputs. For merge sort: T(n) = 2T(n/2) + n. Solving this recurrence gives you T(n) = Θ(n log n). Without a systematic method, you're guessing. With one, you can analyze any divide-and-conquer algorithm rigorously.

This lesson builds three tools: the recursion tree for visual intuition, the Master Theorem for quick classification, and the substitution method for formal proofs.

## The Concept

### Recurrence Relations

A recurrence relation for a divide-and-conquer algorithm has the form:

```
T(n) = aT(n/b) + f(n)
```

- **a** — number of subproblems at each level
- **b** — factor by which subproblem size shrinks
- **f(n)** — work done outside recursive calls (merging, partitioning, etc.)

For merge sort: a=2, b=2, f(n)=n. For binary search: a=1, b=2, f(n)=1.

### Recursion Tree Method

Expand the recurrence level by level. Each level multiplies the number of subproblems by a and divides their size by b. Sum the work across all levels.

```
T(n) = 2T(n/2) + n

Level 0:           n                    → work: n
                  / \
Level 1:       n/2  n/2                 → work: n
               /\   /\
Level 2:    n/4 n/4 n/4 n/4             → work: n
              ...                       → ...
Level k:    n/2^k each (2^k nodes)      → work: n

Levels: log₂ n
Total: n · log₂ n = Θ(n log n)
```

### Master Theorem

Given T(n) = aT(n/b) + f(n), compute n^(log_b(a)) and compare it to f(n):

| Case | Condition | Solution |
|------|-----------|----------|
| 1 | f(n) = O(n^(log_b(a) - ε)) for some ε > 0 | T(n) = Θ(n^(log_b(a))) |
| 2 | f(n) = Θ(n^(log_b(a)) · log^k n) | T(n) = Θ(n^(log_b(a)) · log^(k+1) n) |
| 3 | f(n) = Ω(n^(log_b(a) + ε)) and regularity holds | T(n) = Θ(f(n)) |

Case 1: the root dominates — work at the top grows faster than leaves accumulate.
Case 2: work is balanced across all levels — each level contributes equally.
Case 3: the leaves dominate — but f(n) must satisfy the regularity condition a·f(n/b) ≤ c·f(n) for c < 1.

### Worked Examples

**Example 1:** T(n) = 2T(n/2) + n → a=2, b=2, n^(log_2 2) = n. f(n) = n = n^1. Case 2 with k=0. **T(n) = Θ(n log n).**

**Example 2:** T(n) = T(n/2) + 1 → a=1, b=2, n^(log_2 1) = n^0 = 1. f(n) = 1 = 1 · log^0 n. Case 2 with k=0. **T(n) = Θ(log n).**

**Example 3:** T(n) = 3T(n/4) + n log n → a=3, b=4, n^(log_4 3) ≈ n^0.793. f(n) = n log n = Ω(n^(0.793+ε)). Regularity: 3·(n/4)·log(n/4) ≤ c·n·log n for c=3/4 < 1. Case 3. **T(n) = Θ(n log n).**

### When the Master Theorem Doesn't Apply

If f(n) oscillates, if the regularity condition fails, or if b is not a constant divisor, fall back to the recursion tree or substitution method. For example, T(n) = 2T(n/2) + n sin n has a non-polynomial f(n) that doesn't cleanly fit any case.

## Build It

### Step 1: Recursion Tree Visualizer

We build a function that expands a recurrence into an ASCII tree, printing the work at each level and computing the total.

### Step 2: Master Theorem Solver

We build a function that takes a, b, and f(n) (as a string descriptor), classifies which case applies, and returns the closed-form solution.

### Step 3: Substitution Method Proof Assistant

We build a helper that generates the inductive step skeleton: assume T(k) ≤ c·g(k) for all k < n, then show T(n) ≤ c·g(n).

## Use It

Recurrence analysis appears everywhere in computer science:

- **CLRS** uses the Master Theorem to analyze merge sort (Ch. 4.3), binary search, and divide-and-conquer matrix multiplication
- **Compiler optimizers** model loop nesting costs using recurrence relations on basic block frequencies
- **Database query planners** estimate the cost of recursive CTEs and recursive join algorithms with recurrence-based cost models
- **Cache-oblivious algorithms** are analyzed using recurrences that account for cache miss patterns

## Read the Source

- [CLRS Ch. 4 — Divide-and-Conquer](https://mitpress.mit.edu/books/introduction-algorithms-fourth-edition) — the canonical treatment of recurrence solving

## Ship It

The final artifact is a recurrence solver CLI script saved in `outputs/`. Run it with `python outputs/recurrence_solver.py` to solve any T(n) = aT(n/b) + f(n).

## Exercises

1. **Easy.** Solve T(n) = 4T(n/2) + n using the Master Theorem. Which case applies?
2. **Medium.** Use the recursion tree method to solve T(n) = 3T(n/3) + n. Verify with Master Theorem.
3. **Hard.** Prove by substitution that T(n) = 2T(n/2) + n log n is Θ(n log² n).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Recurrence relation | "A formula for recursive functions" | An equation that defines a function in terms of its value on smaller inputs |
| Master Theorem | "A magic formula for recurrences" | A theorem that classifies T(n)=aT(n/b)+f(n) into three cases based on how f(n) compares to n^(log_b(a)) |
| Regularity condition | "A technicality in Case 3" | a·f(n/b) ≤ c·f(n) for some c < 1, ensuring f(n) grows fast enough that the root dominates |
| Substitution method | "Guessing the answer" | Guessing a bound and proving it by induction — the guess must come from intuition or recursion tree analysis |

## Further Reading

- [CLRS, Chapter 4](https://mitpress.mit.edu/books/introduction-algorithms-fourth-edition) — rigorous treatment of all three solution methods
- [Jeff Erickson's Algorithms, Ch. 2](http://jeffe.cs.teaching.algorithms/books/algorithms.pdf) — excellent intuition-first approach to recurrence analysis
