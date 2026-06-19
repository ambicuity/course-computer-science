# Values, Types, Variables, Scope

> A value lives in memory and has a *type*; a variable is a name with a *scope*. Get the four ideas straight and most language features become obvious.

**Type:** Learn
**Languages:** C, Rust
**Prerequisites:** Phase 02, Lesson 01
**Time:** ~45 minutes

## Learning Objectives

- Distinguish **value** (a bit pattern in memory), **type** (an interpretation of that pattern), **variable** (a name bound to a value), and **scope** (the region of code where a binding is visible).
- Walk through C's primitive types (signed / unsigned integers of various widths, floats, pointers, `bool`, `char`) and read their sizes via `sizeof`.
- Read Rust's primitive types (`i32`, `u64`, `bool`, `char`, fixed-size arrays, slices, `&T` references); distinguish copy semantics from move semantics.
- Apply the four scope rules — block, function, file (static), program (extern) — and recognize variable shadowing.

## The Problem

Languages talk about "variables" loosely. In math, a variable is a placeholder. In imperative languages, a variable is a name for a *mutable storage location*. In functional languages, a variable is a *name for an immutable value*. Confusing these makes simple code unreadable:

```c
int x = 5;          /* x is a name for a storage location holding 5 */
x = 7;              /* same location, new value */
```

vs

```rust
let x = 5;          // x is bound to the value 5; immutable by default
let x = 7;          // shadows: new binding to a *new* location with value 7
```

Same syntax, different meaning. This lesson disentangles values, types, variables, and scope across the two language traditions C lives in (mutable storage) and Rust lives in (immutable-by-default bindings + ownership).

## The Concept

### Values and bit patterns

A **value** is a bit pattern in memory. The bits themselves are meaningless until interpreted by a **type**. The pattern `0x42` (= 66) means:

- The integer 66, if interpreted as an unsigned 8-bit int.
- The character 'B', if interpreted as an ASCII char.
- Part of a larger value (e.g., one byte of a 32-bit int) if pointed at by a 32-bit pointer.

### Types

A **type** tells the compiler:
- How many bytes the value occupies.
- How to interpret those bytes (signed vs unsigned int, IEEE 754 float, pointer, struct layout).
- What operations are legal (you can add ints, you can't subtract pointers of different types, you can index an array).

**C primitive types** (sizes are typical on a 64-bit Linux/macOS):

| Type | Bytes | Range |
|------|-------|-------|
| `char` (signed) | 1 | -128 .. 127 |
| `unsigned char` | 1 | 0 .. 255 |
| `short` | 2 | -32768 .. 32767 |
| `int` | 4 | ~-2.1 × 10⁹ .. 2.1 × 10⁹ |
| `long` | 8 (Linux/mac), 4 (Windows) | platform-dependent |
| `long long` | 8 | -2⁶³ .. 2⁶³ - 1 |
| `float` | 4 | IEEE 754 single |
| `double` | 8 | IEEE 754 double |
| `void *` | 8 (64-bit) | any object pointer |

Prefer the **fixed-width** versions from `<stdint.h>`: `int8_t`, `int16_t`, `int32_t`, `int64_t`, and unsigned counterparts. Same on every platform.

**Rust primitive types** are fixed-width by spec:

| Type | Bytes | Notes |
|------|-------|-------|
| `i8`, `i16`, `i32`, `i64`, `i128` | 1, 2, 4, 8, 16 | signed |
| `u8`, `u16`, `u32`, `u64`, `u128` | same | unsigned |
| `isize`, `usize` | pointer-sized | array indices |
| `f32`, `f64` | 4, 8 | IEEE 754 |
| `bool` | 1 | `true` / `false` |
| `char` | 4 | a *Unicode scalar value*, NOT a byte |
| `&T`, `&mut T` | pointer-sized | borrow / mut borrow |

### Variables: names → storage

A **variable** is a binding from a name to either a storage location (C, mutable Rust) or a value (default Rust). Three useful properties:

| Property | C | Rust (default) |
|----------|---|----------------|
| Mutable? | Yes (unless `const`) | No (unless `let mut`) |
| Can be reassigned? | Yes | No (need `let mut`); but `let x = ...; let x = ...;` *shadows* |
| Lifetime | Block / function / program | Bound by scope; ownership rules enforce no use-after-free |

### Scope

A **scope** is the region of source code where a name is visible. Four levels in C:

1. **Block scope**: `{ int x = ...; }` — visible inside the braces only.
2. **Function scope**: rarely-used; only labels (`goto target:`) have it.
3. **File (translation unit) scope**: `static int g = ...;` — visible inside this `.c` file only.
4. **Program scope**: non-`static` globals — visible to every linked translation unit (after `extern` declarations).

Rust has block scope, module scope, and crate scope; `pub` opens a module's items to others.

### Shadowing

Same name redeclared in a nested scope **shadows** the outer:

```c
int x = 1;
{
    int x = 2;        /* new variable, shadows the outer x */
    printf("%d", x);  /* 2 */
}
printf("%d", x);      /* 1 */
```

In Rust, shadowing is encouraged for local refinement:

```rust
let x = "5";          // x: &str
let x: i32 = x.parse().unwrap();   // shadows; x is now i32
```

This is a *different* binding — type can change, the original is shadowed inside the function but its type is unchanged outside.

### Copy vs move

- In C: assignment `b = a` *always* copies the bit pattern. Two storage locations, two values.
- In Rust: assignment of a non-`Copy` type *moves* the value: the source is invalidated.

```rust
let s1 = String::from("hi");
let s2 = s1;          // moves; s1 is no longer valid
println!("{}", s1);   // compile error: use of moved value
```

Primitive types (`i32`, `f64`, etc.) implement `Copy`, so assignment is a copy. Owning types (`String`, `Vec<T>`, etc.) move. This becomes Lesson 10 (ownership).

## Build It

Open `code/main.c` and `code/main.rs`.

### Step 1: `sizeof` survey (C)

Print the byte size of every primitive type. Notice `sizeof(char) == 1` (by definition), and `sizeof(int)` is usually 4 even on 64-bit platforms.

### Step 2: Scope demo (C)

A nested block shadows an outer variable; reads at each level confirm the binding in scope.

### Step 3: Rust shadowing + type change

Show `let x = ...; let x: T = ...;` with a type change — something C can't do.

### Step 4: Copy vs move

In Rust, primitive types (`i32`) are `Copy`, so assignment copies; `String` is not `Copy`, so assignment moves.

## Use It

- **Debugging stack-corruption bugs**: knowing exactly when a local variable's storage stops being valid.
- **Reading C standard-library headers**: `size_t`, `ptrdiff_t`, `off_t` are typedefs; understand them by going back to `int*` widths.
- **Lifetime errors in Rust**: the borrow checker enforces scope discipline at compile time.
- **Embedded programming**: knowing exact widths matters when the platform is 16-bit or has weird alignment.

## Read the Source

- *The C Programming Language* (K&R), Chapter 2 — definitive on C types and storage classes.
- *The Rust Programming Language* by Klabnik & Nichols, Chapters 3-4 — variables, types, ownership.
- [`<stdint.h>` reference](https://en.cppreference.com/w/c/types/integer) — when you should reach for fixed-width types.

## Ship It

This lesson ships **`outputs/type_sizes.c`** — a portable program that prints the size and signedness of every C primitive on the build platform. Useful as a sanity check on a new toolchain.

## Exercises

1. **Easy.** What's `sizeof(long)` on macOS x86_64? On Linux x86_64? On Windows x86_64? On a 32-bit ARM Cortex-M? (Hint: the C standard only requires `≥ 4`.)
2. **Medium.** Show using a Rust program that `let x = String::from("a"); let y = x; println!("{}", x);` fails to compile, but the same with `i32` works fine. Explain why.
3. **Hard.** In C, demonstrate undefined behavior from signed integer overflow: a loop `for (int i = 0; i < INT_MAX + 1; i++)` (note the +1!). Compile with `-O0` and `-O2`; observe different behavior. Mitigate with `__builtin_add_overflow` or `-fsanitize=signed-integer-overflow`.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Value | "What a variable holds" | A specific bit pattern at a storage location, interpreted under a type |
| Type | "Kind of data" | A static interpretation of a bit pattern: size + interpretation + allowed operations |
| Variable | "A box holding a value" | A name bound to either a storage location (C) or a value (default Rust) |
| Scope | "Where the name is visible" | The region of source code where a binding is in effect |
| Shadowing | "Same name twice" | A nested-scope binding that hides an outer binding of the same name (same lifetime, possibly different type in Rust) |

## Further Reading

- *Expert C Programming* by Peter van der Linden — Chapter 1 on "C through the mists of time," a stunning tour of why C's type system is what it is.
- *Programming Rust* by Blandy, Orendorff, Tindall — Chapter 4 (Ownership) and 5 (References).
- [The Itanium C++ ABI](https://itanium-cxx-abi.github.io/cxx-abi/abi.html) — for the masochist who wants to see how types lower to actual bytes.
