From Coq Require Import Arith List.
Import ListNotations.

Theorem plus_n_O : forall n : nat, n + 0 = n.
Proof.
  intro n.
  induction n as [| n' IH].
  - reflexivity.
  - simpl. rewrite IH. reflexivity.
Qed.

Theorem plus_comm_small : forall a b : nat, a + b = b + a.
Proof.
  intros a b.
  apply Nat.add_comm.
Qed.

Theorem length_app : forall (A : Type) (xs ys : list A),
  length (xs ++ ys) = length xs + length ys.
Proof.
  intros A xs ys.
  induction xs as [| x xs' IH].
  - reflexivity.
  - simpl. rewrite IH. reflexivity.
Qed.
