# Ownership and Borrowing — the Rust Model

> Rust's ownership rules are three sentences. Internalize them and you have memory safety without garbage collection — for free, at compile time.

**Type:** Learn
**Languages:** Rust
**Prerequisites:** Phase 02, Lessons 02, 05, 06
**Time:** ~75 minutes

## Learning Objectives

- State Rust's three ownership rules and explain what each one prevents.
- Distinguish *moves* from *copies*; recognize which types implement `Copy` and why.
- Use shared borrows (`&T`) and exclusive borrows (`&mut T`); explain why they can't co-exist.
- Resolve common borrow-checker errors: use-after-move, mutable + shared, dangling reference.

## The Problem

In Phase 02 Lessons 05–06, we saw the canonical C bugs: dangling pointer, use-after-free, double free, data race. Every one of them is a *temporal* memory bug — using a memory location after its lifetime ended, or in a way that conflicts with what another thread is doing.

Languages with garbage collection (Java, Go, Python) solve these by keeping memory alive as long as anyone references it. The cost is a runtime tracing collector + GC pauses + indirection.

Rust takes a third path. The *compiler* tracks the ownership of every value through the source code; programs that would have temporal bugs fail to compile. No runtime cost.

This lesson explains the model.

## The Concept

### The three rules

1. **Each value has exactly one owner** (a binding that "owns" it).
2. **When the owner goes out of scope, the value is dropped** (its destructor runs).
3. **You can borrow the value (`&T` shared, or `&mut T` exclusive) — but a value can either have any number of shared borrows OR exactly one exclusive borrow at a time.**

These three rules — enforced at compile time — eliminate use-after-free, double-free, and data races (in safe Rust).

### Moves vs copies

When you assign a non-`Copy` value, Rust **moves** ownership:

```rust
let s1 = String::from("hi");
let s2 = s1;             // ownership transferred to s2
// println!("{}", s1);   // ERROR: borrow of moved value
```

The compiler tracks that `s1` is no longer valid. No double-free risk: when `s2` goes out of scope, the destructor runs once, on s2.

Primitive types (`i32`, `f64`, `bool`, `char`, fixed-size arrays of these) implement the `Copy` trait — assignment makes a bitwise copy:

```rust
let a: i32 = 5;
let b = a;       // copy; a is still valid
```

`String`, `Vec<T>`, `Box<T>`, `File`, `Mutex<T>` — anything that owns heap memory or external resources — do NOT implement `Copy` (you'd risk double-free). Assignment moves.

### Borrows

Borrowing lets a function read or modify a value without taking ownership:

```rust
fn read(s: &String) {        // shared borrow
    println!("{}", s);
}

fn append(s: &mut String) {  // exclusive borrow
    s.push_str(" world");
}

let mut s = String::from("hello");
read(&s);                     // s remains owned by the caller
append(&mut s);
read(&s);                     // can borrow again — append's borrow ended
```

The borrowing rules:
- At any point, you can have either **N shared borrows (`&T`)** OR **one exclusive borrow (`&mut T`)** — never both, never multiple exclusive.
- The borrow must NOT outlive the owner.

Why both at once would be unsafe:

```rust
let mut v = vec![1, 2, 3];
let first = &v[0];          // shared borrow
v.push(4);                  // exclusive borrow (push may reallocate)
println!("{}", first);      // would point at freed memory!
```

The borrow checker rejects this. Result: no use-after-free.

### Function signatures spell out ownership

| Signature | Means |
|-----------|-------|
| `fn f(s: String)` | f takes ownership; caller's binding is moved away |
| `fn f(s: &String)` | f borrows immutably; caller still owns |
| `fn f(s: &mut String)` | f borrows mutably; caller still owns but can't use during call |

Idiom: take `&T` unless you need to consume or mutate.

### Borrow checker error messages

The compiler will spell out exactly what went wrong:

```
error[E0382]: borrow of moved value: `s1`
 --> src/main.rs:3:20
  |
1 | let s1 = String::from("hi");
2 | let s2 = s1;
  |          -- value moved here
3 | println!("{}", s1);
  |                ^^ value borrowed here after move
```

Three classic errors to recognize:

1. **Use after move**: `let s2 = s1; println!("{}", s1);`
2. **Mutable + shared simultaneously**: `let r1 = &v; let r2 = &mut v; *r2 = ...; println!("{}", r1);`
3. **Dangling reference**: `fn f() -> &String { let s = String::from("..."); &s }` — `s` dies at end of `f`; can't return a reference to it.

### Internal mutability (preview)

Sometimes you genuinely need shared *and* mutating access (e.g., a cache). Rust provides:

- `Cell<T>` / `RefCell<T>` — single-threaded, runtime-checked.
- `Mutex<T>` / `RwLock<T>` — multi-threaded.
- `Atomic*` — lock-free primitives.

These wrap an unsafe core in a safe API. Phase 13 covers them.

### `Drop`

When a value goes out of scope, its destructor (`Drop::drop`) runs. For `String`, that's freeing the heap buffer. For `File`, closing the handle. For `MutexGuard`, releasing the lock. **No `free` calls in user code**, ever — the compiler emits them.

```rust
fn f() {
    let s = String::from("hi");
    // ... use s ...
}   // s's destructor runs here; heap memory freed
```

This is RAII (Resource Acquisition Is Initialization), borrowed from C++ — but here enforced by the type system.

## Build It

Open `code/main.rs`. Each section demonstrates one rule; intentional-error variants are commented out so you can uncomment and see the borrow checker complain.

### Step 1: Move semantics

`let s2 = s1` moves `s1`; using s1 afterward is rejected at compile time.

### Step 2: Borrowing

Demonstrate one mutable XOR many shared.

### Step 3: Avoiding the dangling-reference bug

A function that tries to return a reference to a local — compile-time rejected.

### Step 4: Working around with `Clone`

When you really need a duplicate, call `.clone()` — explicit deep copy:

```rust
let s1 = String::from("hi");
let s2 = s1.clone();    // s1 still valid
```

### Step 5: `Drop` in action

Implement `Drop` for a custom type; print the drop event to see exactly when it fires.

## Use It

- **No segfaults in safe Rust.** The borrow checker enforces what `valgrind` / ASan can only detect after the fact.
- **Foundation of `Send`/`Sync`** (Phase 13): the compiler extends ownership tracking to threads, eliminating most data races.
- **FFI bridges**: when binding to C, you mark functions `unsafe` — the borrow checker doesn't follow pointers across language boundaries.
- **Lifetimes** (Lesson 11): when borrow analysis can't be inferred, you annotate the function with lifetime parameters.

## Read the Source

- *The Rust Programming Language*, Chapter 4 (Ownership) — definitive.
- *Programming Rust* by Blandy/Orendorff/Tindall — Chapter 4-6 covers ownership, refs, and lifetimes thoroughly.
- [Niko Matsakis's blog](http://smallcultfollowing.com/babysteps/) — design rationale from one of Rust's architects.

## Ship It

This lesson ships **`outputs/ownership-cheatsheet.md`** — a one-page reference: rules, common errors with explanations, idioms to know.

## Exercises

1. **Easy.** Write a function `fn longest<'a>(s1: &'a str, s2: &'a str) -> &'a str` that returns the longer string. (Lesson 11 explains the `'a` syntax.)
2. **Medium.** Build a struct `Counter` with `new()`, `increment(&mut self)`, `read(&self) -> u64`. Demonstrate that you can call `read` many times in parallel but `increment` exclusively.
3. **Hard.** Write a Rust function equivalent to C's `strcpy` — but the function signature must make memory safety impossible to break. (Hint: take `&str` for source, `&mut String` for dest.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Ownership | "Who has the value" | The binding that owns a value; when it goes out of scope, the value's destructor runs |
| Move | "Transfer ownership" | Assignment of a non-Copy type; source becomes invalid |
| Borrow | "Reference" | `&T` shared or `&mut T` exclusive; cannot outlive the owner |
| Drop | "Destructor" | The `Drop::drop` method automatically called when ownership ends |
| Borrow checker | "Compile-time checker" | The pass in `rustc` that enforces ownership and borrowing rules |

## Further Reading

- *The Rustonomicon* — covers the unsafe corners of Rust; helpful for understanding what the borrow checker is protecting you from.
- [The borrow checker is your friend — Federico Mena Quintero](https://people.gnome.org/~federico/blog/the-borrow-checker.html) — practical perspective.
- *Rust for Rustaceans* by Jon Gjengset — Chapter 1 (foundations) revisits ownership rigorously.
