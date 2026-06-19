# Control Flow — Branches, Loops, Recursion

> Three machines: a tape (the program counter), a stack (call/return), and a register holding a comparison flag. Every if, for, while, function call is just bookkeeping over these three.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** Phase 02, Lessons 01-02
**Time:** ~60 minutes

## Learning Objectives

- Map `if`, `else`, `switch` to the conditional-jump instructions the CPU actually executes.
- Map `for`, `while`, `do-while` to the same conditional jumps; explain why C's `for(init; cond; step)` is just sugar over `while`.
- Write a loop with a clear **loop invariant** — and use the invariant to argue correctness (foundation for Hoare logic, Phase 17).
- Recognize when recursion is the right tool, and when to convert to iteration (tail recursion, manual stack).

## The Problem

Every program's behavior reduces to one of three patterns:

- **Sequence**: do A, then B.
- **Selection**: depending on a condition, do A or B.
- **Iteration**: keep doing A while a condition holds.

(Recursion is a fourth pattern, but it's iteration + a stack.) The Structured Program Theorem (Böhm & Jacopini, 1966) says these three suffice — no `goto` needed. The whole control-flow analysis of compilers (Phase 08), correctness reasoning (Phase 17), and parallelization (Phase 13) builds on understanding these three deeply.

## The Concept

### Branches

A conditional `if (cond) X else Y` becomes:

```
   compare operands of cond
   conditional jump to Y-label if !cond
   X-code
   jump to end-label
Y-label:
   Y-code
end-label:
```

CPU instructions for this: `cmp`, `je`, `jne`, `jl`, `jg`, `jmp`, etc. Modern CPUs use **branch prediction** to guess which path will run; mispredicts cost ~10-20 cycles. Phase 06 dives into this.

`switch` typically compiles to either a *jump table* (for dense cases) or a chain of `if`s. Compilers heuristically choose.

### Loops

A `while (cond) X` becomes:

```
loop:
   compare operands of cond
   jump to end-label if !cond
   X-code
   jump to loop
end-label:
```

`for (init; cond; step) X` desugars to:

```
init;
while (cond) {
    X;
    step;
}
```

`do { X } while (cond)` runs the body at least once; the conditional jump is at the bottom.

### Loop invariants

A **loop invariant** is a statement that's true *every time control reaches the top of the loop*. It's the most important tool for proving a loop correct.

Standard structure of an invariant proof:

1. **Initialization**: the invariant holds before the first iteration.
2. **Maintenance**: if the invariant holds at the top of iteration k, it holds at the top of iteration k+1.
3. **Termination**: when the loop exits, the invariant + the exit condition imply the postcondition (what you wanted to prove).

Example (sum of [0..n)):

```c
int sum_to(int n) {
    int sum = 0;
    for (int i = 0; i < n; ++i) {
        sum += i;
    }
    return sum;
}
```

Invariant: **at the top of each iteration, `sum == 0 + 1 + ... + (i-1)`**.

- Init (before i=0): sum = 0, empty sum. ✓
- Maint: if sum = 0+...+(i-1), then after `sum += i` we have sum = 0+...+i, and i becomes i+1, so the invariant restores.
- Term: when the loop exits, i = n and sum = 0+...+(n-1). ✓

Writing the invariant explicitly is what lets you reason about more interesting algorithms (binary search, partition, gcd) — every CS proof of a loop's correctness uses this pattern.

### Recursion

Recursion uses the **call stack** instead of an explicit loop variable. The stack frame holds the local state at each level.

```c
int factorial(int n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}
```

Each `factorial(n - 1)` call pushes a new frame; when the base case returns, frames pop and multiply on the way out.

**Tail recursion**: when the recursive call is the *last* operation, the compiler can replace the call with a jump and reuse the current frame — turning recursion into a loop with O(1) stack use. Languages like Scheme and Haskell mandate tail-call optimization (TCO); C compilers do it opportunistically; Python and JavaScript don't (intentionally — they want clearer stack traces).

Rewrite of factorial to be tail-recursive:

```c
int factorial_tail(int n, int acc) {
    if (n <= 1) return acc;
    return factorial_tail(n - 1, n * acc);
}
/* caller: factorial_tail(n, 1) */
```

The recursive call is the last thing — eligible for TCO.

### When to use which

| Situation | Use |
|-----------|-----|
| Tree / graph traversal | Recursion (natural fit) |
| Backtracking | Recursion |
| Iterating over a sequence | A loop |
| Tail-recursive accumulator | Loop (manual TCO) |
| Stack depth might exceed limit (~10K calls) | Convert to explicit stack |

## Build It

Open `code/main.c`. Five patterns:

### Step 1: `if` + `switch`

A function that classifies an int into "negative, zero, positive" using both forms.

### Step 2: Three loop forms producing the same output

`for`, `while`, `do-while` summing 1..10.

### Step 3: Binary search with explicit loop invariant

Invariant: "if the target exists in the array, it's in `arr[lo..hi)`."

### Step 4: Recursive factorial + tail-recursive form

Compare results.

### Step 5: Manual recursion-to-iteration

Convert recursive `fibonacci(n)` to an iterative one.

### Rust counterpart

`code/main.rs` implements the same patterns plus shows Rust's pattern matching (`match`) — the modern descendant of `switch`.

## Use It

- **Algorithm design** (Phase 04): every algorithm is built from these three patterns.
- **Loop invariants** (Phase 17): the basis of Hoare-logic proofs, TLA+ specifications, and most algorithm correctness arguments.
- **Compilers** (Phase 08): control-flow graphs derive directly from branch/loop structure.
- **Performance engineering** (Phase 15): branch prediction, loop unrolling, vectorization — all techniques to make these primitives faster.
- **Embedded / kernel code**: explicit `goto` is sometimes preferred over deeply nested loops for clarity (Linux kernel style).

## Read the Source

- *Code Complete* by Steve McConnell — Chapter 15 (Using Conditionals), 16 (Controlling Loops) — practical patterns.
- *Algorithms* by Sedgewick & Wayne — loop-invariant style of algorithm presentation.
- [LLVM IR control flow](https://llvm.org/docs/LangRef.html#br-instruction) — how compilers represent branches after parsing.

## Ship It

This lesson ships **`outputs/binary_search.c`** — a textbook binary-search with the loop invariant in a comment, plus property-based-style tests on arrays of varied sizes.

## Exercises

1. **Easy.** Write all three loop forms (for, while, do-while) computing the same Fibonacci value F_10. Confirm all return 55.
2. **Medium.** State the loop invariant of binary search precisely. Prove correctness via the three-step pattern (init, maint, term).
3. **Hard.** Convert a recursive in-order tree traversal to iterative form using an explicit stack. Verify on a 1000-node tree that both produce the same sorted sequence.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Branch | "An if" | A conditional jump in machine code; CPU's branch predictor guesses the outcome |
| Loop invariant | "What's always true in the loop" | A logical statement holding at the top of every iteration; main tool for correctness proofs |
| Tail recursion | "Recursion that's secretly a loop" | A recursive call as the very last operation; compilers can rewrite as a jump (O(1) stack) |
| Switch / match | "Jump table" | Multi-way selection; compiles to a jump table for dense cases, if-chain otherwise |
| Structured Program Theorem | "goto is unnecessary" | Sequence + selection + iteration suffice to express any computable function |

## Further Reading

- *Dijkstra — "Go To Statement Considered Harmful"* (1968) — the historical case for structured programming.
- *Communicating Sequential Processes* by Hoare — the formal calculus, foundation of concurrency theory.
- *Programming Pearls* by Bentley, Chapter 4 — the classic essay on binary search showing how easy it is to get wrong.
