# Lambda Calculus — Reduction, Encodings

> Small substitution rules can encode surprising computational power.

**Type:** Learn
**Languages:** Haskell, Python
**Prerequisites:** Phase 18 lesson 01
**Time:** ~90 minutes

## Learning Objectives

- Explain alpha/beta reduction and normal forms.
- Perform safe substitution avoiding capture.
- Understand Church encodings for booleans and numerals.
- Connect lambda-calculus mechanics to language runtime semantics.

## The Problem

Every functional language compiler, from GHC to the OCaml runtime, performs beta-reduction: substituting arguments into function bodies. But substitution is trickier than it looks. Consider this Python expression:

```python
(lambda x: (lambda y: x))(y)
```

If you naively substitute `y` for `x`, you get `lambda y: y`. The free variable `y` got captured by the inner binder. The meaning changed entirely: the original returned its argument unchanged, the substituted version applies the identity function to itself.

This is variable capture, and it's the reason every compiler, macro system, and template engine needs careful substitution logic. Rust's macro hygiene, Haskell's desugaring, and even C's `#define` pitfalls all trace back to the same fundamental problem: how do you replace names without accidentally changing meaning?

Lambda calculus gives us the minimal framework to study this precisely. Three constructs (variables, abstraction, application), one reduction rule (beta), and a rich theory of evaluation strategies that directly maps to how real languages choose between call-by-value, call-by-name, and call-by-need.

## The Concept

### Lambda calculus syntax

The untyped lambda calculus has exactly three constructs:

```
e ::= x             -- variable
    | λx. e         -- abstraction (function definition)
    | e1 e2         -- application (function call)
```

That's it. No numbers, no booleans, no strings. Everything is encoded as functions.

### Alpha-conversion (α)

Alpha-conversion renames bound variables. These two expressions are alpha-equivalent:

```
λx. x        ≡α       λy. y
```

Both represent the identity function. The name of the bound variable doesn't matter, only its relationship to the body.

### Beta-reduction (β)

Beta-reduction is function application via substitution:

```
(λx. e1) e2  →β   e1[x := e2]
```

Read: "apply `λx. e1` to `e2`" reduces to "`e1` with `e2` substituted for `x`."

Example:
```
(λx. x x) (λy. y)  →β  (λy. y) (λy. y)  →β  λy. y
```

### Capture-avoiding substitution

Naive substitution can change meaning. The safe version:

```
e1[x := e2]:
  x[x := e2]           = e2
  y[x := e2]           = y              (y ≠ x)
  (λx. e)[x := e2]    = λx. e          (x shadows, don't descend)
  (λy. e)[x := e2]    = λy. e[x := e2]  if y ∉ FV(e2)
                       = λz. e[y := z][x := e2]  otherwise (rename y to fresh z)
  (e1 e2)[x := e]      = (e1[x := e]) (e2[x := e])
```

The critical case: when substituting into a binder `λy. e`, if `y` appears free in `e2`, we must alpha-rename `y` to a fresh variable `z` first. This prevents capture.

### Free and bound variables

```
FV(x)         = {x}
FV(λx. e)     = FV(e) \ {x}
FV(e1 e2)     = FV(e1) ∪ FV(e2)
```

A variable is **free** if nothing binds it. A closed term (no free variables) is a **combinator**.

### Normal forms

A **beta-redex** is a subexpression of the form `(λx. e1) e2`. A term in **normal form** has no beta-redexes. Not every term has a normal form:

```
(λx. x x) (λx. x x)  →β  (λx. x x) (λx. x x)  →β  ...
```

This is the classic omega combinator: it reduces forever.

### Evaluation strategies

| Strategy | Redex choice | Termination | Used by |
|----------|-------------|-------------|---------|
| Normal order | Leftmost-outermost | Finds normal form if one exists | Haskell (lazy) |
| Applicative order | Leftmost-innermost | May not find normal form | Python, Java, Rust |
| Call-by-need | Normal order + memoization | Finds normal form, shares work | Haskell (actual) |
| Call-by-name | Like normal order, no sharing | Finds normal form | Scala (by-name params) |

```
Applicative:   (λx. y) ((λx. x x)(λx. x x))
               inner first → loops forever

Normal order:  (λx. y) ((λx. x x)(λx. x x))
               outer first → β → y (done)
```

## Build It

### Step 1: Define the AST (Python)

```python
from dataclasses import dataclass
from typing import Union

@dataclass(frozen=True)
class Var:
    name: str

@dataclass(frozen=True)
class Lam:
    param: str
    body: 'Term'

@dataclass(frozen=True)
class App:
    func: 'Term'
    arg: 'Term'

Term = Union[Var, Lam, App]
```

### Step 2: Free variables

```python
def free_vars(t: Term) -> set:
    match t:
        case Var(name):
            return {name}
        case Lam(param, body):
            return free_vars(body) - {param}
        case App(func, arg):
            return free_vars(func) | free_vars(arg)
```

### Step 3: Capture-avoiding substitution

```python
def fresh_var(avoid: set) -> str:
    i = 0
    while f"v{i}" in avoid:
        i += 1
    return f"v{i}"

def subst(t: Term, x: str, replacement: Term) -> Term:
    match t:
        case Var(name):
            return replacement if name == x else t
        case Lam(param, body) if param == x:
            return t  # x is shadowed
        case Lam(param, body):
            if param in free_vars(replacement):
                z = fresh_var(free_vars(body) | free_vars(replacement) | {x})
                body = subst(body, param, Var(z))
                return Lam(z, subst(body, x, replacement))
            return Lam(param, subst(body, x, replacement))
        case App(func, arg):
            return App(subst(func, x, replacement), subst(arg, x, replacement))
```

### Step 4: Beta-reduction (one step)

```python
def beta_step(t: Term) -> Term | None:
    match t:
        case App(Lam(param, body), arg):
            return subst(body, param, arg)  # beta-reduction
        case App(func, arg):
            new_func = beta_step(func)
            if new_func:
                return App(new_func, arg)
            new_arg = beta_step(arg)
            if new_arg:
                return App(func, new_arg)
        case Lam(param, body):
            new_body = beta_step(body)
            if new_body:
                return Lam(param, new_body)
    return None
```

### Step 5: Church encodings

Church booleans:
```python
TRUE  = Lam("t", Lam("f", Var("t")))
FALSE = Lam("t", Lam("f", Var("f")))

# IF = λc. λt. λf. c t f
IF = Lam("c", Lam("t", Lam("f", App(App(Var("c"), Var("t")), Var("f")))))
```

Church numerals:
```python
# 0 = λf. λx. x
ZERO = Lam("f", Lam("x", Var("x")))
# 1 = λf. λx. f x
ONE  = Lam("f", Lam("x", App(Var("f"), Var("x"))))
# 2 = λf. λx. f (f x)
TWO  = Lam("f", Lam("x", App(Var("f"), App(Var("f"), Var("x")))))

# SUCC = λn. λf. λx. f (n f x)
SUCC = Lam("n", Lam("f", Lam("x",
    App(Var("f"), App(App(Var("n"), Var("f")), Var("x"))))))
```

### Step 6: Haskell equivalent

```haskell
data Term = Var String | Lam String Term | App Term Term
  deriving (Eq, Show)

freeVars :: Term -> [String]
freeVars (Var x)     = [x]
freeVars (Lam x e)   = filter (/= x) (freeVars e)
freeVars (App e1 e2) = freeVars e1 ++ freeVars e2

subst :: String -> Term -> Term -> Term
subst x rep (Var y)     | x == y    = rep
                        | otherwise = Var y
subst x rep (Lam y e)   | x == y    = Lam y e
                        | y `elem` freeVars rep =
                            let z = freshVar (freeVars e ++ freeVars rep ++ [x])
                            in Lam z (subst x rep (subst y (Var z) e))
                        | otherwise = Lam y (subst x rep e)
subst x rep (App e1 e2) = App (subst x rep e1) (subst x rep e2)

freshVar :: [String] -> String
freshVar used = head [v | i <- [0..], let v = "v" ++ show i, v `notElem` used]
```

## Use It

These concepts map directly to production systems:

- **GHC Core**: GHC compiles Haskell to a lambda-calculus-like intermediate language. Every optimization pass is a rewrite on this core. Beta-reduction, let-floating, and inlining are all lambda calculus operations.
- **Closure conversion**: When a compiler converts from lambda calculus to machine code, it must close over free variables. The `free_vars` function we wrote is exactly what the compiler needs.
- **Macro systems**: Rust's `macro_rules!` and Lisp macros need hygienic substitution. The capture-avoidance logic is the same.
- **Evaluation strategy**: Python uses applicative order (call-by-value). Haskell uses call-by-need (lazy). The lambda calculus makes the tradeoffs precise.

## Read the Source

- *Types and Programming Languages* (Pierce), Chapters 5-6: lambda calculus and evaluation.
- *The Implementation of Functional Programming Languages* (Peyton Jones), Chapter 2: the lambda calculus backbone.
- GHC's Core language: `compiler/GHC/Core.hs` in the GHC source tree.

## Ship It

- `code/main.py`: tiny reducer with Church encodings.
- `code/Main.hs`: minimal beta-reduction demo.
- `outputs/README.md`: reduction workflow checklist.

## Quiz

**Q1 (Pre).** What is the result of beta-reducing `(λx. x x) (λy. y)`?

- A) `λy. y`
- B) `(λy. y) (λy. y)`
- C) `x x`
- D) `λx. x x`

**Answer: B.** Beta-reduction substitutes the argument `(λy. y)` for `x` in the body `x x`, yielding `(λy. y) (λy. y)`. This can then reduce further to `λy. y`, but the single beta-step gives the application.

**Q2 (Pre).** Why does naive substitution fail for `(λx. λy. x) y`?

- A) The expression has no normal form.
- B) Substituting `y` for `x` gives `λy. y`, where `y` got captured by the inner binder.
- C) Lambda calculus doesn't allow nested abstractions.
- D) The expression is not well-typed.

**Answer: B.** Naive substitution yields `λy. y` (the identity function), but the original returns its argument unchanged applied to the outer `y`. The free `y` got captured by `λy`, changing the meaning. Capture-avoiding substitution would rename the inner `y` first.

**Q3 (Post).** Which evaluation strategy guarantees finding a normal form if one exists?

- A) Applicative order (call-by-value)
- B) Normal order (leftmost-outermost)
- C) Random order
- D) Rightmost-innermost

**Answer: B.** Normal order always reduces the leftmost-outermost redex first. By the standardization theorem, if a term has a normal form, normal order will find it. Applicative order may loop on terms like `(λx. y) omega` even though a normal form exists.

**Q4 (Post).** What is the Church numeral for 2?

- A) `λf. λx. f x`
- B) `λf. λx. f (f x)`
- C) `λf. λx. f (f (f x))`
- D) `λf. λx. x`

**Answer: B.** Church numerals encode natural numbers as repeated function application. 0 = `λf. λx. x` (apply `f` zero times), 1 = `λf. λx. f x` (once), 2 = `λf. λx. f (f x)` (twice). The numeral `n` applies `f` exactly `n` times to `x`.

**Q5 (Post).** Why is the omega combinator `(λx. x x) (λx. x x)` significant?

- A) It demonstrates that lambda calculus is Turing-complete.
- B) It shows that some terms have no normal form, proving the halting problem's relevance.
- C) It's the simplest term that evaluates to itself under beta-reduction.
- D) It proves alpha-conversion is necessary.

**Answer: C.** Omega reduces to itself in one beta-step: `(λx. x x)(λx. x x) →β (λx. x x)(λx. x x)`. This is the simplest non-terminating term. It's significant because it shows beta-reduction doesn't always terminate, connecting to the halting problem and motivating normal-form analysis.

## Exercises

1. **Easy.** Add Church numerals for 3 and implement the `PLUS` combinator `λm. λn. λf. λx. m f (n f x)`. Verify `PLUS ONE TWO` reduces to the Church numeral for 3.
2. **Medium.** Compare normal-order vs applicative-order reduction on `(λx. y) ((λx. x x)(λx. x x))`. Show the reduction sequence for each. Which terminates?
3. **Hard.** Implement a free-variable analyzer and a pretty-printer for your lambda calculus AST. Handle precedence and parentheses correctly.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Beta reduction | "function call" | Substitution of argument into lambda body: `(λx. e1) e2 → e1[x := e2]` |
| Alpha conversion | "rename vars" | Renaming bound variables while preserving meaning |
| Normal form | "final result" | A term with no remaining beta-redexes |
| Capture avoidance | "safe substitution" | Preventing free variables from becoming accidentally bound during substitution |
| Church encoding | "data as functions" | Representing data (booleans, numbers, pairs) purely as lambda terms |
| Redex | "reducible expression" | A subexpression of the form `(λx. e1) e2` that can be beta-reduced |

## Further Reading

- [Types and Programming Languages](https://mitpress.mit.edu/9780262162095/types-and-programming-languages/)
- [Lambda Calculus (Stanford Encyclopedia)](https://plato.stanford.edu/entries/lambda-calculus/)
- [An Introduction to Lambda Calculus](https://www.inf.fu-berlin.de/lehre/WS03/alpi/lambda.pdf)
