# Proof Techniques — Direct, Contradiction, Induction

> The four moves that finish most proofs. Once you can spot which to use, the proof writes itself.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 01, Lessons 01–02
**Time:** ~75 minutes

## Learning Objectives

- Identify when a claim is best proved directly, by contrapositive, by contradiction, or by induction.
- Write a clean induction proof — base case, inductive hypothesis, inductive step — and recognize when *strong* induction is needed.
- Apply *structural induction* to recursively defined data (lists, trees, expressions) — the form most used in CS.
- Recognize the most common proof bugs: assuming what you want to prove, induction on the wrong variable, missing base case.

## The Problem

CS lives on proofs whether the textbook says so or not. Loop invariants, algorithm correctness, type-system soundness, security guarantees, distributed-systems safety — every one of those is a proof obligation. The "proof" in industry rarely reaches the rigor of a math textbook, but the *technique* is the same: you state precisely what's true, then derive that statement from things already known to be true.

The four moves below cover more than 90% of proofs you'll write in this course:

1. **Direct proof** — assume the premises, derive the conclusion.
2. **Contrapositive** — to prove `P → Q`, prove `¬Q → ¬P` instead.
3. **Contradiction** — assume `¬Q`, derive a contradiction, conclude `Q`.
4. **Induction** — base case + inductive step.

This lesson is the field guide: when to reach for each, what the standard pitfalls are, and what a clean proof looks like.

## The Concept

### Direct proof

To prove `P → Q`: assume P, derive Q.

> **Claim.** If n is an even integer, then n² is even.
> **Proof.** Suppose n is even. Then n = 2k for some integer k. So n² = (2k)² = 4k² = 2(2k²), which is even. ∎

The structure is "assume the hypothesis, manipulate, arrive at the conclusion." Most CS proofs are direct.

### Proof by contrapositive

`P → Q ≡ ¬Q → ¬P`. To prove the implication, prove its contrapositive — often easier.

> **Claim.** If n² is odd, then n is odd.
> **Proof (by contrapositive).** Suppose n is even, i.e., n = 2k. Then n² = 4k² = 2(2k²), which is even. So n² is not odd. Contrapositive established. ∎

The trick: "if n² is odd then n is odd" is awkward to start with (working from n² is messy). Flipping it to "if n is even then n² is even" is natural.

### Proof by contradiction (reductio ad absurdum)

To prove Q: assume `¬Q`, derive a contradiction (anything of the form `R ∧ ¬R`). Since contradictions are impossible, your assumption was wrong, so Q holds.

> **Claim (Euclid).** There are infinitely many primes.
> **Proof.** Suppose, for contradiction, that there are only finitely many primes: p₁, p₂, …, pₙ. Consider N = p₁ · p₂ · … · pₙ + 1. N is not divisible by any pᵢ (it leaves remainder 1), so either N is prime (a new prime, contradiction) or N has a prime factor not in the list (contradiction). Either way, the assumption fails. So there are infinitely many primes. ∎

Use contradiction when the conclusion is hard to derive forward but easy to attack indirectly ("suppose this thing didn't exist; here's why that can't be").

### Induction

To prove `∀n ∈ ℕ. P(n)`:
1. **Base case.** Prove `P(0)` (or `P(1)`, or whatever the smallest case is).
2. **Inductive step.** Prove `∀k. P(k) → P(k+1)`.

By the principle of mathematical induction, `P` holds for every natural number.

> **Claim.** For all n ≥ 0, 1 + 2 + 3 + … + n = n(n+1)/2.
> **Proof (by induction on n).**
> **Base.** n=0: LHS = 0, RHS = 0·1/2 = 0. ✓
> **Step.** Assume P(k): 1 + 2 + … + k = k(k+1)/2. Show P(k+1):
> 1 + 2 + … + k + (k+1) = k(k+1)/2 + (k+1) = (k+1)(k/2 + 1) = (k+1)(k+2)/2. ✓
> So P(n) holds for all n ≥ 0. ∎

### Strong induction

Sometimes proving `P(k+1)` requires assuming *all* of `P(0), P(1), …, P(k)`, not just `P(k)`.

> **Claim.** Every integer n ≥ 2 has a prime factorization.
> **Proof (by strong induction on n).**
> **Base.** n=2: 2 is prime. ✓
> **Step.** Suppose every 2 ≤ m < k has a prime factorization. If k is prime, done. Else k = a · b for some 2 ≤ a, b < k. By the IH, both a and b have prime factorizations; their concatenation factors k. ∎

Strong induction is equivalent to standard induction (you can convert one to the other) but often more natural.

### Structural induction

For recursively defined data (lists, trees, expressions), induct on the structure:

- **Base.** Prove the property for the base constructors (empty list, leaf).
- **Step.** Assume the property for the substructures; prove it for the composite constructors.

> **Claim.** For any *full* binary tree T (every internal node has exactly two children), `count_nodes(T) = 2·count_leaves(T) - 1`.
> **Proof (by structural induction on T).**
> **Base.** T is a single leaf: count_nodes = 1, count_leaves = 1, 2·1 - 1 = 1. ✓
> **Step.** T = `Node(L, R)` where L and R are full binary trees. By the IH, count_nodes(L) = 2·count_leaves(L) - 1, similarly for R.
> count_nodes(T) = 1 + count_nodes(L) + count_nodes(R) = 1 + (2·leaves(L) - 1) + (2·leaves(R) - 1) = 2(leaves(L) + leaves(R)) - 1 = 2·count_leaves(T) - 1. ✓ ∎

This is *the* most-used technique in CS proofs (type soundness, compiler correctness, recursive data structure analysis).

### Common proof bugs

| Bug | What it looks like | Fix |
|-----|---------------------|------|
| Circular reasoning | Using P somewhere in your proof of P | Trace dependencies; nothing you derive can depend on P |
| Missing base case | "By induction" with only the step | Always start with the base; check it explicitly |
| Off-by-one in base | Base proved for `n=1` but claim is for `n ≥ 0` | Match the base case to the claim's domain |
| Wrong induction variable | Trying to induct on a quantity that doesn't decrease in the recursive call | Pick a measure that strictly decreases (size, depth, sum) |
| Sloppy "without loss of generality" | "WLOG suppose a ≤ b" with no justification | Show that swapping a and b preserves the claim |

## Build It

Open `code/main.py`. We'll *check* (but not prove) several claims computationally so you can verify your proofs against the data.

### Step 1: Direct verification — Gauss's formula

```python
for n in range(0, 100):
    assert sum(range(n+1)) == n*(n+1)//2
```

That's not a proof — only verifying 100 cases — but a failure here would tell you your candidate proof is wrong.

### Step 2: Counterexample search

```python
# Conjecture: every odd number is prime.
for n in range(1, 50, 2):
    if not is_prime(n):
        print(f"counterexample to 'every odd is prime': {n}")
        break
```

Counterexamples are the cheapest disproof. Once you have one, the question becomes "is this an exception, or is the claim just false?"

### Step 3: The four template proofs as runnable assertions

The lesson's `main.py` walks through Gauss's formula (direct + induction), "if n² is odd then n is odd" (contrapositive), Euclid's infinity of primes (contradiction), and the full-binary-tree count invariant (structural induction). Each is verified computationally on many cases.

### Step 4: Structural induction in code

A tree datatype + a recursive function + a `verify_property` that recursively confirms the invariant.

```python
@dataclass
class Leaf: pass
@dataclass
class Node: l: "Tree"; r: "Tree"

def count_nodes(t):
    if isinstance(t, Leaf): return 1
    return 1 + count_nodes(t.l) + count_nodes(t.r)

def count_leaves(t):
    if isinstance(t, Leaf): return 1
    return count_leaves(t.l) + count_leaves(t.r)

# For every full binary tree, the invariant holds — because of the inductive proof above.
```

Generate random full binary trees, verify the invariant. It always holds — exactly because the inductive proof above is correct.

## Use It

Real-world proofs in this curriculum:

- **Loop invariants** (Phase 03/04): "After iteration k, the prefix `arr[0..k]` is sorted." Maintained by induction over k.
- **Algorithm correctness** (Phase 04): Dijkstra's algorithm correctness is an induction on the size of the settled set.
- **Type soundness** (Phase 17): "If `Γ ⊢ e : T` and `e ⇓ v` then `v : T`." Structural induction on the typing derivation.
- **Distributed consensus** (Phase 11): Raft safety is proved by induction over election terms.

## Read the Source

- *How to Prove It* (Velleman) — the canonical bridge from informal to formal proof.
- *Mathematics for Computer Science* by Lehman, Leighton, Meyer — MIT OCW textbook; free, with hundreds of CS-flavored exercises.
- [`Software Foundations` (Pierce et al.)](https://softwarefoundations.cis.upenn.edu/) — induction proofs in Coq, with deep CS examples.

## Ship It

This lesson ships **`outputs/proof-checklist.md`** — a one-page guide: which technique fits which claim shape, plus the standard skeleton for each.

## Exercises

1. **Easy.** Prove that the sum of two odd numbers is even. (Direct proof, two lines.)
2. **Medium.** Prove by induction: `2ⁿ > n` for all n ≥ 1. State the base case, IH, and step explicitly.
3. **Hard.** Prove by structural induction: for any binary tree T with `n` nodes and height `h`, `h ≥ log₂(n+1) - 1`. (Hint: induct on h, find the maximum n for trees of height ≤ h.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Direct proof | "Just show it" | Assume hypotheses, derive conclusion via standard rules |
| Contrapositive | "Flip and negate" | Replace P → Q with the equivalent ¬Q → ¬P |
| Contradiction | "Reductio" | Assume the negation of what you want to prove, derive a contradiction |
| Induction | "Domino effect" | Base case + inductive step gives you the proposition for all naturals |
| Structural induction | "Induction on data" | Induction on the recursive structure of a datatype (list, tree, expression) |

## Further Reading

- [The Cambridge Mathematics of Computing curriculum](https://www.cl.cam.ac.uk/teaching/0708/DiscMath/) — clear lecture notes on each technique.
- *Concrete Mathematics* by Graham, Knuth, Patashnik — the proof-heavy companion to TAOCP.
- [Hammack — *Book of Proof*](https://www.people.vcu.edu/~rhammack/BookOfProof/) — free, beginner-friendly, very clear on the four techniques.
