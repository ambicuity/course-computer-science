# Phase Capstone — A Bidirectional Type-Checker for STLC + Extensions

> Synthesize when possible, check when required, and make errors local.

**Type:** Build
**Languages:** Haskell, Rust
**Prerequisites:** Phase 18 lessons 01-14
**Time:** ~150 minutes

## Learning Objectives

- Implement a bidirectional checker core for STLC-style terms.
- Separate synthesis (`infer`) from checking (`check`) paths.
- Support annotations and basic lambda/application forms.
- Produce an assurance bundle linking rules, implementation, and tests.

## The Problem

Standard type inference (lesson 05) tries to infer every type automatically. This works for simple terms, but as the language grows richer, inference becomes undecidable or requires annotations everywhere. Full inference for System F is undecidable. Dependent types make it worse.

Bidirectional typing splits the problem in two: **synthesis** (inferring a type from a term) and **checking** (verifying a term matches an expected type). By flowing type information in both directions, the checker needs fewer annotations than full checking but stays decidable and produces better error messages.

Every modern typed language uses some form of bidirectional checking: GHC's typechecker, Rust's type inference, TypeScript's flow analysis, and proof assistants like Agda and Lean. Understanding bidirectional typing means understanding how real type checkers work.

## The Concept

### Two judgments

```
Γ ⊢ e ⇒ T    -- synthesis: from term e, compute type T
Γ ⊢ e ⇐ T    -- checking: verify e has type T
```

Synthesis flows information up (from subterms to the whole expression). Checking flows information down (from the expected type into subterms).

### Which terms synthesize and which check?

| Term form | Mode | Why |
|-----------|------|-----|
| Variable `x` | Synthesize | Look up type in context |
| Annotation `(e : T)` | Synthesize | Check `e` against `T`, return `T` |
| Lambda `λx. e` | Check | Need expected arrow type to know parameter type |
| Application `e1 e2` | Synthesize | Infer `e1 : T1 → T2`, check `e2 : T1`, return `T2` |
| Literal `true`/`false` | Synthesize | Always `Bool` |

### The switching rule

When synthesis encounters a checking context, it "switches" modes:

```
Γ ⊢ e ⇒ T    T ≡ T'
─────────────────────── (switch)
Γ ⊢ e ⇐ T'
```

And vice versa: when checking needs to synthesize, it inserts a fresh variable.

### ASCII typing rules

```
(T-Synth-Var)  (x : T) ∈ Γ
               ─────────────
               Γ ⊢ x ⇒ T

(T-Check-Lam)  Γ, x : T1 ⊢ e ⇐ T2
               ─────────────────────
               Γ ⊢ λx. e ⇐ T1 → T2

(T-Synth-App)  Γ ⊢ e1 ⇒ T1 → T2    Γ ⊢ e2 ⇐ T1
               ────────────────────────────────────
               Γ ⊢ e1 e2 ⇒ T2

(T-Synth-Ann)  Γ ⊢ e ⇐ T
               ───────────────
               Γ ⊢ (e : T) ⇒ T

(T-Check-Switch)  Γ ⊢ e ⇒ T    T ≡ T'
                  ──────────────────────
                  Γ ⊢ e ⇐ T'
```

### Why bidirectional?

| Approach | Annotations needed | Error quality | Decidability |
|----------|-------------------|---------------|-------------|
| Full inference (HM) | Few (mostly at let) | Can be confusing | Yes (rank-1) |
| Full checking | Everywhere | Clear | Always |
| Bidirectional | At mode-switch points | Localized | Yes |

Bidirectional finds the sweet spot: you annotate where type information changes direction (lambda parameters, sometimes function signatures), and the checker infers the rest.

## Build It

### Step 1: Define the AST (Haskell)

```haskell
data Type
  = TBool
  | TArrow Type Type
  | TVar String        -- for inference variables
  deriving (Eq, Show)

data Term
  = Var String
  | Lam String Term
  | App Term Term
  | Ann Term Type      -- type annotation
  | TmTrue
  | TmFalse
  | If Term Term Term
  deriving (Eq, Show)

type Context = [(String, Type)]
```

### Step 2: Implement the two modes

```haskell
data TypeError
  = UnboundVar String
  | TypeMismatch Type Type
  | NotArrowType Type
  | IfBranchMismatch Type Type
  | IfCondNotBool Type
  deriving (Show)

-- Synthesis: infer the type
synth :: Context -> Term -> Either TypeError Type
synth ctx (Var x) = case lookup x ctx of
  Just t  -> Right t
  Nothing -> Left (UnboundVar x)

synth ctx (Ann e t) = do
  check ctx e t
  return t

synth ctx (App e1 e2) = do
  t1 <- synth ctx e1
  case t1 of
    TArrow tArg tRet -> do
      check ctx e2 tArg
      return tRet
    _ -> Left (NotArrowType t1)

synth _ TmTrue  = Right TBool
synth _ TmFalse = Right TBool

synth ctx (If cond thenE elseE) = do
  check ctx cond TBool
  tThen <- synth ctx thenE
  tElse <- synth ctx elseE
  if tThen == tElse
    then return tThen
    else Left (IfBranchMismatch tThen tElse)

-- For lambda and other checking-mode terms, synthesize by
-- creating a fresh variable and checking
synth ctx (Lam x body) = do
  -- Lambda can't be synthesized without annotation
  -- This is where bidirectional typing needs an annotation
  Left (UnboundVar $ "Cannot infer type of lambda parameter '" ++ x
       ++ "': use an annotation")

synth ctx (If _ _ _) = error "handled above"

-- Checking: verify against expected type
check :: Context -> Term -> Type -> Either TypeError ()
check ctx (Lam x body) (TArrow t1 t2) = do
  check ((x, t1) : ctx) body t2
check ctx (Lam _ _) t =
  Left (TypeMismatch (TArrow (TVar "a") (TVar "b")) t)

check ctx term expected = do
  inferred <- synth ctx term
  if inferred == expected
    then Right ()
    else Left (TypeMismatch expected inferred)
```

### Step 3: Test it

```haskell
main :: IO ()
main = do
  -- λx: Bool. x : Bool → Bool  (annotated lambda)
  let idBool = Ann (Lam "x" (Var "x")) (TArrow TBool TBool)
  print $ synth [] idBool
  -- Right (TArrow TBool TBool)

  -- (λx: Bool. x) true : Bool
  let app = App idBool TmTrue
  print $ synth [] app
  -- Right TBool

  -- λx. x without annotation: fails
  print $ synth [] (Lam "x" (Var "x"))
  -- Left (UnboundVar "Cannot infer type of lambda parameter 'x'...")

  -- (λx: Bool → Bool. x) (λy: Bool. y) : Bool → Bool
  let idArr = Ann (Lam "x" (Var "x")) (TArrow (TArrow TBool TBool) (TArrow TBool TBool))
  let idBoolAnn = Ann (Lam "y" (Var "y")) (TArrow TBool TBool)
  print $ synth [] (App idArr idBoolAnn)
  -- Right (TArrow TBool TBool)

  -- Type error: applying non-function
  print $ synth [] (App TmTrue TmFalse)
  -- Left (NotArrowType TBool)
```

### Step 4: Rust implementation sketch

```rust
#[derive(Debug, Clone, PartialEq)]
enum Type {
    Bool,
    Arrow(Box<Type>, Box<Type>),
}

#[derive(Debug, Clone)]
enum Term {
    Var(String),
    Lam(String, Box<Term>),
    App(Box<Term>, Box<Term>),
    Ann(Box<Term>, Type),
    True,
    False,
}

type Context = Vec<(String, Type)>;

fn synth(ctx: &Context, term: &Term) -> Result<Type, String> {
    match term {
        Term::Var(x) => {
            ctx.iter().find(|(name, _)| name == x)
                .map(|(_, t)| t.clone())
                .ok_or_else(|| format!("Unbound variable: {}", x))
        }
        Term::Ann(e, t) => {
            check(ctx, e, t)?;
            Ok(t.clone())
        }
        Term::App(e1, e2) => {
            let t1 = synth(ctx, e1)?;
            match t1 {
                Type::Arrow(t_arg, t_ret) => {
                    check(ctx, e2, &t_arg)?;
                    Ok(*t_ret)
                }
                _ => Err(format!("Not a function type: {:?}", t1)),
            }
        }
        Term::True | Term::False => Ok(Type::Bool),
        Term::Lam(_, _) => Err("Cannot infer lambda type; use annotation".into()),
    }
}

fn check(ctx: &Context, term: &Term, expected: &Type) -> Result<(), String> {
    match (term, expected) {
        (Term::Lam(x, body), Type::Arrow(t1, t2)) => {
            let mut new_ctx = ctx.clone();
            new_ctx.push((x.clone(), *t1.clone()));
            check(&new_ctx, body, t2)
        }
        _ => {
            let inferred = synth(ctx, term)?;
            if inferred == *expected {
                Ok(())
            } else {
                Err(format!("Expected {:?}, got {:?}", expected, inferred))
            }
        }
    }
}
```

### Step 5: Add booleans and conditional

```haskell
-- (Already included in synth above)
-- The If rule synthesizes by:
-- 1. Checking condition is Bool
-- 2. Synthesizing both branches
-- 3. Requiring them to be equal
```

## Use It

Bidirectional checking appears in:

- **GHC**: the typechecker uses bidirectional ideas for GADTs, type applications, and visible type quantification.
- **Rust**: type inference is bidirectional. Function bodies are checked against signatures; local expressions are inferred.
- **Agda/Lean**: proof assistants use bidirectional typing to reduce annotation burden.
- **TypeScript**: flow-based analysis is a form of bidirectional checking.
- **Domain-specific type checkers**: any DSL with types benefits from the bidirectional approach.

The key insight: synthesis and checking are duals. Synthesis asks "what type is this?" Checking asks "does this have this type?" By splitting the work, you get better errors and fewer annotations.

## Read the Source

- [Local Type Inference](https://www.cis.upenn.edu/~bcpierce/papers/) (Pierce & Turner) — the foundational paper.
- [Practical Typechecking](https://arxiv.org/abs/1908.05839) — modern survey.
- GHC's typechecker: `compiler/GHC/Tc/Gen/Expr.hs`.
- [Bidirectional Typing Rules](https://arxiv.org/abs/1908.05839) — comprehensive rules.

## Ship It

- `code/Main.hs` and `code/main.rs`: compact checker skeleton.
- `outputs/README.md`: bidirectional checker validation checklist.

## Quiz

**Q1 (Pre).** What's the difference between synthesis and checking in bidirectional typing?

- A) They're the same thing.
- B) Synthesis infers a type from a term; checking verifies a term has an expected type.
- C) Synthesis is for functions; checking is for variables.
- D) Checking is always faster.

**Answer: B.** Synthesis (Γ ⊢ e ⇒ T) computes the type T from the term e. Checking (Γ ⊢ e ⇐ T) verifies that e has type T. Synthesis flows information up; checking flows information down. The checker alternates between modes depending on the term structure.

**Q2 (Pre).** Why can't lambda terms be synthesized without annotations?

- A) Lambda terms are always ambiguous.
- B) Without an expected arrow type, the parameter type is unknown; checking provides it.
- C) Lambda terms don't have types.
- D) The checker can't handle lambda.

**Answer: B.** In `λx. e`, the type of `x` isn't determined by the syntax alone. If checking expects `T1 → T2`, it provides `x : T1`. Without that context, the checker can't know what type to assign `x`. Annotations or checking context supply the missing information.

**Q3 (Post).** How does bidirectional typing reduce annotation burden compared to full checking?

- A) It doesn't; you still need annotations everywhere.
- B) Terms that naturally synthesize (variables, applications, annotations) don't need annotations; only checking-mode terms (lambdas) do.
- C) It eliminates all annotations.
- D) It only works for simple types.

**Answer: B.** Variables synthesize by lookup, applications synthesize by combining function and argument types, literals synthesize their known types. Only lambdas (and sometimes other checking-mode terms) need annotations. This is far fewer annotations than checking every subterm.

**Q4 (Post).** What is the "switch" rule in bidirectional typing?

- A) A rule that changes the language.
- B) When a synthesized type needs to be checked against an expected type, the checker switches from synthesis to checking mode.
- C) A rule for switching between languages.
- D) A rule that converts types.

**Answer: B.** The switch rule says: if Γ ⊢ e ⇒ T and T matches the expected type T', then Γ ⊢ e ⇐ T'. This allows synthesized terms to appear in checking contexts. It's the bridge between the two modes.

**Q5 (Post).** Why does GHC use bidirectional typing?

- A) It's simpler than unidirectional typing.
- B) It handles GADTs, type applications, and visible type quantification more naturally than pure inference.
- C) It's required by the Haskell specification.
- D) It's faster than Algorithm W.

**Answer: B.** GHC's typechecker uses bidirectional ideas for: GADTs (where constructors refine types), type applications (explicit type args are checked), and visible type quantification. Bidirectional typing lets GHC handle these extensions while keeping inference decidable and error messages localized.

## Exercises

1. **Easy.** Add booleans and the conditional typing rule to your checker. Test with `if true then false else true`.
2. **Medium.** Add let-bindings with simple generalization: `let x = e1 in e2`. The let-bound variable gets a polymorphic type.
3. **Hard.** Add source locations to your AST and produce error messages that point to the exact subterm that failed.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Synthesis | "infer type" | Compute a type from a term and context (information flows up) |
| Checking | "verify type" | Confirm a term matches an expected type (information flows down) |
| Annotation | "type hint" | User-provided type guiding the checker at mode-switch points |
| Bidirectional | "two-judgment typing" | Structured split between synthesis and checking modes |
| Mode switch | "flip direction" | Transition from synthesis to checking (or vice versa) at type boundaries |

## Further Reading

- [Local Type Inference](https://www.cis.upenn.edu/~bcpierce/papers/)
- [Practical Typechecking](https://arxiv.org/abs/1908.05839)
- [GHC Typechecker Architecture](https://gitlab.haskell.org/ghc/ghc/-/wikis/commentary/compiler/type-checker)
- [Software Foundations: Type Checking](https://softwarefoundations.cis.upenn.edu/plf-current/index.html)
