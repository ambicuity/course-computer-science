# C Macro Pitfalls — and What to Use Instead

Six classic ways C function-like macros go wrong, with safer alternatives.

## 1. Missing parens around arguments

```c
#define SQ(x) x * x

SQ(3 + 4)   // expands: 3 + 4 * 3 + 4 = 19   ← wrong!
```

**Fix:** parenthesize every macro argument when used.

```c
#define SQ(x) ((x) * (x))
```

## 2. Missing parens around the whole expression

```c
#define ADD(a, b) (a) + (b)

ADD(1, 2) * 5   // expands: (1) + (2) * 5 = 11   ← wrong!
```

**Fix:** wrap the whole expression too.

```c
#define ADD(a, b) ((a) + (b))
```

## 3. Multiple evaluation of arguments

```c
#define MAX(a, b) ((a) > (b) ? (a) : (b))

int i = 0;
MAX(i++, 5);    // i incremented twice if i > 5
```

**Fix (GCC/Clang):** statement expressions with typeof.

```c
#define MAX(a, b) ({              \
    __typeof__(a) _a = (a);       \
    __typeof__(b) _b = (b);       \
    _a > _b ? _a : _b;            \
})
```

Or just use `static inline int max(int a, int b) { ... }` — no macro needed.

## 4. Comma trap inside arguments

```c
#define DECL(t, name) t name

DECL(struct { int a, b; }, x);   // breaks: comma inside the brace is parsed as macro-arg separator
```

**Fix:** wrap in parens / typedef the type first.

```c
typedef struct { int a, b; } Pair;
DECL(Pair, x);
```

## 5. Dangling-else trap with multi-statement macros

```c
#define LOG_IF(cond, msg) if (cond) fputs(msg, stderr)

if (x)
    LOG_IF(y, "...");
else
    do_other();
```

The `else` binds to the macro's `if`, not the outer.

**Fix:** wrap with `do { ... } while (0)`.

```c
#define LOG_IF(cond, msg) do {        \
    if (cond) fputs(msg, stderr);     \
} while (0)
```

This forces the macro to behave as a single statement requiring a trailing `;`.

## 6. Token clashes / shadowing

Macros aren't hygienic. A locally-named `_t` in your macro can collide with a `_t` in the surrounding code.

**Fix:** Use unusual prefixes (`__macro_t`) or, again, prefer `static inline` functions.

## When to use macros anyway

- `#ifdef`, `#define` for conditional compilation.
- Token pasting (`##`) — no C equivalent.
- Per-call site capture: `__FILE__`, `__LINE__`, `__func__`.
- X-macros — keep parallel lists in sync from a single source-of-truth file.

Otherwise default to `static inline` functions or, in C++/Rust, real generics.
