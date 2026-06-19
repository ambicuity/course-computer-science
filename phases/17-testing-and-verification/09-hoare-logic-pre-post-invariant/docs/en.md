# Hoare Logic - Pre/Post/Invariant

> Reason about programs by proving what must be true before and after each step.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 17 lessons 01-08
**Time:** ~75 minutes

## Learning Objectives

- Use Hoare triples `{P} C {Q}` for precise behavioral reasoning.
- Write loop invariants that support partial correctness proofs.
- Connect preconditions and postconditions to executable assertions.
- Understand limits: partial correctness vs termination proofs.

## The Problem

Tests can show many examples pass, but they cannot by themselves show why logic
is correct for all valid inputs. Teams then miss edge-case reasoning around
loops and state updates.

Consider a binary search implementation. You test it with 100 random arrays and
targets. All pass. But the classic binary search bug (integer overflow in
`mid = (lo + hi) / 2`) only manifests when `lo + hi > INT_MAX`. Your tests
never hit that case. Hoare logic would force you to reason about what `mid`
actually equals and whether the invariant `arr[lo..hi] contains target if it
exists` holds after each iteration.

Hoare logic provides a structured proof discipline for this gap. It's not a
replacement for testing. It's a complementary reasoning framework that catches
what tests miss: logical errors that manifest only in rare states.

## The Concept

### Hoare Triples

A Hoare triple has the form `{P} C {Q}`:

- `P` is the **precondition**: what must be true before `C` executes.
- `C` is the **program fragment**: a statement or block.
- `Q` is the **postcondition**: what is guaranteed after `C` if it terminates.

The triple `{P} C {Q}` is valid if: whenever `P` holds before `C` executes and
`C` terminates, `Q` holds after `C`.

```
    {P}           Precondition: must be true before C
     │
     ▼
    ┌───┐
    │ C │         Program fragment
    └───┘
     │
     ▼
    {Q}           Postcondition: guaranteed after C (if it terminates)
```

### Simple Examples

**Assignment:**
`{x = 5} y := x + 1 {y = 6}`

If `x` is 5 before `y := x + 1`, then `y` is 6 after. The precondition
determines the postcondition.

**Sequencing:**
If `{P} C1 {R}` and `{R} C2 {Q}`, then `{P} C1; C2 {Q}`.

You chain reasoning: the postcondition of the first statement becomes the
precondition of the second.

**Conditional:**
If `{P and b} C1 {Q}` and `{P and not b} C2 {Q}`, then
`{P} if b then C1 else C2 {Q}`.

Both branches must establish the same postcondition.

### Loop Invariants

Loops are where Hoare logic gets interesting. A loop invariant `I` is a
property that:

1. **Init:** Is true before the loop starts.
2. **Preservation:** If `I` holds before an iteration and the loop condition
   is true, then `I` holds after the iteration.
3. **Termination:** When the loop exits (condition is false), `I` combined
   with the negated loop condition implies the postcondition `Q`.

```
    {I}                    Invariant established before loop
     │
     ▼
    ┌──────────────┐
    │ while b do   │◄──┐
    │   {I and b}  │   │
    │   C          │   │  {I} after each iteration
    │   {I}        │───┘
    └──────┬───────┘
           │
           ▼
    {I and not b}        Invariant + exit condition = postcondition
```

### The Summation Example

Prove that this loop computes `sum(0..n)`:

```python
def sum_to(n: int) -> int:
    """Compute 0 + 1 + 2 + ... + n."""
    assert n >= 0  # Precondition
    acc = 0
    i = 0
    # Invariant: acc == sum(0..i-1) and 0 <= i <= n+1
    while i <= n:
        assert acc == i * (i - 1) // 2  # Invariant check
        acc += i
        i += 1
    # Postcondition: acc == sum(0..n) == n * (n + 1) // 2
    assert acc == n * (n + 1) // 2
    return acc
```

Proof sketch:

1. **Init:** Before loop, `acc = 0`, `i = 0`. Invariant: `0 == 0*(0-1)//2`
   and `0 <= 0 <= n+1`. Holds.

2. **Preservation:** Assume invariant holds at start of iteration. `i <= n`
   (loop condition). After `acc += i; i += 1`:
   - New `acc` = old `acc + i` = `sum(0..i-1) + i` = `sum(0..i)` = `i*(i+1)//2`
     (using old `i`, new `i` is old `i + 1`)
   - New `i` = old `i + 1`, so `0 <= new_i <= n+1`
   - Invariant holds.

3. **Termination:** Loop exits when `i > n`. Combined with `i <= n+1`
   (invariant), we get `i = n+1`. Then `acc = sum(0..n) = n*(n+1)//2`.
   Postcondition holds.

### Partial Correctness vs Total Correctness

**Partial correctness:** `{P} C {Q}` — if `C` terminates, then `Q` holds.
Doesn't prove `C` actually terminates.

**Total correctness:** `C` terminates AND `{P} C {Q}`. Requires a **variant**
(also called a ranking function): an expression that decreases with each
iteration and is bounded below.

For the summation loop, the variant is `n - i + 1`. Each iteration increases
`i` by 1, so the variant decreases by 1. When the variant reaches 0, the loop
exits. Since `n - i + 1 >= 0` (from the invariant `i <= n+1`), the variant
is bounded below, guaranteeing termination.

## Build It

We instrument a summation loop with executable assertions that mirror Hoare
logic proof obligations.

### Step 1: Define the function with invariant assertions

```python
def sum_to(n: int) -> int:
    """
    Hoare Logic Proof:
        Precondition:  n >= 0
        Postcondition: result == n * (n + 1) // 2
        
        Invariant: acc == i * (i - 1) // 2 and 0 <= i <= n + 1
        Variant:   n - i + 1  (decreases each iteration, bounded below by 0)
    """
    assert n >= 0, "Precondition: n >= 0"
    
    acc = 0
    i = 0
    
    while i <= n:
        # Invariant check: acc == sum(0..i-1)
        assert acc == i * (i - 1) // 2, \
            f"Invariant violated: acc={acc}, expected {i * (i - 1) // 2}"
        assert 0 <= i <= n + 1, \
            f"Invariant violated: i={i} out of range [0, {n+1}]"
        
        # Variant check: must decrease
        old_variant = n - i + 1
        
        acc += i
        i += 1
        
        # Variant must decrease
        new_variant = n - i + 1
        assert new_variant < old_variant, \
            f"Variant not decreasing: {old_variant} -> {new_variant}"
    
    # Postcondition
    assert acc == n * (n + 1) // 2, \
        f"Postcondition violated: acc={acc}, expected {n * (n + 1) // 2}"
    
    return acc
```

### Step 2: Test with multiple values

```python
def test_sum_to():
    assert sum_to(0) == 0
    assert sum_to(1) == 1
    assert sum_to(5) == 15
    assert sum_to(100) == 5050
    assert sum_to(1000) == 500500

test_sum_to()
print("All Hoare logic assertions passed.")
```

### Step 3: Maximum of array with loop invariant

```python
def find_max(arr: list[int]) -> int:
    """
    Hoare Logic Proof:
        Precondition:  len(arr) >= 1
        Postcondition: result == max(arr)
        
        Invariant: max_so_far == max(arr[0..i-1]) and 1 <= i <= len(arr)
        Variant:   len(arr) - i
    """
    assert len(arr) >= 1, "Precondition: non-empty array"
    
    max_so_far = arr[0]
    i = 1
    
    while i < len(arr):
        # Invariant: max_so_far == max(arr[0..i-1])
        assert max_so_far == max(arr[:i]), \
            f"Invariant violated at i={i}: {max_so_far} != max({arr[:i]})"
        
        if arr[i] > max_so_far:
            max_so_far = arr[i]
        i += 1
    
    # Postcondition
    assert max_so_far == max(arr), \
        f"Postcondition violated: {max_so_far} != max({arr})"
    return max_so_far
```

## Use It

In production systems, Hoare-style reasoning appears in:

- **Formal proof assistants** (Coq, Lean, Isabelle) where loop invariants are
  machine-checked.
- **Static analyzers with contracts** (SPARK/Ada, Frama-C) that verify
  invariants at compile time.
- **Code reviews for safety-critical loops** where engineers annotate
  invariants as comments and reviewers verify them manually.

The practical gain: explicit reasoning notes reduce hidden assumptions and
review load. When a reviewer sees "Invariant: acc == sum(0..i-1)" above a
loop, they can verify the logic without tracing every iteration mentally.

## Read the Source

- C.A.R. Hoare, "An Axiomatic Basis for Computer Programming" (1969) — the
  foundational paper.
- Edsger Dijkstra, "A Discipline of Programming" (1976) — weakest
  precondition calculus.
- [Software Foundations](https://softwarefoundations.cis.upenn.edu/) —
  mechanized proofs including Hoare logic in Coq.
- [Why3](https://why3.lri.fr/) — a platform for program verification using
  loop invariants and SMT solvers.

## Ship It

This lesson ships:

- `code/main.py`: executable loop with assertions mirroring proof obligations.
- `outputs/README.md`: invariant-writing checklist.

```bash
python code/main.py
# Output: All Hoare logic assertions passed.
```

## Quiz

**Pre-questions:**

**Q1.** In the Hoare triple `{P} C {Q}`, what does `Q` represent?

- A) The precondition that must hold before C.
- B) The postcondition guaranteed after C if it terminates.
- C) The loop invariant for C.
- D) The variant function proving termination.

**Answer: B.** `Q` is the postcondition: the property that is guaranteed to
hold after `C` executes, provided `C` terminates. It says nothing about
whether `C` actually terminates (that requires total correctness with a
variant).

**Q2.** What three properties must a loop invariant satisfy?

- A) Init, preservation, and termination.
- B) Precondition, postcondition, and variant.
- C) Correctness, completeness, and consistency.
- D) Base case, inductive step, and conclusion.

**Answer: A.** A loop invariant must (1) be true before the loop starts (init),
(2) be preserved by each iteration (preservation), and (3) combined with the
negated loop condition, imply the postcondition (termination/exit reasoning).

**Post-questions:**

**Q3.** A loop has invariant `I` and variant `V`. The variant `V` decreases by
1 each iteration. What does this prove?

- A) The loop produces the correct result.
- B) The loop will eventually terminate.
- C) The invariant is always true.
- D) The postcondition holds.

**Answer: B.** A decreasing variant that is bounded below proves termination.
Each iteration reduces `V` by at least 1, and `V >= 0` (from the invariant),
so the loop must terminate after at most `V_initial` iterations. This is
separate from correctness (which the invariant handles).

**Q4.** What is the difference between partial and total correctness?

- A) Partial correctness proves termination; total does not.
- B) Partial correctness proves "if it terminates, the result is right";
   total correctness also proves it terminates.
- C) Partial correctness is for loops; total is for functions.
- D) There is no difference.

**Answer: B.** Partial correctness `{P} C {Q}` means: if `C` terminates, then
`Q` holds. It says nothing about whether `C` actually terminates. Total
correctness adds a termination proof (via a variant), guaranteeing both
correctness and termination.

**Q5.** You write an invariant `acc == sum(0..i-1)` for a summation loop. At
iteration `i = 3` with `n = 5`, what must be true?

- A) `acc == 3` (sum of 0, 1, 2).
- B) `acc == 6` (sum of 0, 1, 2, 3).
- C) `acc == 15` (sum of 0 through 5).
- D) `acc == 10` (sum of 1 through 4).

**Answer: A.** The invariant says `acc == sum(0..i-1)`. When `i = 3`, this is
`sum(0..2) = 0 + 1 + 2 = 3`. The invariant describes the state *before*
processing element `i`, not after.

## Exercises

**Easy:** Add a variant function to the `find_max` loop that proves
termination. What expression decreases with each iteration?

**Medium:** Write and prove a loop that computes the maximum value in an array.
State the precondition, invariant, postcondition, and variant. Instrument the
code with assertions.

**Hard:** Convert the Hoare logic assertions in `sum_to` into contract
annotations using Python's `typing` module and a decorator that checks
preconditions and postconditions. The decorator should capture `old` values
(like Eiffel's `old` keyword) for postcondition checking.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Hoare triple | "spec notation" | `{P} C {Q}`: assertion of pre/post relation around command |
| Invariant | "loop rule" | Property preserved every iteration; combined with exit condition gives postcondition |
| Partial correctness | "proof of correctness" | If program terminates, postcondition holds (no termination guarantee) |
| Total correctness | "full proof" | Program terminates AND postcondition holds |
| Variant | "decreasing metric" | Expression proving progress toward termination; decreases each iteration |
| Weakest precondition | "wp" | Minimal condition on state that guarantees postcondition after execution |
| Precondition strengthening | "assuming more" | Replacing a precondition with a stronger one that implies it |
| Postcondition weakening | "promising less" | Replacing a postcondition with a weaker one that is implied by it |

## Further Reading

- [Hoare Logic (Wikipedia)](https://en.wikipedia.org/wiki/Hoare_logic) — concise notation overview.
- [Software Foundations](https://softwarefoundations.cis.upenn.edu/) — mechanized proofs including Hoare logic in Coq.
- [A Discipline of Programming](https://www.cs.utexas.edu/users/EWD/ewd06xx/EWD616.PDF) — Dijkstra's foundational work on weakest preconditions.
- [Frama-C](https://frama-c.com/) — industrial tool for C program verification using Hoare-style reasoning.
