# Simply Typed Lambda Calculus

> Types constrain terms so whole classes of runtime errors become unrepresentable.

**Type:** Learn
**Languages:** Haskell
**Prerequisites:** Phase 18 lessons 01-02
**Time:** ~75 minutes

## Learning Objectives

- Define STLC syntax and typing judgments.
- Implement a small type checker for variables, lambdas, and application.
- Understand progress/preservation intuition.
- Explain why STLC rejects self-application patterns.

## The Problem

Untyped lambda calculus can express `(λx. x x) (λx. x x)`, the self-application that loops forever. It can also express `(λx. x + 1) true`, applying arithmetic to a boolean. Both are "legal" programs in the untyped world, but one diverges and the other is nonsensical.

In production, these correspond to real bugs. A function expecting an integer receives a boolean. A handler calls itself recursively with no base case. In Python, you discover these at runtime with a `TypeError` or a stack overflow. In Haskell or Rust, the compiler catches them before the program runs.

The simply typed lambda calculus (STLC) is the minimal type system that prevents these problems. It adds one thing to the lambda calculus: every term has a type, and the type system rejects terms that could go wrong. The cost is that STLC is no longer Turing-complete: every well-typed STLC program terminates. This is a feature, not a bug, for many domains (total functions, configuration, type-level computation).

## The Concept

### STLC syntax

Types:
```
T ::= Bool          -- base type
    | T1 → T2       -- function type (arrow)
```

Terms (same as lambda calculus, plus booleans):
```
e ::= x             -- variable
    | λx: T. e      -- annotated lambda
    | e1 e2         -- application
    | true | false  -- boolean literals
    | if e then e else e  -- conditional
```

The key addition: lambda annotations (`λx: T. e`). Every parameter carries its type.

### Typing judgments

A typing judgment has the form:

```
Γ ⊢ e : T
```

Read: "Under context Γ, term `e` has type `T`." The context Γ is a mapping from variable names to types.

The typing rules:

```
(T-Var)    (x : T) ∈ Γ
           ─────────────
           Γ ⊢ x : T

(T-Abs)    Γ, x : T1 ⊢ e : T2
           ─────────────────────
           Γ ⊢ λx: T1. e : T1 → T2

(T-App)    Γ ⊢ e1 : T1 → T2    Γ ⊢ e2 : T1
           ───────────────────────────────────
           Γ ⊢ e1 e2 : T2

(T-True)   Γ ⊢ true : Bool

(T-False)  Γ ⊢ false : Bool

(T-If)     Γ ⊢ e1 : Bool    Γ ⊢ e2 : T    Γ ⊢ e3 : T
           ─────────────────────────────────────────────
           Γ ⊢ if e1 then e2 else e3 : T
```

### Progress and Preservation

These two theorems make STLC sound:

**Progress:** If `⊢ e : T` (closed, well-typed term), then either `e` is a value or there exists some `e'` such that `e → e'`. A well-typed term never gets stuck.

**Preservation:** If `⊢ e : T` and `e → e'`, then `⊢ e' : T`. Evaluation preserves types.

Together: well-typed programs don't go wrong. Every step produces a well-typed term, and every non-value can take a step.

### Why self-application is rejected

```
(λx: T. x x) (λx: T. x x)
```

For `x x` to type-check, `x` must have type `T1 → T2` (to be applied). But `x`'s type is `T`, which is the same as `T1 → T2` only if `T = T1 → T2`. Then the argument `λx: T. x x` must also have type `T = T1 → T2`. This creates a circular constraint with no finite solution. The type checker rejects it.

```
x : T         (from lambda binder)
x : T → ?     (from application position)
=> T = T → ?  (unification fails: infinite type)
```

## Build It

### Step 1: Define types and terms (Haskell)

```haskell
data Type = TBool | TArrow Type Type
  deriving (Eq, Show)

data Term
  = TmVar String
  | TmLam String Type Term
  | TmApp Term Term
  | TmTrue
  | TmFalse
  | TmIf Term Term Term
  deriving (Eq, Show)
```

### Step 2: Context and lookup

```haskell
type Context = [(String, Type)]

lookupVar :: Context -> String -> Either String Type
lookupVar [] x = Left $ "Unbound variable: " ++ x
lookupVar ((y, t):rest) x
  | x == y    = Right t
  | otherwise = lookupVar rest x
```

### Step 3: Type checker

```haskell
typeOf :: Context -> Term -> Either String Type
typeOf ctx (TmVar x) = lookupVar ctx x

typeOf ctx (TmLam x tyT1 body) = do
  tyT2 <- typeOf ((x, tyT1) : ctx) body
  return (TArrow tyT1 tyT2)

typeOf ctx (TmApp func arg) = do
  tyFunc <- typeOf ctx func
  case tyFunc of
    TArrow tyT1 tyT2 -> do
      tyArg <- typeOf ctx arg
      if tyArg == tyT1
        then return tyT2
        else Left $ "Type mismatch: expected " ++ show tyT1
             ++ ", got " ++ show tyArg
    _ -> Left $ "Not a function type: " ++ show tyFunc

typeOf _ TmTrue  = return TBool
typeOf _ TmFalse = return TBool

typeOf ctx (TmIf cond thenBr elseBr) = do
  tyCond <- typeOf ctx cond
  case tyCond of
    TBool -> do
      tyThen <- typeOf ctx thenBr
      tyElse <- typeOf ctx elseBr
      if tyThen == tyElse
        then return tyThen
        else Left $ "If branches have different types: "
             ++ show tyThen ++ " vs " ++ show tyElse
    _ -> Left $ "If condition must be Bool, got " ++ show tyCond
```

### Step 4: Test it

```haskell
main :: IO ()
main = do
  -- λx: Bool. x : Bool → Bool
  let id = TmLam "x" TBool (TmVar "x")
  print $ typeOf [] id
  -- Right (TArrow TBool TBool)

  -- (λx: Bool. x) true : Bool
  let app = TmApp id TmTrue
  print $ typeOf [] app
  -- Right TBool

  -- λx: Bool. x x : REJECTED
  let selfApp = TmLam "x" TBool (TmApp (TmVar "x") (TmVar "x"))
  print $ typeOf [] selfApp
  -- Left "Not a function type: TBool"

  -- if true then 1 else "hello" : REJECTED (different types)
  -- (encoded with Bool as proxy)
  let badIf = TmIf TmTrue TmTrue TmFalse
  print $ typeOf [] badIf
  -- Right TBool (this one works; branches match)

  -- if true then true else (λx: Bool. x) : REJECTED
  let badIf2 = TmIf TmTrue TmTrue id
  print $ typeOf [] badIf2
  -- Left "If branches have different types: TBool vs TArrow TBool TBool"
```

### Step 5: Small-step evaluator

```haskell
isValue :: Term -> Bool
isValue TmTrue  = True
isValue TmFalse = True
isValue TmLam{} = True
isValue _       = False

subst :: String -> Term -> Term -> Term
subst x rep (TmVar y)     | x == y    = rep
                          | otherwise = TmVar y
subst x rep (TmLam y ty body)
  | x == y    = TmLam y ty body
  | otherwise = TmLam y ty (subst x rep body)
subst x rep (TmApp f a)   = TmApp (subst x rep f) (subst x rep a)
subst _ _ TmTrue          = TmTrue
subst _ _ TmFalse         = TmFalse
subst x rep (TmIf c t e)  = TmIf (subst x rep c) (subst x rep t) (subst x rep e)

step :: Term -> Maybe Term
step (TmApp (TmLam x _ body) arg) | isValue arg = Just (subst x arg body)
step (TmApp func arg) = do
  func' <- step func
  Just (TmApp func' arg)
step (TmIf TmTrue thenBr _)  = Just thenBr
step (TmIf TmFalse _ elseBr) = Just elseBr
step (TmIf cond t e) = do
  cond' <- step cond
  Just (TmIf cond' t e)
step _ = Nothing
```

## Use It

STLC ideas scale into modern typed languages and compilers:

- **Bidirectional checking** (lesson 15): splits inference into synthesis and checking modes, building on STLC's judgments.
- **Inference extensions** (lesson 5): Hindley-Milner adds let-polymorphism on top of STLC foundations.
- **GHC's typechecker**: starts from STLC-like rules, then layers on polymorphism, typeclasses, GADTs, and more.
- **Rust's type system**: begins with STLC-like function types, adds ownership/borrowing as additional constraints.
- **Proof assistants**: Coq and Lean use dependent extensions of STLC for theorem proving.

## Read the Source

- *Types and Programming Languages* (Pierce), Chapter 8-11: STLC, evaluation, and simple extensions.
- *Software Foundations* Volume PLF: Coq formalization of STLC with progress/preservation proofs.
- GHC's typechecker: `compiler/GHC/Tc/Gen/Expr.hs` for how Haskell extends these rules.

## Ship It

- `code/Main.hs`: compact STLC checker with evaluator.
- `outputs/README.md`: typing-rule checklist.

## Quiz

**Q1 (Pre).** What does the judgment `Γ ⊢ e : T` mean?

- A) The expression `e` evaluates to value `T`.
- B) Under context Γ, term `e` has type `T`.
- C) The variable `e` is bound in context Γ.
- D) The type `T` is a subtype of `e`.

**Answer: B.** The judgment reads: "Under the typing context Γ (a mapping from variables to types), we can derive that term `e` has type `T`." It's a formal claim about the term, not about its runtime behavior.

**Q2 (Pre).** Why does STLC reject `λx: Bool. x x`?

- A) Variables can't appear twice in a lambda body.
- B) `x` has type `Bool`, but application requires a function type.
- C) Bool is not a valid type annotation.
- D) Self-application is syntactically invalid.

**Answer: B.** In `x x`, the first `x` is in function position, requiring type `T1 → T2`. But `x` was annotated as `Bool`. Since `Bool ≠ T1 → T2` for any `T1, T2`, the application fails. The type system prevents applying non-functions.

**Q3 (Post).** What do the Progress and Preservation theorems guarantee together?

- A) All programs terminate.
- B) Well-typed programs never get stuck and evaluation preserves types.
- C) Type inference always succeeds.
- D) All expressions have normal forms.

**Answer: B.** Progress says a well-typed closed term is either a value or can take a step (never stuck). Preservation says stepping preserves the type. Together: well-typed programs can't go wrong at runtime. Note that STLC programs do terminate (strong normalization), but that's a separate property.

**Q4 (Post).** In the typing rule for `T-App`, what must be true about the function's type?

- A) It must be `Bool`.
- B) It must be an arrow type `T1 → T2` where the argument's type matches `T1`.
- C) It must be the same type as the argument.
- D) It can be any type.

**Answer: B.** Application `e1 e2` requires `e1 : T1 → T2` and `e2 : T1`. The function must have an arrow type, and the argument must match the domain. This is exactly what prevents applying booleans to integers.

**Q5 (Post).** STLC is not Turing-complete. Why is this considered a feature in some domains?

- A) It makes programs run faster.
- B) It guarantees all well-typed programs terminate, useful for total functions and configuration languages.
- C) It simplifies the syntax.
- D) It allows more expressive types.

**Answer: B.** Strong normalization means every well-typed STLC program terminates. This is valuable for: configuration languages (no infinite loops), type-level computation (decidable type checking), proof assistants (every proof term terminates), and embedded DSLs where termination is a requirement.

## Exercises

1. **Easy.** Add booleans and the `if-then-else` typing rule to your checker. Test with `if true then false else true`.
2. **Medium.** Add pair types `(T1, T2)`, pair construction `(e1, e2)`, and projections `fst e`, `snd e`. Write the typing rules and implement them.
3. **Hard.** Add a type pretty-printer that renders `TArrow (TArrow TBool TBool) TBool` as `(Bool → Bool) → Bool`.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Context (Γ) | "scope" | A mapping from variable names to their types |
| Typing judgment | "type statement" | Formal claim that a term has a type under a context |
| Progress | "won't get stuck" | Well-typed closed terms are either values or can take a step |
| Preservation | "type stability" | If `e : T` and `e → e'`, then `e' : T` |
| Arrow type | "function type" | The type `T1 → T2` of functions from `T1` to `T2` |

## Further Reading

- [Types and Programming Languages](https://mitpress.mit.edu/9780262162095/types-and-programming-languages/)
- [Software Foundations PLF](https://softwarefoundations.cis.upenn.edu/plf-current/index.html)
- [Pierce's STLC lecture notes](https://www.cis.upenn.edu/~bcpierce/courses/670Fall04/)
