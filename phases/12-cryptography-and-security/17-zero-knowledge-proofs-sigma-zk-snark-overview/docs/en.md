# Zero-Knowledge Proofs — Sigma, zk-SNARK overview

> You have a secret. Someone else wants proof you know it. Zero-knowledge proofs let you convince them without revealing the secret itself.

**Type:** Learn (with Build elements)
**Languages:** Python, Rust
**Prerequisites:** Phase 12 lessons 01–16 (especially 09-11 for public key crypto, 01-02 for discrete log)
**Time:** ~75 minutes

## Learning Objectives

- Define the three pillars of zero-knowledge proofs: completeness, soundness, zero-knowledge.
- Implement a Schnorr Sigma protocol for proving knowledge of a discrete log.
- Transform an interactive proof into a non-interactive one using the Fiat-Shamir heuristic.
- Explain how R1CS and QAP convert a computation into a constraint system for SNARKs.
- Distinguish SNARKs from STARKs on trusted setup, proof size, and post-quantum security.
- Identify real-world ZKP applications: Zcash, zk-rollups, anonymous credentials.

## The Problem

You are a prover. I am a verifier. You claim to know the discrete log `x` of a public key `y = g^x`. If you just tell me `x`, I can trivially verify — but now you've revealed your secret. If you don't tell me, how can I possibly be sure you know it?

This is the core question of zero-knowledge proofs (ZKPs). The tension is fundamental: proof usually requires revelation. ZK breaks that link.

Without ZK:
- Authentication means sending a password (or a hash of it) — now the verifier holds what they need to impersonate you.
- Anonymous credentials require a trusted third party to attest your attributes.
- Blockchain transactions reveal sender, receiver, and amount to every node.

With ZK:
- You prove you know the password without transmitting it.
- You prove you're over 18 without showing your birthdate.
- You prove a transaction is valid (inputs exist, signatures correct, no double-spend) without revealing the parties or amounts.

The phase capstone (a TLS 1.3 library + mini-CTF) doesn't use ZK directly, but the same reasoning — separating "what you prove" from "what you reveal" — underlies the design of anonymous signatures, private certificate verification, and side-channel-resistant authentication.

## The Concept

### The Three Pillars

Every zero-knowledge proof must satisfy three properties:

1. **Completeness:** If the prover knows the secret (the *witness*) and follows the protocol, the verifier will accept the proof.
2. **Soundness:** If the prover does *not* know the secret, the verifier will reject the proof with overwhelming probability (except for a negligible "soundness error").
3. **Zero-Knowledge:** The verifier learns nothing except that the statement is true. The transcript of the interaction could be simulated without the prover's secret.

### Interactive Proofs: Sigma Protocols

A Sigma protocol is a three-move interactive proof:

```
Prover                     Verifier
  |                            |
  |—— commitment t ————————→  |
  |                            |
  |←—— challenge c —————————— |
  |                            |
  |—— response s ———————————→ |
  |                            |
  |                    accepts iff check(t, c, s) passes
```

The name "Sigma" comes from the Greek letter Σ, representing the three passes.

### Schnorr Protocol: Proving Knowledge of a Discrete Log

The canonical Sigma protocol. Given a group G of prime order q, generator g, and public key y = g^x:

1. **Prover** picks random r ← Z_q, computes t = g^r, sends t.
2. **Verifier** picks random c ← Z_q, sends c.
3. **Prover** computes s = r + c·x (mod q), sends s.

**Verification:** Verifier checks g^s == t · y^c.

**Completeness:** g^(r + c·x) = g^r · g^(c·x) = t · y^c. Always works.

**Soundness:** A prover who doesn't know x can commit to t, but after seeing c, can only respond correctly with probability 1/q (negligible). From two transcripts (t, c₁, s₁) and (t, c₂, s₂) with the same t and different challenges, an extractor can compute x = (s₁ − s₂)/(c₁ − c₂) mod q.

**Zero-knowledge:** A simulator picks c and s first, then computes t = g^s · y^(−c). The transcript (t, c, s) is indistinguishable from a real one, and the simulator never needed x.

### Fiat-Shamir Heuristic: Making Proofs Non-Interactive

Replace the verifier's random challenge with a hash of the commitment (and optionally the public inputs):

```
c = Hash(g, y, t)
```

The proof becomes `(t, s)` — a single message from prover to verifier. The verifier recomputes c = Hash(g, y, t) and checks g^s == t · y^c.

This is the **Fiat-Shamir transform**: any public-coin interactive proof can be made non-interactive by replacing the verifier with a hash function (modeled as a random oracle). The resulting proof is publicly verifiable — anyone can check it, not just the original verifier.

### How Proof Systems Scale: Interactive → Non-Interactive → Succinct

```
Interactive (Sigma)    Non-Interactive (Fiat-Shamir)    Succinct (SNARKs)
   3 moves                  1 message                    constant-size proof
   verifier must             publicly verifiable         sublinear verification
   be online                                             trusted setup (Groth16)
```

### zk-SNARKs (Zero-Knowledge Succinct Non-interactive ARguments of Knowledge)

SNARKs compress an arbitrary computation into a tiny proof that can be verified in milliseconds.

**How they work (high level):**

1. **Arithmetic Circuit:** The computation is expressed as a circuit of addition and multiplication gates over a finite field.

2. **R1CS (Rank-1 Constraint System):** The circuit is flattened into a set of constraints of the form `<A, w> * <B, w> = <C, w>`, where w is a vector of all wire values (including the witness).

   Example — proving "I know x such that x³ + x + 5 = 35":
   - Introduce intermediate variables: sym₁ = x*x, sym₂ = sym₁*x, sym₃ = sym₂ + x, out = sym₃ + 5
   - Constraints: sym₁ · 1 = sym₁ (but really sym₁ = x*x), sym₂ = sym₁*x, etc.
   - This is a constraint system over 4 equations.

3. **QAP (Quadratic Arithmetic Program):** The R1CS constraints are transformed into polynomials via Lagrange interpolation. The prover knows polynomials A(z), B(z), C(z) such that A(z)*B(z) − C(z) is divisible by a target polynomial Z(z) — *if and only if* they have a valid witness.

4. **Trusted Setup:** A one-time ceremony generates "proving key" and "verification key" parameters. In Groth16 (the most efficient pairing-based SNARK), this involves a "toxic waste" of random values that must be destroyed — if leaked, anyone can forge proofs.

5. **Proof:** The prover computes a proof consisting of just 3 group elements (Groth16). The verifier checks 1 pairing equation.

**Applications:**
- **Zcash:** The first production SNARK deployment — private transactions using Groth16.
- **zkSync, Scroll, Polygon zkEVM:** zk-rollups that batch thousands of Ethereum transactions into a single SNARK proof.
- **MACI (Minimum Anti-Collusion Infrastructure):** Anonymous voting with ZK proofs to prevent bribery.

### zk-STARKs (Scalable Transparent ARguments of Knowledge)

| Property | SNARK (Groth16) | STARK |
|----------|-----------------|-------|
| Proof size | ~200 bytes | ~100 KB |
| Verification time | ~2 ms | ~10 ms |
| Trusted setup | Required (per-circuit) | None (transparent) |
| Post-quantum | No (uses pairings) | Yes (hash-based) |
| Prover time | Moderate | Slower |

STARKs replace the trusted setup with a **transparent** (public coin) protocol based on polynomial commitments using hash functions. They have larger proofs but are quantum-resistant.

### Security Model

ZK proofs argue security in terms of **knowledge soundness**: not just that the statement is true (e.g., a satisfying assignment exists) but that the prover *knows* a witness for it. This is formalized via an extractor: for any prover that produces an acceptable proof, there exists an extractor that, given access to the prover's internal state, outputs a valid witness.

## Build It

### Step 1: Interactive Schnorr Proof (Python)

We implement a Sigma protocol for proving knowledge of a discrete log. We work in a subgroup of Z_p^* with a safe prime.

```python
import hashlib
import random

# Public parameters: safe prime p = 2q + 1, generator g
# For demonstration only — real ZK needs 2048+ bit primes
p = 0xB10B8F96A080E01DDE92DE5EAE5D54EC52C99FBCFB06A3C6
q = (p - 1) // 2
g = 2

def schnorr_prover(x, y):
    """Interactive prover: returns commitment t, then waits for challenge c."""
    r = random.randrange(1, q)
    t = pow(g, r, p)
    return t, r  # r saved to compute response after receiving c

def schnorr_respond(r, c, x):
    """Compute response s = r + c*x (mod q)."""
    return (r + c * x) % q

def schnorr_verify(y, t, c, s):
    """Verifier checks g^s ≡ t * y^c (mod p)."""
    lhs = pow(g, s, p)
    rhs = (t * pow(y, c, p)) % p
    return lhs == rhs
```

The interactive protocol runs as:
1. Prover: compute `t, r = schnorr_prover(x, y)`, send `t`.
2. Verifier: pick random `c`, send it.
3. Prover: compute `s = schnorr_respond(r, c, x)`, send `s`.
4. Verifier: `schnorr_verify(y, t, c, s)` → True/False.

### Step 2: Non-Interactive Schnorr Proof (Python — Fiat-Shamir)

Replace the verifier's random challenge with `c = Hash(p || g || y || t)`:

```python
def hash_to_challenge(*args):
    h = hashlib.sha256()
    for a in args:
        h.update(str(a).encode())
    return int(h.hexdigest(), 16) % q

def schnorr_prove(x, y):
    """Non-interactive proof: returns (t, s)."""
    r = random.randrange(1, q)
    t = pow(g, r, p)
    c = hash_to_challenge(p, g, y, t)
    s = (r + c * x) % q
    return (t, s)

def schnorr_verify_proof(y, t, s):
    """Verify a non-interactive proof."""
    c = hash_to_challenge(p, g, y, t)
    return schnorr_verify(y, t, c, s)
```

The prover outputs `(t, s)`. The verifier recomputes `c` from the transcript and checks the equation. This is publicly verifiable — anyone with `(y, t, s)` can check.

### Step 3: R1CS and zk-SNARK Concepts (Rust)

The Rust program demonstrates:
1. **Schnorr proof** using `num-bigint` for large modular arithmetic.
2. **Fiat-Shamir transformation** using SHA-256.
3. **R1CS constraint system** for an NP statement: "I know x such that x² + 3x + 1 = 11."

The R1CS demonstration converts the computation into constraints:

```
Expression: x² + 3x + 1 = 11

Step 1: v1 = x * x          (multiplication constraint)
Step 2: v2 = 3 * x          (scalar multiplication)
Step 3: v3 = v1 + v2 + 1   (addition)
Step 4: v3 = 11             (output constraint)
```

Each step becomes an R1CS constraint of the form:
```
<a, w> * <b, w> = <c, w>
```

The witness vector `w` contains `(1, x, v1, v2, v3)` and the constraint matrices encode the arithmetic.

## Use It

- **Zcash** was the first major deployment of zk-SNARKs in production. Each shielded transaction uses Groth16 to prove: (1) the input notes exist on-chain, (2) the prover holds the spending keys, (3) the output notes commit to valid addresses, (4) no value is created or destroyed. The proof is ~200 bytes and verifies in <7 ms.
- **zk-rollups** (zkSync, StarkNet, Scroll) post a single SNARK or STARK proof to Ethereum L1 that validates an entire batch of transactions. The L1 verifier checks one proof instead of re-executing thousands of transactions — this is the scalability breakthrough.
- **MACI** uses ZK proofs to guarantee that votes are counted correctly while preventing bribery: each vote is accompanied by a proof that it was cast by a valid key, and the final tally includes a proof that all votes were tallied honestly.

Your Schnorr implementation is simpler than production ZK (no pairings, no QAP, no trusted setup), but it shares the core insight: the prover commits first, then demonstrates consistency. That pattern — commit, challenge, respond — shows up in every ZK protocol.

## Read the Source

- [bellman (Rust ZK crate)](https://github.com/zkcrypto/bellman) — the standard Groth16 prover. Look at `src/groth16/mod.rs` for the proof generation and `src/groth16/verifier.rs` for verification.
- [libsnark (C++)](https://github.com/scipr-lab/libsnark) — the reference implementation of R1CS-to-QAP and Groth16. Check `src/relations/r1cs/r1cs.hpp` for the constraint definition.
- [Groth16 paper](https://eprint.iacr.org/2016/260.pdf) — "On the Size of Pairing-Based Non-interactive Arguments" by Jens Groth. The 3-element proof standard.
- [R1CS explained](https://medium.com/@VitalikButerin/quadratic-arithmetic-programs-from-zero-to-hero-f6dbbcea5c1d) — Vitalik Buterin's blog post walking through R1CS and QAP with concrete numbers.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`outputs/zkp-lib/`** — a self-contained zero-knowledge proof library implementing Schnorr proofs (interactive and non-interactive) along with an R1CS constraint system demonstration. The Python version provides the implementation; the Rust version provides a systems-language implementation with num-bigint arithmetic.

## Exercises

1. **Easy** — Run `schnorr_prove` for three different secrets. Verify each proof. Now modify one byte of `t` in the proof tuple and verify that `schnorr_verify_proof` returns False. Explain why this fails each pillar.

2. **Medium** — Implement the Pedersen commitment scheme in Python: `commit(x, r) = g^x · h^r` where `h` is a second generator with unknown discrete log. Combine it with the Schnorr proof to prove that a committed value equals the discrete log of a public key. This is the foundation of anonymous credentials.

3. **Hard** — Extend the R1CS Rust demonstration to support a more complex computation (e.g., `x³ + 2x² + 3x + 4 = y` for an arbitrary `y`). Add a `verify_proof` function that checks the satisfiability of the constraint system given a witness. This is a minimal step toward a real SNARK prover.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Zero-knowledge proof | Proving something without revealing it | A protocol where the verifier learns nothing except the truth of the statement; formally, there exists a simulator that produces indistinguishable transcripts without the witness |
| Sigma protocol | Three-move interactive proof | Commit, challenge, response; the canonical form for proofs of knowledge like Schnorr |
| Schnorr proof | Prove you know a discrete log | The prover shows g^s = t·y^c where s = r + c·x, proving knowledge of x without revealing it |
| Fiat-Shamir heuristic | Make interactive proofs non-interactive | Replace the verifier's random challenge with a hash of the transcript; secure in the random oracle model |
| zk-SNARK | Succinct zero-knowledge proof | Constant-size proof (Groth16: 3 group elements), sublinear verification, but requires a trusted setup per circuit |
| zk-STARK | Scalable transparent argument | No trusted setup, post-quantum secure, larger proofs (~100 KB), uses hash-based polynomial commitments |
| R1CS | Rank-1 Constraint System | A constraint of the form <a,w>·<b,w> = <c,w> that flattens an arithmetic circuit into equations over a finite field |
| QAP | Quadratic Arithmetic Program | Polynomial encoding of R1CS constraints; checking satisfiability reduces to checking polynomial divisibility |
| Trusted setup | Ceremony to generate proving/verification keys | A multi-party computation that produces public parameters and then discards the secret randomness ("toxic waste") |
| Arithmetic circuit | Computation as gates over a field | Addition and multiplication gates connecting wires; the universal representation for SNARK computations |
| Witness | The secret the prover knows | The satisfying assignment to the circuit wires that the prover keeps hidden |
| Extractor | A machine that extracts the witness | Used in the security proof: given a successful prover as a black box, the extractor outputs a valid witness |

## Further Reading

- [Groth16: On the Size of Pairing-Based Non-interactive Arguments](https://eprint.iacr.org/2016/260.pdf) — the paper that achieved the minimum possible proof size (3 group elements).
- [Zcash Protocol Specification](https://github.com/zcash/zips/blob/master/protocol/protocol.pdf) — how SNARKs are deployed in production for private payments.
- Vitalik Buterin, [Quadratic Arithmetic Programs: from Zero to Hero](https://medium.com/@VitalikButerin/quadratic-arithmetic-programs-from-zero-to-hero-f6dbbcea5c1d) — the best intuitive explanation of R1CS and QAP.
- StarkWare, [STARKs vs. SNARKs](https://starkware.co/stark-vs-snark/) — comparison of the two approaches.
- Boneh and Shoup, *A Graduate Course in Applied Cryptography* — Chapters 18–19 cover Sigma protocols and Fiat-Shamir with full proofs.
- [ZKProof Standards](https://zkproof.org) — the community-driven standardization effort for zero-knowledge proofs.
