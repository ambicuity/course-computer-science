# Lesson 11: Type Checking — Mono, Sub, Inference (HM)

## Overview

Semantic analysis confirms that names are declared. Type checking confirms that operations receive **values of the right kind**. This lesson covers three type-system strategies — monomorphic checking, subtyping, and Hindley-Milner type inference — then implements Algorithm W in Haskell and a monomorphic checker in Rust.

---

## The Problem

After name resolution, you know that `x` is a declared variable. But is `x + true` valid? Can you pass an `int` to a function expecting `bool`? The type checker enforces these constraints. Without it, the code generator would produce nonsensical or dangerous machine code.

---

## Monomorphic Type Checking

A **monomorphic** type system requires every expression to have exactly one, fixed type. Operations check that operands match expected types.

```
if (x: int) + (y: int)    →  valid, result is int
if (x: int) + (y: bool)   →  error: '+' expects int, got bool
```

The checker walks the AST. For each expression, it computes a type and checks consistency:

```
check(expr) → Type

check(BinOp("+", a, b)):
    ta = check(a)         // e.g., Int
    tb = check(b)         // e.g., Int
    if ta == Int && tb == Int → return Int
    else → type error
```

A **type environment** (Γ) maps variables to their declared types:

```
Γ = { x: Int, f: fn(Int) → Bool }
```

---

## Subtyping

In OOP languages, if `Dog` is a subtype of `Animal` (`Dog <: Animal`), a `Dog` value can be used wherever an `Animal` is expected. The type checker must verify this relationship at every assignment, argument passing, and return.

```
class Animal { }
class Dog extends Animal { }

let a: Animal = new Dog();   // OK — Dog <: Animal
let d: Dog = new Animal();   // ERROR — Animal is not a subtype of Dog
```

**Substitution principle:** a subtype can always stand in for its supertype. The checker needs a subtyping relation and must decide when to apply it. Covariance (preserving the subtype direction), contravariance (reversing it for function arguments), and invariance (requiring exact match) are the key concepts.

---

## Type Inference: Hindley-Milner

Type inference lets the compiler figure out types without explicit annotations. The **Hindley-Milner (HM)** system is the foundation behind Haskell, OCaml, Rust's local inference, and many others.

### Key Concepts

**Type variables** represent unknown types: `α`, `β`, `γ`.

**Types:**
```
τ ::= Int | Bool | τ → τ | α        (concrete types and type variables)
```

**Type schemes** (for polymorphism):
```
σ ::= τ | ∀α. σ                     (quantified: "for all α")
```

**Unification** finds a substitution that makes two types equal:

```
unify(Int, Int)       → {}              (trivial)
unify(α, Int)         → {α ↦ Int}      (bind α to Int)
unify(α → Bool, Int)  → FAIL            (arrow ≠ Int)
unify(α → β, Int → Bool) → {α↦Int, β↦Bool}
```

The algorithm applies substitution eagerly and checks for **occurs check** (α cannot unify with α → β — infinite type).

### Algorithm W

Algorithm W takes an environment Γ and an expression e and returns a substitution S and a type τ.

```
W(Γ, e):
  e is integer literal      → return ({}, Int)
  e is boolean literal      → return ({}, Bool)
  e is variable x           → instantiate(Γ(x)), return ({}, fresh_type)

  e is λx. body:
      fresh α
      (S, τ) = W(Γ ∪ {x: α}, body)
      return (S, S(α) → τ)

  e is f arg:
      (S1, τ1) = W(Γ, f)
      (S2, τ2) = W(S1(Γ), arg)
      fresh β
      V = unify(S2(τ1), τ2 → β)
      return (V ∘ S2 ∘ S1, V(β))

  e is let x = def in body:
      (S1, τ1) = W(Γ, def)
      σ = generalize(S1(Γ), τ1)        ← let-polymorphism
      (S2, τ2) = W(S1(Γ) ∪ {x: σ}, body)
      return (S2 ∘ S1, τ2)
```

**Let-polymorphism** is the key innovation: `let id = λx. x in (id 1, id true)` type-checks because each use of `id` instantiates its type scheme `∀α. α → α` with fresh variables.

---

## Build It

### Haskell: Hindley-Milner Type Inference (code/Main.hs)

See `code/Main.hs` for a complete implementation of Algorithm W on a small lambda calculus with `Int`, `Bool`, function types, variables, lambda abstraction, application, and `let` bindings.

### Rust: Monomorphic Type Checker (code/main.rs)

See `code/main.rs` for a monomorphic type checker that walks an AST, maintains a type environment, and reports type mismatches.

---

## Use It

Production compilers and languages that use HM inference:

| Language | System | Notes |
|----------|--------|-------|
| Haskell (GHC) | Full HM + extensions | GADTs, type families, rank-n types extend the base system |
| OCaml | HM with value restriction | Restricts generalization to avoid unsoundness |
| Rust | Local HM inference | Types inferred within function bodies; function signatures are explicit |
| TypeScript | Partial inference | Infers types for variables and return values, but no unification over generics |

GHC's type checker lives in `compiler/GHC/Tc/`. The core algorithm is in `TcSimplify.hs` and `Unify.hs`. Rust's inference is in `rustc_typeck/src/infer/`.

### Read the Source

- `rustc/compiler/rustc_typeck/src/infer/` — Rust's type inference engine. Look at `unify.rs` for the unification table.
- GHC `compiler/GHC/Tc/TcSimplify.hs` — constraint simplification during type checking.

---

## Ship It

The reusable artifact from this lesson is:

- A type representation with type variables, constructors, and arrows
- A unification function with substitution and occurs check
- An environment-based type-checking function (monomorphic or HM)

---

## Exercises

### Level 1: Extend the Monomorphic Checker

Add support for `if/else` expressions in the Rust checker: `if (cond: Bool) { body1: T } else { body2: T }` should type-check if `cond` is `Bool` and both branches have the same type `T`.

### Level 2: Add Pairs to HM

Extend the Haskell type inference to support pairs: `(e1, e2)` with type `(τ1, τ2)`. Add `fst` and `snd` as built-in functions with appropriate types.

### Level 3: Implement Occurs Check and Mutual Recursion

The Haskell version skips the occurs check in unification. Implement it (reject `α = α → β`). Then add `letrec` bindings (mutual recursion) where the bound variable can be used in its own definition.

---

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Monomorphic | "One type per expression" | Every expression has exactly one, fixed type |
| Subtyping | "A Dog is-an Animal" | A subtype can be used wherever its supertype is expected |
| Type inference | "The compiler figures it out" | Deriving types without explicit annotations |
| Unification | "Making two types equal" | Finding a substitution that makes two types structurally identical |
| Type variable | "A placeholder type" | An unknown type to be determined, denoted α, β |
| Let-polymorphism | "Generics without annotations" | A `let`-bound variable can be instantiated with different types at each use site |
| Occurs check | "No infinite types" | Rejecting α = α → β, which would create an infinite type |

## Further Reading

- Robin Milner — "A Theory of Type Polymorphism in Programming" (1978) — the original Algorithm W paper
- Benjamin Pierce — *Types and Programming Languages* (TAPL), Chapters 10–23
- Oleg Kiselyov — "Hindley-Milner type inference in Haskell" — step-by-step walkthrough
