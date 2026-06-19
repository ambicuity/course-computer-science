"""Information theory: entropy, Huffman codes, Hamming (7,4), BSC capacity.

Run:  python3 main.py
"""
from __future__ import annotations

import heapq
import math
from typing import Dict, List, Tuple


# ── Entropy ────────────────────────────────────────────────────────

def entropy(probs) -> float:
    return -sum(p * math.log2(p) for p in probs if p > 0)


# ── Huffman coding ────────────────────────────────────────────────

def huffman_codes(freq: Dict[str, float]) -> Dict[str, str]:
    if not freq:
        return {}
    if len(freq) == 1:
        return {next(iter(freq)): "0"}

    heap = [[w, i, [sym, ""]] for i, (sym, w) in enumerate(freq.items())]
    heapq.heapify(heap)
    counter = len(heap)
    while len(heap) > 1:
        lo = heapq.heappop(heap)
        hi = heapq.heappop(heap)
        for pair in lo[2:]:
            pair[1] = "0" + pair[1]
        for pair in hi[2:]:
            pair[1] = "1" + pair[1]
        merged = [lo[0] + hi[0], counter] + lo[2:] + hi[2:]
        counter += 1
        heapq.heappush(heap, merged)
    return dict(sorted([pair for pair in heap[0][2:]]))


def avg_code_length(codes: Dict[str, str], freq: Dict[str, float]) -> float:
    return sum(freq[s] * len(codes[s]) for s in freq)


# ── Hamming (7, 4) ────────────────────────────────────────────────

def hamming74_encode(data: List[int]) -> List[int]:
    """Encode 4 data bits into 7 codeword bits.
    Output positions (1-indexed): p1 p2 d1 p4 d2 d3 d4."""
    assert len(data) == 4
    d1, d2, d3, d4 = data
    p1 = d1 ^ d2 ^ d4
    p2 = d1 ^ d3 ^ d4
    p4 = d2 ^ d3 ^ d4
    return [p1, p2, d1, p4, d2, d3, d4]


def hamming74_decode(code: List[int]) -> Tuple[List[int], int]:
    """Returns (corrected_data_bits, error_position_or_0_if_none)."""
    assert len(code) == 7
    # 1-indexed positions; group parity bits cover bits with their position-bit set
    c1, c2, c3, c4, c5, c6, c7 = code
    s1 = c1 ^ c3 ^ c5 ^ c7   # bit-0 of position
    s2 = c2 ^ c3 ^ c6 ^ c7   # bit-1
    s4 = c4 ^ c5 ^ c6 ^ c7   # bit-2
    err = s1 + (s2 << 1) + (s4 << 2)
    if err != 0:
        code = code[:]
        code[err - 1] ^= 1
    return [code[2], code[4], code[5], code[6]], err


# ── BSC capacity ──────────────────────────────────────────────────

def H_binary(p: float) -> float:
    if p in (0.0, 1.0): return 0.0
    return -p * math.log2(p) - (1 - p) * math.log2(1 - p)


def bsc_capacity(p: float) -> float:
    return 1 - H_binary(p)


# ── Demo ──────────────────────────────────────────────────────────

def demo_entropy():
    print("== Entropy ==")
    print(f"  Fair coin H = {entropy([0.5, 0.5]):.4f} bits")
    print(f"  Fair 8-sided die H = {entropy([1/8]*8):.4f} bits")
    print(f"  Degenerate (p=1) H = {entropy([1.0, 0.0]):.4f} bits")
    print(f"  Biased coin (0.9, 0.1) H = {entropy([0.9, 0.1]):.4f} bits")


def demo_huffman():
    print("\n== Huffman codes for English letters ==")
    freq = {
        'a': 0.0817, 'b': 0.0149, 'c': 0.0278, 'd': 0.0425, 'e': 0.1270,
        'f': 0.0223, 'g': 0.0202, 'h': 0.0609, 'i': 0.0697, 'j': 0.0015,
        'k': 0.0077, 'l': 0.0403, 'm': 0.0241, 'n': 0.0675, 'o': 0.0751,
        'p': 0.0193, 'q': 0.0010, 'r': 0.0599, 's': 0.0633, 't': 0.0906,
        'u': 0.0276, 'v': 0.0098, 'w': 0.0236, 'x': 0.0015, 'y': 0.0197,
        'z': 0.0007,
    }
    total = sum(freq.values())
    freq = {k: v / total for k, v in freq.items()}

    H = entropy(list(freq.values()))
    codes = huffman_codes(freq)
    L = avg_code_length(codes, freq)
    print(f"  Entropy:           {H:.4f} bits per letter")
    print(f"  Huffman avg length: {L:.4f} bits per letter")
    print(f"  Bound H ≤ L < H+1: {H:.4f} ≤ {L:.4f} < {H + 1:.4f}  ✓")
    print(f"  Sample codes: e={codes['e']!r}, t={codes['t']!r}, z={codes['z']!r}")


def demo_hamming():
    print("\n== Hamming (7, 4) — single-bit error correction ==")
    data = [1, 0, 1, 1]
    code = hamming74_encode(data)
    print(f"  Encode {data} → {code}")
    for flip_pos in range(7):
        corrupted = code[:]
        corrupted[flip_pos] ^= 1
        recovered, err = hamming74_decode(corrupted)
        ok = "✓" if recovered == data and err == flip_pos + 1 else "✗"
        print(f"  flip bit {flip_pos+1}: received={corrupted}, recovered={recovered}, err_pos={err}  {ok}")
        assert recovered == data and err == flip_pos + 1


def demo_capacity():
    print("\n== Binary symmetric channel capacity C = 1 - H(p) ==")
    for p in [0.0, 0.01, 0.05, 0.1, 0.2, 0.3, 0.4, 0.5]:
        print(f"  p = {p:>4.2f}:  C = {bsc_capacity(p):.4f} bits per channel use")


def main():
    demo_entropy()
    demo_huffman()
    demo_hamming()
    demo_capacity()


if __name__ == "__main__":
    main()
