# The Preprocessor and Macros (C, Rust)

> Macros are "code that writes code." C's preprocessor does textual substitution; Rust's macros operate on syntactic tokens. Both are powerful and both bite you when used badly.

**Type:** Learn
**Languages:** C, Rust
**Prerequisites:** Phase 02, Lessons 01, 12, 13
**Time:** ~45 minutes

## Learning Objectives

- Use the C preprocessor directives: `#include`, `#define`, `#if`/`#ifdef`/`#elif`/`#endif`, `#error`, `#pragma`.
- Recognize and avoid the C-macro footguns: missing parens, multiple-evaluation, comma-pitfall.
- Write Rust `macro_rules!` patterns; understand the token-tree based syntax (vs C's text-based).
- Recognize the X-macro pattern in C — a powerful template for keeping parallel arrays in sync.

## The Problem

Sometimes the language doesn't let you express what you want. You need code that generates code:

- Different code for debug vs release builds.
- Repeating boilerplate (an enum + its name table + its parser).
- Generic-like behavior in C, which has no generics.
- A logging macro that captures `__FILE__:__LINE__` automatically.

C's preprocessor and Rust's macros each address these — with very different mechanisms and trade-offs.

## The Concept

### C preprocessor — textual substitution

The C preprocessor (`cpp`) runs before the compiler. Its job is purely *textual*:

| Directive | Effect |
|-----------|--------|
| `#include "foo.h"` | Replace this line with the contents of foo.h |
| `#define X 42` | Replace every subsequent token `X` with `42` |
| `#define SQ(x) ((x) * (x))` | Function-like macro: text substitution at the call site |
| `#if`/`#ifdef`/`#elif`/`#else`/`#endif` | Conditional inclusion |
| `#error "msg"` | Halt with an error |
| `##` | Token-pasting (concatenate adjacent tokens) |
| `#x` | Stringify (turn x into a string literal) |
| `#pragma` | Compiler-specific directive |

The preprocessor has no knowledge of C semantics. It pastes tokens; the compiler then interprets them.

### Classic C-macro footguns

**1. Missing parens around args**:

```c
#define SQ(x) x * x

SQ(3 + 4)   /* expands to 3 + 4 * 3 + 4 = 19, not 49 */
```

Fix: parenthesize args AND the whole expression:

```c
#define SQ(x) ((x) * (x))
```

**2. Multiple evaluation**:

```c
#define MAX(a, b) ((a) > (b) ? (a) : (b))

int i = 0;
MAX(i++, 5);   /* expands to: ((i++) > (5) ? (i++) : (5))
                   i is incremented twice if i > 5 — surprising! */
```

Fix: use `inline` functions, or in GCC/Clang use statement expressions:

```c
#define MAX(a, b) ({ \
    __typeof__(a) _a = (a); \
    __typeof__(b) _b = (b); \
    _a > _b ? _a : _b; \
})
```

**3. The comma pitfall**:

```c
#define DECLARE(t, name1, name2) t name1, name2

DECLARE(int, x, y);   /* OK: int x, y; */
DECLARE(struct Pair{int a; int b;}, p, q);  /* breaks — first comma split confuses preprocessor */
```

### X-macros — the macro-as-template idiom

When you need parallel arrays / cases / strings in sync:

```c
/* colors.def — single source of truth */
X(RED,   "Red",   0xFF0000)
X(GREEN, "Green", 0x00FF00)
X(BLUE,  "Blue",  0x0000FF)
```

```c
/* Now generate each derived structure */
typedef enum {
#define X(name, label, rgb) COLOR_##name,
#include "colors.def"
#undef X
} Color;

const char *color_name(Color c) {
    switch (c) {
#define X(name, label, rgb) case COLOR_##name: return label;
#include "colors.def"
#undef X
    }
    return "?";
}
```

Adding a fourth color = adding one line in `colors.def`. Both the enum and the name table update.

### Rust `macro_rules!` — syntactic, hygienic

Rust macros operate on *tokens*, not text. They're hygienic by default (won't accidentally capture names from the call site).

```rust
macro_rules! max {
    ($a:expr, $b:expr) => {{
        let a = $a;
        let b = $b;
        if a > b { a } else { b }
    }}
}

let v = max!(3 + 4, 5);   // no multiple-evaluation; expands to a block with one let each
```

Pattern syntax:

- `$name:ident` — an identifier.
- `$name:expr` — any expression.
- `$name:ty` — a type.
- `$name:pat` — a pattern.
- `$name:tt` — a token tree.
- `$($x:expr),*` — a comma-separated list of expressions.

Plus procedural macros (`#[derive(Debug)]`, `#[tokio::main]`, etc.) for more power.

### Const vs macro vs inline function

In modern C, prefer `static inline` functions over function-like macros — they avoid all three footguns above. Use macros only when you need:

- Token pasting (`x ## y`).
- Conditional compilation (`#ifdef DEBUG`).
- Repetition over a list (X-macros).

In Rust, prefer regular generic functions; use macros only when you need:

- A variable number of args (`println!`, `vec!`).
- Compile-time DSL (`html!`, `query!`).
- Per-call inlining of source-location info (`file!()`, `line!()`).

## Build It

Open `code/main.c` and `code/main.rs`.

### Step 1: Function-like macro vs inline function

Define `SQUARE_MACRO(x)` (with parens) and `static inline int square_fn(int x)`. Test both with `SQ(3 + 4)` — both give 49. Then call `SQ_BAD(3 + 4)` (a deliberately-broken version) and see the wrong answer.

### Step 2: Conditional compilation

`#ifdef DEBUG` switches between two implementations.

### Step 3: Stringify + token paste

`#define MAKE_GETTER(field) int get_ ## field(void) { return obj.field; }` — generate getter functions automatically.

### Step 4: X-macro

A `colors.def` file used three ways: as enum, as string table, as switch.

### Step 5: Rust `macro_rules!`

The `max!` macro and the recursive `sum!` macro.

## Use It

- **Logging libraries** (slf4j, log, tracing): macros capture file/line/function automatically.
- **Test frameworks** (`#[test]`, gtest): macros register test functions.
- **Embedded code** (Linux kernel `container_of`): textual address arithmetic only possible via macros.
- **Domain-specific languages** in Rust (`sql_query!`, `html!`): compile-time validated DSLs via proc-macros.
- **Compatibility shims**: `#ifdef __linux__` / `#ifdef _WIN32` for portable code.

## Read the Source

- *The C Programming Language* (K&R), §A.12 — preprocessor reference.
- [C99 standard chapter 6.10](https://www.iso.org/standard/29237.html) — formal preprocessor spec.
- *The Little Book of Rust Macros* — free online guide to `macro_rules!`.
- [The Rust Reference — Macros](https://doc.rust-lang.org/reference/macros.html)

## Ship It

This lesson ships **`outputs/macro-pitfalls.md`** — a one-page list of the canonical C macro footguns with safer alternatives.

## Exercises

1. **Easy.** Write a C macro `MIN(a, b)`. Demonstrate the multiple-evaluation bug with `MIN(i++, 5)`; fix it with a GCC statement-expression.
2. **Medium.** Use an X-macro to define an enum of 5 HTTP status codes, plus a `const char *status_name(int code)` function and a `bool is_error(int code)` function — all from one .def file.
3. **Hard.** Write a Rust `macro_rules! json` that lets you write `json!({ "key": 42, "list": [1, 2, 3] })` and produces a `serde_json::Value`. (Smaller scope is fine — recurse over an inner subset of JSON.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Preprocessor | "Macros + includes" | A textual transformer run before the C compiler proper; expands `#define` and `#include` |
| Hygiene | "Macros that don't capture variables" | A macro system property where names introduced by the macro can't accidentally collide with names at the call site |
| Token paste / stringify | "`##` and `#`" | C preprocessor ops: concatenate tokens, or turn tokens into string literals |
| X-macro | "Macro as template" | The pattern of defining a list of items in a single file, then including it multiple times with different `X` definitions to generate parallel structures |
| `macro_rules!` | "Rust declarative macro" | Token-tree based, hygienic, type-aware pattern matching for code generation |

## Further Reading

- *Modern C* by Jens Gustedt — Chapter 14 covers preprocessor pitfalls in detail.
- [Rust by Example: Macros](https://doc.rust-lang.org/rust-by-example/macros.html) — interactive examples.
- [GCC's statement expressions](https://gcc.gnu.org/onlinedocs/gcc/Statement-Exprs.html) — the trick for hygienic-ish C macros.
