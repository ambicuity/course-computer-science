# Subtyping, Variance, Higher-Kinded Types

> Generic APIs are safe only when variance assumptions are explicit.

**Type:** Learn
**Languages:** Haskell, TypeScript
**Prerequisites:** Phase 18 lessons 01-05
**Time:** ~75 minutes

## Learning Objectives

- Explain covariance, contravariance, and invariance.
- Understand subtyping safety in producer/consumer positions.
- Describe higher-kinded type intuition (`* -> *`).
- Spot variance bugs in API design.

## The Problem

Java's arrays are covariant: `String[]` is a subtype of `Object[]`. This compiles:

```java
String[] strings = {"a", "b"};
Object[] objects = strings;  // covariant: String[] <: Object[]
objects[0] = 42;             // puts an Integer into a String array
String s = strings[0];       // ClassCastException at runtime!
```

The array type system lied. It said "this is an array of Objects" when really it was an array of Strings, and now you're pulling Integers out of it. The bug is silent at compile time and explodes at runtime.

This is a variance bug. Java's arrays are covariant (subtyping flows in the same direction) when they should be invariant (no subtyping) because they support both reading and writing. The fix is to make variance depend on position: read-only containers can be covariant, write-only sinks can be contravariant, and read-write containers must be invariant.

TypeScript has the same issue with function types and callbacks. Haskell sidesteps it entirely (no subtyping), but introduces higher-kinded types to abstract over type constructors. Understanding variance is understanding when it's safe to substitute one type for another in a generic context.

## The Concept

### Subtyping review

`S <: T` means "S is a subtype of T": anywhere a `T` is expected, an `S` is safe to use.

```
Dog <: Animal
String <: Object (in Java/TypeScript)
never <: T       (in TypeScript, bottom type)
T <: unknown     (in TypeScript, top type)
```

### Variance: how subtyping flows through constructors

Given `S <: T`, when is `F<S> <: F<T>`?

| Variance | Rule | Direction |
|----------|------|-----------|
| Covariant | `S <: T` implies `F<S> <: F<T>` | Same direction |
| Contravariant | `S <: T` implies `F<T> <: F<S>` | Reversed |
| Invariant | Neither direction | No substitution |

### Position determines variance

Functions are the canonical example:

```
type Consumer<T> = (arg: T) => void     -- T in input position
type Producer<T> = () => T               -- T in output position
```

If `Dog <: Animal`:

- **Producer (covariant):** A function producing `Dog` can substitute where `Animal` is expected. It gives you at least as specific a result.
- **Consumer (contravariant):** A function consuming `Animal` can substitute where `Dog` is expected. It can handle anything a `Dog` handler can, plus more.

```
Producer<Dog> <: Producer<Animal>     -- covariant: OK
Consumer<Animal> <: Consumer<Dog>     -- contravariant: OK
Consumer<Dog> <: Consumer<Animal>     -- WRONG: Dog handler can't handle all Animals
```

### TypeScript variance examples

```typescript
// Producer: covariant (output position)
type Producer<T> = () => T;
declare const dogProducer: Producer<Dog>;
const animalProducer: Producer<Animal> = dogProducer;  // OK

// Consumer: contravariant (input position)
type Consumer<T> = (x: T) => void;
declare const animalConsumer: Consumer<Animal>;
const dogConsumer: Consumer<Dog> = animalConsumer;  // OK

// Both: invariant (both positions)
type Box<T> = { get(): T; set(x: T): void };
// Box<Dog> is NOT <: Box<Animal> and NOT :> Box<Animal>
```

### Variance in practice

| Language | Arrays | Functions | Immutable collections |
|----------|--------|-----------|----------------------|
| Java | Covariant (unsound!) | Invariant (with wildcards for variance) | Covariant (`List<? extends T>`) |
| TypeScript | Invariant | Covariant in return, contravariant in params | Depends on declaration |
| Rust | N/A (no inheritance) | Trait bounds control | N/A |
| Haskell | N/A (no subtyping) | N/A | N/A |

### Higher-kinded types

Regular types classify values. Higher-kinded types classify type constructors.

```
Kind *         : types that classify values (Int, Bool, String)
Kind * -> *    : type constructors taking one type (List, Maybe, IO)
Kind * -> * -> *: taking two types (Either, Map)
```

In Haskell:
```haskell
class Functor f where
  fmap :: (a -> b) -> f a -> f b

-- f has kind * -> *
-- List, Maybe, IO are all * -> * and can be Functor instances
```

Without higher-kinded types, you'd need separate `Functor` classes for `List`, `Maybe`, `IO`, etc. HKTs let you write one class that works for any `* -> *` type.

### HKTs across languages

| Language | HKT support | How |
|----------|------------|-----|
| Haskell | Native | Kind system, typeclasses |
| Scala | Native | `F[_]` syntax |
| TypeScript | Emulated | Via encoding tricks (fp-ts style) |
| Rust | Limited | GATs (Generic Associated Types) approximate HKTs |
| Java | None | Must duplicate per container |

## Build It

### Step 1: TypeScript variance examples

```typescript
// Covariant producer
type Producer<T> = () => T;

function dogProducer(): Dog { return new Dog(); }
const animalProducer: Producer<Animal> = dogProducer;  // OK

// Contravariant consumer
type Consumer<T> = (x: T) => void;

function animalHandler(a: Animal): void { console.log(a.name); }
const dogHandler: Consumer<Dog> = animalHandler;  // OK

// Invariant box (both read and write)
interface MutableBox<T> {
  get(): T;
  set(v: T): void;
}

// MutableBox<Dog> is NOT assignable to MutableBox<Animal>
// TypeScript structurally allows it, but it's unsound
```

### Step 2: Covariance for read-only collections

```typescript
interface ReadonlyList<T> {
  readonly length: number;
  get(index: number): T;
  // no mutation methods
}

declare const dogs: ReadonlyList<Dog>;
const animals: ReadonlyList<Animal> = dogs;  // Safe: only reads
```

### Step 3: Haskell higher-kinded typeclass

```haskell
class Functor f where
  fmap :: (a -> b) -> f a -> f b

instance Functor Maybe where
  fmap _ Nothing  = Nothing
  fmap f (Just x) = Just (f x)

instance Functor [] where
  fmap = map

instance Functor (Either e) where
  fmap _ (Left e)  = Left e
  fmap f (Right x) = Right (f x)

-- Works for ANY * -> * type that supports mapping
double :: Functor f => f Int -> f Int
double = fmap (*2)

main :: IO ()
main = do
  print $ double (Just 5)     -- Just 10
  print $ double [1, 2, 3]    -- [2, 4, 6]
  print $ double (Right 7 :: Either String Int)  -- Right 14
```

### Step 4: The Traversable pattern (HKTs in action)

```haskell
class Functor t => Traversable t where
  traverse :: Applicative f => (a -> f b) -> t a -> f (t b)

instance Traversable [] where
  traverse _ []     = pure []
  traverse f (x:xs) = (:) <$> f x <*> traverse f xs

-- parseAll parses each string to Int, collecting results
parseAll :: [String] -> Maybe [Int]
parseAll = traverse readMaybe

main :: IO ()
main = do
  print $ parseAll ["1", "2", "3"]  -- Just [1, 2, 3]
  print $ parseAll ["1", "bad", "3"]  -- Nothing
```

## Use It

Variance and HKTs appear in:

- **Collections and callbacks:** Java's `List<? extends T>` for read-only access, `List<? super T>` for write-only. TypeScript's `ReadonlyArray<T>`.
- **Functional abstractions:** `Functor`, `Monad`, `Traversable` all require HKTs. Libraries like fp-ts (TypeScript), cats (Scala), and Haskell's base depend on them.
- **Framework extension interfaces:** Plugin systems often use HKTs to abstract over the container type.
- **Reactive streams:** `Observable<T>`, `Future<T>` are covariant in `T` because they only produce values.

Java's `Arrays.copyOf` uses `? extends T` (covariant read) and `? super T` (contravariant write) to handle variance safely. This is the PECS pattern: Producer Extends, Consumer Super.

## Read the Source

- TypeScript's `--strictFunctionTypes` flag: enables sound function variance.
- Scala's variance annotations: `+T` (covariant), `-T` (contravariant).
- Haskell's kind system: `Data.Kind` extension for kind signatures.
- Kotlin's `in`/`out` variance keywords.

## Ship It

- `code/main.ts`: variance examples.
- `code/Main.hs`: higher-kinded typeclass usage.
- `outputs/README.md`: variance review checklist.

## Quiz

**Q1 (Pre).** If `Dog <: Animal`, which is safe?

- A) `Array<Dog> <: Array<Animal>` always.
- B) `(Animal => void) <: (Dog => void)` (contravariant consumer).
- C) `(Dog => void) <: (Animal => void)` (covariant consumer).
- D) Mutable `Box<Dog> <: Box<Animal>`.

**Answer: B.** A function that handles any `Animal` can certainly handle a `Dog`. So `Consumer<Animal> <: Consumer<Dog>`: input positions are contravariant. A is unsound (arrays are mutable). C is wrong (a `Dog` handler can't handle all `Animal`s). D is unsound (mutable containers are invariant).

**Q2 (Pre).** What kind does `Maybe` have in Haskell?

- A) `*`
- B) `* -> *`
- C) `* -> * -> *`
- D) `(* -> *) -> *`

**Answer: B.** `Maybe` takes one type argument: `Maybe Int`, `Maybe Bool`, etc. Its kind is `* -> *`: a type constructor waiting for one type to become a concrete type (`*`).

**Q3 (Post).** Why are Java arrays covariant but unsound?

- A) Covariance allows writing a supertype into a subtype array.
- B) Arrays should be contravariant.
- C) Java arrays should be immutable.
- D) Covariance is always unsound.

**Answer: A.** `String[] <: Object[]` lets you assign a `String[]` to `Object[]`, then write an `Integer` into it (since `Integer <: Object`). Reading from the original `String[]` reference now yields an `Integer`, causing a `ClassCastException`. The fix: make arrays invariant, or use `List<? extends T>` for read-only access.

**Q4 (Post).** What do higher-kinded types enable that regular generics don't?

- A) Faster compilation.
- B) Abstracting over type constructors, writing one `Functor` class for all `* -> *` types.
- C) Runtime type inspection.
- D) Implicit type conversion.

**Answer: B.** Without HKTs, you need separate `FunctorList`, `FunctorMaybe`, `FunctorIO` classes. With HKTs, `class Functor f where fmap :: (a -> b) -> f a -> f b` works for any type constructor `f` of kind `* -> *`. This is the foundation of the Haskell typeclass hierarchy.

**Q5 (Post).** What is the PECS rule in Java?

- A) Producer Extends, Consumer Super: use `? extends T` for read-only, `? super T` for write-only.
- B) A memory management pattern.
- C) A concurrency pattern.
- D) An error handling pattern.

**Answer: A.** PECS (from Effective Java): when a generic type produces `T` values, use `? extends T` (covariant). When it consumes `T` values, use `? super T` (contravariant). When it does both, use exact type (invariant). This safely handles variance in Java's invariant generics.

## Exercises

1. **Easy.** For each of these positions, determine the variance: function return type, function parameter type, mutable field type, read-only property type.
2. **Medium.** Refactor an unsafe generic API (e.g., a mutable list used as read-only) into a safe producer/consumer split using TypeScript's `ReadonlyArray<T>`.
3. **Hard.** Implement a `Bifunctor` typeclass in Haskell (kind `* -> * -> *` that supports mapping over both type parameters). Provide instances for `Either` and pairs.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Covariance | "read-safe widening" | Preserves subtyping direction in output/read positions |
| Contravariance | "input-safe narrowing" | Reverses subtyping direction in input positions |
| Invariance | "exact match" | No safe subtype substitution in either direction |
| Higher-kinded type | "type constructor level" | A type parameter that itself expects type parameters (kind `* -> *`) |
| PECS | "extends for get, super for put" | Java mnemonic for safe variance with wildcards |

## Further Reading

- [TypeScript Handbook: Variance](https://www.typescriptlang.org/docs/handbook/2/generics.html)
- [Haskell Kinds](https://wiki.haskell.org/Kind)
- [Effective Java, Item 31: PECS](https://www.oreilly.com/library/view/effective-java/9780134686097/)
- [fp-ts: HKT encoding in TypeScript](https://gcanti.github.io/fp-ts/)
