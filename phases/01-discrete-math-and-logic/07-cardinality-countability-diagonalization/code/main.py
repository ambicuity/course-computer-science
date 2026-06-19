"""Cardinality, countability, diagonalization — concrete demos.

Run:  python3 main.py
"""
from __future__ import annotations

import math
from typing import List, Tuple


# ── Cantor pairing ─────────────────────────────────────────────────

def cantor_pair(x: int, y: int) -> int:
    return (x + y) * (x + y + 1) // 2 + y


def cantor_unpair(z: int) -> Tuple[int, int]:
    # w(w+1)/2 ≤ z < (w+1)(w+2)/2 ; solve for w
    w = int((math.isqrt(8 * z + 1) - 1) // 2)
    t = w * (w + 1) // 2
    y = z - t
    x = w - y
    return x, y


# ── Demos ──────────────────────────────────────────────────────────

def demo_pairing():
    print("== Cantor pairing: ℕ × ℕ ↔ ℕ (bijection) ==")
    # Verify bijectivity on 10×10
    codes = []
    for x in range(10):
        for y in range(10):
            codes.append(cantor_pair(x, y))
    assert len(codes) == len(set(codes)) == 100, "collision detected"
    # And invertibility
    for x in range(10):
        for y in range(10):
            assert cantor_unpair(cantor_pair(x, y)) == (x, y)
    print(f"  100 (x,y) pairs in [0..9]² mapped to 100 distinct codes; round-trip verified")

    print(f"  first 12 z → (x,y): " + ", ".join(f"{z}→{cantor_unpair(z)}" for z in range(12)))


def demo_rationals():
    print("\n== ℚ⁺ is countable: enumerate via coprime pairs ==")
    seen = []
    z = 0
    while len(seen) < 15:
        x, y = cantor_unpair(z)
        z += 1
        if y == 0:
            continue  # skip undefined denominator
        # require lowest terms: gcd(x, y) == 1; x is numerator-1 so we use (x+1, y)
        p, q = x + 1, y
        if math.gcd(p, q) == 1:
            seen.append((p, q))
    print(f"  first 15 positive rationals in lowest terms (numerator/denominator):")
    print(f"    {['/'.join(map(str, pq)) for pq in seen]}")


def demo_diagonal():
    print("\n== Cantor's diagonal: ℝ is uncountable (finite witness) ==")
    rows = [
        [0, 1, 0, 1, 1, 0, 1, 0],
        [1, 1, 1, 0, 0, 1, 0, 1],
        [0, 0, 1, 1, 0, 0, 1, 1],
        [1, 0, 0, 0, 1, 1, 1, 0],
        [1, 1, 0, 1, 1, 0, 0, 0],
        [0, 1, 1, 0, 0, 1, 1, 0],
        [1, 0, 1, 1, 0, 0, 1, 1],
        [0, 0, 0, 1, 1, 1, 0, 1],
    ]
    diagonal = [1 - rows[i][i] for i in range(len(rows))]
    print(f"  rows (8 'reals' in [0,1) — binary):")
    for i, r in enumerate(rows):
        marker = "  ".join("[" + str(v) + "]" if j == i else " " + str(v) + " "
                            for j, v in enumerate(r))
        print(f"    r{i}: {marker}")
    print(f"  diagonal d = {diagonal}    (flipped each rᵢ at position i)")
    for i, r in enumerate(rows):
        assert diagonal[i] != r[i], (i, diagonal[i], r[i])
    print(f"  ✓ d differs from each rᵢ at position i ⇒ d ∉ enumeration.")


def demo_powerset():
    print("\n== Cantor's theorem (finite witness): no enumeration of 𝒫(ℕ) is complete ==")
    # candidate enumeration of 6 subsets of ℕ
    subsets = [
        {0, 2, 4},          # S₀
        {1, 3},             # S₁
        {0, 1, 2, 3},       # S₂
        set(),              # S₃
        {2, 5, 7, 11, 13},  # S₄
        {0, 5},             # S₅
    ]
    D = {i for i in range(len(subsets)) if i not in subsets[i]}
    print(f"  Candidate enumeration of 6 subsets of ℕ:")
    for i, s in enumerate(subsets):
        in_s = "in" if i in s else "NOT in"
        print(f"    S{i} = {sorted(s) if s else '∅'}   (index i={i} is {in_s} Sᵢ)")
    print(f"  Diagonal D = {{i : i ∉ Sᵢ}} = {sorted(D)}")
    for i, s in enumerate(subsets):
        contradicts = (i in D) != (i in s)
        assert contradicts, i  # by construction D differs from Sᵢ on the index i
    print(f"  ✓ D differs from every Sᵢ at index i ⇒ D ∉ candidate list.")


def main():
    demo_pairing()
    demo_rationals()
    demo_diagonal()
    demo_powerset()


if __name__ == "__main__":
    main()
