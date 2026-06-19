"""
Bits, Bytes, Two's Complement, IEEE 754
Phase 06 — Digital Logic & Computer Architecture

Toolkit: bit printing, two's complement negation, IEEE 754 analysis,
float comparison pitfalls.
"""
import struct


def print_bits(n: int, width: int = 32) -> str:
    """Print binary representation of n with the given bit width."""
    if n < 0:
        n = n & ((1 << width) - 1)  # reinterpret as unsigned
    bits = ""
    for i in range(width - 1, -1, -1):
        bits += str((n >> i) & 1)
        if i % 4 == 0 and i > 0:
            bits += " "
    return bits


def twos_complement_negate(n: int, bits: int = 32) -> int:
    """Negate a signed integer using two's complement: invert + 1."""
    mask = (1 << bits) - 1
    return (~n + 1) & mask


def to_signed(n: int, bits: int = 32) -> int:
    """Interpret an unsigned n-bit value as a signed two's complement integer."""
    if n >= (1 << (bits - 1)):
        return n - (1 << bits)
    return n


def sign_extend(value: int, from_bits: int, to_bits: int = 32) -> int:
    """Sign-extend a value from from_bits to to_bits."""
    sign_bit = 1 << (from_bits - 1)
    if value & sign_bit:
        mask = ((1 << to_bits) - 1) & ~((1 << from_bits) - 1)
        return value | mask
    return value & ((1 << from_bits) - 1)


def ieee754_analysis(f: float) -> dict:
    """Decompose a Python float into IEEE 754 double-precision fields."""
    # Pack as 64-bit double, unpack as uint64
    raw = struct.pack(">d", f)
    bits = int.from_bytes(raw, "big")

    sign = (bits >> 63) & 1
    exponent = (bits >> 52) & 0x7FF
    mantissa = bits & 0xFFFFFFFFFFFFF

    info = {
        "value": f,
        "hex": f"0x{bits:016X}",
        "bits": print_bits(bits, 64),
        "sign": sign,
        "sign_label": "negative" if sign else "positive",
        "exponent_raw": exponent,
        "exponent_bias": 1023,
        "exponent_actual": exponent - 1023 if exponent not in (0, 0x7FF) else None,
        "mantissa": mantissa,
    }

    if exponent == 0 and mantissa == 0:
        info["classification"] = f"{'-' if sign else '+'}zero"
    elif exponent == 0 and mantissa != 0:
        info["classification"] = "denormalized (subnormal)"
        val = (mantissa / (1 << 52)) * (2 ** (1 - 1023))
        if sign:
            val = -val
        info["computed_value"] = val
    elif exponent == 0x7FF and mantissa == 0:
        info["classification"] = f"{'-' if sign else '+'}infinity"
    elif exponent == 0x7FF and mantissa != 0:
        info["classification"] = "NaN"
    else:
        info["classification"] = "normalized"
        val = (1 + mantissa / (1 << 52)) * (2 ** (exponent - 1023))
        if sign:
            val = -val
        info["computed_value"] = val

    return info


def float_compare(a: float, b: float, rel_tol: float = 1e-9, abs_tol: float = 0.0) -> dict:
    """Demonstrate why exact float comparison fails and show robust alternatives."""
    exact = a == b
    abs_diff = abs(a - b)
    rel_diff = abs_diff / max(abs(a), abs(b)) if max(abs(a), abs(b)) > 0 else 0.0
    math_isclose = abs(a - b) <= max(rel_tol * max(abs(a), abs(b)), abs_tol)

    return {
        "a": a,
        "b": b,
        "exact_equal": exact,
        "abs_diff": abs_diff,
        "rel_diff": rel_diff,
        "math_isclose": math_isclose,
    }


def count_set_bits(n: int) -> int:
    """Count set bits using Kernighan's trick."""
    count = 0
    n = n & 0xFFFFFFFF  # limit to 32 bits
    while n:
        n &= n - 1
        count += 1
    return count


def rotate_left(n: int, shift: int, width: int = 32) -> int:
    """Rotate left by shift positions within width bits."""
    n &= (1 << width) - 1
    shift &= (width - 1)
    return ((n << shift) | (n >> (width - shift))) & ((1 << width) - 1)


def rotate_right(n: int, shift: int, width: int = 32) -> int:
    """Rotate right by shift positions within width bits."""
    n &= (1 << width) - 1
    shift &= (width - 1)
    return ((n >> shift) | (n << (width - shift))) & ((1 << width) - 1)


def safe_add(a: int, b: int, bits: int = 32) -> tuple[bool, int]:
    """Detect signed overflow using only unsigned arithmetic."""
    mask = (1 << bits) - 1
    half = 1 << (bits - 1)

    ua = a & mask
    ub = b & mask
    result = (ua + ub) & mask

    # Check if result sign disagrees with input signs
    a_sign = ua >> (bits - 1)
    b_sign = ub >> (bits - 1)
    r_sign = result >> (bits - 1)

    overflow = (a_sign == b_sign) and (a_sign != r_sign)
    signed_result = to_signed(result, bits)
    return (not overflow, signed_result)


def main() -> None:
    print("=== Two's Complement ===\n")
    for v in [0, 1, -1, 127, -128, 2147483647, -2147483648]:
        unsigned = v & 0xFFFFFFFF
        neg = twos_complement_negate(v)
        print(f"{v:>12d} : {print_bits(v)}  neg = {to_signed(neg):d}")

    print("\n=== Overflow Detection ===\n")
    ok, r = safe_add(2147483647, 1)
    print(f"INT32_MAX + 1  overflow? {'no' if ok else 'YES'}")
    ok, r = safe_add(-2147483648, -1)
    print(f"INT32_MIN + -1 overflow? {'no' if ok else 'YES'}")
    ok, r = safe_add(100, 200)
    print(f"100 + 200      overflow? {'no' if ok else 'YES'} (result = {r})")

    print("\n=== Sign Extension ===\n")
    small = to_signed(0b11111011, 8)  # -5 in 8 bits
    extended = sign_extend(0b11111011, 8, 16)
    print(f"8-bit  -5 : {print_bits(0b11111011, 8)}")
    print(f"16-bit -5 : {print_bits(extended, 16)}")

    print("\n=== IEEE 754 Double Precision ===\n")
    for val in [3.14, -6.75, 0.0, 1.0, float("inf"), float("nan")]:
        info = ieee754_analysis(val)
        print(f"value        = {info['value']}")
        print(f"hex          = {info['hex']}")
        print(f"bits         = {info['bits']}")
        print(f"sign         = {info['sign']} ({info['sign_label']})")
        print(f"exponent raw = {info['exponent_raw']}, bias = {info['exponent_bias']}, actual = {info['exponent_actual']}")
        print(f"mantissa     = 0x{info['mantissa']:013X}")
        print(f"class        = {info['classification']}")
        if "computed_value" in info:
            print(f"computed      = {info['computed_value']}")
        print()

    print("=== Bit Counting ===\n")
    for val, name in [(0, "0"), (0xFF, "0xFF"), (0xDEADBEEF, "0xDEADBEEF"), (0x80000000, "0x80000000")]:
        print(f"{name} has {count_set_bits(val)} bits set")

    print("\n=== Bitwise Operations ===\n")
    a = 0xF0F0F0F0
    b = 0x0FF00FF0
    print(f"a         = {print_bits(a)}")
    print(f"b         = {print_bits(b)}")
    print(f"a & b     = {print_bits(a & b)}")
    print(f"a | b     = {print_bits(a | b)}")
    print(f"a ^ b     = {print_bits(a ^ b)}")
    print(f"~a        = {print_bits(~a)}")
    print(f"a << 4    = {print_bits(a << 4)}")
    print(f"a >> 4    = {print_bits(a >> 4)}")

    print("\n=== Rotation ===\n")
    rot = 0x80000001
    print(f"original    = {print_bits(rot)}")
    print(f"rotl(4)     = {print_bits(rotate_left(rot, 4))}")
    print(f"rotr(4)     = {print_bits(rotate_right(rot, 4))}")

    print("\n=== Bit Tricks ===\n")
    x = 0b10110100
    print(f"x            = {print_bits(x, 8)}")
    print(f"x & (x-1)    = {print_bits(x & (x - 1), 8)}  (clear lowest set bit)")
    print(f"x & -x       = {print_bits(x & (-x), 8)}  (isolate lowest set bit)")

    print("\n=== Float Equality Pitfall ===\n")
    fa = 0.1 + 0.2
    fb = 0.3
    print(f"0.1 + 0.2 = {fa:.20f}")
    print(f"0.3        = {fb:.20f}")
    print(f"exact == ? {fa == fb}")

    comp = float_compare(0.1 + 0.2, 0.3)
    print(f"abs_diff   = {comp['abs_diff']:.2e}")
    print(f"math.isclose? {comp['math_isclose']}")

    print("\n=== Single-precision IEEE 754 (via struct) ===\n")
    raw_pi = struct.pack(">f", 3.14)
    bits_pi = int.from_bytes(raw_pi, "big")
    print(f"float 3.14f hex  = 0x{bits_pi:08X}")
    print(f"float 3.14f bits = {print_bits(bits_pi)}")
    s = (bits_pi >> 31) & 1
    e = (bits_pi >> 23) & 0xFF
    m = bits_pi & 0x7FFFFF
    print(f"sign={s} exponent={e} (actual={e-127}) mantissa=0x{m:06X}")
    print(f"reconstructed = {(1 + m / (1 << 23)) * (2 ** (e - 127)):.10f}")


if __name__ == "__main__":
    main()
