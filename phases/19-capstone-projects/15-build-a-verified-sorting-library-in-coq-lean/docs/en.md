# Build a Verified Sorting Library in Coq/Lean

> Proving code correct changes reliability for high-assurance systems.

**Type:** Build
**Languages:** Coq
**Prerequisites:** Phase 19 lessons 01-14
**Time:** ~720 minutes

## Learning Objectives

- Define sorting correctness as two properties: sortedness and permutation.
- Implement insertion sort in Coq.
- Prove helper lemmas for insert and sort.
- Understand the extraction path from verified code to executable code.

## The Problem

In ordinary code, `sort` is trusted by tests. You write 100 test cases, they all pass, and you ship. But tests only check specific inputs. A verified sort proves that for all possible inputs, the output is always sorted and always a permutation of the input. No test suite can provide this guarantee.

This matters for high-assurance systems. If a sort function in an avionics system has a bug that triggers on a specific input pattern, the consequence could be catastrophic. Formal verification eliminates this class of risk entirely: the Coq kernel checks that the proof is valid, and if it accepts the proof, the property holds for all inputs.

The challenge: writing the proof is harder than writing the function. You need to think inductively, decompose goals into lemmas, and guide the proof engine (tactics) through each step. The reward: mathematical certainty that no test can provide.

## The Concept

Sorting correctness has two independent properties:

```
sort(xs) must satisfy:

1. Sortedness:    ∀i. result[i] ≤ result[i+1]
2. Permutation:   multiset(result) = multiset(xs)

Both are needed:
- Sortedness alone: [1,2,3] is sorted, but sort([3,1,2]) = [1,2,3] must also be a permutation
- Permutation alone: [3,1,2] is a permutation of [3,1,2], but it's not sorted
```

In Coq, we express these as inductive predicates:

```
Sorted:  For all adjacent pairs, the first ≤ the second
Permutation: The output list is a reordering of the input list
```

The proof strategy: induction on the list structure.

```
insert(x, xs) preserves sortedness:

  insert(3, [1,2,5]) = [1,2,3,5]

Proof by induction on xs:
  Base case: insert(3, []) = [3], which is sorted.
  Inductive case: insert(3, [1|rest]):
    If 3 ≤ 1: return [3, 1|rest] (need to show sorted)
    If 3 > 1: return [1 | insert(3, rest)] (by IH, insert(3,rest) is sorted)
```

## Build It

### Step 1: Definitions in Coq

```coq
(* Verified sorting library in Coq *)

Require Import List.
Require Import PeanoNat.
Require Import Sorting.Permutation.
Require Import Lia.
Import ListNotations.

(* Definition of sorted *)
Inductive sorted : list nat -> Prop :=
  | sorted_nil  : sorted []
  | sorted_one  : forall x, sorted [x]
  | sorted_cons : forall x y l,
      x <= y -> sorted (y :: l) -> sorted (x :: y :: l).

(* Helper: insert into a sorted list *)
Fixpoint insert (x : nat) (l : list nat) : list nat :=
  match l with
  | [] => [x]
  | h :: t => if x <=? h then x :: h :: t else h :: insert x t
  end.

(* Insertion sort *)
Fixpoint isort (l : list nat) : list nat :=
  match l with
  | [] => []
  | h :: t => insert h (isort t)
  end.
```

### Step 2: Helper Lemmas

```coq
(* Lemma: inserting into a sorted list preserves sortedness *)
Lemma insert_sorted : forall x l,
  sorted l -> sorted (insert x l).
Proof.
  intros x l H.
  induction H.
  - (* l = [] *)
    simpl. apply sorted_one.
  - (* l = [x0] *)
    simpl. destruct (x <=? x0) eqn:E.
    + apply Nat.leb_le in E. apply sorted_cons; auto. apply sorted_one.
    + apply Nat.leb_gt in E. apply sorted_cons; auto.
      * lia.
      * apply sorted_one.
  - (* l = x0 :: y :: l *)
    simpl. destruct (x <=? x0) eqn:E.
    + apply Nat.leb_le in E. apply sorted_cons; auto. apply sorted_cons; auto.
    + apply Nat.leb_gt in E. simpl in IHsorted.
      destruct (x <=? y) eqn:E2.
      * apply Nat.leb_le in E2. apply sorted_cons; auto. apply sorted_cons; auto.
        lia.
      * apply Nat.leb_gt in E2. apply sorted_cons; auto.
        apply IHsorted.
Qed.

(* Lemma: insert preserves permutation *)
Lemma insert_perm : forall x l,
  Permutation (x :: l) (insert x l).
Proof.
  intros x l. induction l as [|h t IH].
  - simpl. apply Permutation_refl.
  - simpl. destruct (x <=? h) eqn:E.
    + apply Permutation_refl.
    + apply Permutation_sym. apply Permutation_trans with (h :: x :: t).
      * apply Permutation_swap.
      * apply Permutation_cons. apply Permutation_refl. apply IH.
Qed.

(* Main theorem: isort produces sorted output *)
Theorem isort_sorted : forall l,
  sorted (isort l).
Proof.
  induction l as [|h t IH].
  - simpl. apply sorted_nil.
  - simpl. apply insert_sorted. apply IH.
Qed.

(* Main theorem: isort produces a permutation of input *)
Theorem isort_perm : forall l,
  Permutation l (isort l).
Proof.
  induction l as [|h t IH].
  - simpl. apply Permutation_refl.
  - simpl. apply Permutation_trans with (h :: isort t).
    + apply Permutation_cons. apply Permutation_refl. apply IH.
    + apply insert_perm.
Qed.

(* Bonus: idempotence *)
Theorem isort_idempotent : forall l,
  isort (isort l) = isort l.
Proof.
  intros l.
  (* This requires showing that sorting an already-sorted list is identity *)
  (* Omitted for brevity, but follows from insert_sorted and uniqueness *)
Admitted.
```

### Step 3: Extraction to OCaml

```coq
(* Extract to OCaml for executable code *)
Extraction Language OCaml.
Extraction "sort.ml" insert isort.
```

This generates OCaml code:

```ocaml
(* Generated OCaml from Coq extraction *)
let rec insert x = function
  | [] -> [x]
  | h :: t ->
    if x <= h then x :: h :: t
    else h :: (insert x t)

let rec isort = function
  | [] -> []
  | h :: t -> insert h (isort t)
```

## Use It

CompCert, seL4, and verified crypto stacks use this style at larger scale:

- **CompCert**: a formally verified C compiler. The proof shows that the compiled code behaves exactly as the source specifies. Every optimization pass is proven correct. Used in avionics and nuclear systems.
- **seL4**: a formally verified microkernel. The proof covers the kernel's C implementation, showing that the binary implements the specification. The proof is 200,000 lines of Isabelle/HOL.
- **HACL***: a verified cryptographic library used in Firefox, Linux, and WireGuard. Every function (AES, SHA-256, Curve25519) is proven correct and constant-time.

The key production lesson: **verification is an investment that pays off in maintenance**. Writing the proof is expensive. But once you have it, any change that breaks the property is caught at proof-check time, not at test time. For code that rarely changes (crypto, compiler optimizations, kernel primitives), this is worth the cost.

## Read the Source

- [Software Foundations](https://softwarefoundations.cis.upenn.edu/) — The definitive Coq textbook. Volume 1 (Logical Foundations) covers inductive predicates, proof tactics, and extraction.
- [CompCert source](https://github.com/AbsInt/CompCert) — Verified C compiler. The `lib/` directory contains verified sorting and data structures.
- [seL4 verification](https://sel4.systems/About/proof.pml) — Overview of the seL4 proof structure.

## Ship It

- `outputs/sort.v`: Coq file exporting `isort` with proofs of `isort_sorted` and `isort_perm`.
- `outputs/README.md`: extraction path documentation (OCaml/Haskell) for runtime use.

## Exercises

1. **Easy** — Prove sort idempotence: `isort (isort xs) = isort xs`. This follows from the fact that inserting into a sorted list at the correct position doesn't change it. You'll need a lemma about insert on sorted lists.
2. **Medium** — Generalize from `nat` to arbitrary ordered types. Use Coq's typeclasses (`Order`, `Comparable`) to make `isort` polymorphic. Prove the same theorems for any type with a total order.
3. **Hard** — Compare proof ergonomics between Coq and Lean for the same algorithm. Implement insertion sort and prove sortedness in Lean 4. Compare: tactic language, automation, library support, and proof size.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Theorem | "proven claim" | A statement that has been accepted by the proof kernel. In Coq, a theorem is a term of a specific type (the proposition). The kernel checks that the proof term has the correct type. |
| Sortedness | "in order" | A predicate over lists: for all adjacent pairs, the first element is ≤ the second. Defined inductively with three constructors: empty, singleton, and cons with ordering constraint. |
| Permutation | "same elements" | A relation between lists: one list is a reordering of the other. Preserves the multiset of elements. Used to show that sort doesn't lose or invent elements. |
| Extraction | "run verified code" | Converting Coq (Gallina) definitions to executable code in OCaml or Haskell. The extracted code inherits the correctness properties proven in Coq. |
| Tactic | "proof command" | A script step that transforms proof goals. Tactics decompose goals into subgoals, apply lemmas, perform induction, and simplify expressions. The proof is complete when all subgoals are solved. |

## Further Reading

- [Software Foundations](https://softwarefoundations.cis.upenn.edu/) — The standard Coq textbook.
- [Theorem Proving in Lean 4](https://leanprover.github.io/theorem_proving_in_lean4/) — Lean's equivalent reference.
- [CompCert](https://github.com/AbsInt/CompCert) — Verified C compiler.
