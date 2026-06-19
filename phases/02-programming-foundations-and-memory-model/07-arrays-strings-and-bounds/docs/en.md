# Arrays, Strings, and Bounds

> A C array isn't a "thing." It's a `(pointer, length)` pair the language gives you only half of. Half the security bugs in CS come from forgetting the other half.

**Type:** Build
**Languages:** C, Rust
**Prerequisites:** Phase 02, Lessons 02, 05
**Time:** ~60 minutes

## Learning Objectives

- Distinguish a C *array* (fixed-size, type-aware in its declaring scope) from a *pointer* (no length info).
- Explain *array decay*: when an array name is passed to a function, it becomes a pointer to its first element.
- Identify and avoid the canonical string bugs: missing null terminator, `strcpy` buffer overflow, off-by-one with `strlen`, mixed-up byte vs char length.
- Use Rust's slice type (`&[T]`, `&str`) — a `(pointer, length)` pair the language enforces — to see what C is missing.

## The Problem

C's array model is uniquely fragile. Three traps every C programmer eventually hits:

1. `void foo(int arr[10])` — looks like a 10-element array but is actually `int *arr`. `sizeof(arr) == sizeof(void*)`.
2. `strcpy(dst, src)` — copies until null. If `dst` is smaller than `src`, you've overflowed.
3. `char buf[64]; gets(buf);` — the function that, single-handedly, motivated removing functions from the C standard. Reads arbitrarily long input into a 64-byte buffer.

These bugs are responsible for an astounding fraction of CVEs (Common Vulnerabilities and Exposures). Microsoft estimates ~70% of security bugs in C/C++ code are memory-safety issues. This lesson treats them seriously.

## The Concept

### Arrays vs pointers

In C:

```c
int arr[10];           /* array type. sizeof(arr) == 40 (= 10 * sizeof(int)) */
int *p = arr;          /* pointer to first element. sizeof(p) == 8 (= sizeof(void*)) */
```

In the *declaring scope*, `arr` knows its size. In a function parameter, `int arr[]` and `int *arr` are equivalent — the size is dropped:

```c
void foo(int arr[10]) {
    printf("%zu\n", sizeof(arr));   /* prints 8, not 40 */
}
```

This is **array decay**. The conventional fix is to always pass length explicitly:

```c
void foo(int *arr, size_t n);     /* clear contract */
```

### C strings

A C string is a contiguous run of `char`s terminated by `'\0'`. The string `"hello"` occupies 6 bytes: `h e l l o \0`.

| Function | What it does | Hazard |
|----------|--------------|--------|
| `strlen(s)` | Count bytes until `\0`, exclusive | O(n); UB if no null in the buffer |
| `strcpy(dst, src)` | Copy until and including `\0` | Buffer overflow if dst smaller than src |
| `strcat(dst, src)` | Find dst's `\0`; append src | Same hazard, doubled |
| `strncpy(dst, src, n)` | Copy at most n bytes | Does NOT guarantee null terminator if src ≥ n bytes |
| `snprintf(dst, size, fmt, ...)` | Safe formatted print | Returns the *would-be* length; check it |

The safer functions: `strlcpy`/`strlcat` (BSD; also on macOS/Linux via libbsd), `memcpy_s` (Annex K — uneven support), or just always-bound-checked C code.

### Bounds errors are silent

C does no runtime bounds checking. Writing `arr[100]` when `arr` is 10 elements just writes 360 bytes past the buffer's end — silently corrupting whatever's there. Detection requires:

- **Compile-time**: warnings only catch simple cases.
- **Runtime sanitizers** (`-fsanitize=address`): catch via shadow memory + redzones.
- **Discipline**: explicit length-tracking, length-checked accessors, container abstractions.

### Rust's slice: the missing half

Rust replaces C's bare pointers with **slices**:

```rust
let arr = [10, 20, 30, 40, 50];
let s: &[i32] = &arr;       // slice: (pointer, length=5)
println!("len = {}", s.len());     // 5
println!("first = {}", s[0]);      // 10
println!("oob  = {}", s[10]);      // PANIC at runtime — bounds-checked
```

`&[T]` is a "fat pointer" — a pair (data pointer, length). Indexing is bounds-checked. The compiler turns out-of-bounds at compile time when it can; at runtime when it must.

`&str` is the same thing for strings — a slice of UTF-8 bytes. Rust strings carry length explicitly; no null terminators.

### The string-length swamp

For Unicode-aware text:

- **Byte length**: bytes in the buffer. Useful for allocation.
- **Char length** (UTF-8): number of "scalar values" / codepoints.
- **Grapheme length**: human-perceived characters; "👨‍👩‍👧" is one grapheme but many codepoints.

Most APIs that say "length" mean one or the other. Mixing them is a common bug source. Rust's `s.len()` is byte length; `s.chars().count()` is codepoint count.

## Build It

Open `code/main.c` and `code/main.rs`.

### Step 1: Array decay in action

A function takes `int arr[10]` and prints `sizeof(arr)` inside vs outside. Outside: 40 (= 10 × sizeof(int)). Inside: 8 (size of a pointer).

### Step 2: Off-by-one with strlen

For a 4-char string in a 5-byte buffer, `strlen == 4` and `sizeof == 5`. Mixing these is a classic bug.

### Step 3: strcpy overflow (under ASan)

`strcpy(dst, src)` into a too-small dst is UB. ASan reports stack-buffer-overflow.

### Step 4: snprintf the right way

Truncates safely; returns the *would-be* length so you can detect truncation.

### Step 5: Rust slice safety

`arr[10]` on a 5-element array panics in Rust; reads garbage in C.

## Use It

- **Every C library you'll write or maintain**: pass `(ptr, len)` instead of bare pointers.
- **CVE triage**: read CVE descriptions for memory-safety issues; you'll start to recognize the patterns.
- **Bindings between languages**: a C function `void foo(char *s)` is ambiguous about ownership and length; Rust's `extern "C"` bindings often replace it with `(*const c_char, usize)` or convert via CString.
- **Data formats**: HTTP, JSON, Protobuf — each has explicit length-prefixed fields. C strings are an outlier in choosing the null-terminator approach.

## Read the Source

- *C: A Reference Manual* by Harbison & Steele — definitive treatment of array decay.
- [Microsoft's safe-strings APIs (Annex K)](https://en.cppreference.com/w/c/string/byte) — controversial but instructive.
- Rust slice docs: [https://doc.rust-lang.org/std/primitive.slice.html](https://doc.rust-lang.org/std/primitive.slice.html)

## Ship It

This lesson ships **`outputs/strlcpy.c`** — a portable implementation of OpenBSD's `strlcpy` and `strlcat`, plus tests showing they truncate safely where `strcpy`/`strcat` would overflow.

## Exercises

1. **Easy.** Write a function `int sum(int *arr, size_t n)` that sums the first n elements. Why is the size_t parameter essential?
2. **Medium.** Write `safe_strcpy(char *dst, size_t dst_size, const char *src)` that copies up to `dst_size - 1` bytes and always null-terminates. Compare with `strncpy`'s subtle "may not terminate" footgun.
3. **Hard.** Write a function that takes a UTF-8 byte buffer and returns the count of grapheme clusters using a state machine over UTF-8 code-point boundaries. (Or use a library; understand what it's doing.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Array decay | "Array becomes a pointer" | When an array is used as an rvalue (e.g., passed to a function), it decays to `T *` pointing at its first element; size info is lost |
| C string | "A char array" | Bytes terminated by `'\0'`; length is implicit (must be scanned with strlen) |
| Buffer overflow | "Writing past the end" | Writing to memory past the bounds of a buffer; UB and a security vulnerability |
| Slice (Rust) | "Fat pointer" | `(pointer, length)` pair; carries the missing half C arrays drop |
| Off-by-one | "Forgot the null terminator" | Allocating exactly N bytes for an N-char string; forgetting the trailing `\0` |

## Further Reading

- *Smashing The Stack For Fun And Profit* (Aleph One, 1996) — the classic exploitation paper.
- [The strlcpy/strlcat case study](https://www.usenix.org/legacy/event/usenix99/full_papers/millert/millert.pdf) — design rationale.
- *Secure Coding in C and C++* by Robert Seacord — definitive reference.
