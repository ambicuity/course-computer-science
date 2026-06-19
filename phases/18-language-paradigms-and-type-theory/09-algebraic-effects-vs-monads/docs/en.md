# Algebraic Effects vs Monads

> Both structure effects; the tradeoff is composition style and handler ergonomics.

**Type:** Learn
**Languages:** Haskell
**Prerequisites:** Phase 18 lessons 01-08
**Time:** ~75 minutes

## Learning Objectives

- Contrast monadic effect composition with algebraic effects/handlers.
- Understand modularity tradeoffs for adding new operations.
- Identify when monads are sufficient and when handlers simplify architecture.
- Build a small monadic interpreter baseline for comparison.

## The Problem

You're building an interpreter. It needs state (for the environment), errors (for type mismatches), and IO (for printing). In Haskell, you stack monad transformers:

```haskell
type EvalM = StateT Env (ExceptT EvalError IO)
```

This works. But adding a new effect (say, logging) means modifying `EvalM` everywhere. And the order of transformers matters: `StateT s (ExceptT e m)` behaves differently from `ExceptT e (StateT s m)` when an error occurs (does the state roll back or not?). The stack is rigid and the interaction between layers is implicit.

Algebraic effects propose a different architecture: you declare effect operations abstractly, then interpret them with handlers. Adding a new effect doesn't change existing code. The interaction between effects is explicit in the handler composition. This is closer to how we think about effects: "this function does state and errors" without specifying the implementation order.

The tradeoff: monads have decades of ecosystem, clear laws, and predictable performance. Algebraic effects are more modular but less mature, with ongoing research on efficient implementation.

## The Concept

### Monads for effects

A monad `m` gives you `return :: a -> m a` and `(>>=) :: m a -> (a -> m b) -> m b`. Effects are encoded by choosing the monad:

| Effect | Monad |
|--------|-------|
| Failure | `Maybe`, `Either e` |
| State | `State s` |
| Environment | `Reader r` |
| Output | `Writer w` |
| IO | `IO` |

Multiple effects: stack transformers.

```haskell
-- State + Error + IO
type AppM = StateT Int (ExceptT String IO)

runApp :: AppM a -> Int -> IO (Either String (a, Int))
runApp m s = runExceptT (runStateT m s)
```

### The problem with transformer stacks

```
StateT s (ExceptT e m)    -- error discards state changes
ExceptT e (StateT s m)    -- error preserves state changes
```

The semantics depend on ordering. This is a design choice, not an implementation detail, but it's easy to get wrong and hard to change later.

Adding a new effect requires changing the type:

```haskell
-- Before: State + Error
type AppM = StateT Int (ExceptT String IO)

-- After: State + Error + Logging
type AppM = StateT Int (ExceptT String (WriterT [String] IO))
-- Every function using AppM must be updated
```

### Algebraic effects

Algebraic effects separate declaration from interpretation:

```haskell
-- Declare effects (abstract operations)
data State s a where
  Get :: State s s
  Put :: s -> State s ()

data Error e a where
  Throw :: e -> Error e a

-- Write code using effects (no implementation details)
example :: Member (State Int) r => Eff r Int
example = do
  x <- send Get
  if x > 10
    then send (Throw "too big")
    else do
      send (Put (x + 1))
      send Get
```

### Handlers interpret effects

```haskell
-- Interpret State with a mutable variable
runState :: s -> Eff (State s ': r) a -> Eff r (a, s)
runState s (Return a) = return (a, s)
runState s (Effect Get k) = runState s (k s)
runState s (Effect (Put s') k) = runState s' (k ())

-- Interpret Error with Either
runError :: Eff (Error e ': r) a -> Eff r (Either e a)
runError (Return a) = return (Right a)
runError (Effect (Throw e) _) = return (Left e)
```

### The key difference

| Aspect | Monad transformers | Algebraic effects |
|--------|-------------------|-------------------|
| Adding effects | Change the type | Add a handler |
| Effect order | Matters (different semantics) | Handlers compose independently |
| Reinterpretation | Hard (need new transformers) | Easy (write a new handler) |
| Ecosystem | Mature (mtl, transformers) | Emerging (polysemy, fused-effects, eff) |
| Performance | Predictable | Depends on implementation strategy |

### Monad transformer analogy

Think of transformer stacks like function composition:

```
runReader . runState . runExcept :: ReaderT r (StateT s (ExceptT e m)) a
                                  -> r -> s -> m (Either e (a, s))
```

Each layer wraps the next. Effects are handled inside-out.

### Algebraic effect analogy

Think of handlers like try/catch for effects:

```haskell
handle stateHandler $
  handle errorHandler $
    myComputation
```

Each handler catches its effect and passes others through. The order of handlers can matter (just like catch nesting), but adding a new handler doesn't require changing the computation.

### The free monad connection

Algebraic effects are often implemented using free monads or similar structures:

```haskell
data Free f a = Pure a | Free (f (Free f a))

-- Free monad: effects are data constructors
-- Handlers are natural transformations f -> m
```

The `Eff` type in libraries like `polysemy` and `fused-effects` is an optimized free monad.

## Build It

### Step 1: Monadic evaluator baseline

```haskell
import Control.Monad.Trans.State
import Control.Monad.Trans.Except
import Control.Monad.Trans.Class (lift)

data Expr = Val Int | Add Expr Expr | Div Expr Expr
  deriving Show

type EvalM = ExceptT String (State Int)  -- State tracks operation count

eval :: Expr -> EvalM Int
eval (Val n) = do
  lift $ modify (+1)  -- count operations
  return n
eval (Add a b) = do
  x <- eval a
  y <- eval b
  lift $ modify (+1)
  return (x + y)
eval (Div a b) = do
  x <- eval a
  y <- eval b
  lift $ modify (+1)
  if y == 0
    then throwError "division by zero"
    else return (x `div` y)

runEval :: Expr -> (Either String Int, Int)
runEval e = runState (runExceptT (eval e)) 0

main :: IO ()
main = do
  print $ runEval (Add (Val 1) (Val 2))        -- (Right 3, 3)
  print $ runEval (Div (Val 10) (Val 0))        -- (Left "division by zero", 2)
  print $ runEval (Add (Val 1) (Div (Val 6) (Val 2)))  -- (Right 4, 5)
```

### Step 2: Adding logging (requires type change)

```haskell
import Control.Monad.Trans.Writer

-- Must change the type!
type EvalM2 = ExceptT String (StateT Int (Writer [String]))

eval2 :: Expr -> EvalM2 Int
eval2 (Val n) = do
  lift $ lift $ tell ["val " ++ show n]
  lift $ modify (+1)
  return n
-- ... rest of evaluator must be rewritten
```

### Step 3: Algebraic effect approach (conceptual)

```haskell
-- With polysemy or fused-effects:
-- data Error e m a where
--   Throw' :: e -> Error e m a
--
-- data State s m a where
--   Get' :: State s m s
--   Put' :: s -> State s m ()
--
-- data Log m a where
--   Log' :: String -> Log m ()
--
-- eval :: Members '[Error String, State Int, Log] r
--      => Expr -> Eff r Int
-- eval (Val n) = do
--   log' ("val " ++ show n)
--   modify (+1)
--   return n
--
-- -- Adding logging didn't change the eval function signature
-- -- just added a new Member constraint
```

### Step 4: Handler composition

```haskell
-- Handlers compose: inner effects are handled first
-- runLog :: Eff (Log ': r) a -> Eff r a
-- runState :: s -> Eff (State s ': r) a -> Eff r (a, s)
-- runError :: Eff (Error e ': r) a -> Eff r (Either e a)
--
-- result = runError . runState 0 . runLog $ eval expr
-- Order of handlers determines interaction semantics
```

## Use It

Production systems and languages exploring both paths:

- **Haskell mtl/transformers**: the mature monad transformer ecosystem. Used in production at Standard Chartered, Facebook (Sigma), and most Haskell shops.
- **polysemy / fused-effects / eff**: algebraic effect libraries for Haskell. Growing adoption for new projects.
- **Koka**: Microsoft Research language with native algebraic effects. Effects are part of the type system.
- **Eff**: research language specifically for algebraic effects.
- **OCaml 5**: added effect handlers to the language.
- **Unison**: uses abilities (algebraic effects) instead of monads.

The practical choice depends on team familiarity, tooling maturity, and whether the modular extension story matters for your domain.

## Read the Source

- [Effect Handlers in Scope](https://www.cs.kuleuven.be/~tom.schrijvers/Research/papers/ifl2015_post.pdf) — introduction to effect handlers.
- [Monad Transformers and Modular Interpreters](https://www.cs.princeton.edu/~dpw/papers/tlca-93.pdf) — the original transformer paper.
- [polysemy](https://hackage.haskell.org/package/polysemy) — algebraic effects for Haskell.
- [Koka language](https://koka-lang.github.io/) — native algebraic effects.

## Ship It

- `code/Main.hs`: monadic baseline evaluator.
- `outputs/README.md`: effects architecture checklist.

## Quiz

**Q1 (Pre).** What's the main problem with monad transformer stacks as effects grow?

- A) They're too slow.
- B) Adding a new effect requires changing the type everywhere; transformer order affects semantics.
- C) They can't express IO.
- D) Monad laws don't hold for transformers.

**Answer: B.** Each new effect adds a layer to the stack, changing the type. Every function using the stack must be updated. And the order of layers matters: `StateT s (ExceptT e m)` rolls back state on error, while `ExceptT e (StateT s m)` preserves it. This rigidity is what algebraic effects solve.

**Q2 (Pre).** In the algebraic effects model, what is a "handler"?

- A) An exception catcher.
- B) A function that gives concrete semantics to abstract effect operations.
- C) A type annotation.
- D) A monad transformer.

**Answer: B.** A handler interprets an effect by defining what each operation does. A `State` handler for `Get` returns the current value; for `Put` it updates it. The same effect can have different handlers (e.g., persistent state vs in-memory state).

**Q3 (Post).** How do algebraic effects improve modularity over monad transformers?

- A) They're faster.
- B) Adding a new effect doesn't require changing existing code; you just add a new handler.
- C) They don't use types.
- D) They eliminate the need for effect tracking.

**Answer: B.** With algebraic effects, a function declares `Members '[State Int, Error String] r =>` and adding `Log` only requires adding `Member Log r` to the constraint. Existing functions using `State` and `Error` don't change. With transformers, the entire type changes.

**Q4 (Post).** What's the relationship between free monads and algebraic effects?

- A) They're completely unrelated.
- B) Algebraic effects are often implemented using free monads or optimized variants.
- C) Free monads replace algebraic effects.
- D) Free monads are a type of algebraic effect.

**Answer: B.** The `Eff` type in libraries like `polysemy` is built on free monad ideas. Effects are data constructors in a free monad; handlers are natural transformations. Modern implementations use optimized representations (freer monads, fused effects) for better performance.

**Q5 (Post).** When would you choose monad transformers over algebraic effects?

- A) Always.
- B) When the team knows transformers well, the effect set is stable, and ecosystem maturity matters.
- C) Never.
- D) Only for IO-heavy code.

**Answer: B.** Monad transformers have decades of ecosystem (mtl, transformers, lens), predictable performance, and clear laws. If your effect set is small and stable, transformers are fine. Algebraic effects shine when you need to add effects frequently or reinterpret effects in different ways.

## Exercises

1. **Easy.** Add a state effect (counting operations) to the monadic evaluator. Show how the type changes.
2. **Medium.** Compare the transformer stack `StateT s (ExceptT e m)` with `ExceptT e (StateT s m)`. What happens to state when an error is thrown in each?
3. **Hard.** Implement a minimal free monad in Haskell and use it to build a small effect system with state and error handlers.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Monad | "effect wrapper" | Abstraction for sequencing computations with a shared context |
| Transformer stack | "monad composition" | Layered monad transformers combining multiple effects |
| Algebraic effect | "operation signature" | A declared effect operation, interpreted by handlers |
| Handler | "effect interpreter" | A function giving concrete semantics to abstract effect operations |
| Free monad | "reified computation" | A monad built from a functor, separating structure from interpretation |

## Further Reading

- [Effect Handlers Overview](https://www.eff-lang.org/)
- [Haskell mtl](https://hackage.haskell.org/package/mtl)
- [polysemy](https://hackage.haskell.org/package/polysemy)
- [Koka Language](https://koka-lang.github.io/)
- [OCaml 5 Effects](https://v2.ocaml.org/api/Effect.html)
