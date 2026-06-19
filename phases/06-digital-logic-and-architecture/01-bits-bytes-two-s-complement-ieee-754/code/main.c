/*
 * Bits, Bytes, Two's Complement, IEEE 754
 * Phase 06 — Digital Logic & Computer Architecture
 *
 * Toolkit: bit printing, two's complement negation, IEEE 754
 * decomposition/construction, bit counting, rotation.
 */
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdbool.h>
#include <math.h>

/* ------------------------------------------------------------------ */
/* print_bits — print binary representation of a 32-bit value         */
/* ------------------------------------------------------------------ */
void print_bits(uint32_t x) {
    for (int i = 31; i >= 0; i--) {
        printf("%d", (x >> i) & 1);
        if (i % 4 == 0 && i > 0)
            printf(" ");
    }
}

/* ------------------------------------------------------------------ */
/* negate_twos_complement — negate via invert + 1 (two's complement)  */
/* ------------------------------------------------------------------ */
int32_t negate_twos_complement(int32_t x) {
    return (int32_t)(~(uint32_t)x + 1);
}

/* ------------------------------------------------------------------ */
/* float_to_ieee754 — decompose a float into sign/exponent/mantissa   */
/* ------------------------------------------------------------------ */
void float_to_ieee754(float f) {
    uint32_t bits;
    memcpy(&bits, &f, sizeof(bits));

    uint32_t sign     = (bits >> 31) & 1;
    uint32_t exponent = (bits >> 23) & 0xFF;
    uint32_t mantissa = bits & 0x7FFFFF;

    printf("float = %.10g\n", f);
    printf("hex   = 0x%08X\n", bits);
    printf("bits  = ");
    print_bits(bits);
    printf("\n\n");

    printf("sign     = %u (%s)\n", sign, sign ? "negative" : "positive");
    printf("exponent = %u (raw), bias = 127, actual = %d\n",
           exponent, (int)exponent - 127);
    printf("mantissa = 0x%06X (fraction bits)\n\n", mantissa);

    if (exponent == 0 && mantissa == 0) {
        printf("classification: %czero\n", sign ? '-' : '+');
    } else if (exponent == 0 && mantissa != 0) {
        printf("classification: denormalized (subnormal)\n");
        /* value = (-1)^s * 0.mantissa * 2^(1-127) */
        double val = ldexp((double)mantissa / 0x800000, -126);
        if (sign) val = -val;
        printf("value         = %.10g\n", val);
    } else if (exponent == 0xFF && mantissa == 0) {
        printf("classification: %cinfinity\n", sign ? '-' : '+');
    } else if (exponent == 0xFF && mantissa != 0) {
        printf("classification: NaN (Not a Number)\n");
    } else {
        printf("classification: normalized\n");
        double val = ldexp(1.0 + (double)mantissa / 0x800000, (int)exponent - 127);
        if (sign) val = -val;
        printf("value         = %.10g\n", val);
    }
}

/* ------------------------------------------------------------------ */
/* ieee754_to_float — reconstruct a float from raw 32-bit pattern      */
/* ------------------------------------------------------------------ */
float ieee754_to_float(uint32_t bits) {
    float f;
    memcpy(&f, &bits, sizeof(f));
    return f;
}

/* ------------------------------------------------------------------ */
/* count_set_bits — Kernighan's trick: O(set bits)                    */
/* ------------------------------------------------------------------ */
uint32_t count_set_bits(uint32_t x) {
    uint32_t count = 0;
    while (x) {
        x &= x - 1;   /* clears lowest set bit */
        count++;
    }
    return count;
}

/* ------------------------------------------------------------------ */
/* rotate_left / rotate_right — circular shifts                       */
/* ------------------------------------------------------------------ */
uint32_t rotate_left(uint32_t x, unsigned n) {
    n &= 31;
    return (x << n) | (x >> (32 - n));
}

uint32_t rotate_right(uint32_t x, unsigned n) {
    n &= 31;
    return (x >> n) | (x << (32 - n));
}

/* ------------------------------------------------------------------ */
/* safe_add — detect signed overflow without UB                        */
/* ------------------------------------------------------------------ */
bool safe_add(int32_t a, int32_t b, int32_t *result) {
    uint32_t ua = (uint32_t)a;
    uint32_t ub = (uint32_t)b;
    uint32_t sum = ua + ub;

    /* Overflow when both operands have the same sign but result differs. */
    /* For positive overflow: a > 0, b > 0, result < 0 */
    /* For negative overflow: a < 0, b < 0, result >= 0 */
    if ((a > 0 && b > 0 && (int32_t)sum < 0) ||
        (a < 0 && b < 0 && (int32_t)sum >= 0)) {
        return false;  /* overflow */
    }
    *result = (int32_t)sum;
    return true;
}

/* ------------------------------------------------------------------ */
/* main — demonstrations                                              */
/* ------------------------------------------------------------------ */
int main(void) {
    printf("=== Two's Complement ===\n\n");

    int32_t vals[] = {0, 1, -1, 127, -128, INT32_MAX, INT32_MIN};
    int n = sizeof(vals) / sizeof(vals[0]);
    for (int i = 0; i < n; i++) {
        int32_t v = vals[i];
        printf("%12d : ", v);
        print_bits((uint32_t)v);
        printf("  neg = %d\n", negate_twos_complement(v));
    }

    printf("\n=== Overflow Detection ===\n\n");
    int32_t result;
    printf("INT32_MAX + 1  overflow? %s\n",
           safe_add(INT32_MAX, 1, &result) ? "no" : "YES");
    printf("INT32_MIN + -1 overflow? %s\n",
           safe_add(INT32_MIN, -1, &result) ? "no" : "YES");
    printf("100 + 200      overflow? %s (result = %d)\n",
           safe_add(100, 200, &result) ? "no" : "YES", result);

    printf("\n=== Sign Extension ===\n\n");
    int8_t  small = -5;
    int16_t wide  = small;     /* sign-extended by compiler */
    printf("int8_t  -5 : "); print_bits((uint32_t)(uint8_t)small);  printf("\n");
    printf("int16_t -5 : "); print_bits((uint32_t)(uint16_t)wide);  printf("\n");

    printf("\n=== IEEE 754 Floats ===\n\n");
    float_to_ieee754(3.14f);
    printf("\n");
    float_to_ieee754(-6.75f);
    printf("\n");
    float_to_ieee754(0.0f);
    printf("\n");
    float_to_ieee754(1.0f / 0.0f);   /* +inf */
    printf("\n");
    float_to_ieee754(0.0f / 0.0f);   /* NaN */

    printf("\n=== Round-trip: bits -> float -> bits ===\n\n");
    uint32_t raw = 0x40490FDB;  /* pi approx */
    float reconstructed = ieee754_to_float(raw);
    printf("raw = 0x%08X -> float = %.10g\n", raw, reconstructed);

    printf("\n=== Bit Counting ===\n\n");
    uint32_t demo_vals[] = {0, 0xFF, 0xDEADBEEF, 0x80000000};
    const char *demo_names[] = {"0", "0xFF", "0xDEADBEEF", "0x80000000"};
    for (int i = 0; i < 4; i++) {
        printf("%s has %u bits set\n",
               demo_names[i], count_set_bits(demo_vals[i]));
    }

    printf("\n=== Bitwise Operations ===\n\n");
    uint32_t a = 0xF0F0F0F0;
    uint32_t b = 0x0FF00FF0;
    printf("a         = "); print_bits(a); printf("\n");
    printf("b         = "); print_bits(b); printf("\n");
    printf("a & b     = "); print_bits(a & b); printf("\n");
    printf("a | b     = "); print_bits(a | b); printf("\n");
    printf("a ^ b     = "); print_bits(a ^ b); printf("\n");
    printf("~a        = "); print_bits(~a); printf("\n");
    printf("a << 4    = "); print_bits(a << 4); printf("\n");
    printf("a >> 4 (u)= "); print_bits(a >> 4); printf("\n");

    printf("\n=== Rotation ===\n\n");
    uint32_t rot = 0x80000001;
    printf("original    = "); print_bits(rot); printf("\n");
    printf("rotl(4)     = "); print_bits(rotate_left(rot, 4)); printf("\n");
    printf("rotr(4)     = "); print_bits(rotate_right(rot, 4)); printf("\n");

    printf("\n=== Bit Tricks ===\n\n");
    uint32_t x = 0b10110100;
    printf("x            = "); print_bits(x); printf("\n");
    printf("x & (x-1)    = "); print_bits(x & (x - 1)); printf("  (clear lowest set bit)\n");
    printf("x & -x       = "); print_bits(x & (uint32_t)(-(int32_t)x)); printf("  (isolate lowest set bit)\n");

    printf("\n=== Float Equality Pitfall ===\n\n");
    float sum = 0.0f;
    for (int i = 0; i < 10; i++)
        sum += 0.1f;
    printf("0.1f added 10 times = %.20f\n", sum);
    printf("1.0f                 = %.20f\n", 1.0f);
    printf("equal? %s (accumulated rounding error)\n",
           sum == 1.0f ? "yes" : "NO");

    return 0;
}
