# Functional Programming Deep — Haskell Idioms

> Idiomatic FP is about compositional clarity, not clever syntax.

**Type:** Learn
**Languages:** Haskell
**Prerequisites:** Phase 18 lessons 01-09
**Time:** ~75 minutes

## Learning Objectives

- Apply common Haskell idioms (`map`, `fold`, composition, point-free selectively).
- Recognize when explicit style is clearer than point-free density.
- Use algebraic data types and pattern matching effectively.
- Structure pure core + effectful shell designs.

## The Problem

New Haskell programmers write one of two kinds of code. The first: explicit recursion everywhere, reimplementing `map` and `filter` by hand, threading state through function arguments. The second: point-free chains so dense that nobody, including the author, can read them a week later.

Both miss the sweet spot: idiomatic Haskell that uses standard combinators, keeps the data flow visible, separates pure logic from effects, and leverages the type system to document intent. The goal isn't to be clever. It's to be clear.

Consider parsing a CSV file, computing statistics, and writing a report. The non-idiomatic version mixes file IO with parsing with computation. The idiomatic version has three layers: read bytes (IO), parse into records (pure), compute statistics (pure). Each layer is independently testable, composable, and the types document the data flow.

## The Concept

### Transform, don't iterate

The single most important Haskell idiom: use `map`, `filter`, `fold` instead of explicit loops.

```haskell
-- Explicit recursion (non-idiomatic)
sumSquares :: [Int] -> Int
sumSquares [] = 0
sumSquares (x:xs) = x * x + sumSquares xs

-- Idiomatic: pipeline of transformations
sumSquares :: [Int] -> Int
sumSquares = sum . map (^2) . filter even
```

The pipeline reads left to right: keep evens, square them, sum. Each piece is reusable.

### Fold as the universal iterator

Any function that consumes a list and produces a value can be written as a fold:

```haskell
-- foldr: replace (:) with f, [] with z
-- [a, b, c]  →  f a (f b (f c z))

sum' :: [Int] -> Int
sum' = foldr (+) 0

length' :: [a] -> Int
length' = foldr (\_ acc -> acc + 1) 0

and' :: [Bool] -> Bool
and' = foldr (&&) True

-- foldl' (strict left fold) for accumulation
reverse' :: [a] -> [a]
reverse' = foldl' (flip (:)) []
```

### ADTs and exhaustive pattern matching

Algebraic data types model domain states. Pattern matching makes invalid states unrepresentable:

```haskell
data PaymentStatus
  = Pending
  | Authorized Amount
  | Captured Amount TxId
  | Refunded Amount TxId
  | Failed String

processPayment :: PaymentStatus -> String
processPayment Pending          = "waiting for authorization"
processPayment (Authorized amt) = "authorized: " ++ show amt
processPayment (Captured amt _) = "captured: " ++ show amt
processPayment (Refunded amt _) = "refunded: " ++ show amt
processPayment (Failed reason)  = "failed: " ++ reason
```

The compiler warns if you miss a case. Adding a new constructor forces you to handle it everywhere.

### Composition and point-free (selectively)

```haskell
-- Point-free: clear when the pipeline is short
process = filter even . map (*2) . take 10

-- Point-ful: clearer when logic is complex
process xs =
  let doubled = map (*2) xs
      evens = filter even doubled
  in take 10 evens
```

Rule of thumb: point-free is fine for 2-3 composed functions. Beyond that, name intermediate results.

### Pure core, effectful shell

Structure programs as:
1. Pure functions that transform data (testable, composable).
2. A thin effectful shell that handles IO, reads config, writes output.

```haskell
-- Pure core: parse and compute
data Record = Record { name :: String, score :: Double }

parseLine :: String -> Maybe Record
parseLine line = case splitOn ',' line of
  [n, s] -> Just (Record n (read s))
  _      -> Nothing

averageScore :: [Record] -> Double
averageScore [] = 0
averageScore rs = sum (map score rs) / fromIntegral (length rs)

topPerformers :: Double -> [Record] -> [Record]
topPerformers threshold = filter (\r -> score r >= threshold)

-- Effectful shell: IO only here
main :: IO ()
main = do
  contents <- readFile "data.csv"
  let lines_ = lines contents
  let records = mapMaybe parseLine lines_
  let avg = averageScore records
  putStrLn $ "Average: " ++ show avg
  putStrLn $ "Top performers:"
  mapM_ (\r -> putStrLn $ "  " ++ name r ++ ": " ++ show (score r))
        (topPerformers avg records)
```

### Common combinators

| Combinator | Type | What it does |
|-----------|------|-------------|
| `map` | `(a -> b) -> [a] -> [b]` | Transform each element |
| `filter` | `(a -> Bool) -> [a] -> [a]` | Keep matching elements |
| `foldr` | `(a -> b -> b) -> b -> [a] -> b` | Right fold |
| `foldl'` | `(b -> a -> b) -> b -> [a] -> b` | Strict left fold |
| `concatMap` | `(a -> [b]) -> [a] -> [b]` | Map then flatten |
| `mapMaybe` | `(a -> Maybe b) -> [a] -> [b]` | Map, dropping failures |
| `zipWith` | `(a -> b -> c) -> [a] -> [b] -> [c]` | Combine two lists |
| `(&)` | `a -> (a -> b) -> b` | Reverse application |
| `(<&>)` | `Functor f => f a -> (a -> b) -> f b` | Flipped fmap |

### The `&` operator for readable pipelines

```haskell
import Data.Function ((&))

result = [1..100]
  & filter even
  & map (^2)
  & take 10
  & sum
-- Reads top to bottom like a Unix pipe
```

### Record syntax and lenses (preview)

```haskell
data Config = Config
  { cfgHost :: String
  , cfgPort :: Int
  , cfgDebug :: Bool
  }

-- Update syntax (verbose for deep nesting)
setDebug :: Config -> Config
setDebug cfg = cfg { cfgDebug = True }

-- Lenses (lesson 10+ in other courses) solve deep update
-- cfg & debug .~ True
```

## Build It

### Step 1: Parse and score records

```haskell
import Data.List (intercalate)
import Data.Maybe (mapMaybe)

data Student = Student
  { studentName  :: String
  , studentScore :: Double
  } deriving Show

parseStudent :: String -> Maybe Student
parseStudent line = case break (== ',') line of
  (name, ',' : scoreStr) ->
    case reads scoreStr of
      [(score, "")] -> Just (Student name score)
      _             -> Nothing
  _ -> Nothing

-- Pure computation
classAverage :: [Student] -> Double
classAverage [] = 0
classAverage ss = sum (map studentScore ss) / fromIntegral (length ss)

grade :: Double -> String
grade s
  | s >= 90   = "A"
  | s >= 80   = "B"
  | s >= 70   = "C"
  | s >= 60   = "D"
  | otherwise  = "F"

report :: [Student] -> String
report ss = unlines $
  ("Average: " ++ show (classAverage ss)) :
  map (\s -> studentName s ++ ": " ++ grade (studentScore s)) ss
```

### Step 2: Use folds for aggregation

```haskell
-- Histogram of grades
gradeHistogram :: [Student] -> [(String, Int)]
gradeHistogram = foldl' acc []
  where
    acc hist s = let g = grade (studentScore s)
                 in insertWith (+) g 1 hist

    insertWith f k v [] = [(k, v)]
    insertWith f k v ((k', v'):rest)
      | k == k'   = (k', f v' v) : rest
      | otherwise = (k', v') : insertWith f k v rest
```

### Step 3: Keep main thin

```haskell
main :: IO ()
main = do
  contents <- readFile "students.csv"
  let students = mapMaybe parseStudent (lines contents)
  putStr $ report students
```

### Step 4: Property testing of pure functions

```haskell
import Test.QuickCheck

-- Property: average of a non-empty list is between min and max
prop_average_bounds :: [Double] -> Property
prop_average_bounds xs =
  not (null xs) ==>
    let avg = classAverage (map (Student "test") xs)
    in avg >= minimum xs && avg <= maximum xs
```

## Use It

These idioms map directly to production:

- **Data pipelines**: Parse → transform → aggregate → output. Each stage is a pure function, composable with `.` or `&`.
- **Compiler passes**: Lex → parse → type-check → optimize → codegen. Each pass is a pure transformation.
- **Configuration analysis**: Read config (IO), validate (pure), report (IO). Pure core is testable.
- **Web handlers**: Parse request (pure-ish), process (pure), format response (pure), send (IO). Frameworks like Servant enforce this structure.

GHC's own codebase follows these patterns: the typechecker is pure, the driver is effectful. Libraries like `aeson` (JSON), `optparse-applicative` (CLI), and `servant` (web) are built on composition and ADTs.

## Read the Source

- *Learn You a Haskell for Great Good!* — accessible introduction to idioms.
- *Real World Haskell* — practical Haskell with real-world patterns.
- *Haskell Programming from First Principles* — thorough treatment of idioms.
- GHC and Cabal codebases for idiomatic patterns in large projects.

## Ship It

- `code/Main.hs`: small data-transform pipeline.
- `outputs/README.md`: functional-style review checklist.

## Quiz

**Q1 (Pre).** What's the idiomatic Haskell way to transform every element of a list?

- A) Write explicit recursion.
- B) Use `map` with a function.
- C) Use a `for` loop.
- D) Mutate the list in place.

**Answer: B.** `map f xs` applies `f` to each element. It's the standard combinator for element-wise transformation. Explicit recursion reimplements the pattern; Haskell has no `for` loops or in-place mutation.

**Q2 (Pre).** What does "pure core, effectful shell" mean?

- A) All code is pure.
- B) Business logic is pure functions; IO, config, and output are in a thin outer layer.
- C) Effects are hidden from the type system.
- D) The shell is a Unix shell.

**Answer: B.** The pure core contains parsing, validation, computation, and transformation. The effectful shell reads input, calls the core, and writes output. This separation makes the core testable without IO mocking.

**Q3 (Post).** When is point-free style a bad idea?

- A) Always.
- B) When the composed chain is long enough that the data flow becomes hard to follow.
- C) Never; shorter code is always better.
- D) Only in library code.

**Answer: B.** Point-free is clear for 2-3 function compositions: `sum . map (^2) . filter even`. Beyond that, naming intermediate results (`let xs = ...`) improves readability. The goal is clarity, not brevity.

**Q4 (Post).** What does `foldl'` give you that `foldl` doesn't?

- A) Right-to-left traversal.
- B) Strict evaluation, preventing space leaks from accumulated thunks.
- C) The ability to fold over trees.
- D) A different result.

**Answer: B.** `foldl` builds a chain of thunks: `(((0 + 1) + 2) + 3)`. Each addition is deferred. For large lists, this causes a stack overflow or excessive memory. `foldl'` evaluates the accumulator at each step, keeping memory constant.

**Q5 (Post).** Why use ADTs instead of strings or integers to represent domain states?

- A) ADTs are faster.
- B) ADTs make invalid states unrepresentable; the compiler enforces exhaustive handling.
- C) Strings are more flexible.
- D) ADTs use less memory.

**Answer: B.** A `PaymentStatus` ADT with `Pending | Authorized Amount | Captured Amount TxId` can't represent "captured without a transaction ID." Pattern matching on ADTs forces you to handle every case. Adding a new constructor produces compile errors at every unhandled site.

## Exercises

1. **Easy.** Replace the following explicit recursion with `foldr`: `product [] = 1; product (x:xs) = x * product xs`.
2. **Medium.** Refactor a function that mixes IO and pure logic into pure core + effectful shell. Write QuickCheck properties for the pure part.
3. **Hard.** Implement a small CSV parser using `mapMaybe` and `break`. Handle quoted fields (fields containing commas).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Point-free | "short code" | Composition style that omits explicit arguments |
| Fold | "reducer" | Structural recursion over a data structure, replacing constructors |
| ADT | "enum+payload" | Closed set of constructors modeling domain states |
| Pure function | "no side effects" | Deterministic mapping from inputs to outputs with no observable side effects |
| Pipeline | "chain" | Sequence of composed transformations |

## Further Reading

- [Learn You a Haskell](http://learnyouahaskell.com/)
- [Real World Haskell](http://book.realworldhaskell.org/)
- [Haskell Programming from First Principles](http://haskellbook.com/)
- [What I Wish I Knew When Learning Haskell](http://stephendiehl.com/what-i-wish-i-knew-when-learning-haskell.html)
