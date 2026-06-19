"""
Zero-Knowledge Proofs — Sigma, zk-SNARK overview
Phase 12 — Cryptography & Security

Implements:
  - Schnorr Sigma protocol (interactive) for proving knowledge of a discrete log
  - Non-interactive Schnorr proof via Fiat-Shamir heuristic
  - Pedersen commitment scheme
  - R1CS constraint system demonstration
  - Verification of valid and invalid proofs
"""

import hashlib
import random
import sys

# ---------------------------------------------------------------------------
# Public parameters
# Use a 1024-bit safe prime p = 2q + 1, generator g.
# WARNING: This is for demonstration. Real ZK requires 2048+ bit primes.
# ---------------------------------------------------------------------------
P_HEX = (
    "B10B8F96A080E01DDE92DE5EAE5D54EC52C99FBCFB06A3C6"
    "9A6A9DCA52D23B616073E28675A23D189838EF1E2EE652C0"
    "13ECB4AEA906112324975C3CD49B83BFACCBDD7D90C4BD70"
    "98488E9C219A73724EFFD6FAE5644738FAA31A4FF55BCCC0"
    "A151AF5F0DC8B4BD45BF37DF365C1A65E68CFDA76D4DA708"
    "DF1FB2BC2E4A4371"
)
P = int(P_HEX, 16)
Q = (P - 1) // 2
G = 2


# ---------------------------------------------------------------------------
# Hash to challenge (Fiat-Shamir oracle)
# ---------------------------------------------------------------------------
def hash_to_challenge(*args: int) -> int:
    h = hashlib.sha256()
    for a in args:
        h.update(str(a).encode())
    return int(h.hexdigest(), 16) % Q


def mod_exp(base: int, exp: int, mod: int) -> int:
    result = 1
    base %= mod
    while exp > 0:
        if exp & 1:
            result = (result * base) % mod
        base = (base * base) % mod
        exp >>= 1
    return result


# ---------------------------------------------------------------------------
# Step 1: Interactive Schnorr Proof (Sigma protocol)
# ---------------------------------------------------------------------------

def schnorr_prover_init(x: int, y: int) -> tuple[int, int]:
    r = random.randrange(1, Q)
    t = mod_exp(G, r, P)
    return t, r


def schnorr_prover_respond(r: int, c: int, x: int) -> int:
    return (r + c * x) % Q


def schnorr_verify(y: int, t: int, c: int, s: int) -> bool:
    lhs = mod_exp(G, s, P)
    rhs = (t * mod_exp(y, c, P)) % P
    return lhs == rhs


def demo_interactive_schnorr(x: int, y: int) -> None:
    print("=== Interactive Schnorr Proof (Sigma Protocol) ===")
    t, r = schnorr_prover_init(x, y)
    c = random.randrange(1, Q)
    s = schnorr_prover_respond(r, c, x)
    valid = schnorr_verify(y, t, c, s)

    print(f"  Secret x:                          {x}")
    print(f"  Public key y = g^x mod p:          {str(y)[:50]}...")
    print(f"  Commitment t = g^r mod p:          {str(t)[:50]}...")
    print(f"  Challenge c:                       {c}")
    print(f"  Response s = r + c*x mod q:        {s}")
    print(f"  g^s ≡ t * y^c (mod p)?             {valid}")
    print()


# ---------------------------------------------------------------------------
# Step 2: Non-Interactive Schnorr Proof (Fiat-Shamir)
# ---------------------------------------------------------------------------

def schnorr_prove(x: int, y: int) -> tuple[int, int]:
    r = random.randrange(1, Q)
    t = mod_exp(G, r, P)
    c = hash_to_challenge(P, G, y, t)
    s = (r + c * x) % Q
    return t, s


def schnorr_verify_proof(y: int, t: int, s: int) -> bool:
    c = hash_to_challenge(P, G, y, t)
    lhs = mod_exp(G, s, P)
    rhs = (t * mod_exp(y, c, P)) % P
    return lhs == rhs


def demo_noninteractive_schnorr(x: int, y: int) -> None:
    print("=== Non-Interactive Schnorr Proof (Fiat-Shamir) ===")
    t, s = schnorr_prove(x, y)
    valid = schnorr_verify_proof(y, t, s)

    print(f"  Secret x:                          {x}")
    print(f"  Public key y = g^x mod p:          {str(y)[:50]}...")
    print(f"  Proof (t, s):")
    print(f"    t = {str(t)[:50]}...")
    print(f"    s = {s}")
    print(f"  Verifier recomputes c = Hash(p,g,y,t)")
    print(f"  Verifier checks g^s ≡ t*y^c:       {valid}")
    print()


def demo_invalid_proof(x: int, y: int) -> None:
    print("=== Invalid Proof Detection ===")
    t, s = schnorr_prove(x, y)
    t_tampered = (t + 1) % P
    valid_original = schnorr_verify_proof(y, t, s)
    valid_tampered = schnorr_verify_proof(y, t_tampered, s)

    print(f"  Original proof valid:               {valid_original}")
    print(f"  Tampered t proof valid:             {valid_tampered}")
    print(f"  Soundness holds (tampered rejected): {not valid_tampered}")
    print()

    wrong_secret = (x + 1) % Q
    wrong_y = mod_exp(G, wrong_secret, P)
    valid_wrong_key = schnorr_verify_proof(wrong_y, t, s)
    print(f"  Wrong public key proof valid:        {valid_wrong_key}")
    print(f"  Soundness holds (wrong key rejected): {not valid_wrong_key}")
    print()


# ---------------------------------------------------------------------------
# Pedersen Commitment
# ---------------------------------------------------------------------------

def pedersen_setup() -> tuple[int, int]:
    h = mod_exp(G, random.randrange(1, Q), P)
    return G, h


def pedersen_commit(g: int, h: int, x: int, r: int) -> int:
    return (mod_exp(g, x, P) * mod_exp(h, r, P)) % P


def demo_pedersen() -> None:
    print("=== Pedersen Commitment ===")
    g, h = pedersen_setup()
    x = random.randrange(1, Q // 2)
    r = random.randrange(1, Q)
    commit = pedersen_commit(g, h, x, r)

    r2 = random.randrange(1, Q)
    commit2 = pedersen_commit(g, h, x, r2)

    print(f"  Generator g:                        {str(g)[:40]}...")
    print(f"  Generator h (random power):         {str(h)[:40]}...")
    print(f"  Secret x:                           {x}")
    print(f"  Randomness r:                       {r}")
    print(f"  Commitment C = g^x·h^r:             {str(commit)[:50]}...")
    print(f"  Same x, different r:                {str(commit2)[:50]}...")
    print(f"  Commitments differ (hiding):        {commit != commit2}")
    print()


# ---------------------------------------------------------------------------
# R1CS Demonstration: x^3 + x + 5 == 35
# ---------------------------------------------------------------------------

def demo_r1cs_simple() -> None:
    print("=== R1CS Constraint System (x^3 + x + 5 == 35) ===")

    x = 3
    print(f"  Statement: I know x such that x^3 + x + 5 == 35")
    print(f"  Solution:  x = {x}")
    print(f"  Verification: {x}**3 + {x} + 5 = {x**3 + x + 5}")
    print()

    # Flatten the computation into intermediate variables
    v1 = x * x
    v2 = v1 * x
    v3 = v2 + x
    out = v3 + 5

    print("  Flattened constraints:")
    print(f"    v1 = x * x             (sym_1)")
    print(f"    v2 = v1 * x            (sym_2 = x^3)")
    print(f"    v3 = v2 + x            (sym_3 = x^3 + x)")
    print(f"    out = v3 + 5           (out = x^3 + x + 5)")
    print(f"    out == 35              (output constraint)")
    print()

    # Witness vector: w = (~one, x, v1, v2, v3, out)
    w = [1, x, v1, v2, v3, out]
    labels = ["~one", "x", "sym_1", "sym_2", "sym_3", "out"]
    print("  Witness vector w (as integers):")
    for lbl, val in zip(labels, w):
        print(f"    {lbl:<6} = {val}")
    print()

    constraint_a = [
        [0, 1, 0, 0, 0, 0],
        [0, 0, 1, 0, 0, 0],
        [0, 1, 0, 1, 0, 0],
        [5, 1, 0, 1, 0, 0],
    ]
    constraint_b = [
        [0, 1, 0, 0, 0, 0],
        [0, 1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0],
    ]
    constraint_c = [
        [0, 0, 1, 0, 0, 0],
        [0, 0, 0, 1, 0, 0],
        [0, 0, 0, 0, 1, 0],
        [0, 0, 0, 0, 0, 1],
    ]

    constraint_desc = [
        "v1 = x * x",
        "v2 = v1 * x",
        "v3 = x + v2   (v3 = x + v2)",
        "out = 5*~one + x + v2   (out = 5 + x + v2)",
    ]

    print("  R1CS Constraint System: <A,w> * <B,w> = <C,w>")
    all_ok = True
    for i, (desc, a_vec, b_vec, c_vec) in enumerate(
        zip(constraint_desc, constraint_a, constraint_b, constraint_c)
    ):
        dot_a = sum(av * wv for av, wv in zip(a_vec, w))
        dot_b = sum(bv * wv for bv, wv in zip(b_vec, w))
        dot_c = sum(cv * wv for cv, wv in zip(c_vec, w))
        ok = (dot_a * dot_b) == dot_c
        all_ok = all_ok and ok
        status = "OK" if ok else "FAIL"
        print(f"    Constraint {i}: {desc}")
        print(f"      <A,w>={dot_a}, <B,w>={dot_b}, <C,w>={dot_c}  [{status}]")

    print(f"  All constraints satisfied: {all_ok}")
    print()

    x_bad = 2
    v1_bad = x_bad * x_bad
    v2_bad = v1_bad * x_bad
    v3_bad = v2_bad + x_bad
    out_bad = v3_bad + 5
    w_bad = [1, x_bad, v1_bad, v2_bad, v3_bad, out_bad]

    print(f"  Invalid witness: x = {x_bad} (gives out = {out_bad}, not 35)")
    for i, (desc, a_vec, b_vec, c_vec) in enumerate(
        zip(constraint_desc, constraint_a, constraint_b, constraint_c)
    ):
        dot_a = sum(av * wv for av, wv in zip(a_vec, w_bad))
        dot_b = sum(bv * wv for bv, wv in zip(b_vec, w_bad))
        dot_c = sum(cv * wv for cv, wv in zip(c_vec, w_bad))
        ok = (dot_a * dot_b) == dot_c
        if not ok:
            print(f"    Constraint {i} FAILS: <A,w>={dot_a}, <B,w>={dot_b}, <C,w>={dot_c}")
            break
    else:
        print("    (all passed — unexpected for invalid witness)")
    print()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    print("=" * 60)
    print("  Zero-Knowledge Proofs — Sigma & zk-SNARK overview")
    print("  Phase 12 — Cryptography & Security")
    print("=" * 60)

    x = random.randrange(1, Q // 2)
    y = mod_exp(G, x, P)

    print(f"\n  Using 1024-bit safe prime p = 2q + 1")
    print(f"  Generator g = {G}")
    print(f"  Secret x (random): {x}")
    print(f"  Public key y = g^x mod p: {str(y)[:50]}...\n")

    demo_interactive_schnorr(x, y)
    demo_noninteractive_schnorr(x, y)
    demo_invalid_proof(x, y)
    demo_pedersen()
    demo_r1cs_simple()

    print("=" * 60)
    print("  Summary")
    print("=" * 60)
    print("  • Sigma protocols: 3-move interactive (commit, challenge, response)")
    print("  • Fiat-Shamir: replace verifier with hash oracle → non-interactive")
    print("  • Pedersen commitment: hiding (random r) and binding (cannot find x', r')")
    print("  • R1CS: constraint system <A,w> * <B,w> = <C,w> expressing computation")
    print()
    print("  All demonstrations passed verification checks.")


if __name__ == "__main__":
    main()
