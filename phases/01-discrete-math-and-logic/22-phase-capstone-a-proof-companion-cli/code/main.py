"""cs01 — Phase 01 capstone CLI.

Subcommands integrate the libraries built across L01–L21.

Run examples:
    python3 main.py truth "P -> Q"
    python3 main.py gcd 462 1071
    python3 main.py prime 2305843009213693951
    python3 main.py mod-pow 7 200 13
    python3 main.py count combo 52 5
    python3 main.py count catalan 10
    python3 main.py count factorial 20
    python3 main.py entropy 0.5 0.3 0.15 0.05
    python3 main.py huffman 0.5 0.3 0.15 0.05
    python3 main.py topo "A:B C, B:D, C:D, D:"
    python3 main.py pagerank "A:B C, B:C, C:A, D:C"
    python3 main.py verify all
"""
from __future__ import annotations

import argparse
import heapq
import itertools
import math
import random
import re
import sys
from collections import defaultdict, deque
from typing import Dict, List, Optional, Tuple


# ── L01: truth-table evaluator ─────────────────────────────────────

TOKEN_RE = re.compile(r"\s*(<->|->|[A-Za-z][A-Za-z0-9_]*|[~&|()])")

def tokenize(src):
    out, i = [], 0
    while i < len(src):
        m = TOKEN_RE.match(src, i)
        if not m: raise SyntaxError(src[i:i+10])
        out.append(m.group(1)); i = m.end()
    return out

class P:
    def __init__(self, toks): self.t = toks; self.i = 0
    def pk(self): return self.t[self.i] if self.i < len(self.t) else None
    def eat(self): self.i += 1; return self.t[self.i - 1]
    def iff(self):
        x = self.imp()
        while self.pk() == "<->":
            self.eat(); x = ("iff", x, self.imp())
        return x
    def imp(self):
        x = self.orr()
        if self.pk() == "->":
            self.eat(); return ("imp", x, self.imp())
        return x
    def orr(self):
        x = self.andr()
        while self.pk() == "|":
            self.eat(); x = ("or", x, self.andr())
        return x
    def andr(self):
        x = self.notr()
        while self.pk() == "&":
            self.eat(); x = ("and", x, self.notr())
        return x
    def notr(self):
        if self.pk() == "~":
            self.eat(); return ("not", self.notr())
        return self.atom()
    def atom(self):
        t = self.eat()
        if t == "(":
            f = self.iff()
            assert self.eat() == ")"
            return f
        return ("var", t)

def ev(f, env):
    op = f[0]
    if op == "var": return env[f[1]]
    if op == "not": return not ev(f[1], env)
    if op == "and": return ev(f[1], env) and ev(f[2], env)
    if op == "or":  return ev(f[1], env) or ev(f[2], env)
    if op == "imp": return (not ev(f[1], env)) or ev(f[2], env)
    if op == "iff": return ev(f[1], env) == ev(f[2], env)

def vars_of(f):
    op = f[0]
    if op == "var": return {f[1]}
    if op == "not": return vars_of(f[1])
    return vars_of(f[1]) | vars_of(f[2])


def cmd_truth(args):
    expr = " ".join(args)
    f = P(tokenize(expr)).iff()
    vs = sorted(vars_of(f))
    header = "  ".join(vs) + "  |  " + expr
    print(header); print("─" * len(header))
    all_t = all_f = True
    for bits in itertools.product([False, True], repeat=len(vs)):
        env = dict(zip(vs, bits))
        out = ev(f, env)
        all_t &= out; all_f &= (not out)
        row = "  ".join("T" if env[v] else "F" for v in vs)
        print(f"{row}  |  {'T' if out else 'F'}")
    print()
    print("→ " + ("tautology" if all_t else "contradiction" if all_f else "contingency"))


# ── L13: gcd + Bezout ─────────────────────────────────────────────

def ext_gcd(a, b):
    if b == 0: return abs(a), (1 if a >= 0 else -1), 0
    g, x1, y1 = ext_gcd(b, a % b)
    return g, y1, x1 - (a // b) * y1


def cmd_gcd(args):
    a, b = int(args[0]), int(args[1])
    g, x, y = ext_gcd(a, b)
    print(f"gcd({a}, {b}) = {g}")
    print(f"Bezout: {a}·{x} + {b}·{y} = {a*x + b*y}")


# ── L14: mod-pow ─────────────────────────────────────────────────

def cmd_mod_pow(args):
    a, e, n = int(args[0]), int(args[1]), int(args[2])
    print(f"{a}^{e} mod {n} = {pow(a, e, n)}")


# ── L15: prime test (Miller-Rabin) ───────────────────────────────

def miller_rabin(n):
    if n < 2: return False
    if n in (2, 3): return True
    if n % 2 == 0: return False
    d, s = n - 1, 0
    while d % 2 == 0: d //= 2; s += 1
    for a in [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37]:
        if a >= n: continue
        x = pow(a, d, n)
        if x == 1 or x == n - 1: continue
        ok = False
        for _ in range(s - 1):
            x = (x * x) % n
            if x == n - 1: ok = True; break
        if not ok: return False
    return True


def cmd_prime(args):
    n = int(args[0])
    print(f"is_prime({n}) = {miller_rabin(n)}")


# ── L08: counts ───────────────────────────────────────────────────

def cmd_count(args):
    if not args:
        print("usage: count {combo n k | catalan n | factorial n}"); return
    kind = args[0]
    if kind == "combo":
        n, k = int(args[1]), int(args[2])
        print(f"C({n}, {k}) = {math.comb(n, k)}")
    elif kind == "catalan":
        n = int(args[1])
        print(f"C_{n} = {math.comb(2*n, n) // (n + 1)}")
    elif kind == "factorial":
        n = int(args[1])
        print(f"{n}! = {math.factorial(n)}")
    else:
        print(f"unknown: {kind}")


# ── L06/L17: topological sort ────────────────────────────────────

def parse_dag(s):
    """'A:B C, B:D, C:D, D:'  →  {'A': ['B','C'], 'B': ['D'], 'C': ['D'], 'D': []}"""
    g = {}
    for chunk in s.split(","):
        chunk = chunk.strip()
        if not chunk: continue
        node, _, succ = chunk.partition(":")
        succs = succ.split()
        g[node.strip()] = succs
    return g


def topo_sort(g):
    indeg = defaultdict(int)
    for u in g:
        indeg.setdefault(u, 0)
        for v in g[u]:
            indeg[v] += 1
            indeg.setdefault(u, 0)
    q = deque(sorted([n for n in indeg if indeg[n] == 0]))
    out = []
    while q:
        u = q.popleft()
        out.append(u)
        for v in g.get(u, []):
            indeg[v] -= 1
            if indeg[v] == 0: q.append(v)
    return out if len(out) == len(indeg) else None


def cmd_topo(args):
    g = parse_dag(" ".join(args))
    order = topo_sort(g)
    if order is None:
        print("cycle detected; no topological order")
    else:
        print(" → ".join(order))


# ── L21: entropy + Huffman ────────────────────────────────────────

def shannon_entropy(probs):
    return -sum(p * math.log2(p) for p in probs if p > 0)


def cmd_entropy(args):
    probs = [float(p) for p in args]
    print(f"H = {shannon_entropy(probs):.6f} bits")


def huffman(freq_dict):
    if not freq_dict: return {}
    if len(freq_dict) == 1: return {next(iter(freq_dict)): "0"}
    heap = [[w, i, [s, ""]] for i, (s, w) in enumerate(freq_dict.items())]
    heapq.heapify(heap)
    c = len(heap)
    while len(heap) > 1:
        lo = heapq.heappop(heap); hi = heapq.heappop(heap)
        for p in lo[2:]: p[1] = "0" + p[1]
        for p in hi[2:]: p[1] = "1" + p[1]
        heapq.heappush(heap, [lo[0] + hi[0], c] + lo[2:] + hi[2:]); c += 1
    return dict(sorted([p for p in heap[0][2:]]))


def cmd_huffman(args):
    probs = [float(p) for p in args]
    freq = {f"x{i+1}": p for i, p in enumerate(probs)}
    codes = huffman(freq)
    H = shannon_entropy(probs)
    L = sum(freq[s] * len(codes[s]) for s in freq)
    print(f"H = {H:.4f}    Huffman avg L = {L:.4f}")
    for s, code in codes.items():
        print(f"  {s} (p={freq[s]:.3f}): {code}")


# ── L20: PageRank ─────────────────────────────────────────────────

def cmd_pagerank(args):
    g = parse_dag(" ".join(args))
    nodes = list(g)
    idx = {n: i for i, n in enumerate(nodes)}
    n = len(nodes)
    P = [[0.0] * n for _ in range(n)]
    for u, outs in g.items():
        i = idx[u]
        if not outs:
            for j in range(n): P[i][j] = 1.0 / n
        else:
            for v in outs:
                P[i][idx[v]] += 1.0 / len(outs)
    damping = 0.85
    for i in range(n):
        for j in range(n):
            P[i][j] = damping * P[i][j] + (1 - damping) / n
    pi = [1 / n] * n
    for _ in range(500):
        new = [sum(pi[i] * P[i][j] for i in range(n)) for j in range(n)]
        if max(abs(new[i] - pi[i]) for i in range(n)) < 1e-10:
            pi = new; break
        pi = new
    print("PageRank (sorted descending):")
    for r, j in sorted(enumerate(pi), key=lambda kv: -kv[1]):
        print(f"  {nodes[r]:5s} = {j:.4f}")


# ── verify demos ──────────────────────────────────────────────────

def demo_fermat():
    print("Fermat's little theorem: a^(p-1) ≡ 1 (mod p) for prime p, gcd(a, p) = 1")
    for p in [13, 17, 23, 101]:
        for a in [2, 5, 12]:
            assert pow(a, p - 1, p) == 1
    print("  ✓ holds for primes 13, 17, 23, 101")


def demo_birthday():
    print("Birthday paradox: probability of collision in 365 days")
    for n in [10, 20, 23, 30]:
        p_no = 1.0
        for i in range(n): p_no *= (365 - i) / 365
        print(f"  n={n:3d}: P(collision) = {1 - p_no:.4f}")
    print("  n=23 hits ~0.5 — the birthday paradox.")


def demo_coupon():
    print("Coupon collector: E[T] = n · H_n")
    for n in [10, 100, 365]:
        Hn = sum(1.0 / k for k in range(1, n + 1))
        print(f"  n={n:3d}: n·H_n = {n * Hn:.2f}")


def demo_master():
    print("Master theorem: merge sort T(n) = 2T(n/2) + n")
    crit = math.log(2, 2)  # = 1
    print(f"  log_b(a) = {crit}, f_degree = 1 → case 2 → Θ(n log n)")


def demo_hamming():
    print("Hamming (7,4): single-bit error correction")
    data = [1, 0, 1, 1]
    d1, d2, d3, d4 = data
    code = [d1 ^ d2 ^ d4, d1 ^ d3 ^ d4, d1, d2 ^ d3 ^ d4, d2, d3, d4]
    # Flip bit 5
    code[4] ^= 1
    c1, c2, c3, c4, c5, c6, c7 = code
    s1 = c1 ^ c3 ^ c5 ^ c7; s2 = c2 ^ c3 ^ c6 ^ c7; s4 = c4 ^ c5 ^ c6 ^ c7
    err = s1 + (s2 << 1) + (s4 << 2)
    print(f"  flipped bit 5; decoder identified position: {err}  ({'✓' if err == 5 else '✗'})")


def cmd_verify(args):
    if not args or args[0] == "all":
        for d in [demo_fermat, demo_birthday, demo_coupon, demo_master, demo_hamming]:
            d(); print()
    else:
        m = {"fermat": demo_fermat, "birthday": demo_birthday,
             "coupon": demo_coupon, "master": demo_master, "hamming": demo_hamming}
        fn = m.get(args[0])
        if fn: fn()
        else: print(f"unknown demo: {args[0]}; available: {sorted(m)}")


# ── Dispatch ──────────────────────────────────────────────────────

HANDLERS = {
    "truth":     cmd_truth,
    "gcd":       cmd_gcd,
    "mod-pow":   cmd_mod_pow,
    "prime":     cmd_prime,
    "count":     cmd_count,
    "topo":      cmd_topo,
    "entropy":   cmd_entropy,
    "huffman":   cmd_huffman,
    "pagerank":  cmd_pagerank,
    "verify":    cmd_verify,
}


def main():
    argv = sys.argv[1:]
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        print("Subcommands:", ", ".join(sorted(HANDLERS)))
        return
    cmd = argv[0]
    handler = HANDLERS.get(cmd)
    if not handler:
        print(f"unknown subcommand: {cmd}"); print("Available:", ", ".join(sorted(HANDLERS)))
        sys.exit(1)
    handler(argv[1:])


if __name__ == "__main__":
    main()
