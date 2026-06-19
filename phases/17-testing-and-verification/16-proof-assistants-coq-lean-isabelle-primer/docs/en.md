# Proof Assistants - Coq/Lean/Isabelle Primer

> Machine-checked proofs trade convenience for very high assurance.

**Type:** Learn
**Languages:** Coq
**Prerequisites:** Phase 17 lessons 01-15
**Time:** ~90 minutes

## Learning Objectives

- Understand theorem/proof workflow in interactive provers.
- Differentiate propositions, terms, and tactics at a practical level.
- Prove small arithmetic/list properties.
- Connect proof artifacts to software assurance claims.

## The Problem

Testing and model checking provide powerful evidence but remain bounded by inputs
or model abstraction. For core invariants, teams sometimes need stronger,
machine-checked proofs over formal definitions.

Consider the claim "this sorting function returns a sorted permutation of its
input." You can test it with a million random arrays. But a proof assistant can
verify it for *all* arrays. The proof is a mathematical argument checked by a
machine: if the proof passes, the property holds for every possible input, not
just the ones you tested.

This matters for safety-critical software. The seL4 microkernel has a machine-checked
proof that its implementation matches its specification. CompCert has a proof that
its compiler preserves program semantics. These aren't claims backed by test
coverage. They're mathematical theorems verified by a computer.

Proof assistants make this possible. They're not easy to use. The learning curve
is steep, and proofs require maintenance when code changes. But for the properties
where failure is catastrophic, they provide assurance that nothing else can match.

## The Concept

### Workflow

```
    Proof Assistant Workflow:
    
    ┌──────────────────┐
    │ Define objects   │  Inductive nat := O | S nat.
    │ and functions    │  Fixpoint plus (n m : nat) := ...
    └────────┬─────────┘
             │
    ┌────────▼─────────┐
    │ State theorem    │  Theorem plus_n_O : forall n, n + O = n.
    └────────┬─────────┘
             │
    ┌────────▼─────────┐
    │ Apply tactics    │  induction n. simpl. reflexivity.
    │ interactively    │  ...
    └────────┬─────────┘
             │
    ┌────────▼─────────┐
    │ Qed              │  Proof complete. Machine checks proof term.
    └──────────────────┘
```

The key insight: you don't write the proof directly. You apply **tactics** that
transform the **proof state** (the current goal and hypotheses). Each tactic
makes progress toward the goal. When the goal is fully discharged, `Qed` closes
the proof and the machine verifies the resulting proof term.

### Curry-Howard Correspondence

Proof assistants are built on a deep connection between logic and programming:

| Logic | Programming |
|---|---|
| Proposition | Type |
| Proof | Program |
| Proving a theorem | Writing a program of that type |
| `A ∧ B` (and) | `A * B` (pair) |
| `A → B` (implies) | `A -> B` (function) |
| `∀ x, P(x)` (forall) | `Π x, P(x)` (dependent function) |

In Coq, `Prop` is the type of propositions. A proof of `P : Prop` is a term
of type `P`. If you can construct such a term, the proposition is true.

This means: **proving a theorem is the same as writing a program.** The type
checker verifies both.

### Tactics

Tactics are commands that transform the proof state. Common tactics:

| Tactic | What it does |
|---|---|
| `intro` | Move a hypothesis from the goal into the context |
| `destruct` | Case analysis on an inductive type |
| `induction` | Structural induction on an inductive type |
| `simpl` | Simplify the goal by computation |
| `reflexivity` | Prove `a = a` (goal must be syntactically equal) |
| `rewrite` | Replace one side of an equation with the other |
| `apply` | Apply a hypothesis or lemma to the goal |

### Proof State

As you apply tactics, the proof state evolves:

```
    Initial state:
    ============================
    forall n : nat, n + O = n
    
    After "intros n":
    n : nat
    ============================
    n + O = n
    
    After "induction n":
    Case 1 (base):
    ============================
    O + O = O
    
    Case 2 (step):
    n : nat
    IH : n + O = n
    ============================
    S n + O = S n
```

## Build It

We prove small arithmetic and list properties in Coq.

### Step 1: Prove `n + O = n`

```coq
Theorem plus_n_O : forall n : nat, n + O = n.
Proof.
  intros n.
  induction n as [| n' IH].
  - (* base case: O + O = O *)
    simpl. reflexivity.
  - (* inductive case: S n' + O = S n' *)
    simpl. rewrite -> IH. reflexivity.
Qed.
```

Walkthrough:

1. `intros n` moves the universal quantifier into the context.
2. `induction n as [| n' IH]` does structural induction on `n`:
   - Base case: `n = O`. Goal becomes `O + O = O`. `simpl` computes this to
     `O = O`. `reflexivity` finishes it.
   - Inductive case: `n = S n'`. We have hypothesis `IH : n' + O = n'`. Goal
     becomes `S n' + O = S n'`. `simpl` reduces to `S (n' + O) = S n'`.
     `rewrite -> IH` replaces `n' + O` with `n'`. Goal becomes `S n' = S n'`.
     `reflexivity` finishes it.

### Step 2: Prove `n + S m = S (n + m)`

```coq
Theorem plus_n_Sm : forall n m : nat, n + S m = S (n + m).
Proof.
  intros n m.
  induction n as [| n' IH].
  - simpl. reflexivity.
  - simpl. rewrite -> IH. reflexivity.
Qed.
```

### Step 3: Prove list length preservation under reversal

```coq
Require Import List.
Import ListNotations.

Fixpoint rev {A : Type} (l : list A) : list A :=
  match l with
  | [] => []
  | x :: xs => rev xs ++ [x]
  end.

Theorem rev_length : forall (A : Type) (l : list A),
  length (rev l) = length l.
Proof.
  intros A l.
  induction l as [| x xs IH].
  - simpl. reflexivity.
  - simpl. rewrite -> app_length. simpl.
    rewrite -> IH. simpl. rewrite -> Nat.add_1_r. reflexivity.
Qed.
```

### Step 4: The same theorem in Lean 4 syntax

```lean
-- Lean 4 equivalent
theorem plus_n_O (n : Nat) : n + 0 = n := by
  induction n with
  | zero => rfl
  | succ n' ih =>
    simp [Nat.add_succ]
    exact ih

-- List reversal preserves length
theorem rev_length {α : Type} (l : List α) :
    l.reverse.length = l.length := by
  induction l with
  | nil => rfl
  | cons x xs ih =>
    simp [List.reverse_cons, List.length_append]
    exact ih
```

Lean and Coq have different syntax but the same underlying idea: you state
a theorem and interactively build a proof using tactics.

## Use It

Proof assistants are used in high-assurance domains:

- **seL4 microkernel:** The kernel's implementation is proven to match its
  specification. The proof covers functional correctness, security properties,
  and absence of common bugs (null pointer dereference, buffer overflow, etc.).
- **CompCert compiler:** The compiler is proven to preserve program semantics.
  If the source program has behavior X, the compiled program has behavior X.
  This eliminates miscompilation bugs.
- **Cryptographic libraries:** Formal proofs that crypto implementations match
  their specifications, preventing subtle implementation bugs.
- **Mathematics:** The Lean mathlib library contains thousands of formalized
  mathematical theorems.

The costs are real:

- **Learning curve:** Proof assistants require learning a new way of thinking
  about programs. The tactic language is unfamiliar, and debugging failed proofs
  is frustrating.
- **Maintenance burden:** When code changes, proofs must be updated. This is
  like having a second codebase that must stay in sync.
- **Scope limitation:** You can't prove everything. Focus on the properties
  where the cost of being wrong justifies the cost of proof.

## Read the Source

- [Software Foundations](https://softwarefoundations.cis.upenn.edu/) — the
  standard Coq textbook, free online.
- [Theorem Proving in Lean 4](https://leanprover.github.io/theorem_proving_in_lean4/) — Lean's official tutorial.
- [Concrete Semantics](https://www.concrete-semantics.org/) — Isabelle/HOL
  textbook with proofs about programming languages.

## Ship It

This lesson ships:

- `code/Main.v`: compact Coq examples and proofs.
- `outputs/README.md`: proof workflow checklist.

```bash
coqc code/Main.v
```

## Quiz

**Pre-questions:**

**Q1.** What does `Qed` do in Coq?

- A) Starts a proof.
- B) Closes a proof and checks that the proof term is complete and correct.
- C) Defines a new theorem.
- D) Runs the proof.

**Answer: B.** `Qed` (quod erat demonstrandum) terminates the proof and
instructs Coq to type-check the proof term. If the proof is incomplete or
incorrect, `Qed` fails. This is the moment where the machine verifies the
entire argument.

**Q2.** What is the Curry-Howard correspondence?

- A) A way to write proofs in JavaScript.
- B) The correspondence between logic and programming: propositions are types,
   proofs are programs, and type-checking is proof-checking.
- C) A method for optimizing proof search.
- D) A technique for converting Coq to Lean.

**Answer: B.** Curry-Howard says that proving a theorem is the same as writing
a program of a specific type. A proof of `A → B` is a function from `A` to `B`.
A proof of `∀ x, P(x)` is a function that takes any `x` and returns a proof of
`P(x)`. The type checker verifies both programs and proofs.

**Post-questions:**

**Q3.** You're proving `n + O = n` by induction. In the inductive case, you
have hypothesis `IH : n' + O = n'` and goal `S n' + O = S n'`. What tactic
do you use next?

- A) `reflexivity`
- B) `simpl` then `rewrite -> IH` then `reflexivity`
- C) `destruct n'`
- D) `apply IH`

**Answer: B.** First, `simpl` simplifies the goal by computation, reducing
`S n' + O` to `S (n' + O)`. Then `rewrite -> IH` replaces `n' + O` with `n'`
using the inductive hypothesis. Finally, `reflexivity` closes the goal
`S n' = S n'`.

**Q4.** Why are proof assistants expensive to use?

- A) The software is expensive.
- B) Proofs require learning a new paradigm, proofs must be maintained when
   code changes, and not all properties are worth proving.
- C) They're only for mathematicians.
- D) They can only prove simple properties.

**Answer: B.** The costs are human costs: learning curve, proof maintenance,
and scope judgment. Proofs are like a second codebase that must stay in sync
with the implementation. When the code changes, proofs break and must be
repaired. Teams must decide which properties justify this investment.

**Q5.** How does CompCert use proof assistants?

- A) To generate test cases.
- B) To prove that the compiler preserves program semantics: if the source
   program has behavior X, the compiled program has behavior X.
- C) To optimize compilation speed.
- D) To write the compiler in a proof assistant.

**Answer: B.** CompCert is a C compiler written in Coq. It has a machine-checked
proof that compilation preserves program semantics. This eliminates the class of
bugs where the compiler introduces behavior not present in the source code
(miscompilation). The proof covers all possible inputs, not just tested ones.

## Exercises

**Easy:** Prove the associativity of addition: `forall n m p, n + (m + p) = (n + m) + p`.
Follow the same induction pattern as `plus_n_O`.

**Medium:** Prove that reversing a list twice gives the original list:
`forall l, rev (rev l) = l`. You'll need a helper lemma about `rev (l1 ++ l2)`.

**Hard:** Compare the same theorem in Coq and Lean. Prove `n + O = n` in both
languages. Write a side-by-side comparison of the syntax and tactic differences.
What concepts map directly? What requires different approaches?

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Theorem | "statement" | Proposition requiring a proof term |
| Tactic | "proof command" | Interactive step transforming proof state |
| Qed | "proof done" | Terminates proof and checks completeness |
| Curry-Howard | "types as propositions" | Programs/proofs correspondence principle |
| Induction | "base + step" | Proof method: prove base case, then prove step assuming IH |
| Proof state | "current goal" | The current goal and hypotheses during interactive proof |
| Dependent types | "types that depend on values" | Types that can mention runtime values (e.g., "list of length n") |

## Further Reading

- [Software Foundations](https://softwarefoundations.cis.upenn.edu/) — the
  standard Coq textbook, free online with exercises.
- [Theorem Proving in Lean 4](https://leanprover.github.io/theorem_proving_in_lean4/) — Lean's official tutorial.
- [Concrete Semantics](https://www.concrete-semantics.org/) — Isabelle/HOL
  textbook.
- [Coq Reference Manual](https://coq.inria.fr/refman/) — official Coq documentation.
- [Lean 4 Documentation](https://leanprover.github.io/lean4/doc/) — Lean 4 reference.
