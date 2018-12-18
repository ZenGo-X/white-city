

Require Import Setoid.

Definition SET (T : Type):= T -> Prop.

Bind Scope subset_scope with SET.

Definition elem {T : Type} (e : T) (S : SET T) :=
  S e.

Notation "e ∈ S" := (elem e S) (at level 0).
Notation "e ∉ S" := (not (elem e S)) (at level 0).


Theorem elem_not_elem:
  forall T : Type, forall a b : T, forall S : SET T,
      a ∈ S -> b ∉ S -> a <> b.
Proof.
  intros.
  unfold elem in *.
  intro Hcontra.
  rewrite <- Hcontra in H0.
  contradiction.
Qed.

Theorem elem_classic:
  forall T : Type, forall e : T, forall S : SET T,
        e ∈ S -> e ∉ S -> False.
Proof.
  intros.
  assert (Hcontra: e <> e).
  { apply elem_not_elem with (S:=S) ; assumption. }
  apply Hcontra.
  reflexivity.
Qed.
  
Definition Emptyset {T : Type} :=
  fun (_ : T) => False.

Notation "∅" := Emptyset. 

Lemma Emptyset_notIn:
  forall T : Type, forall e : T,
      e ∉ ∅.
Proof.
  auto.
Qed.

Hint Resolve Emptyset_notIn.

(* This is an encoding of   S' = S ∪ {e} *)
Definition add {T : Type} (e : T) (S : SET T) :=
  fun e' : T => e' = e \/ (e' ∈ S).

Notation "e # s" := (add e s) (at level 60, right associativity).

Theorem add_in:
  forall T : Type, forall e : T, forall S : SET T, e ∈ (e # S).
Proof.
  intros. left. reflexivity.
Qed.

Hint Resolve add_in.

Theorem add_sup:
  forall T : Set, forall a b : T, forall S : SET T,
        a ∈ S -> a ∈ (b # S).
Proof.
  intros. right. trivial.
Qed.

Hint Resolve add_sup.

Definition seteq {T : Type} (S1 : SET T) (S2 : SET T) : Prop :=
  forall x : T, x ∈ S1 <-> x ∈ S2.

Notation "S1 ~ S2" := (seteq S1 S2)  (at level 70, no associativity).

Theorem add_idem:
  forall T : Set, forall e : T, forall S : T -> Prop,
        e ∈ S -> e # S ~ S.
Proof.
  intros T e S Hin.
  split.
  - intro H1.
    destruct H1 as [H1 | H2].
    + subst.
      assumption.    
    + assumption.
  - intro H1.
    apply add_sup.
    assumption.
Qed.

Notation "{ e }" := (e # ∅).


Definition remove {T : Set} (e : T) (S : SET T) :=
  fun e' : T => e' ∈ S /\ e' <> e.

Lemma remove_notin:
  forall T : Set, forall e : T, forall S : SET T,
        e ∉ (remove e S).
Proof.
  intros.
  unfold not. unfold remove.
  intros.
  inversion H.
  contradiction.
Qed.  

Hint Resolve remove_notin.

Lemma remove_others:
  forall T : Set, forall a b : T, forall S : SET T,
        a ∈ S -> a <> b -> a ∈ (remove b S).
Proof.
  intros.
  split ; assumption.
Qed.

Hint Resolve remove_others.

Lemma remove_elim:
  forall T : Set, forall e : T, forall S : SET T,
    e ∉ S -> remove e S ~ S.
Proof.
  intros.
  split.
  - intro H1.
    inversion H1.
    assumption.
  - intro H2.
    firstorder.
    unfold remove.
    unfold elem.
    apply elem_not_elem with (S:=S) ; assumption.    
Qed.

Definition swap {T : Set} (e e' : T) (S : SET T) :=
  e' # (remove e S).

Lemma swap_in:
  forall T : Set, forall e e': T, forall S : SET T, 
        e ∈ (swap e' e S).
Proof.
  intros.
  unfold swap.
  apply add_in.
Qed.  

Theorem swap_idem: (* only with decidable equality ? *)
  forall T : Set, forall T_eqdec : forall a b : T, {a=b} + {a<>b},
      forall e : T, forall S : SET T,
        e ∈ S -> (swap e e S ~ S).
Proof.
  intros.
  split.
  - intro H1.
    inversion H1.
    subst.
    assumption.
    inversion H0.
    assumption.
  - intro H2.
    unfold swap.
    unfold remove.
    unfold add.
    unfold elem.
    firstorder.
Qed.

Theorem swap_diff:
  forall T : Set, forall e : T, forall S : SET T,
        e ∉ S -> (swap e e S ~ e # S).
Proof.
  intros.
  split.
  - intro H1.
    inversion H1.
    + rewrite H0.
      auto.
    + inversion H0.
      apply add_sup.
      assumption.
  - intro H2.
    unfold swap.
    unfold add.
    unfold remove.
    unfold elem.
    inversion H2.
    + left. assumption.
    + right.
      split.
      * assumption.
      * apply elem_not_elem with (S:=S) ; assumption.
Qed.

