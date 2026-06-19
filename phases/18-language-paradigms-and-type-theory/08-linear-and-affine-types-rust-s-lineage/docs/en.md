# Linear and Affine Types — Rust's Lineage

> Ownership disciplines are type-theoretic constraints on resource usage.

**Type:** Learn
**Languages:** Rust, Haskell
**Prerequisites:** Phase 18 lessons 01-07
**Time:** ~60 minutes

## Learning Objectives

- Differentiate linear vs affine usage constraints.
- Relate Rust ownership/borrowing to affine typing intuition.
- Model single-use resource flows safely.
- Spot double-use and use-after-move classes early.

## The Problem

File handles, database connections, network sockets, cryptographic keys, transaction tokens. All of these are resources that must be used exactly once (or at most once) and then released. If you close a file handle twice, you might close a different file that reused the same descriptor. If you use a crypto key after freeing it, you leak memory or worse. If you send a transaction token twice, you double-spend.

Traditional type systems don't track usage. A function takes a `File` argument and the type says nothing about whether it closes it, leaks it, or uses it twice. Resource safety relies on convention, code review, and runtime checks.

Linear and affine types move usage constraints into the type system. A linear type says "you must use this value exactly once." An affine type says "you must use this value at most once." Rust's ownership system is the most successful industrial implementation of affine typing: values are moved on assignment, and the compiler rejects programs that use a value after it's been moved.

## The Concept

### Linear vs affine

```
Unrestricted:  Use any number of times (including zero)
Affine:        Use at most once           (0 or 1)
Linear:        Use exactly once           (1)
Relevant:      Use at least once          (1 or more)
```

```
           Relevant
          /        \
     Linear ---- Affine
          \        /
          Unrestricted
```

Linear types guarantee a value is consumed exactly once: no leaks, no double-use. Affine types guarantee at most once: no double-use, but the value might be dropped unused. Rust is affine: you can drop a value without using it (it goes out of scope), but you can't use it twice.

### Why affine, not linear?

Rust chose affine over linear because requiring every value to be used is too restrictive. You often want to ignore a function's return value, or drop a temporary. Affine typing with explicit `Drop` gives the best tradeoff: the compiler tracks moves and borrows, ensures no use-after-move, and runs destructors automatically.

### Rust's ownership as affine typing

```rust
fn main() {
    let s = String::from("hello");
    let t = s;              // s is moved to t
    // println!("{}", s);   // ERROR: use after move
    println!("{}", t);      // OK: t owns the string
}   // t is dropped here
```

Each value has exactly one owner at a time. Assignment moves ownership. The old binding becomes invalid. This is affine: at most one use.

### Borrowing: controlled sharing

Pure affine typing would be impractical: you couldn't read a value twice. Rust adds borrowing:

```rust
fn main() {
    let s = String::from("hello");
    let len = calculate_length(&s);  // borrow, not move
    println!("{} has length {}", s, len);  // s still valid
}

fn calculate_length(s: &String) -> usize {
    s.len()
}
```

Borrowing rules:
- Any number of shared borrows (`&T`) OR exactly one exclusive borrow (`&mut T`).
- Borrows cannot outlive the owner.

This gives controlled aliasing without losing the affine guarantee on ownership.

### Linear types in research languages

Haskell's `LinearTypes` extension adds actual linear types:

```haskell
{-# LANGUAGE LinearTypes #-}

-- Consumes the file handle exactly once
closeFile :: FileHandle %1 -> ()

-- This won't compile:
-- bad f = (closeFile f, closeFile f)  -- double use

-- This won't compile either:
-- bad2 f = ()  -- leaked (not used at all)
```

The `%1` annotation means the argument must be used exactly once. This is stricter than Rust's affine typing.

### Resource protocol modeling

Linear/affine types can encode protocol state machines:

```
Socket  →[connect]→  Connected  →[send]→  Sent  →[close]→  Closed
```

Each state is a different type. Transitions are functions that consume one state and produce the next. You can't skip a state or go back:

```rust
struct Unconnected;
struct Connected { stream: TcpStream }
struct Closed;

impl Unconnected {
    fn connect(self, addr: &str) -> Connected {
        // consumes Unconnected, produces Connected
        Connected { stream: TcpStream::connect(addr).unwrap() }
    }
}

impl Connected {
    fn send(self, data: &[u8]) -> Connected {
        self.stream.write_all(data).unwrap();
        self  // returns ownership back
    }

    fn close(self) -> Closed {
        drop(self.stream);
        Closed
    }
}
```

After `close`, the `Connected` value is consumed. You can't send on a closed socket. The type system enforces the protocol.

## Build It

### Step 1: Move semantics (Rust)

```rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1;        // s1 moved to s2
    // println!("{}", s1);  // compile error: use after move
    println!("{}", s2);     // OK
}
```

### Step 2: Borrowing

```rust
fn print_len(s: &String) {
    println!("len = {}", s.len());
}

fn append_world(s: &mut String) {
    s.push_str(" world");
}

fn main() {
    let mut s = String::from("hello");
    print_len(&s);          // shared borrow
    append_world(&mut s);   // exclusive borrow
    print_len(&s);          // shared borrow again
}
```

### Step 3: File handle protocol

```rust
use std::fs::File;
use std::io::Write;

fn write_and_close(path: &str, content: &str) -> std::io::Result<()> {
    let mut f = File::create(path)?;  // f owns the handle
    f.write_all(content.as_bytes())?;
    // f is dropped here, closing the handle
    // Can't accidentally use f after this
    Ok(())
}

fn main() {
    write_and_close("/tmp/test.txt", "hello").unwrap();
    // File handle is guaranteed closed after write_and_close returns
}
```

### Step 4: Haskell linear types

```haskell
{-# LANGUAGE LinearTypes #-}

-- Linear function: must use argument exactly once
linId :: a %1 -> a
linId x = x

-- Consuming a resource exactly once
consume :: FilePath %1 -> IO ()
consume path = putStrLn $ "Consumed: " ++ show path

-- This would fail:
-- doubleUse :: a %1 -> (a, a)
-- doubleUse x = (x, x)  -- linear type error: used twice
```

### Step 5: Unique types (Clean language concept)

Clean's uniqueness types are related to linear types:

```clean
// In Clean (conceptual):
// *File means "unique reference to a file handle"
openFile :: String -> *File
writeFile :: *File String -> *File
closeFile :: *File -> ()

// The type system ensures the file handle is used linearly
```

## Use It

These ideas generalize to:

- **File handles and connections**: Rust's `Drop` trait ensures handles are closed. Linear types could enforce "must close exactly once."
- **Cryptographic keys**: Linear types prevent key material from being duplicated or leaked.
- **Transaction tokens**: A linear token proves a transaction was committed exactly once.
- **Protocol enforcement**: State machine transitions encoded as type-level consumption.
- **Arena allocators**: Linear typing ensures memory is freed exactly when the arena is done.

Rust's `Drop`, `Send`, and `Sync` traits all derive from the ownership/borrowing model. The borrow checker is, at its core, an affine type checker.

## Read the Source

- *The Rust Programming Language*, Chapter 4: Ownership.
- [Linear Haskell](https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/linear_types.html) — GHC's linear types extension.
- [Rustonomicon](https://doc.rust-lang.org/nomicon/) — unsafe Rust and what the borrow checker protects you from.
- *Linear Types Can Change the World!* (Wadler) — foundational paper.

## Ship It

- `code/main.rs`: ownership and move demo.
- `code/Main.hs`: linear-style conceptual snippet.
- `outputs/README.md`: resource-safety checklist.

## Quiz

**Q1 (Pre).** What's the difference between linear and affine types?

- A) Linear requires exactly one use; affine allows at most one use.
- B) Linear is for numbers; affine is for strings.
- C) They're the same thing.
- D) Affine is stricter than linear.

**Answer: A.** Linear types require the value to be consumed exactly once (no leaks, no double-use). Affine types allow at most one use (no double-use, but dropping unused is OK). Rust is affine: values can go out of scope unused, but can't be used after move.

**Q2 (Pre).** In Rust, what happens when you assign a `String` to another variable?

- A) The string is copied.
- B) Ownership is moved; the original binding becomes invalid.
- C) Both variables share ownership.
- D) A reference is automatically created.

**Answer: B.** `String` doesn't implement `Copy`, so `let t = s` moves ownership from `s` to `t`. Using `s` afterward is a compile error. This is the affine guarantee: at most one use of the ownership.

**Q3 (Post).** Why did Rust choose affine over linear types?

- A) Linear types are unsound.
- B) Requiring every value to be used is too restrictive; dropping unused values is common.
- C) Affine types are more expressive.
- D) Linear types can't express borrowing.

**Answer: B.** Linear typing would reject code that ignores return values or drops temporaries. Affine typing with automatic `Drop` gives the best tradeoff: the compiler prevents use-after-move and double-use, while allowing values to be dropped without explicit consumption.

**Q4 (Post).** How does borrowing extend affine typing?

- A) It removes the affine restriction.
- B) It allows shared read access without transferring ownership, preserving the "at most one owner" invariant.
- C) It makes types linear instead of affine.
- D) It's unrelated to affine typing.

**Answer: B.** Borrowing (`&T`, `&mut T`) lets functions read or temporarily modify a value without taking ownership. Multiple shared borrows can coexist, but an exclusive borrow requires no other borrows. This gives controlled aliasing while keeping the affine ownership model.

**Q5 (Post).** How can linear types encode protocol state machines?

- A) They can't.
- B) Each protocol state is a type; transitions are functions that consume one state type and produce the next, preventing skipped or repeated states.
- C) By using runtime checks.
- D) By using inheritance hierarchies.

**Answer: B.** If `Connected` is a linear type, a function `close : Connected %1 -> Closed` consumes the `Connected` value. You can't call `close` twice (the value is consumed). You can't call `send` after `close` (the type changed). The protocol is enforced by the type system.

## Exercises

1. **Easy.** Write a Rust function that takes ownership of a `String` and returns its length. Show that the original binding is invalid after the call.
2. **Medium.** Model a file-handle protocol with three states (Unopened, Open, Closed) as Rust structs. Write methods that transition between states, consuming the old state.
3. **Hard.** Implement a `Ref` type in Haskell with `LinearTypes` that must be dereferenced exactly once. Show that double-dereference and forgotten-dereference both fail to compile.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Linear type | "single use" | Value must be consumed exactly once in the computation |
| Affine type | "move-only-ish" | Value may be consumed at most once (can be dropped unused) |
| Ownership | "who controls value" | Unique authority over a value's lifecycle; when owner ends, value is dropped |
| Borrow | "temporary access" | Scoped reference without ownership transfer (`&T` or `&mut T`) |
| Move | "transfer ownership" | Assignment that invalidates the source binding |

## Further Reading

- [Rust Ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [Linear Haskell](https://ghc.gitlab.haskell.org/ghc/doc/users_guide/exts/linear_types.html)
- [Linear Types Can Change the World!](https://homepages.inf.ed.ac.uk/wadler/papers/lineartaste/lineartaste-revised.pdf)
- [Rustonomicon](https://doc.rust-lang.org/nomicon/)
