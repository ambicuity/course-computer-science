# Type Inference — Hindley-Milner Reconstruction

> Inference turns many explicit annotations into derivable constraints.

**Type:** Learn
**Languages:** Haskell, Rust
**Prerequisites:** Phase 18 lessons 01-04
**Time:** ~90 minutes

## Learning Objectives

- Explain HM inference pipeline: constraints, unification, generalization.
- Understand principal types intuition.
- Recognize where inference needs annotations.
- Build a tiny unification-style demo.

## The Problem

Write this in Haskell:

```haskell
f x y = x
```

What's the type of `f`? You didn't annotate it, but GHC infers `f :: a -> b -> a`. This works because Hindley-Milner inference deduces the most general type from the function body alone. The same algorithm powers OCaml, Elm, and influences Rust's type inference.

But inference has limits. Write this:

```haskell
f x = x x
```

GHC rejects it: "Occurs check: cannot construct the infinite type: a ~ a -> b." The function tries to apply `x` to itself, which requires `x : a` and `x : a -> b` simultaneously, forcing `a = a -> b` (an infinite type). Without annotations, the algorithm can't solve this.

Understanding inference means understanding what the compiler can and can't figure out on its own. When inference works, code is concise and annotations are optional. When it fails, you need to understand the constraint-solving machinery to diagnose the error.

## The Concept

### The four stages

```
Source code
    │
    ▼
┌─────────────────┐
│ 1. Assign fresh  │   Each expression gets a fresh type variable
│    type vars     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 2. Generate      │   Application: t_func = t_arg → t_result
│    constraints    │   Let-binding: generalize
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 3. Unify         │   Solve equations: t1 = t2 by substitution
│    constraints    │   Occurs check prevents infinite types
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 4. Generalize    │   Free type variables become ∀-bound
│    let-bindings   │
└─────────────────┘
```

### Constraint generation

For each expression, the algorithm generates type equations:

```haskell
-- f x y = x
-- Stage 1: assign fresh vars
--   f : t_f, x : t_x, y : t_y

-- Stage 2: generate constraints
--   body is x, so t_f = t_x (return type is x's type)
--   f is a function: t_f = t_x → t_y → t_result
--   result = t_x, so t_f = t_x → t_y → t_x

-- Stage 3: unify
--   t_f = t_x → t_y → t_x (solved)

-- Stage 4: generalize
--   t_x, t_y are free → forall a b. a -> b -> a
```

### Unification

Unification solves type equations. Given pairs `(t1, t2)`, find a substitution σ such that `σ(t1) = σ(t2)`.

```python
def unify(constraints):
    substitution = {}
    for (t1, t2) in constraints:
        t1 = apply_subst(substitution, t1)
        t2 = apply_subst(substitution, t2)
        if t1 == t2:
            continue
        elif is_var(t1):
            if t1 in free_vars(t2):
                raise Error("occurs check: " + t1 + " in " + t2)
            substitution[t1] = t2
        elif is_var(t2):
            substitution[t2] = t1
        elif is_arrow(t1) and is_arrow(t2):
            constraints.append((t1.arg, t2.arg))
            constraints.append((t1.ret, t2.ret))
        else:
            raise Error("can't unify: " + t1 + " with " + t2)
    return substitution
```

### The occurs check

```
unify(a, a -> b)
```

Substituting `a = a -> b` creates an infinite type: `a -> b = (a -> b) -> b = ((a -> b) -> b) -> b = ...`

The occurs check (`a` appears free in `a -> b`) rejects this. This is exactly why `f x = x x` fails to type-check.

### Generalization and instantiation

At `let`-bindings, HM generalizes:

```haskell
let id = \x -> x in (id 1, id True)
-- id : forall a. a -> a  (generalized)
-- id 1 : Int              (instantiated with a = Int)
-- id True : Bool           (instantiated with a = Bool)
```

Without `let`-polymorphism, `id` would get type `a -> a` with a single `a`, and you couldn't use it at two different types in the same expression.

### Principal types

HM guarantees a **principal type**: the most general type from which all valid typings can be derived by specialization. If a term has any typing, HM finds the most general one.

```
\x -> x          : forall a. a -> a          (principal)
-- also valid:   Int -> Int, Bool -> Bool, etc.
-- but forall a. a -> a is the most general
```

## Build It

### Step 1: Type representation (Python)

```python
from dataclasses import dataclass
from typing import Union

@dataclass
class TypeVar:
    name: str

@dataclass
class TypeArrow:
    arg: 'Type'
    ret: 'Type'

Type = Union[TypeVar, TypeArrow]
```

### Step 2: Unification

```python
def free_vars(t: Type) -> set:
    match t:
        case TypeVar(name):
            return {name}
        case TypeArrow(arg, ret):
            return free_vars(arg) | free_vars(ret)

def apply_subst(subst: dict, t: Type) -> Type:
    match t:
        case TypeVar(name):
            if name in subst:
                return apply_subst(subst, subst[name])
            return t
        case TypeArrow(arg, ret):
            return TypeArrow(apply_subst(subst, arg), apply_subst(subst, ret))

def unify(t1: Type, t2: Type) -> dict:
    t1 = apply_subst({}, t1)
    t2 = apply_subst({}, t2)
    match (t1, t2):
        case (TypeVar(a), TypeVar(b)) if a == b:
            return {}
        case (TypeVar(a), _):
            if a in free_vars(t2):
                raise TypeError(f"Occurs check: {a} in {t2}")
            return {a: t2}
        case (_, TypeVar(_)):
            return unify(t2, t1)
        case (TypeArrow(a1, r1), TypeArrow(a2, r2)):
            s1 = unify(a1, a2)
            s2 = unify(apply_subst(s1, r1), apply_subst(s1, r2))
            return {**s1, **s2}
    raise TypeError(f"Can't unify {t1} with {t2}")
```

### Step 3: Constraint-based inference

```python
fresh_counter = [0]
def fresh_var():
    fresh_counter[0] += 1
    return TypeVar(f"t{fresh_counter[0]}")

def infer(env: dict, term) -> tuple[Type, list]:
    """Returns (type, constraints) for a term."""
    match term:
        case Var(x):
            if x not in env:
                raise TypeError(f"Unbound: {x}")
            return (env[x], [])
        case Lam(x, body):
            tv = fresh_var()
            body_ty, cs = infer({**env, x: tv}, body)
            return (TypeArrow(tv, body_ty), cs)
        case App(func, arg):
            func_ty, cs1 = infer(env, func)
            arg_ty, cs2 = infer(env, arg)
            ret_ty = fresh_var()
            cs3 = [(func_ty, TypeArrow(arg_ty, ret_ty))]
            return (ret_ty, cs1 + cs2 + cs3)
```

### Step 4: Haskell equivalent

```haskell
import Data.Map (Map)
import qualified Data.Map as Map

data Type = TVar String | TArrow Type Type
  deriving (Eq, Show)

type Subst = Map String Type

applySubst :: Subst -> Type -> Type
applySubst s (TVar v) = case Map.lookup v s of
  Just t  -> applySubst s t
  Nothing -> TVar v
applySubst s (TArrow a r) = TArrow (applySubst s a) (applySubst s r)

unify :: Type -> Type -> Subst
unify (TVar a) (TVar b) | a == b = Map.empty
unify (TVar a) t
  | a `elem` freeVars t = error $ "Occurs check: " ++ a ++ " in " ++ show t
  | otherwise = Map.singleton a t
unify t (TVar a) = unify (TVar a) t
unify (TArrow a1 r1) (TArrow a2 r2) =
  let s1 = unify a1 a2
      s2 = unify (applySubst s1 r1) (applySubst s1 r2)
  in Map.union s2 s1

freeVars :: Type -> [String]
freeVars (TVar v) = [v]
freeVars (TArrow a r) = freeVars a ++ freeVars r
```

### Step 5: Test the unifier

```python
# Test: unify(a, Int) => {a: Int}
print(unify(TypeVar("a"), TypeInt))  # {'a': TypeInt}

# Test: unify(a -> b, Int -> Bool) => {a: Int, b: Bool}
print(unify(
    TypeArrow(TypeVar("a"), TypeVar("b")),
    TypeArrow(TypeInt, TypeBool)
))  # {'a': TypeInt, 'b': TypeBool}

# Test: occurs check
try:
    unify(TypeVar("a"), TypeArrow(TypeVar("a"), TypeVar("b")))
except TypeError as e:
    print(e)  # "Occurs check: a in a -> b"
```

## Use It

HM-style inference powers production systems:

- **GHC**: full HM inference extended with typeclasses, GADTs, and more. The constraint solver is one of the most complex parts of the compiler.
- **OCaml**: HM inference with some extensions. The `value restriction` limits generalization to prevent unsoundness with mutable references.
- **Rust**: local type inference (not full HM). Function signatures must be annotated, but local variables are inferred.
- **TypeScript**: flow-based type narrowing, but not HM. Less powerful inference, more explicit annotations.
- **Elm**: full HM inference, deliberately simple.

When inference fails, the error messages point to the constraint that couldn't be solved. Understanding unification helps you read these messages.

## Read the Source

- Damas-Hindley-Milner original paper: "Principal type-schemes for functional programs."
- [Algorithm W step-by-step](https://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.65.7733&rep=rep1&type=pdf) — detailed walkthrough.
- GHC's constraint solver: `compiler/GHC/Tc/Solver/` in the GHC source tree.

## Ship It

- `code/Main.hs`: concise constraint/unification sketch.
- `code/main.py`: equivalent mini unifier example.
- `outputs/README.md`: inference debugging checklist.

## Quiz

**Q1 (Pre).** What is a "principal type"?

- A) The type the programmer intended.
- B) The most general valid type from which all other valid typings follow by specialization.
- C) The simplest type with no type variables.
- D) The type with the fewest function arrows.

**Answer: B.** The principal type is the most general typing. From `∀a. a -> a`, you can derive `Int -> Int`, `Bool -> Bool`, etc. by substituting for `a`. HM always finds the principal type if one exists.

**Q2 (Pre).** Why does `f x = x x` fail to type-check under HM?

- A) Variables can't appear twice.
- B) It requires `x : a` and `x : a -> b`, forcing `a = a -> b` (infinite type, rejected by occurs check).
- C) Self-application is syntactically invalid.
- D) HM doesn't support function application.

**Answer: B.** For `x x`, `x` must have type `a -> b` (to be applied). But `x` also has type `a` (the parameter). Unifying `a = a -> b` creates an infinite type. The occurs check (`a` appears in `a -> b`) rejects this.

**Q3 (Post).** What does the unification algorithm produce?

- A) A type annotation.
- B) A substitution mapping type variables to types, making all constraint equations hold.
- C) A list of type errors.
- D) A parse tree.

**Answer: B.** Unification takes a set of type equations and produces a most general unifier: a substitution σ such that applying σ to both sides of each equation yields the same type. This substitution is the solution to the constraint system.

**Q4 (Post).** Why does HM generalize at `let`-bindings but not at lambda parameters?

- A) Lambda parameters are always monomorphic.
- B) Generalizing at lambda would lose principal types with mutable references.
- C) `let` bindings are syntactically special.
- D) Lambda parameters don't need types.

**Answer: B.** With mutable references, generalizing at lambda can be unsound. The value restriction: only syntactic values (not arbitrary expressions) get generalized at `let`. This prevents `let r = ref [] in (r := [1], r := [True])` from type-checking with a polymorphic ref.

**Q5 (Post).** What's the relationship between unification and type inference?

- A) They're unrelated.
- B) Unification is the constraint-solving step that makes type inference work.
- C) Unification replaces type inference.
- D) Type inference is a special case of unification.

**Answer: B.** Type inference generates constraints from the syntax (e.g., `f x` generates `typeof(f) = typeof(x) -> t_fresh`). Unification solves those constraints to produce a substitution. The substitution, applied to the fresh variables, gives the inferred types.

## Exercises

1. **Easy.** Add an occurs-check failure example to your unifier. Show the error message.
2. **Medium.** Add a tuple type constructor `(T1, T2)` to your type language. Extend unification to handle it.
3. **Hard.** Explain why polymorphic recursion (a recursive call at a different type) needs explicit annotations under HM. Provide an example.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Unification | "type solving" | Finding a substitution making two type expressions structurally equal |
| Principal type | "best type" | The most general valid typing, from which all others follow by specialization |
| Generalization | "forall introduction" | Converting free type variables to universally quantified at let-bindings |
| Occurs check | "infinite type guard" | Rejecting `a = T` when `a` appears free in `T`, preventing infinite types |
| Algorithm W | "the inference algorithm" | The classic HM type inference procedure: constraint generation + unification + generalization |

## Further Reading

- [Hindley-Milner Type Inference (Wikipedia)](https://en.wikipedia.org/wiki/Hindley%E2%80%93Milner_type_system)
- [Algorithm W Step-by-Step](https://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.65.7733&rep=rep1&type=pdf)
- [Real World OCaml: Type Inference](https://dev.realworldocaml.org)
