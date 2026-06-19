"""pairing.py — bijective ℕ ↔ ℕ² codecs.

Two flavors:
  - Cantor pairing: zigzag along diagonals; concise formula.
  - Szudzik pairing: faster, tighter (better cache locality); used by Unity, etc.

Use cases:
  - serialize a (row, col) into a single integer key (Redis, KV stores)
  - dense indexing of a sparse 2D coordinate space
  - shard composite keys without losing locality
"""
from __future__ import annotations

import math
from typing import Tuple


# ── Cantor ─────────────────────────────────────────────────────────

def cantor_encode(x: int, y: int) -> int:
    return (x + y) * (x + y + 1) // 2 + y


def cantor_decode(z: int) -> Tuple[int, int]:
    w = (math.isqrt(8 * z + 1) - 1) // 2
    t = w * (w + 1) // 2
    y = z - t
    x = w - y
    return x, y


# ── Szudzik ────────────────────────────────────────────────────────

def szudzik_encode(x: int, y: int) -> int:
    """Bijection ℕ² ↔ ℕ. Faster than Cantor and slightly tighter."""
    if x >= y:
        return x * x + x + y
    return y * y + x


def szudzik_decode(z: int) -> Tuple[int, int]:
    q = math.isqrt(z)
    l = z - q * q
    if l < q:
        return l, q
    return q, l - q


# ── Smoke test ─────────────────────────────────────────────────────

if __name__ == "__main__":
    for fn_enc, fn_dec, name in [(cantor_encode, cantor_decode, "Cantor"),
                                  (szudzik_encode, szudzik_decode, "Szudzik")]:
        codes = set()
        for x in range(20):
            for y in range(20):
                z = fn_enc(x, y)
                codes.add(z)
                assert fn_dec(z) == (x, y), (name, x, y, z, fn_dec(z))
        assert len(codes) == 400, name
        print(f"{name}: ✓ bijection verified on 20×20 grid")
