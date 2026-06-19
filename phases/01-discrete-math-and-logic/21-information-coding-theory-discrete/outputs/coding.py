"""coding.py — entropy + Huffman + Hamming (7,4).

Used by Phase 04 (compression / streaming algorithms), Phase 12 (cryptographic
entropy), and Phase 17 (formal coding-correctness models).
"""
from __future__ import annotations

import heapq
import math
from typing import Dict, List, Tuple


def entropy(probs) -> float:
    return -sum(p * math.log2(p) for p in probs if p > 0)


def huffman_codes(freq: Dict[str, float]) -> Dict[str, str]:
    if not freq: return {}
    if len(freq) == 1: return {next(iter(freq)): "0"}
    heap = [[w, i, [sym, ""]] for i, (sym, w) in enumerate(freq.items())]
    heapq.heapify(heap)
    counter = len(heap)
    while len(heap) > 1:
        lo = heapq.heappop(heap)
        hi = heapq.heappop(heap)
        for pair in lo[2:]: pair[1] = "0" + pair[1]
        for pair in hi[2:]: pair[1] = "1" + pair[1]
        merged = [lo[0] + hi[0], counter] + lo[2:] + hi[2:]
        counter += 1
        heapq.heappush(heap, merged)
    return dict(sorted([pair for pair in heap[0][2:]]))


def hamming74_encode(data: List[int]) -> List[int]:
    d1, d2, d3, d4 = data
    return [d1 ^ d2 ^ d4, d1 ^ d3 ^ d4, d1, d2 ^ d3 ^ d4, d2, d3, d4]


def hamming74_decode(code: List[int]) -> Tuple[List[int], int]:
    c1, c2, c3, c4, c5, c6, c7 = code
    s1 = c1 ^ c3 ^ c5 ^ c7
    s2 = c2 ^ c3 ^ c6 ^ c7
    s4 = c4 ^ c5 ^ c6 ^ c7
    err = s1 + (s2 << 1) + (s4 << 2)
    if err != 0:
        code = code[:]
        code[err - 1] ^= 1
    return [code[2], code[4], code[5], code[6]], err


if __name__ == "__main__":
    assert abs(entropy([0.5, 0.5]) - 1.0) < 1e-9
    # round-trip 4 random data bits + single-bit flip
    import random
    rng = random.Random(0)
    for _ in range(100):
        data = [rng.randint(0, 1) for _ in range(4)]
        code = hamming74_encode(data)
        pos = rng.randint(0, 6)
        code[pos] ^= 1
        recovered, err = hamming74_decode(code)
        assert recovered == data and err == pos + 1
    print("coding library smoke-test OK")
