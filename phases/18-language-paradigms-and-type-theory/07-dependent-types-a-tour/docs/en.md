# Dependent Types — A Tour

> Types can encode value-level invariants, turning some tests into type checks.

**Type:** Learn
**Languages:** Idris, Haskell
**Prerequisites:** Phase 18 lessons 01-06
**Time:** ~75 minutes

## Learning Objectives

- Explain what dependent types add beyond simple parametric polymorphism.
- See how value-indexed types prevent illegal states.
- Compare lightweight emulation in Haskell with native support in Idris.
- Identify when dependent typing is worth the complexity.

## The Problem

Consider a function that takes two vectors and appends them. In most languages, the type says nothing about lengths:

```python
def append(v1: list, v2: list) -> list:
    return v1 + v2
```

You can call `append` with any lists, and the return type tells you nothing about the result's length. If downstream code expects the result to have exactly `len(v1) + len(v2)` elements, you need a runtime assertion or a test. If someone changes the implementation and breaks this property, nothing catches it until runtime.

In Idris, the type itself encodes the length:

```idris
append : Vect n a -> Vect m a -> Vect (n + m) a
```

The compiler verifies this at compile time. If you mess up the implementation, it won't type-check. No tests needed for this property. The type is the test.

This is the promise of dependent types: types that depend on values. Instead of writing a test that checks "the result has length n+m," you write a type that states it, and the compiler proves it.

## The Concept

### Types that mention values

In standard type systems, types and values live in separate worlds:

```
Type level:   Int, Bool, List Int, a -> b
Value level:  42, True, [1,2,3], \x -> x + 1
```

Dependent types merge these worlds. A type can mention a value:

```
Vect 3 Int    -- a vector of exactly 3 integers
Fin n         -- an index into a vector of length n
```

Here `3` and `n` are values that appear in types.

### Indexed families

A family of types indexed by values:

```idris
-- Vect is indexed by its length (a Nat) and element type
data Vect : Nat -> Type -> Type where
  Nil  : Vect 0 a
  (::) : a -> Vect n a -> Vect (S n) a
```

`Vect 0 a` is empty. `Vect 3 a` has exactly 3 elements. The length is part of the type.

### Safe head

```idris
-- Can't take head of empty vector: type forbids it
head : Vect (S n) a -> a
head (x :: _) = x
```

The pattern `(S n)` requires the vector to have at least one element (successor of `n`). Calling `head` on an empty vector is a type error, not a runtime crash.

```idris
-- This compiles:
head (1 :: 2 :: 3 :: Nil)  -- returns 1

-- This doesn't compile:
head Nil  -- TYPE ERROR: can't match S n with 0
```

### Append with proven length

```idris
append : Vect n a -> Vect m a -> Vect (n + m) a
append Nil       ys = ys           -- 0 + m = m
append (x :: xs) ys = x :: append xs ys  -- S n + m = S (n + m)
```

The type checker verifies each case. In the `Nil` case, `n = 0`, so `n + m = m`. In the `::` case, `n = S n'`, so `n + m = S (n' + m)`. The recursion's termination and the length arithmetic are both checked.

### GADTs in Haskell (lightweight emulation)

Haskell can't do full dependent types, but GADTs (Generalized Algebraic Data Types) can encode some of the same ideas:

```haskell
{-# LANGUAGE GADTs, DataKinds, TypeFamilies #-}

data Nat = Z | S Nat

data Vect n a where
  Nil  :: Vect Z a
  (::) :: a -> Vect n a -> Vect (S n) a

-- Safe head: only works on non-empty vectors
head :: Vect (S n) a -> a
head (x :: _) = x
```

The `DataKinds` extension lifts `Nat` to the type level. GADTs let constructors refine the index. It's less powerful than Idris (no arbitrary value-level computation in types), but covers common cases.

### When dependent types help

| Use case | Benefit | Cost |
|----------|---------|------|
| Length-indexed vectors | Prevent out-of-bounds at compile time | Proof obligations for indexing |
| Matrix dimensions | Catch shape mismatches in linear algebra | Type annotations for dimensions |
| Protocol state machines | Enforce valid state transitions | Encoding states as types |
| Verified sorting | Type proves output is sorted | Significant proof burden |
| Safe SQL queries | Type-level schema checking | Complex type-level programming |

### When they don't help

- **Prototyping**: the proof burden slows iteration.
- **Simple domains**: if runtime checks suffice, dependent types add overhead without proportional benefit.
- **Team unfamiliarity**: the learning curve is steep; error messages can be cryptic.

### Erasure

Dependent types that are only needed for proofs can be erased at runtime. Idris erases `Nat` arguments that only appear in types. The compiled code runs as fast as the untyped version. Proofs cost compile time, not runtime.

## Build It

### Step 1: Length-indexed vectors in Idris

```idris
data Vect : Nat -> Type -> Type where
  Nil  : Vect 0 a
  (::) : a -> Vect n a -> Vect (S n) a

-- Safe head
head : Vect (S n) a -> a
head (x :: _) = x

-- Safe tail
tail : Vect (S n) a -> Vect n a
tail (_ :: xs) = xs

-- Append with proven length
append : Vect n a -> Vect m a -> Vect (n + m) a
append Nil       ys = ys
append (x :: xs) ys = x :: append xs ys

-- Safe index: can't go out of bounds
data Fin : Nat -> Type where
  FZ : Fin (S n)
  FS : Fin n -> Fin (S n)

index : Fin n -> Vect n a -> a
index FZ     (x :: _)  = x
index (FS i) (_ :: xs) = index i xs
```

### Step 2: Haskell approximation with GADTs

```haskell
{-# LANGUAGE GADTs, DataKinds, TypeFamilies, TypeOperators #-}

import GHC.TypeNats

data Vect (n :: Nat) a where
  Nil  :: Vect 0 a
  (::) :: a -> Vect n a -> Vect (n + 1) a

infixr 5 ::

head :: Vect (n + 1) a -> a
head (x :: _) = x

append :: Vect n a -> Vect m a -> Vect (n + m) a
append Nil       ys = ys
append (x :: xs) ys = x :: append xs ys

example :: Vect 3 Int
example = 1 :: 2 :: 3 :: Nil

-- head example compiles: 1
-- head Nil would be a type error
```

### Step 3: Proving properties (Idris)

```idris
-- Prove that reversing a vector preserves its length
reverse : Vect n a -> Vect n a
reverse []        = []
reverse (x :: xs) = append (reverse xs) [x]

-- The type guarantees: length(reverse v) = length(v)
-- No test needed. The compiler verifies it.
```

### Step 4: Type-level natural number arithmetic

```idris
-- Idris knows about Nat arithmetic
plus_commutes : (n : Nat) -> (m : Nat) -> n + m = m + n
plus_commutes 0     m = Refl  -- 0 + m = m by definition
plus_commutes (S k) m = cong S (plus_commutes k m)
```

This is a proof that `+` is commutative, expressed as a function. The type checker verifies the proof is correct.

## Use It

Dependent types appear in:

- **Theorem provers**: Coq, Lean, Agda, Idris use dependent types as their foundation. Every theorem is a type, every proof is a program.
- **Verified software**: seL4 (verified microkernel), CompCert (verified C compiler) use dependent-style reasoning.
- **Haskell GADTs**: widely used for type-safe embedded DSLs, phantom types, and encoding invariants.
- **Rust const generics**: `fn foo<const N: usize>(arr: [i32; N])` is a lightweight form of value-parametric types.
- **F* (F-star)**: Microsoft Research language combining dependent types with effects, used for verified crypto implementations.

## Read the Source

- [Idris 2 Documentation](https://idris2.readthedocs.io/) — the canonical tutorial.
- *Type-Driven Development with Idris* (Edwin Brady) — book-length treatment.
- [GHC DataKinds](https://downloads.haskell.org/~ghc/latest/docs/users_guide/exts/data_kinds.html) — Haskell's lightweight emulation.
- *Software Foundations* Volume 1 — Coq proofs as dependent types.

## Ship It

- `code/Main.idr`: dependent vector examples.
- `code/Main.hs`: Haskell GADT approximation.
- `outputs/README.md`: value-indexed type checklist.

## Quiz

**Q1 (Pre).** What makes a type "dependent"?

- A) It uses inheritance.
- B) It can mention values, so the type changes based on runtime data.
- C) It's polymorphic.
- D) It has a recursive structure.

**Answer: B.** A dependent type depends on a value. `Vect 3 Int` mentions the value `3` in the type. The type `Vect n a` varies with `n`. This lets types encode properties like "this vector has exactly 3 elements" that are verified at compile time.

**Q2 (Pre).** What does `Vect (S n) a` in the `head` signature guarantee?

- A) The vector has at least one element.
- B) The vector has exactly `n` elements.
- C) The vector is sorted.
- D) The element type is `a`.

**Answer: A.** `S n` is the successor of `n`, meaning at least 1 (since `S 0 = 1`, `S 1 = 2`, etc.). By requiring `Vect (S n) a`, the type forbids empty vectors. `head` can never fail with "empty list" at runtime.

**Q3 (Post).** How do GADTs in Haskell emulate dependent types?

- A) They don't; Haskell has full dependent types.
- B) GADTs let constructors refine type indices, so different constructors can produce different types.
- C) GADTs are just syntactic sugar for regular ADTs.
- D) GADTs allow runtime type inspection.

**Answer: B.** GADTs let each constructor specify a more precise type. `Nil :: Vect 0 a` says "Nil has index 0." `(::) :: a -> Vect n a -> Vect (S n) a` says ":: increments the index." The type checker uses these refined types to verify invariants. It's less powerful than full dependent types but covers many practical cases.

**Q4 (Post).** What is type erasure in the context of dependent types?

- A) Removing types from source code.
- B) Compile-time-only proof terms are removed from the executable; no runtime cost.
- C) Forgetting type information at runtime.
- D) Disabling type checking.

**Answer: B.** In Idris, type arguments used only for proofs (like `n` in `Vect n a` when you don't inspect the value) are erased during compilation. The compiled code runs as fast as if the types didn't exist. Proofs cost compile time, not runtime.

**Q5 (Post).** Why might you NOT use dependent types in a production project?

- A) They're always slower at runtime.
- B) The proof burden and learning curve may not pay off for simple domains where runtime checks suffice.
- C) They can't express real-world invariants.
- D) No production language supports them.

**Answer: B.** Dependent types add significant complexity: proofs, type annotations, and a steep learning curve. For domains where runtime checks or tests are adequate, the overhead isn't justified. They shine in high-assurance settings (crypto, OS kernels, protocol verification) where bugs are expensive.

## Exercises

1. **Easy.** Write a `safeTail` function in Idris or Haskell GADTs that takes a non-empty vector and returns the rest. Verify it rejects empty vectors at compile time.
2. **Medium.** Encode matrix dimensions in type parameters. Write a `multiply : Matrix m n a -> Matrix n p a -> Matrix m p a` that catches dimension mismatches at compile time.
3. **Hard.** Compare the error messages from Idris vs Haskell GADTs when you make a dimension mistake. Which is more helpful and why?

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Dependent type | "type depends on value" | A type-level expression indexed by a term-level value |
| Indexed family | "parameterized type" | A family of types selected by index values (e.g., `Vect n a`) |
| Proof term | "witness" | A program artifact certifying a proposition (the proof is the program) |
| Erasure | "compile away proofs" | Removing proof-only terms from the compiled executable |
| GADT | "refined ADT" | Algebraic data type where constructors can refine type indices |

## Further Reading

- [Idris 2 Documentation](https://idris2.readthedocs.io/)
- [GHC DataKinds](https://downloads.haskell.org/~ghc/latest/docs/users_guide/exts/data_kinds.html)
- *Type-Driven Development with Idris* (Edwin Brady)
- [Agda Tutorial](https://agda.readthedocs.io/)
