# Bits, Bytes, Two's Complement, IEEE 754

> Every number, letter, and instruction in your computer is a pattern of bits. How those patterns are interpreted determines whether your program is correct or subtly broken.

**Type:** Learn
**Languages:** C, Python
**Prerequisites:** Phase 05
**Time:** ~90 minutes

## Learning Objectives

- Convert fluently between binary, hexadecimal, and octal. Explain why byte = 8 bits and what 16/32/64-bit word sizes mean.
- Negate any signed integer using two's complement, detect overflow, and sign-extend correctly.
- Parse and construct IEEE 754 single- and double-precision floats from raw bits.
- Use bitwise operations (AND, OR, XOR, NOT, shifts, rotation) to solve real problems.

## The Problem

This lesson sits in **Phase 06 — Digital Logic & Computer Architecture**. Without the concept it teaches, you cannot build the phase's capstone (A 5-stage pipelined RISC-V CPU in HDL with assembler.). Concretely, *not* knowing this means you get stuck the moment you try to walk down from instruction to transistor, then back up: alu, pipeline, cache, mmu.

Every ALU operation, every memory load/store, every branch decision in the CPU you'll build operates on bit patterns. This lesson gives you the number systems and encoding rules that make the hardware legible.

## The Concept

### Binary, Hex, Octal

A **bit** is a 0 or 1. **Byte** = 8 bits (256 values) — the smallest addressable unit on most architectures. **Word sizes** (register width): 16-bit (embedded), 32-bit (ARM32), 64-bit (x86-64, AArch64).

**Hex** groups 4 bits per digit: `0xBEEF` = `1011 1110 1110 1111` = 48879. **Octal** groups 3 bits: `0755` = `111 101 101`. Hex is the standard shorthand; octal survives in Unix permissions.

### Two's Complement

Every major architecture uses two's complement because **addition and subtraction use the same hardware regardless of sign**. The ALU doesn't know or care about sign — the circuitry is identical.

**Representation** for n-bit signed: range **-2^(n-1) to 2^(n-1) - 1** (8-bit: -128 to 127; 32-bit: -2,147,483,648 to 2,147,483,647).

**Negation** = invert + 1:

```
 5 = 0000 0101
~5 = 1111 1010
~5+1 = 1111 1011 = -5    (verify: 0000 0101 + 1111 1011 = (1)0000 0000 ✓)
```

**The asymmetry:** -2^(n-1) has no positive counterpart. `~(-128) + 1 = -128` — negating the most negative number overflows back to itself.

**Overflow detection:** Signed overflow = same-sign operands produce opposite-sign result. Unsigned overflow = carry-out of MSB. These are different; confusing them is a common bug.

```
  0111 1111  (+127)
+ 0000 0001  (+1)
= 1000 0000  (-128)  ← positive + positive = negative: overflow
```

**Sign extension:** Widening copies the sign bit leftward, preserving the value: `1111 1011` (-5 in 8-bit) → `1111 1111 1111 1011` (-5 in 16-bit).

### IEEE 754 Floating Point

Represents a number as: `value = (-1)^sign × 1.mantissa × 2^(exponent - bias)`

**Single precision (32-bit):** 1 sign + 8 exponent (bias 127) + 23 mantissa bits.

**Double precision (64-bit):** 1 sign + 11 exponent (bias 1023) + 52 mantissa bits.

**Worked example:** `-6.75f = 110.11₂ = 1.1011 × 2²`. Sign=1, Exponent=2+127=129=0x81, Mantissa=101100... → `1 10000001 10110000000000000000000` = `0xC0D80000`.

**Special values:**

| Exponent | Mantissa | Meaning |
|----------|----------|---------|
| 0 | 0 | ±zero |
| 0 | ≠0 | Denormalized (subnormal) |
| 255 (max) | 0 | ±∞ |
| 255 | ≠0 | NaN |

**Denormalized numbers** fill the gap near zero: exponent field = 0 means the implicit leading bit is 0 (not 1), providing gradual underflow instead of a cliff to zero.

**Precision vs range:** float = ~7 digits / ±3.4×10^38; double = ~15 digits / ±1.8×10^308. More mantissa bits = precision; more exponent bits = range. Can't have both in fixed bits.

**The golden rule:** Never `==` compare floats. Decimal `0.1` has no exact binary representation; accumulated rounding makes equality fragile.

### Bitwise Operations

| Operation | C / Python | Effect |
|-----------|-----------|--------|
| AND | `a & b` | 1 only if both inputs 1 |
| OR | `a \| b` | 1 if either input 1 |
| XOR | `a ^ b` | 1 if inputs differ |
| NOT | `~a` | Flip every bit |
| Left shift | `a << n` | Multiply by 2^n, fill 0s |
| Logical right | `(unsigned)a >> n` | Divide by 2^n, fill 0s |
| Arithmetic right | `(signed)a >> n` | Divide by 2^n, fill sign bit |
| Rotation | synthesized | Bits wrap around ends |

**Logical vs arithmetic right shift:** `1111 0000 >> 2` → logical: `0011 1100` (zeroes fill); arithmetic: `1111 1100` (sign bit fills). C's `>>` on signed is implementation-defined; use unsigned for logical shifts.

**Rotation** (no hardware primitive): `(x << n) | (x >> (W - n))` for unsigned W-bit word.

**Common tricks:** `x & (x-1)` clears lowest set bit. `x & -x` isolates it. `x & 1` tests odd.

## Build It

`code/main.c` and `code/main.py` implement the full toolkit: print bits, negate via two's complement, parse/construct IEEE 754, count bits, rotate, detect overflow.

### Step 1: Minimal Version

`print_bits` is your window into everything:

```c
void print_bits(uint32_t x) {
    for (int i = 31; i >= 0; i--) {
        printf("%d", (x >> i) & 1);
        if (i % 4 == 0 && i > 0) printf(" ");
    }
}
```

`negate_twos_complement` applies invert + 1: `return (int32_t)(~(uint32_t)x + 1);`

### Step 2: Realistic Version

IEEE 754 decomposition uses `memcpy` to reinterpret float bits (avoiding strict-aliasing UB):

```c
uint32_t bits;
memcpy(&bits, &f, sizeof(bits));
uint32_t sign = (bits >> 31) & 1;
uint32_t exponent = (bits >> 23) & 0xFF;
uint32_t mantissa = bits & 0x7FFFFF;
```

Bit counting uses Kernighan's trick: `while (x) { x &= x - 1; count++; }` — runs in O(set bits).

### Build & run

```sh
clang -O2 -Wall -o bits main.c && ./bits
python3 main.py
```

## Use It

- **C's `int`** on 32-bit is exactly two's complement. `unsigned int` is the same bits reinterpreted. Hardware doesn't change.
- **C's `float`** is IEEE 754 single; `double` is double. GCC/Clang compile `float f = 3.14f` into a literal bit pattern in `.rodata`.
- **Python's `int`** is arbitrary-precision (no fixed width), but `struct.pack(">f", x)` produces IEEE 754 bits.
- **Practical uses:** Linux permissions (`0755`), network masks (`/24`), hash functions (XOR folding), graphics (ARGB packing), cryptography (XOR ciphers).

The `float` ↔ `int` reinterpretation via `memcpy` is the production pattern — what the hardware does on every float operation.

## Read the Source

- [IEEE 754 Standard (2019)](https://ieeexplore.ieee.org/document/8766229) — the definitive spec.
- [Linux kernel `arch/x86/include/asm/fpu/`](https://github.com/torvalds/linux/tree/master/arch/x86/include/asm/fpu) — real FPU state management.

## Ship It

This lesson ships **`outputs/bitlib.h`** (C header with `print_bits`, `twos_negate`, `float_bits`, `count_bits`, rotation macros) and **`outputs/bitlib.py`** (Python mirror).

## Exercises

1. **Easy.** Convert 3.14 to IEEE 754 single by hand. Show every step: normalize, biased exponent, mantissa. Verify against `float_to_ieee754(3.14f)`.

2. **Medium.** Detect signed overflow in C without UB. Write `bool safe_add(int32_t a, int32_t b, int32_t *result)` using only unsigned arithmetic (which wraps cleanly). Hint: overflow when sign of result disagrees with expected sign.

3. **Hard.** Implement **fixed-point arithmetic** in 16.16 format (16 integer + 16 fractional bits in `int32_t`). Support add, subtract, multiply, convert-to/from-float, print. Compare precision vs IEEE 754 for values like 0.1 and 0.2.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Two's complement | "Signed integer encoding" | Negate = invert + 1; range -2^(n-1) to 2^(n-1)-1; add/sub hardware is sign-agnostic |
| Bias (exponent) | "Exponent offset" | IEEE 754 stores exponent + bias; actual exponent = stored - bias |
| Mantissa | "Significand" | Precision bits; leading 1 implicit for normalized numbers |
| Denormalized | "Subnormal" | Exponent=0, mantissa≠0; gradual underflow near zero |
| Arithmetic shift | "Sign-preserving shift" | Right shift replicating sign bit; preserves sign for negatives |
| Rotation | "Circular shift" | Bits falling off one end reappear at the other |

## Further Reading

- [What Every Computer Scientist Should Know About Floating-Point Arithmetic](https://docs.oracle.com/cd/E19957-01/806-3568/ncg_goldberg.html) — David Goldberg's classic.
- [IEEE 754 Converter](https://www.h-schmidt.net/FloatConverter/IEEE754.html) — interactive bit-pattern visualizer.
- *Hacker's Delight* by Henry S. Warren — definitive bit-manipulation reference.
