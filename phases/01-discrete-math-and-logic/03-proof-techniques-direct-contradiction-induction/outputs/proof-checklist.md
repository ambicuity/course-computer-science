# Proof Technique Picker

> One-page guide: read your claim, pick the row, follow the skeleton.

## Decision tree

| Claim shape | First reach for | Why |
|-------------|-----------------|-----|
| `P → Q`, both concrete | **Direct** | Just manipulate algebra/definitions from P to Q |
| `P → Q`, but Q feels "downstream" and hard to derive forward | **Contrapositive** | Often `¬Q → ¬P` is computational, not creative |
| "Q is true" (no antecedent) | **Direct** (constructive) | Build the witness or compute the value |
| "X does not exist" / "X cannot happen" | **Contradiction** | Assume it exists/happens, derive impossible consequence |
| `∀n ∈ ℕ. P(n)` | **Induction (on n)** | Base case + step |
| `∀n ≥ 2. P(n)`, and the step needs many earlier cases | **Strong induction** | Assume P(2), …, P(k); prove P(k+1) |
| `∀ (recursive datum) d. P(d)` (lists, trees, expressions) | **Structural induction** | Base on constructors with no sub-data; step over composites |
| "These two functions / formulas are equal on all inputs" | **Equational reasoning + induction** | Often boils down to "induct on one input, simplify both sides" |

## Skeletons

### Direct
```
Claim: P → Q.
Suppose P.
... [chain of equivalences / known facts] ...
Therefore Q.
∎
```

### Contrapositive
```
Claim: P → Q.
We prove the contrapositive ¬Q → ¬P.
Suppose ¬Q.
... ...
Therefore ¬P.
Hence P → Q.  ∎
```

### Contradiction
```
Claim: Q.
Suppose for contradiction ¬Q.
... derive R ∧ ¬R ...
Contradiction, so Q.  ∎
```

### Induction on ℕ
```
Claim: ∀n ≥ b. P(n).
Base.  Show P(b).  [usually easy; check it explicitly]
Step.  Fix k ≥ b. Assume P(k) (the IH). Show P(k+1).
       ... derive P(k+1) using P(k) ...
By induction, ∀n ≥ b. P(n).  ∎
```

### Strong induction
```
Claim: ∀n ≥ b. P(n).
Base.  P(b).
Step.  Fix k ≥ b. Assume P(b), P(b+1), …, P(k). Show P(k+1).
       ... case-split, then invoke IH on smaller values ...
By strong induction, ∀n ≥ b. P(n).  ∎
```

### Structural induction (on a recursive datatype T = {Leaf} ∪ {Node(l, r)})
```
Claim: ∀t : T. P(t).
Base.  P(Leaf).
Step.  Fix l, r : T. Assume P(l) and P(r). Show P(Node(l, r)).
       ... ...
By structural induction, ∀t : T. P(t).  ∎
```

## Common bugs (quick fixes)

| Bug | Fix |
|-----|-----|
| Used P somewhere in the proof of P | Trace dependencies; no step can depend on the goal |
| "By induction" with no base | Always state and verify the base case |
| Induction step "obvious" with a hidden gap | Write it out — name the IH and the step |
| Induction on the wrong variable | Pick a variable whose value strictly decreases in recursive sub-calls |
| Forgot the strict inequality in strong induction | Always invoke IH on strictly smaller values |
| "WLOG suppose X" with no proof | Show that swapping out X doesn't change the claim |
