# Polymorphism — System F, ML, Bounded Quantification

> Polymorphism lets one abstraction safely serve many concrete types.

**Type:** Learn
**Languages:** Haskell
**Prerequisites:** Phase 18 lessons 01-03
**Time:** ~75 minutes

## Learning Objectives

- Distinguish parametric and ad-hoc polymorphism.
- Understand rank-1 HM polymorphism vs explicit System F ideas.
- Explain bounded quantification motivation.
- Identify where polymorphism improves reuse and where it obscures intent.

## The Problem

Without polymorphism, every function works on exactly one type. Want to reverse a list of integers? Write `reverseInts`. Want to reverse a list of strings? Write `reverseStrings`. Want to reverse a list of user records? Write `reverseUsers`. The logic is identical, duplicated across three functions, and every new type demands another copy.

This is the duplication problem that polymorphism solves. But poorly constrained polymorphism creates the opposite problem: an API that accepts "anything" tells the caller nothing. A function `process :: a -> a` could be the identity function, could loop forever, could crash. Without constraints, parametric polymorphism gives maximum generality but zero information about what the function actually does.

The design space runs from fully parametric (System F: works for all types uniformly) through ad-hoc (typeclasses: behavior selected per type) to bounded (constraints limiting which types are accepted). Each point on this spectrum trades generality for expressiveness.

## The Concept

### Parametric polymorphism

One implementation, works for all types. The type variable is universally quantified:

```
id : ∀a. a → a
id = λx. x

const : ∀a. ∀b. a → b → a
const = λx. λy. x
```

The key property: a parametric function can't inspect its type argument. `id` can't check whether `a` is `Int` and do arithmetic. It must work uniformly. This gives strong guarantees: `∀a. a → a` can only be the identity function (or diverge).

### System F (explicit polymorphism)

System F makes quantification explicit in the term language:

```
e ::= x | λx: T. e | e1 e2     -- STLC terms
    | Λα. e                      -- type abstraction
    | e [T]                      -- type application
```

`Λα. e` abstracts over a type. `e [T]` applies a type argument.

```
id : ∀a. a → a
id = Λa. λx: a. x

id [Int] 5   →β   (λx: Int. x) 5   →β   5
```

System F is powerful enough to encode pairs, booleans, natural numbers, and existential types. But writing explicit type applications everywhere is verbose. Real languages infer them.

### Hindley-Milner (rank-1 polymorphism)

ML-family languages (Haskell, OCaml, SML) use Hindley-Milner, which restricts System F to rank-1: `∀` can only appear at the outermost level of a type.

```
-- Rank-1 (HM allows):
id :: forall a. a -> a

-- Rank-2 (HM forbids):
applyToAll :: (forall a. a -> a) -> [Int] -> [Int]
--          ^^^^^^^^^^^^^^^^^^^^
--          forall is nested inside an argument type
```

HM's restriction enables complete type inference: the compiler can deduce all type annotations. Rank-2 and higher require explicit annotations.

### Ad-hoc polymorphism (typeclasses)

Typeclasses select behavior by type:

```haskell
class Show a where
  show :: a -> String

instance Show Int where
  show = ...  -- Int-specific rendering

instance Show Bool where
  show = ...  -- Bool-specific rendering
```

Unlike parametric polymorphism, `show` inspects the type and does something different for each. This is ad-hoc: each instance is a separate implementation.

### The spectrum

| Kind | Behavior | Inference | Example |
|------|----------|-----------|---------|
| Parametric | Uniform for all types | Full (HM) | `id`, `length`, `map` |
| Ad-hoc | Selected per type | With annotations | `show`, `==`, `+` |
| Bounded | Uniform but constrained | Partial | `sort` requires `Ord` |

### Bounded quantification

Bounded quantification says "for all types `a` that satisfy constraint `C`":

```
sort :: ∀a. Ord a => [a] -> [a]
```

This is more informative than pure parametric (`∀a. [a] -> [a]`) because it tells you elements must be orderable. It's more general than ad-hoc because the same code works for any `Ord` type.

```
-- TypeScript bounded generics:
function max<T extends Comparable<T>>(a: T, b: T): T {
  return a.compareTo(b) >= 0 ? a : b;
}

-- Java bounded wildcards:
public void sort(List<? extends Comparable<?>> list) { ... }
```

### Where polymorphism helps and hurts

| Use case | Polymorphism style | Why |
|----------|-------------------|-----|
| Container operations (`map`, `filter`) | Parametric | Logic is type-independent |
| Serialization (`toJSON`) | Ad-hoc | Each type needs custom encoding |
| Sorting | Bounded | Needs ordering, but logic is uniform |
| Generic maximum | Bounded | Needs comparison, returns same type |
| Type-safe casts | Ad-hoc | Behavior depends on runtime type |

Too much parametric polymorphism: `process :: a -> b` tells you nothing. Too little: you duplicate code per type. The sweet spot is parametric where logic is uniform, bounded where you need minimum capabilities, ad-hoc where behavior genuinely differs.

## Build It

### Step 1: Parametric identity and map (Haskell)

```haskell
-- Parametric: works for ANY type
id' :: a -> a
id' x = x

-- Parametric: transforms any list
map' :: (a -> b) -> [a] -> [b]
map' _ []     = []
map' f (x:xs) = f x : map' f xs

-- The type constrains what the function CAN do
-- id' :: a -> a can ONLY return its argument
-- map' can only apply f and cons, nothing else
```

### Step 2: Ad-hoc behavior with Show

```haskell
class Pretty a where
  pretty :: a -> String

instance Pretty Int where
  pretty n = "Int(" ++ show n ++ ")"

instance Pretty Bool where
  pretty True  = "yes"
  pretty False = "no"

instance Pretty a => Pretty [a] where
  pretty xs = "[" ++ concatMap pretty xs ++ "]"

main :: IO ()
main = do
  putStrLn $ pretty (42 :: Int)        -- "Int(42)"
  putStrLn $ pretty True               -- "yes"
  putStrLn $ pretty [1, 2, 3 :: Int]   -- "[Int(1)Int(2)Int(3)]"
```

### Step 3: Bounded quantification

```haskell
-- Requires Ord constraint
minimum' :: Ord a => [a] -> a
minimum' [x]    = x
minimum' (x:xs) = min x (minimum' xs)

-- Requires both Eq and Show
debug :: (Eq a, Show a) => a -> a -> String
debug expected actual
  | expected == actual = "OK: " ++ show actual
  | otherwise = "FAIL: expected " ++ show expected
                ++ ", got " ++ show actual

main :: IO ()
main = do
  print $ minimum' [3, 1, 4, 1, 5]  -- 1
  putStrLn $ debug (3 :: Int) 3       -- "OK: 3"
  putStrLn $ debug (3 :: Int) 4       -- "FAIL: expected 3, got 4"
```

### Step 4: System F encoding (conceptual Haskell)

```haskell
{-# LANGUAGE RankNTypes #-}

-- System F-style: explicit type application via newtype
newtype Id = Id { runId :: forall a. a -> a }

-- The only inhabitant of `forall a. a -> a` is the identity
identity :: Id
identity = Id (\x -> x)

-- System F pair encoding
newtype Pair a b = Pair { runPair :: forall r. (a -> b -> r) -> r }

mkPair :: a -> b -> Pair a b
mkPair x y = Pair (\f -> f x y)

fst' :: Pair a b -> a
fst' p = runPair p (\x _ -> x)

snd' :: Pair a b -> b
snd' p = runPair p (\_ y -> y)
```

## Use It

Modern languages blend these concepts:

- **Haskell**: parametric + typeclasses. Functions like `map` are parametric; `sort` requires `Ord` (bounded); `show` is ad-hoc.
- **TypeScript**: generics + bounded constraints (`extends`). Less inference than HM, more explicit.
- **JVM/.NET**: generics with bounds and variance rules. Java's `? extends T` is bounded quantification.
- **Rust**: traits are ad-hoc polymorphism. Generic functions with trait bounds are bounded quantification.
- **C++**: templates are ad-hoc (specialization) and parametric (non-specialized). Concepts add bounded quantification.

GHC's type inference handles all three: it infers parametric types, resolves typeclass instances, and checks bounds. The interaction between these is where complexity hides.

## Read the Source

- *Types and Programming Languages* (Pierce), Chapters 15 and 23: system F and bounded quantification.
- GHC docs on type inference and constraints: [GHC User's Guide](https://downloads.haskell.org/~ghc/latest/docs/users_guide/).
- *Advanced Topics in Types and Programming Languages* (Pierce), Chapter 1: bounded quantification.

## Ship It

- `code/Main.hs`: concise polymorphism examples.
- `outputs/README.md`: polymorphism design checklist.

## Quiz

**Q1 (Pre).** What does `∀a. a → a` tell you about a function's behavior?

- A) It takes an integer and returns an integer.
- B) It must work uniformly for all types, so it can only return its argument (or diverge).
- C) It can inspect the type argument to decide what to do.
- D) It requires the type to implement `Show`.

**Answer: B.** Parametricity means the function can't inspect `a`. Since it knows nothing about `a`, the only thing it can produce of type `a` is the argument it received. This is the "free theorem": `∀a. a → a` is necessarily the identity (or bottom).

**Q2 (Pre).** What's the key difference between parametric and ad-hoc polymorphism?

- A) Parametric is faster.
- B) Parametric uses one implementation for all types; ad-hoc selects behavior per type.
- C) Ad-hoc doesn't require type annotations.
- D) Parametric only works with numbers.

**Answer: B.** Parametric polymorphism uses a single, type-uniform implementation. Ad-hoc polymorphism (typeclasses, interfaces) dispatches to type-specific implementations. `length` is parametric (same logic for any list); `show` is ad-hoc (different rendering per type).

**Q3 (Post).** Why does Hindley-Milner restrict polymorphism to rank-1?

- A) Rank-2 types are unsound.
- B) Rank-1 restriction enables complete type inference.
- C) Rank-2 types can't be represented in memory.
- D) Rank-1 is more expressive.

**Answer: B.** With rank-1 types, Damas-Hindley-Milner's Algorithm W can infer all types without annotations. Rank-2 and higher types lose this property: the compiler can't always deduce where type abstractions and applications go. The restriction trades expressiveness for inference.

**Q4 (Post).** In `sort :: Ord a => [a] -> [a]`, what does `Ord a =>` represent?

- A) A runtime type check.
- B) Bounded quantification: the function works for all types `a` that have an ordering.
- C) An ad-hoc dispatch to a sorting algorithm.
- D) A constraint that `a` must be a numeric type.

**Answer: B.** `Ord a =>` is a constraint bounding the universal quantifier. It says: for all types `a` that implement `Ord`, this function works. The same sorting code handles `Int`, `String`, or any orderable type. The constraint ensures comparison operations are available.

**Q5 (Post).** Which style of polymorphism is `map :: (a -> b) -> [a] -> [b]`?

- A) Ad-hoc, because it dispatches per list element type.
- B) Bounded, because `a` must satisfy some constraint.
- C) Parametric, because it works uniformly for all types.
- D) None, because it uses a type variable.

**Answer: C.** `map` is fully parametric: the same implementation applies regardless of what `a` and `b` are. It doesn't inspect the types, doesn't need constraints, and works for any function `a -> b`. The type variables are universally quantified with no bounds.

## Exercises

1. **Easy.** Write a polymorphic `compose :: (b -> c) -> (a -> b) -> (a -> c)` and verify its type. Apply it to two concrete functions.
2. **Medium.** Rewrite a duplicated function (one for `[Int]`, one for `[String]`) as a single parametric function. Show the type the compiler infers.
3. **Hard.** Write a function with a rank-2 type using `{-# LANGUAGE RankNTypes #-}`. Explain why it needs an explicit annotation.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Parametric polymorphism | "generic code" | Uniform behavior across all type instantiations |
| Ad-hoc polymorphism | "overloading" | Behavior selected by type-specific implementation (typeclasses/interfaces) |
| Quantification | "forall" | Type-level universal binding: `∀a. T` |
| Bound | "constraint" | Restriction on allowed type instantiations |
| Rank-1 | "normal generics" | `∀` only at the outermost level of a type |

## Further Reading

- [Types and Programming Languages](https://mitpress.mit.edu/9780262162095/types-and-programming-languages/)
- [GHC Typeclasses](https://downloads.haskell.org/~ghc/latest/docs/users_guide/exts/typeclasses.html)
- [Theorems for Free!](https://www.cs.tufts.edu/~nr/cs257/archive/philip-wadler/theorems-for-free.pdf)
