(* Build a Verified Sorting Library in Coq/Lean *)
(* Run: coqc Main.v *)
(*
   Architecture:
     Sorted predicate → insert function → isort function → Proofs

   Implements insertion sort with formal proofs of:
     1. Sortedness: isort produces sorted output
     2. Permutation: isort produces a permutation of input
     3. Idempotence: sorting twice = sorting once (admitted as exercise)
*)

Require Import List.
Require Import PeanoNat.
Require Import Sorting.Permutation.
Require Import Lia.
Import ListNotations.

(* =============================================================================
   Step 1: Definitions — sorted predicate, insert, isort
   ============================================================================= *)

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

(* =============================================================================
   Step 2: Proofs
   ============================================================================= *)

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

(* Bonus: idempotence (left as exercise) *)
Theorem isort_idempotent : forall l,
  isort (isort l) = isort l.
Proof.
  intros l.
  (* This requires showing that sorting an already-sorted list is identity *)
  (* Omitted for brevity, but follows from insert_sorted and uniqueness *)
Admitted.

(* =============================================================================
   Step 3: Extract to OCaml for executable code
   ============================================================================= *)

(* Extraction Language OCaml. *)
(* Extraction "sort.ml" insert isort. *)
