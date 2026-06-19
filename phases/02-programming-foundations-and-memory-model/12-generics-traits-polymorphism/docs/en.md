# Generics, Traits, Polymorphism

> Three flavors of polymorphism: parametric (generics), ad-hoc (traits/interfaces), and subtype (inheritance). Knowing all three keeps you fluent across C++, Rust, Java, Haskell, Go.

**Type:** Build
**Languages:** Rust
**Prerequisites:** Phase 02, Lessons 02, 10
**Time:** ~75 minutes

## Learning Objectives

- Distinguish parametric polymorphism (one definition reused for many types), ad-hoc polymorphism (different code per type via overloading / traits), and subtype polymorphism (inheritance).
- Read and write Rust generics with trait bounds; understand monomorphization (compile-time code generation) vs dyn dispatch (runtime vtable).
- Apply trait objects (`Box<dyn Trait>`) when you genuinely need a heterogeneous collection or stable ABI.
- Map the Rust patterns to their equivalents in C++ (templates + virtual), Java (generics + interfaces), Go (generics in 1.18+ + interfaces), Haskell (type classes).

## The Problem

Polymorphism = "many forms" of one operation. The motivating example:

```c
void swap_int(int *a, int *b)     { int t = *a; *a = *b; *b = t; }
void swap_double(double *a, double *b) { double t = *a; *a = *b; *b = t; }
```

Two functions, identical logic. C's "fix" is `void *` — type-erased and runtime-checked. Modern languages give you better tools.

## The Concept

### Parametric polymorphism (generics)

One definition, many types. The type is a parameter:

```rust
fn swap<T>(a: &mut T, b: &mut T) {
    std::mem::swap(a, b);
}
```

When you call `swap(&mut x, &mut y)` with `x: i32`, the compiler **monomorphizes** — generates a concrete `swap::<i32>` function. Another caller with `&mut f64` triggers a second specialized copy. Zero runtime cost; some compile-time and binary-size cost.

C++ templates work the same way. Haskell, OCaml, ML are similar but with type inference.

### Ad-hoc polymorphism (traits / interfaces / type classes)

Different code per type, dispatched by the type's *behavior*:

```rust
trait Greet {
    fn greet(&self) -> String;
}

struct English;
struct French;

impl Greet for English { fn greet(&self) -> String { "Hello".into() } }
impl Greet for French  { fn greet(&self) -> String { "Bonjour".into() } }

fn say_hi<T: Greet>(x: T) { println!("{}", x.greet()); }
```

| Concept | Rust | C++ | Java | Go (1.18+) | Haskell |
|---------|------|-----|------|------------|---------|
| Parametric | `<T>` | `template<typename T>` | `<T>` | `[T any]` | `forall a` |
| Ad-hoc | `trait` | `concept` (C++20) / virtual base | `interface` | `interface` | `type class` |
| Subtype | (no inheritance) | virtual + override | `extends` / `implements` | embed + interface | (none — typeclasses suffice) |

### Subtype polymorphism (inheritance)

Java, C++, Python, C# share this:

```java
class Animal { void speak() { ... } }
class Dog extends Animal { @Override void speak() { System.out.println("woof"); } }
```

Rust doesn't have inheritance. Its equivalent is `Box<dyn Trait>` — a trait object — but it's flat (no parent/child class chain).

### Static (monomorphization) vs dynamic (vtable) dispatch

Generics (monomorphized) have:

- Compile time: a copy per concrete type.
- Runtime: zero indirection; the compiler can inline freely.

Trait objects have:

- Compile time: one copy; smaller binary.
- Runtime: indirection through a vtable (~2 cycles per call); cannot inline.

```rust
fn area_static<S: Shape>(s: &S) -> f64 { s.area() }       // static
fn area_dyn(s: &dyn Shape) -> f64 { s.area() }            // dynamic
```

Use static by default. Switch to dyn when:
- You need a heterogeneous collection (`Vec<Box<dyn Shape>>`).
- You're crossing an ABI boundary that can't tolerate generic instantiations.

### Trait bounds and constraints

Trait bounds let you say "T must support these operations":

```rust
use std::fmt::Debug;
use std::cmp::PartialOrd;

fn max<T: PartialOrd + Debug>(a: T, b: T) -> T {
    if a > b { a } else { b }
}
```

C++ concepts (C++20):

```cpp
template <typename T>
requires std::totally_ordered<T>
T max(T a, T b);
```

Haskell:

```haskell
max :: Ord a => a -> a -> a
```

Same idea: declare requirements on the type parameter so the compiler can verify the body type-checks.

## Build It

Open `code/main.rs`.

### Step 1: Generic `swap`

Implement and call with `i32` and `String`. Two specialized copies generated under the hood.

### Step 2: Trait with generic bound

`trait Summable { fn zero() -> Self; fn plus(self, other: Self) -> Self; }` + `fn sum<T: Summable + Copy>(xs: &[T]) -> T`. Implement for `i32` and `f64`.

### Step 3: Trait object — heterogeneous collection

`Vec<Box<dyn Shape>>` holds Circles and Squares; `.iter().map(|s| s.area())` dispatches dynamically.

### Step 4: Compare static vs dyn dispatch

Both work for a homogeneous use. Dyn is required when types differ at runtime.

### Step 5: Operator overloading via traits

Implement `std::ops::Add` for a custom struct so `+` works on it.

## Use It

- **Generic containers** (`Vec<T>`, `HashMap<K, V>`): the type system's bread and butter.
- **Trait-based design** (Rust idiomatic style): a `Reader`/`Writer`/`Display`/`Debug`/`Serialize` per concept; types opt in.
- **Plugin / driver interfaces** (Go, Java): interfaces with multiple implementations.
- **Generic algorithms** (sorting, searching): one definition, every key type.

## Read the Source

- *The Rust Programming Language*, Chapter 10 (Generic Types, Traits, and Lifetimes).
- *Effective Modern C++* by Scott Meyers — Chapter 7 (concurrency) and Chapter 1 (type deduction) cover templates' subtler corners.
- *Type Classes in Haskell* — Wadler & Blott 1989 paper; the origin of ad-hoc polymorphism via class.

## Ship It

This lesson ships **`outputs/polymorphism-table.md`** — a compare-and-contrast of the three polymorphism flavors across five languages.

## Exercises

1. **Easy.** Implement a generic `pair<A, B>(a: A, b: B) -> (A, B)` that wraps two values into a tuple. Call with `(i32, String)` and `(f64, bool)`.
2. **Medium.** Define a `trait Summarizable { fn summary(&self) -> String; }`. Implement it for `Article` and `Tweet` structs. Write `print_summary<S: Summarizable>(s: &S)`.
3. **Hard.** Build a `Vec<Box<dyn Iterator<Item = i32>>>` — a heterogeneous collection of iterators that can each be polled until exhausted.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Generic | "Code with type parameter" | A function or type parameterized by a type T (or types); the compiler generates a specialized copy per concrete type |
| Trait | "Interface" | A set of method signatures that types can implement; defines ad-hoc polymorphism |
| Monomorphization | "Compile-time specialization" | The compiler generates a separate function body for each concrete type combination used |
| Trait object | "dyn dispatch" | A fat pointer (data ptr + vtable ptr) that hides the concrete type behind a trait; runtime indirection |
| Inheritance | "extends" | Subtype polymorphism via class hierarchy; absent in Rust |

## Further Reading

- *Programming Rust* by Blandy/Orendorff/Tindall — Chapter 11 (Traits and Generics).
- [Niko Matsakis on traits as constraints](http://smallcultfollowing.com/babysteps/blog/2017/05/02/spell-of-the-keyword/) — Rust traits compared to type classes.
- *On Understanding Types, Data Abstraction, and Polymorphism* — Cardelli & Wegner 1985; the foundational paper.
