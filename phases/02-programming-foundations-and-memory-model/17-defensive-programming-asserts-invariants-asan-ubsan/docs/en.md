# Defensive Programming — Asserts, Invariants, ASAN/UBSAN

> The cheapest bug is the one that crashes loudly the second it's created. Assertions and sanitizers let you set up that loud crash everywhere it matters.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** Phase 02, Lessons 05, 06, 09
**Time:** ~60 minutes

## Learning Objectives

- Use C's `assert` and Rust's `debug_assert!`/`assert!` macros to express invariants in code.
- Compile C/C++ with `-fsanitize=address` (ASAN) and `-fsanitize=undefined` (UBSAN); recognize their typical output and what each catches.
- Apply the practice of documenting pre/postconditions and class invariants with assertions.
- Decide between debug-only (cheap, instrumented) and release (production, asserts disabled) builds — and when to keep checks in release.

## The Problem

A bug detected at the moment it's introduced costs minutes to fix. The same bug detected later — corrupted state, a customer complaint, a hard-to-reproduce crash — costs days. Defensive programming is the practice of arranging for bugs to fail loudly and immediately:

- **Assert preconditions** at function entry: validate args.
- **Assert invariants** mid-function: after a complex update, sanity-check the data structure.
- **Assert postconditions** at function exit: did the function deliver what it promised?
- **Run sanitizer-instrumented binaries** in CI: catch memory bugs and UB before merge.

These tools shift bug-finding *earlier* in the dev cycle — when it's cheapest.

## The Concept

### `assert` in C

```c
#include <assert.h>

void process(const int *arr, size_t n) {
    assert(arr != NULL);
    assert(n > 0);
    /* ... */
}
```

If the condition is false, `assert` prints a message and `abort()`s the process. With `-DNDEBUG` (release build), all `assert(x)` compile to nothing — zero cost.

Best practice:
- Express preconditions / invariants / postconditions, not user-facing validation. (User input gets explicit error returns; assertion failure means programmer error.)
- Keep assertions side-effect-free. `assert(do_work() == 0)` skips the call in release builds.

### Rust assertions

| Macro | When checked | Use for |
|-------|--------------|---------|
| `assert!(cond)` | Always (debug + release) | Critical safety invariants |
| `debug_assert!(cond)` | Debug builds only | Cheap sanity checks (analogous to C's assert) |
| `panic!("msg")` | Always | Unrecoverable error / unreachable branch |
| `unreachable!()` | Always | A branch the type system can't prove dead but you know is |

Rust doesn't have `NDEBUG`-style suppression; `debug_assert!` is genuinely disabled in `--release`.

### Address Sanitizer (ASAN)

Compile with `-fsanitize=address` (gcc / clang) — produces a binary that, on every load/store, checks against a shadow memory map. Catches:

- **Heap buffer overflow** (write past `malloc`'s returned region).
- **Use-after-free**.
- **Double-free**.
- **Stack buffer overflow** (in many cases).
- **Memory leaks** at process exit.

Overhead: ~2× slower, ~3× more memory. Use in dev/CI; not for production hot paths (though some production systems do run it as a slow but very-safe canary fleet).

```sh
gcc -fsanitize=address -O0 -g main.c -o main_asan
./main_asan
# On a buffer overflow, ASAN aborts with a detailed report:
# ==12345==ERROR: AddressSanitizer: heap-buffer-overflow ...
```

### Undefined Behavior Sanitizer (UBSAN)

`-fsanitize=undefined` instruments for UB:

- Integer overflow (signed).
- Out-of-bounds array index (with `-fsanitize=bounds`).
- Null pointer dereference (with `-fsanitize=null`).
- Misaligned load / store.
- Invalid shift (e.g., `x << 32` on a 32-bit type).
- Reaching the end of a non-void function without `return`.

Overhead: ~5-20% — cheap enough for some production deployments.

### LeakSanitizer (LSAN)

`-fsanitize=leak` (often included with ASAN). At process exit, reports any heap allocations that weren't freed.

### Thread Sanitizer (TSAN)

`-fsanitize=thread` — instruments every memory access to detect data races. Won't be covered here (Phase 13).

### Rust's equivalents

Rust avoids most UB at compile time. For the remainder:

- **Miri** — an interpreter for Rust's MIR that detects undefined behavior in unsafe code.
- **Loom** — model checker for concurrency primitives.
- **`#[deny(unsafe_op_in_unsafe_fn)]` and clippy lints** — static.

### Debug vs Release

| Mode | Asserts | Sanitizers | Optimizations |
|------|---------|------------|---------------|
| Dev / debug | All on | Compose freely | -O0 + -g |
| CI | All on + sanitizers | ASAN, UBSAN, LSAN, TSAN | -O1 + -g (sanitizers prefer some opt) |
| Production | Critical asserts only | UBSAN sometimes; ASAN rarely | -O2 / -O3 |

The split is deliberate: aggressive checks slow dev velocity; full opt hides bugs. CI gives you the best of both — slow, thorough, before merge.

## Build It

Open `code/main.c` and `code/main.rs`.

### Step 1: C assertion preventing a null-deref

A function `sum(int *arr, size_t n)` asserts `arr != NULL` at entry.

### Step 2: A `debug_assert!` in Rust

A function `binary_search` asserts the array is sorted in debug mode only.

### Step 3: ASAN catches a heap-buffer-overflow

Code that writes past a `malloc`'d region.

### Step 4: UBSAN catches signed overflow

`int x = INT_MAX; x + 1;` is UB. UBSAN reports it.

### Step 5: LSAN catches a leak

A `malloc` without a `free`; at exit, LSAN reports the leak's allocation backtrace.

## Use It

- **Library design**: every public function asserts its preconditions. Callers get a loud crash on misuse instead of corrupted internals.
- **Data-structure invariants**: a hash table asserts `count <= capacity`; a sorted array asserts monotonicity after every mutation.
- **CI pipelines**: build a sanitizer variant in addition to release; run the test suite under it; reject PRs with sanitizer errors.
- **Property tests** (Phase 17): generate random inputs and check that asserted invariants hold.

## Read the Source

- *Writing Solid Code* by Steve Maguire — the classic on assertion-driven development.
- [The ASAN paper (Serebryany et al., USENIX 2012)](https://www.usenix.org/conference/atc12/technical-sessions/presentation/serebryany) — clear architecture description.
- Rust [Miri docs](https://github.com/rust-lang/miri) — UB detection for unsafe Rust.

## Ship It

This lesson ships **`outputs/defensive.h`** — a header with `REQUIRE(cond)`, `ENSURE(cond)`, `INVARIANT(cond)`, `UNREACHABLE()` macros that compile to assertions in debug and to `__builtin_unreachable()` hints in release.

## Exercises

1. **Easy.** Take Phase 02 Lesson 06's allocator. Add assertions: pool_free's `obj` must be inside the pool's slab range; double-free triggers an assertion failure.
2. **Medium.** Compile a small C program with `-fsanitize=address,undefined,leak` and run a test suite. Fix every issue the sanitizers find.
3. **Hard.** Add `debug_assert!` invariants to a binary-search-tree implementation: after every insert/delete, traverse and verify that every node satisfies BST order + height balance. The check is O(n); the assertion ensures the tree is consistent at every operation in debug builds.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Assertion | "Sanity check" | A condition expected to be true at a point in execution; aborts if not |
| Invariant | "Always-true property" | A property of a data structure that holds before and after every operation |
| ASAN | "Address sanitizer" | Compiler instrumentation that catches memory errors (overflow, UAF) at runtime |
| UBSAN | "Undefined behavior sanitizer" | Compiler instrumentation that catches signed overflow, null deref, etc. |
| NDEBUG | "Release flag" | C preprocessor macro that, when defined, disables assert(); typically set in release builds |

## Further Reading

- *The Pragmatic Programmer* by Hunt & Thomas — assertion-as-design chapters.
- *Why Are Computers So Hard To Use?* — Tony Hoare's talks on invariants in software.
- [Google's "Sanitizing C++" guide](https://github.com/google/sanitizers) — production deployment tips.
