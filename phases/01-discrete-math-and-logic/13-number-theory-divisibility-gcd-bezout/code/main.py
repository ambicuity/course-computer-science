"""Number theory: gcd, extended gcd, modular inverse, linear Diophantine equations.

Run:  python3 main.py
"""
from __future__ import annotations

from typing import Optional, Tuple


def gcd(a: int, b: int) -> int:
    a, b = abs(a), abs(b)
    while b:
        a, b = b, a % b
    return a


def lcm(a: int, b: int) -> int:
    if a == 0 or b == 0:
        return 0
    return abs(a // gcd(a, b) * b)


def extended_gcd(a: int, b: int) -> Tuple[int, int, int]:
    """Returns (g, x, y) with a*x + b*y = g = gcd(a, b)."""
    if b == 0:
        return abs(a), (1 if a >= 0 else -1), 0
    g, x1, y1 = extended_gcd(b, a % b)
    return g, y1, x1 - (a // b) * y1


def modinv(a: int, m: int) -> int:
    """Modular inverse of a mod m. Raises if gcd(a, m) != 1."""
    g, x, _ = extended_gcd(a, m)
    if g != 1:
        raise ValueError(f"no inverse: gcd({a},{m})={g}")
    return x % m


def diophantine(a: int, b: int, c: int) -> Optional[Tuple[int, int, int, int]]:
    """Solve a*x + b*y = c in integers.
    Returns (x0, y0, dx, dy) so that all solutions are (x0 + k*dx, y0 + k*dy).
    Returns None if c is not divisible by gcd(a, b)."""
    g, x0, y0 = extended_gcd(a, b)
    if c % g != 0:
        return None
    x0 *= c // g
    y0 *= c // g
    return x0, y0, b // g, -a // g


def gcd_steps(a: int, b: int) -> int:
    a, b = abs(a), abs(b)
    steps = 0
    while b:
        a, b = b, a % b
        steps += 1
    return steps


# ── Demo ───────────────────────────────────────────────────────────

def main() -> None:
    print("== Euclidean GCD ==")
    print(f"  gcd(462, 1071) = {gcd(462, 1071)}    (expected 21)")
    print(f"  gcd(48, 18) = {gcd(48, 18)}")
    print(f"  gcd(0, 7) = {gcd(0, 7)}")
    print(f"  gcd(7, 0) = {gcd(7, 0)}")

    print("\n== LCM ==")
    print(f"  lcm(12, 18) = {lcm(12, 18)}    (expected 36)")

    print("\n== Extended Euclidean / Bezout ==")
    for a, b in [(11, 13), (462, 1071), (35, 64), (1, 1), (12, 18)]:
        g, x, y = extended_gcd(a, b)
        check = a * x + b * y
        print(f"  {a:>4d} · {x:>4d}  +  {b:>4d} · {y:>4d}  =  {check:>5d}   (gcd={g})")
        assert check == g

    print("\n== Modular inverse ==")
    for a, m in [(3, 7), (11, 13), (7, 100), (17, 1000000007)]:
        inv = modinv(a, m)
        print(f"  modinv({a:>5d}, {m}) = {inv:>10d}    (verify: {a} · {inv} ≡ {a*inv % m} (mod {m}))")
        assert (a * inv) % m == 1

    print("\n== Linear Diophantine: 11x + 13y = 1 ==")
    sol = diophantine(11, 13, 1)
    if sol is None:
        print("  no solution")
    else:
        x0, y0, dx, dy = sol
        print(f"  base: x={x0}, y={y0}   verify 11·{x0} + 13·{y0} = {11*x0 + 13*y0}")
        print(f"  general: x = {x0} + {dx}·k,  y = {y0} + {dy}·k")
        for k in range(-2, 3):
            print(f"    k={k:+d}: ({x0 + dx*k:>4d}, {y0 + dy*k:>4d})  check {11*(x0+dx*k) + 13*(y0+dy*k)}")

    print("\n== Worst case: Fibonacci pairs ==")
    fib = [1, 1]
    while len(fib) < 25:
        fib.append(fib[-1] + fib[-2])
    print(f"  gcd_steps(F_{{n+1}}, F_n):")
    for n in range(2, 16):
        steps = gcd_steps(fib[n + 1], fib[n])
        print(f"    F_{n+1:<2d}={fib[n+1]:>6d}, F_{n:<2d}={fib[n]:>6d}: {steps} steps")


if __name__ == "__main__":
    main()
