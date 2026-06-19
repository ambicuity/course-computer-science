# Pointers, Addresses, and Indirection (in C)

> A pointer is an *address with a type*. The type tells the compiler what's there and how big to step. Get the mental model right and every C bug you've ever feared becomes diagnosable.

**Type:** Build
**Languages:** C
**Prerequisites:** Phase 02, Lessons 02, 04
**Time:** ~75 minutes

## Learning Objectives

- Distinguish *address* (a number identifying a byte in memory) from *pointer* (an address with a *type*).
- Use `&` (address-of), `*` (dereference), `[i]` (subscript = `*(p + i)`), `.` vs `->` (struct member access) correctly.
- Apply pointer arithmetic: `p + 1` advances by `sizeof(*p)` bytes, not one byte; this is why `int *` and `char *` behave differently when added to.
- Recognize and avoid the canonical pointer bugs: null deref, dangling pointer, double free, out-of-bounds read/write, type-punning UB.

## The Problem

Pointers are where most "real" CS work happens — linked lists, trees, graphs, system calls, kernel memory, FFI. They're also where most security vulnerabilities live (~70% of Microsoft's and Google's high-severity CVEs are memory-safety bugs).

The C textbook treats `int *p = &x;` as syntax. This lesson treats it as: a *typed view* into a specific memory cell, with rules about what you can do with that view, and what happens when you break them.

## The Concept

### Address vs pointer

An **address** is just a number, the index of a byte in the process's virtual address space:

```
   0x7ffd_3210_0000   ←  some byte in memory
```

A **pointer** is an address *plus* a static type. The type tells the compiler:

- How many bytes to read on `*p` (= `sizeof(*p)`).
- How many bytes to step on `p + 1` (= `sizeof(*p)`).
- What operations are legal (you can dereference an `int *` but not a `void *` directly).

Two pointers with different types can hold the same numerical address but mean different things:

```c
int *pi = (int *)0x1000;        /* "read 4 bytes starting at 0x1000 as an int" */
char *pc = (char *)0x1000;      /* "read 1 byte starting at 0x1000 as a char" */
```

### The four core operators

| Operator | What it does | Example |
|----------|--------------|---------|
| `&x`   | "Address of x" — produces a `T *` if x has type T | `int *p = &x;` |
| `*p`   | "Value at p" — produces a `T` if p has type `T *` | `int v = *p;` |
| `p[i]` | Subscript — exactly `*(p + i)` | `arr[3]` ≡ `*(arr + 3)` |
| `p->m` | Member m of struct pointed to by p — `(*p).m` | `node->next` |

### Pointer arithmetic

`p + 1` adds `sizeof(*p)` bytes to the address:

```c
int *pi = ...;        /* sizeof(int) = 4 */
pi + 1;               /* address 4 bytes higher */

char *pc = ...;       /* sizeof(char) = 1 */
pc + 1;               /* address 1 byte higher */
```

This is why `arr[i] = *(arr + i)` works for any element type — the multiplication by element size is baked in.

`p - q` is also defined when both point into the same array; it returns a count of elements, of type `ptrdiff_t` (= `int64_t` on 64-bit Linux).

### Void pointers

`void *` is "address with no type." You can store any object-pointer in a `void *` but can't dereference or do arithmetic on it without casting. Used by `malloc`, generic-container interfaces, `memcpy`/`memset`.

```c
void *buf = malloc(100);
int *intview = (int *)buf;
intview[3] = 42;
```

### Function pointers

A function pointer is the address of code. Type carries the signature:

```c
int (*fn)(int, int) = &add;   /* or just = add; */
int r = fn(3, 4);              /* or (*fn)(3, 4); same thing */
```

Used for callbacks (`qsort`'s compare), virtual dispatch (manual vtables in C), and JIT.

### The classic bug classes

| Bug | Symptom | Mitigation |
|-----|---------|------------|
| **Null deref** | `*p` when `p == NULL` | Always check pointers from APIs that can fail (malloc, fopen). Modern compilers + `-fsanitize=null` catch many |
| **Dangling pointer** | `*p` after the pointee freed/returned | Set pointer to `NULL` after free. Use static analyzers, ASan |
| **Double free** | `free(p); free(p);` | Set p = NULL after first free; modern allocators detect via tagging |
| **Out-of-bounds** | `arr[i]` where i ≥ len(arr) | Pass length alongside pointer; use `-fsanitize=address` |
| **Use-after-free** | Pointee is freed but pointer still used | ASan; Rust borrow checker prevents at compile time |
| **Type punning UB** | Casting `int *` to `float *` and dereferencing | Use `memcpy` or unions instead |

### Strict aliasing

The C standard requires that an object be accessed only through pointers of its declared type (or a few exceptions: `char *`, the signed/unsigned counterpart). Violations are **undefined behavior** and modern optimizers exploit this aggressively. Use `memcpy` to convert between types portably:

```c
float f = 1.5f;
uint32_t bits;
memcpy(&bits, &f, sizeof(bits));   /* well-defined, optimizes to a register move */
```

NOT:

```c
uint32_t bits = *(uint32_t *)&f;   /* UB; may "work" or may produce garbage */
```

## Build It

Open `code/main.c`. The file exercises address-of/deref, pointer arithmetic, subscript-as-sugar, function pointers, and (under `--oob`) an intentional bug for ASan to catch.

### Step 1: Address-of and dereference

Confirm `*p == x` and `&x == p`.

### Step 2: Pointer arithmetic varies by type

`(p + 1) - p` measured in bytes equals `sizeof(*p)`, NOT 1.

### Step 3: Subscript ≡ pointer arithmetic

`arr[2]` and `*(arr + 2)` produce the same value. Bonus: `2[arr]` is also legal C (and yields the same value), because subscript is commutative addition under the hood.

### Step 4: Function pointers and callbacks

Pass `add` and `mul` to a `combine` function expecting `int (*fn)(int, int)`.

### Step 5: Pointer-bug demos

Compile with `-fsanitize=address` and run with `--oob` to see ASan catch a 1-byte overflow.

## Use It

- **Every data structure** in Phase 03 is a graph of pointers.
- **System calls** pass user pointers to the kernel; the kernel must validate every read/write (`copy_from_user`).
- **FFI** between languages (Rust/Python ↔ C) passes pointers — type metadata must match.
- **Garbage-collected languages** (Java, Go) hide pointers but still have them under the hood (object references). Their GC walks the same pointer graph C programmers walk by hand.

## Read the Source

- *Expert C Programming* by Peter van der Linden — Chapter 4 on pointers is unmatched.
- *The C Programming Language* (K&R), Chapter 5 — definitive intro.
- [LWN's "Use-after-free" series](https://lwn.net/Articles/734081/) — production-grade study of these bugs in the Linux kernel.

## Ship It

This lesson ships **`outputs/ptr_safety.h`** — a small header with macros like `SAFE_FREE(p) do { free(p); (p) = NULL; } while(0)` and `BOUNDS_CHECK(arr, i, len)` to catch the common bugs at runtime.

## Exercises

1. **Easy.** Write a function that swaps two `int` values via pointers. Verify on `a=1, b=2`.
2. **Medium.** Implement a generic `void *` `memcpy` from scratch in 10 lines. Compare to libc's optimized version on a 1-MB buffer.
3. **Hard.** Use `mmap` to allocate a guard page right after a 4 KB allocation; demonstrate that reading one byte past the allocated region triggers SIGSEGV — exactly how ASan does its bounds-checking.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Pointer | "Memory address" | A typed value identifying a byte in the address space, with rules about how to step and dereference |
| Dereference | "Following the pointer" | The `*p` operator: read or write the object the pointer points to |
| Pointer arithmetic | "Stepping by bytes" | `p + i` advances by `i * sizeof(*p)` bytes, NOT i bytes; element-typed |
| Dangling pointer | "Stale pointer" | A pointer whose pointee has been freed or gone out of scope; reading is UB |
| Strict aliasing | "Type compatibility rule" | C requires accessing an object only via its declared type's pointer (with explicit exceptions); violations are UB |

## Further Reading

- [The strict-aliasing rule](https://gist.github.com/shafik/848ae25ee209f698763cffee272a58f8) — Shafik Yaghmour's clear writeup.
- [What every programmer should know about memory](https://lwn.net/Articles/250967/) — Ulrich Drepper; doesn't replace this lesson but expands on cache effects of pointer-heavy code.
- *Modern C* by Jens Gustedt — chapter on pointers updated for C11/C17.
