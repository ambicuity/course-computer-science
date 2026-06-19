# Lifetimes, References, RAII

> Lifetimes are how Rust expresses "this reference is valid for *exactly* this region." RAII is how every modern language ties resource release to scope exit. Same idea, two notations.

**Type:** Learn
**Languages:** Rust, C++
**Prerequisites:** Phase 02, Lesson 10
**Time:** ~75 minutes

## Learning Objectives

- Read and write Rust lifetime annotations (`'a`, `'b`); explain what they constrain.
- Apply the lifetime elision rules to know when annotations are required vs auto-inserted.
- Implement RAII in C++: a class whose constructor acquires a resource and whose destructor releases it.
- Apply the smart-pointer patterns (`unique_ptr`, `shared_ptr` in C++; `Box`, `Rc`, `Arc` in Rust) — RAII wrappers around heap allocations.

## The Problem

In Lesson 10 you saw the borrow checker reject programs with dangling references. But how does the compiler know whether a borrow is dangling? Sometimes the analysis crosses function boundaries:

```rust
fn longest(s1: &str, s2: &str) -> &str {  // does the return reference s1 or s2?
    if s1.len() > s2.len() { s1 } else { s2 }
}
```

The compiler can't infer the answer. **Lifetimes** are the annotation that tells it.

RAII is the broader pattern: scope-bound resource management. Same idea as Rust's `Drop`, but invented for C++ in the 1980s. Modern C++ and Rust share this design.

## The Concept

### Lifetimes

Every reference in Rust has a **lifetime** — a scope during which it's valid. Usually the compiler infers it:

```rust
fn first_word(s: &str) -> &str {  /* compiler infers: returned ref lives as long as s */
    s.split(' ').next().unwrap()
}
```

When the analysis is ambiguous (multiple input references, no obvious mapping to the output), you must annotate:

```rust
fn longest<'a>(s1: &'a str, s2: &'a str) -> &'a str {
    if s1.len() > s2.len() { s1 } else { s2 }
}
```

Read: "for some lifetime 'a, both s1 and s2 are valid; the returned reference is also valid for 'a." The compiler chooses 'a as the *shorter* of the two input scopes.

### Lifetime elision rules

In simple cases you don't need to write `'a`. Three rules:

1. **Each input reference gets its own lifetime parameter.** `fn f(s: &str)` ↦ `fn f<'a>(s: &'a str)`.
2. **If there's exactly one input lifetime, the output gets that one.** `fn f(s: &str) -> &str` ↦ `fn f<'a>(s: &'a str) -> &'a str`. (Inferred.)
3. **If there's `&self` or `&mut self`, the output lifetime is `self`'s lifetime.** This is why method signatures rarely need annotations.

If none of these unambiguously determines output lifetimes, the compiler asks you to annotate.

### Struct lifetimes

A struct that holds a reference must annotate the reference's lifetime:

```rust
struct Borrower<'a> {
    name: &'a str,
}

impl<'a> Borrower<'a> {
    fn name(&self) -> &str { self.name }
}
```

Read: "Borrower<'a> lives only as long as the `&'a str` it borrows."

### `'static`

The `'static` lifetime means "lives for the entire program." String literals have `'static`:

```rust
let s: &'static str = "hello, world";
```

Don't reach for `'static` to silence the borrow checker. It's a real claim about the data, not a hack.

### Lifetime variance, briefly

References are *covariant* in their referent: `&'long T` can be used where `&'short T` is expected (a longer-lived reference is a stronger guarantee). The compiler handles this automatically; you'll meet it when implementing collections.

### RAII (Resource Acquisition Is Initialization)

In C++:

```cpp
class FileHandle {
    FILE *fp;
public:
    FileHandle(const char *path) {
        fp = fopen(path, "r");
        if (!fp) throw std::runtime_error("open failed");
    }
    ~FileHandle() {
        if (fp) fclose(fp);
    }
    // delete copy ctor; provide move ctor
    FileHandle(const FileHandle&) = delete;
    FileHandle(FileHandle&& other) noexcept : fp(other.fp) { other.fp = nullptr; }
};

void process() {
    FileHandle f("data.txt");
    // ... if an exception is thrown here, f's destructor still runs on stack unwind ...
}   // f.~FileHandle() called automatically
```

The destructor running on scope exit (including exception unwinding) makes resource leaks structurally impossible — as long as every resource is wrapped in an RAII class.

### Smart pointers

RAII applied to heap allocations:

| C++ | Rust | What |
|-----|------|------|
| `std::unique_ptr<T>` | `Box<T>` | Sole ownership; freed on scope exit |
| `std::shared_ptr<T>` | `Rc<T>` (single-thread) / `Arc<T>` (multi-thread) | Reference-counted shared ownership |
| `std::weak_ptr<T>` | `rc::Weak<T>` / `sync::Weak<T>` | Weak reference for breaking cycles |

```cpp
auto p = std::make_unique<Widget>();  // heap-allocated; freed when p goes out of scope
```

```rust
let b = Box::new(Widget::new());       // same idea
```

No naked `new`/`delete` or `malloc`/`free` in modern code. Smart pointers eliminate the classic memory bugs.

### Drop order

Within a scope, values are dropped in **reverse declaration order**. This matters when a struct holds a borrowed reference: the borrower must be dropped *before* the borrowed.

In Rust, the compiler enforces this. In C++, destructors run in reverse declaration order automatically.

## Build It

Open `code/main.rs` (Rust) and `code/main.cpp` (C++).

### Step 1: Rust lifetime annotation

The `longest` function. Compile, then deliberately break it (return a reference to a local) and read the error.

### Step 2: Struct with a borrowed field

```rust
struct Token<'a> { text: &'a str, kind: TokenKind }
```

Cannot outlive `text`'s source.

### Step 3: C++ RAII

A `FileHandle` class. Open a file in the ctor, close in the dtor. Demonstrate that even when an exception is thrown, the file is closed (destructor runs during stack unwind).

### Step 4: `Box<T>` and `unique_ptr`

Heap allocation with automatic cleanup.

### Step 5: `Arc<T>` for shared ownership

Multiple owners; the last one to drop frees the memory.

## Use It

- **Every Rust API** with a function returning a reference has lifetime annotations (often elided). You'll see them on iterators, traits, and any zero-copy parser.
- **C++ STL** is built on RAII: every container, file stream, lock, smart pointer follows the pattern.
- **Async runtimes** (tokio, async-std) build on lifetime correctness to make sure futures can't outlive their borrowed data.
- **FFI bridges**: a `*const c_char` from C needs a known lifetime; you wrap it with `unsafe` and choose `'static` or a region-bound annotation.

## Read the Source

- *The Rust Programming Language*, Chapter 10 (Generic Types, Traits, and Lifetimes) — lifetimes covered in §10.3.
- *Effective Modern C++* by Scott Meyers — Chapter 4 on smart pointers; mandatory.
- *Rust for Rustaceans* by Jon Gjengset — Chapter 1, §1.4, lifetime variance done rigorously.

## Ship It

This lesson ships **`outputs/raii_examples.cpp`** — three RAII classes (FileHandle, MutexGuard, ScopeGuard) you can drop into any C++ project.

## Exercises

1. **Easy.** Annotate `fn first<'a>(s: &'a [i32]) -> &'a i32` returning the first element. Confirm lifetime elision would have inferred the same.
2. **Medium.** In C++, write a `ScopeGuard` class that takes a lambda in its ctor and runs it in the dtor. Use it to ensure cleanup of a malloc'd buffer in a function with early returns.
3. **Hard.** In Rust, write a `Tokenizer<'src>` struct that borrows a `&'src str` and produces `Token<'src>` references back into it. The compiler should prevent the source string from being dropped while any Token is alive.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Lifetime | "How long a reference is valid" | A compile-time scope parameter; the borrow checker uses it to prevent dangling references |
| Lifetime elision | "Auto-inferred lifetime" | Three rules let the compiler insert annotations in unambiguous cases |
| `'static` | "Lives forever" | A reference valid for the entire program; string literals have this |
| RAII | "Destructor cleans up" | C++ / Rust pattern: ctor acquires a resource, dtor releases it; scope exit guarantees release |
| Smart pointer | "Owning pointer" | RAII wrapper around a heap allocation; `unique_ptr`/`Box`, `shared_ptr`/`Rc`, etc. |

## Further Reading

- [Rust by Example — Lifetimes](https://doc.rust-lang.org/rust-by-example/scope/lifetime.html) — interactive examples.
- [Niko Matsakis on Non-Lexical Lifetimes](https://blog.rust-lang.org/2018/12/06/Rust-1.31-and-rust-2018.html) — modern Rust's lifetime analyzer.
- *C++ Concurrency in Action* by Anthony Williams — Chapter 3 on RAII for locking.
