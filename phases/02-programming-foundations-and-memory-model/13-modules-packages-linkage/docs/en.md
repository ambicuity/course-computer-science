# Modules, Packages, Linkage

> Splitting code across files is half art, half symbol-table mechanics. Every language solved it differently; understanding the differences makes you fluent across all of them.

**Type:** Learn
**Languages:** C, Rust
**Prerequisites:** Phase 02, Lessons 01 (compilation/linking), 04
**Time:** ~45 minutes

## Learning Objectives

- Distinguish *translation unit* (one C `.c` file) from *module* (Rust namespace) from *package* (a unit of distribution).
- Explain C's three linkage levels: internal (file-static), external (default global), and none (block-local).
- Read a Rust `mod` hierarchy and `pub`/`pub(crate)`/`pub(super)` visibility modifiers.
- Recognize the analogous concepts in Python (modules + packages), Go (packages + visibility by capitalization), Java (packages + access modifiers).

## The Problem

You can't put every line of a real project in one file. The mechanisms for splitting code differ wildly:

- **C**: per-`.c` "translation unit"; the linker decides whose symbols win at link time.
- **Rust**: per-file modules in a tree; `mod` and `use` keywords; cargo crates.
- **Python**: `import foo` looks up `foo.py` or `foo/__init__.py`.
- **Go**: per-directory packages; only capitalized names are exported.
- **Java**: per-file classes; packages = directories.

Each is a different answer to: how do names cross file boundaries?

## The Concept

### C: translation units and linkage

A C **translation unit** is one `.c` file after preprocessing. Compiling it produces one `.o`. The linker glues `.o`s into the final binary.

Every top-level name in C has one of three **linkages**:

| Linkage | Declared as | Where visible |
|---------|-------------|---------------|
| **External** | (default) `int g = 5;`, `void f(void);` | Every other `.c` that declares it `extern` can use it |
| **Internal** | `static int g = 5;`, `static void f(void);` | Only inside this `.c` file |
| **None** | (local variables / function parameters) | Inside the enclosing block |

A header file (`foo.h`) declares functions and types that multiple `.c`s share. Convention:

```c
/* foo.h */
#ifndef FOO_H
#define FOO_H
int add(int a, int b);
extern int counter;
#endif
```

```c
/* foo.c */
#include "foo.h"
int counter = 0;
int add(int a, int b) { counter++; return a + b; }
```

```c
/* main.c */
#include "foo.h"
int main(void) { return add(1, 2); }
```

`add`'s prototype lives in `foo.h`; `add`'s definition lives in `foo.c`. Multiple `.c`s `#include`-ing `foo.h` get the same declaration; the linker resolves all the calls to the single `add` defined in `foo.c`.

The "include guard" (`#ifndef FOO_H ... #endif`) prevents double-inclusion errors when multiple chains pull in `foo.h`.

### Rust: modules and crates

A **module** is a Rust namespace. A **crate** is one unit of compilation (a library or binary). A **package** (cargo terminology) is one or more crates.

Modules form a tree. Default is *one file per module*:

```
src/
├── main.rs          # crate root
├── parser.rs        # mod parser;
└── parser/
    ├── lexer.rs     # parser::lexer
    └── ast.rs       # parser::ast
```

`main.rs`:
```rust
mod parser;          // pulls in src/parser.rs

fn main() {
    let tokens = parser::lexer::tokenize("...");
    let tree   = parser::ast::build(tokens);
}
```

`parser.rs` (or `parser/mod.rs`):
```rust
pub mod lexer;
pub mod ast;
```

Visibility modifiers:

| Modifier | Visible to |
|----------|-----------|
| (default) | This module only |
| `pub` | Everywhere this module is reachable from |
| `pub(crate)` | Other code in this crate, but not external users |
| `pub(super)` | The parent module |
| `pub(in path)` | A specific ancestor module |

Cargo manages **crates** as **packages**:

```toml
# Cargo.toml
[package]
name = "myapp"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }
```

`cargo build` resolves the dep tree, downloads from crates.io (or git/path), and compiles them all.

### Python: modules and packages

`import foo` looks up:

- `foo.py` in `sys.path`, OR
- `foo/__init__.py` (a *package* — a directory of modules).

Names defined at module top level are accessible as `foo.bar`. There's no language-level "private" — convention is `_underscore` prefix.

### Go: packages + capitalization

Each directory is one package. Inside a package, all files have the same `package foo` declaration. Names starting with uppercase are exported (`fmt.Println`); lowercase are package-private (`fmt.format`).

### Java: packages = directories

`package com.example.foo;` matches `src/com/example/foo/*.java`. `public`/`protected`/`package-private`/`private` modifiers control visibility.

### The shared idea

All five languages need to answer:

1. How do names cross files? (Headers in C; `mod` in Rust; `import` in Python; capitalization in Go.)
2. What's the unit of distribution? (`*.a`/`*.so` in C; crate in Rust; package in Python; module in Go; JAR in Java.)
3. How do you express "private to this file but visible across files I control"? (`static` in C; `pub(crate)` in Rust; convention in Python; lowercase in Go; package-private in Java.)

## Build It

Open `code/main.c` and the Rust example.

### Step 1: C — split a project into 3 files

`main.c`, `calc.c`, `calc.h`. The header declares; `calc.c` defines; `main.c` `#include`s and calls.

### Step 2: C — internal linkage

Add a `static int counter = 0;` inside `calc.c` and a function to increment it. The variable is invisible outside `calc.c` — even with `extern` declarations, the linker won't find it.

### Step 3: Rust — module tree (sketch)

```rust
mod math {
    pub fn add(a: i32, b: i32) -> i32 { a + b }
    pub(crate) fn double(x: i32) -> i32 { x * 2 }
    fn secret(x: i32) -> i32 { x }   // module-private
}
```

### Step 4: Rust — visibility

Try to call `math::secret(5)` from `main`; observe the compile error.

### Step 5: External crate (Cargo)

A typical `Cargo.toml` lists deps from crates.io.

## Use It

- **Every C library you'll link against** follows the header/.c convention.
- **Every Rust crate on crates.io** is a package + one or more modules. Reading the API requires understanding `pub use` re-exports.
- **Static analysis tools** (clang-tidy, cargo clippy) work at the translation unit / crate level.
- **ABI stability**: C's symbols are public ABI; Rust's are explicitly *not* (no stable mangling); Java's bytecode is stable.

## Read the Source

- *The C Programming Language* (K&R), Chapter 4 — declarations, storage classes, linkage.
- *The Rust Programming Language*, Chapter 7 (Managing Growing Projects with Packages, Crates, and Modules).
- *Effective Go* — the package section is concise and worth reading.
- [PEP 328 — Imports](https://peps.python.org/pep-0328/) — Python's import semantics.

## Ship It

This lesson ships **`outputs/c-project-skeleton/`** — a minimal Makefile + 3-file C project (main.c, calc.c, calc.h) showing the header/.c separation and internal linkage correctly.

## Exercises

1. **Easy.** Take a one-file C program and split a function into `helpers.c` + `helpers.h`. Make sure your Makefile compiles each `.c` separately and links them.
2. **Medium.** In Rust, organize a small project into `src/main.rs` + `src/lib.rs` + `src/utils.rs`. The library exposes `add`, `mul`; main calls them. Build with `cargo run`.
3. **Hard.** In a C library you write, hide an implementation detail (a static-storage cache) from callers. Verify that even with `extern int the_cache;` in another `.c`, the linker rejects the reference.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Translation unit | "A C source file" | One `.c` file plus its included headers, after preprocessing; compiles to one `.o` |
| Linkage | "How visible a name is" | Internal (file-private), external (cross-file), or none (block-local) |
| Module | "Namespace" | A scope with its own names; in Rust, declared via `mod` |
| Crate / package | "Library unit" | The unit of compilation and distribution |
| `extern` | "Imported from elsewhere" | C keyword saying "this name is defined in another translation unit" |

## Further Reading

- *C: A Reference Manual* by Harbison & Steele — definitive on storage classes.
- *Programming Rust* by Blandy/Orendorff/Tindall — Chapter 8 covers crates and modules thoroughly.
- *Java Modules* (JEP 261) — Java's modern module system added in Java 9.
