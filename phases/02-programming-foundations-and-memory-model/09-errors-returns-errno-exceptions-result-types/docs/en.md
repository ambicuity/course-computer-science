# Errors — Returns, errno, exceptions, Result types

> Every language picks an error model. The choice is rarely about "best" — it's about which failure modes you make easy to handle and which you make easy to ignore.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** Phase 02, Lessons 03, 09
**Time:** ~60 minutes

## Learning Objectives

- Compare the four major error-handling models in production languages: C return codes + errno, C++/Python/Java exceptions, Go multi-return, Rust `Result<T, E>` / `Option<T>`.
- Identify each model's failure mode: silent error ignoring (C), invisible non-local jumps (exceptions), forgotten checks (Go), `unwrap()` panics (Rust).
- Write robust error-handling code in C using `errno` and proper return-value checks; in Rust using `?` operator and `Result` propagation.
- Recognize when *panic* (abort) is appropriate vs returning a recoverable error.

## The Problem

A file open call can fail. A network read can fail. A `malloc` can fail. A user can supply a malformed input. How your language handles these *fallible* operations determines:

- How easy it is to ignore an error (the friendlier the model, the more likely the bug).
- How obvious the error path is in the source code.
- Whether resources (file handles, locks, memory) get cleaned up when an error happens.
- How expensive the happy path is (exceptions add zero overhead until thrown; errno costs a register read every syscall).

This lesson shows you four models side by side.

## The Concept

### Model 1: Return codes + errno (C, POSIX)

Most POSIX functions return a sentinel value (often -1 or NULL) on error and set the thread-local global `errno`:

```c
int fd = open("file.txt", O_RDONLY);
if (fd < 0) {
    fprintf(stderr, "open: %s\n", strerror(errno));   /* "No such file or directory" */
    return 1;
}
```

| Pro | Con |
|-----|-----|
| Zero overhead on success | Easy to forget the check |
| Compatible with C ABIs (no exceptions to unwind) | `errno` is global state, easy to clobber |
| Explicit | No exhaustiveness check; the compiler doesn't force you to handle each error |

### Model 2: Exceptions (C++, Python, Java, JS)

```python
try:
    f = open("file.txt")
    data = f.read()
except FileNotFoundError as e:
    print(f"could not open: {e}")
```

| Pro | Con |
|-----|-----|
| Non-error code stays clean | Non-local control flow — surprising to readers |
| Exception types carry rich context | Resource cleanup needs RAII / `with`/`try-finally` |
| Compilers can generate "zero-cost" exception tables (C++) | Runtime cost when thrown; some embedded contexts forbid them |

Exception safety has its own discipline — *strong* vs *basic* guarantee, no-throw guarantees, etc. (Phase 16 covers this for design.)

### Model 3: Multi-return (Go)

```go
data, err := os.ReadFile("file.txt")
if err != nil {
    return fmt.Errorf("could not read: %w", err)
}
```

Every fallible function returns `(T, error)`. The convention is to handle the error on the line after the call.

| Pro | Con |
|-----|-----|
| Explicit — no hidden control flow | Verbose — `if err != nil` everywhere |
| Errors are values; you can wrap, compare, log them | Easy to forget to check (the compiler doesn't always force you) |

### Model 4: `Result<T, E>` / `Option<T>` (Rust, Haskell, OCaml)

```rust
let data: Result<Vec<u8>, _> = std::fs::read("file.txt");
match data {
    Ok(bytes) => println!("{} bytes", bytes.len()),
    Err(e)    => eprintln!("error: {}", e),
}
```

A fallible function returns `Result<T, E>` — a *sum type* that's either `Ok(value)` or `Err(error)`. The compiler forces you to handle both arms. `Option<T>` is the same idea for "value or no value."

The `?` operator propagates errors upward in one character:

```rust
fn parse_config() -> Result<Config, io::Error> {
    let data = std::fs::read("config")?;      // returns the Err on failure
    let cfg  = parse_bytes(&data)?;
    Ok(cfg)
}
```

| Pro | Con |
|-----|-----|
| Compiler-enforced exhaustiveness | `?` requires error types to be compatible (use `thiserror`/`anyhow` for convenience) |
| Composable: `?`, `and_then`, `map`, `?`-chain | Pattern-matching can be verbose without `?` |
| No hidden control flow | Forces the API to be honest — every fallible thing is a `Result` |

### When to panic

Some failures are *unrecoverable* — a violated invariant, an inconsistent state. Then *panic*:

- C: `abort()`, `assert`.
- Rust: `panic!()`, `unwrap()`, `expect()`.

Rule of thumb: panic for *bugs in your code*; return errors for *expected failure modes* (network down, file missing, parse error).

### Resource cleanup

Errors that skip cleanup are leaks. Patterns:

- **C**: goto-cleanup chains, or `cleanup` attribute (gcc/clang).
- **C++**: RAII — destructors run on stack unwind.
- **Python**: `with` blocks (context managers).
- **Go**: `defer`.
- **Rust**: RAII — `Drop::drop` runs at scope exit, also on panic-unwind.

## Build It

Open `code/main.c` and `code/main.rs`.

### Step 1: C return codes + errno

Open a non-existent file; `errno` is set; `strerror(errno)` prints the human message.

### Step 2: C goto-cleanup pattern

A function that allocates resources sequentially uses a single exit point + `goto cleanup_N` labels — the canonical C way to avoid leak-on-error.

### Step 3: Rust `Result` + `?`

A function chains three fallible operations; `?` propagates each error to the caller in one character.

### Step 4: Rust `Option` for "may not exist"

`HashMap::get` returns `Option<&V>`. Handle both arms.

### Step 5: When to panic vs return

A division function returns `Result<f64, DivideError>` (recoverable). An array-index check `arr[i]` for known-valid `i` *panics* if out of bounds (a bug, not an expected failure).

## Use It

- **All POSIX system calls** return -1 + errno. Reading them correctly is half of writing reliable C.
- **All Rust standard-library fallible ops** return `Result`. The `?` operator is what makes that ergonomic.
- **HTTP/RPC handlers** typically convert internal errors into HTTP status codes — a translation layer between the two models.
- **Database drivers** wrap underlying errors into typed enums; your DAO layer matches on them.

## Read the Source

- *The Rust Programming Language*, Chapter 9 (Error Handling).
- [Rust API Guidelines on error types](https://rust-lang.github.io/api-guidelines/type-safety.html#error-types-c-good-err).
- *Go: The Complete Reference* — Chapter on `error` and `defer`.
- [Go's "Error handling and Go"](https://go.dev/blog/error-handling-and-go) — Andrew Gerrand's design rationale.

## Ship It

This lesson ships **`outputs/c_errors.h`** — macros for the C goto-cleanup pattern and for setting/restoring errno across cleanups.

## Exercises

1. **Easy.** In C, write a function that opens a file, reads first N bytes, closes the file. Use a goto-cleanup label so the file is closed on every error path.
2. **Medium.** In Rust, write a function returning `Result<u32, ParseError>` that reads a file, parses it as a number, validates `n < 1000`. Use `?` and a custom `enum ParseError`.
3. **Hard.** Compare exception cost: in C++, write a function that calls a deeply nested function which throws. Measure throw-and-catch latency under `-O2`. (Typical: 1-10 µs for the throw.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| `errno` | "Global error variable" | Thread-local int set by failing POSIX calls; read via `strerror(errno)` for a message |
| Exception | "Throws an error" | Non-local control transfer up the call stack to the nearest matching catch; bypasses normal return |
| `Result<T, E>` | "Either / sum type" | A value that is either `Ok(T)` or `Err(E)`; compiler forces handling both arms |
| `?` operator (Rust) | "Try / propagate" | Returns the `Err` to the caller if a `Result` is `Err`; unwraps to the value if `Ok` |
| Panic / abort | "Crash" | Process-terminating error; reserved for bugs/violated invariants, not expected failures |

## Further Reading

- *Joel Spolsky's "Making Wrong Code Look Wrong"* — informally argues for explicit error markers in the type system.
- *Why use `Result<T, E>` rather than exceptions* — Niko Matsakis's Rust design rationale on the Rust team blog.
- [Errors as values, in Go](https://go.dev/blog/errors-are-values) — Rob Pike's classic essay.
