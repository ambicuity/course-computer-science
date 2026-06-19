# Information & Coding Theory (Discrete)

> Entropy = the average number of bits a message *needs*. Huffman codes hit it; error-correcting codes pay for redundancy. The whole field falls out of one equation.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 01, Lessons 04, 19
**Time:** ~60 minutes

## Learning Objectives

- Compute Shannon entropy `H(X) = -Σ p(x) log₂ p(x)` for a discrete distribution; explain what it means in bits.
- Build a Huffman code for a given symbol distribution; verify it approaches the entropy lower bound.
- Define Hamming distance and Hamming weight; reason about single-error-correcting codes (Hamming (7,4)).
- Connect entropy to compression (lower bound) and channel capacity (upper bound on reliable bits/second).

## The Problem

Two flagship questions in CS:

1. "What's the shortest possible encoding of these symbols?" → Shannon entropy + Huffman.
2. "How do I encode N bits so that a one-bit flip can be detected (or corrected)?" → Hamming codes + parity.

These are the foundations of compression (gzip, JPEG), error correction (CDs, RAID, satellite links), cryptography's notion of "indistinguishability," and ML's cross-entropy loss.

## The Concept

### Shannon entropy

For a random variable X with distribution `p(x)`:

```
H(X) = - Σ_x p(x) · log₂ p(x)
```

(With the convention `0 · log 0 = 0`.) Units: bits, when log is base 2.

**Intuition**: H(X) is the average number of yes/no questions you'd need to learn X. For a fair coin (p = 1/2), H = 1 bit. For an 8-sided fair die, H = 3 bits. For a degenerate distribution (one outcome has p=1), H = 0 — no uncertainty.

Properties:
- 0 ≤ H(X) ≤ log₂ |support of X|.
- Maximum is log₂ n, achieved by the uniform distribution.
- H is concave in p.

### Joint and conditional entropy, mutual information

For two RVs X, Y:

- **Joint entropy**: `H(X, Y) = - Σ p(x, y) log p(x, y)`.
- **Conditional entropy**: `H(X | Y) = H(X, Y) - H(Y)`.
- **Mutual information**: `I(X; Y) = H(X) - H(X | Y) = H(Y) - H(Y | X)` — how many bits of X are revealed by knowing Y.

Mutual information is the basis of:
- Decision-tree splits (information gain).
- Feature selection in ML.
- Privacy / leakage analysis.

### Shannon's source coding theorem

> The minimum expected codeword length to encode i.i.d. samples from X with a *prefix code* is at least H(X). Achievable to within 1 bit per symbol.

In practice, **Huffman coding** achieves this:

```
Build a min-heap of (frequency, symbol).
While >1 elements remain:
    pop the two smallest; combine into a new node with sum of frequencies; push back.
The resulting tree's root-to-leaf paths give optimal prefix codes.
```

Average codeword length L satisfies H(X) ≤ L < H(X) + 1.

### Hamming distance and codes

Hamming distance `d(u, v)` = number of positions where bit-strings u and v differ.

A code C ⊆ {0, 1}^n with minimum distance d can:
- **Detect** up to d - 1 errors (any single flip stays within d of the original).
- **Correct** up to ⌊(d - 1) / 2⌋ errors (closer to original than to any other codeword).

The **Hamming (7,4) code** encodes 4 data bits into 7 transmitted bits using 3 parity bits, with minimum distance 3 → corrects any single-bit error.

```
Data bits:   d₁ d₂ d₃ d₄
Parity bits: p₁ = d₁ ⊕ d₂ ⊕ d₄
             p₂ = d₁ ⊕ d₃ ⊕ d₄
             p₄ = d₂ ⊕ d₃ ⊕ d₄

Codeword (position order, 1-indexed): p₁ p₂ d₁ p₄ d₂ d₃ d₄
```

Decoder computes three syndrome bits = parity over their respective groups in the received word; the three bits as a binary number point to the flipped bit position (or 0 = no error).

### Channel capacity (Shannon's noisy-channel theorem)

For a noisy channel with input X and output Y, the maximum rate of reliable transmission is `C = max_p I(X; Y)`. For a binary symmetric channel with error probability p:

```
C = 1 - H(p)     where H(p) = -p log p - (1-p) log(1-p)
```

For p = 0.1: C ≈ 0.531 — the channel can carry ~half a bit per symbol reliably. Modern codes (LDPC, Turbo) get within 0.1 dB of capacity.

## Build It

Open `code/main.py`.

### Step 1: Entropy

Verify: entropy of fair coin = 1.0; fair 8-sided die = 3.0; degenerate = 0.

### Step 2: Huffman tree

For an English-letter frequency distribution, observe `H ≈ 4.17 bits` (vs naive ~4.7 bits for the 26 letters under uniform), and Huffman achieves `L ≈ 4.22` — within 1 bit.

### Step 3: Hamming (7,4) encode + single-error correct

Flip any one of 7 bits in a codeword; the decoder identifies which bit and recovers the original.

### Step 4: Binary-symmetric-channel capacity

For p ∈ {0, 0.05, 0.1, …, 0.5}, compute 1 - H(p). Capacity = 1 at p=0, drops to 0 at p = 0.5.

## Use It

- **Compression**: gzip uses a variant of Huffman (DEFLATE); zstd uses arithmetic coding (close to H exactly). JPEG and MP3 use entropy coding stages after lossy transforms.
- **Error correction**: RAID 5/6, ECC RAM, CDs (Reed-Solomon), 5G (LDPC), QR codes (Reed-Solomon).
- **Cryptography**: entropy of a key = security level in bits. AES-128 has 128 bits of entropy iff the key is uniformly random.
- **ML**: cross-entropy loss = expected -log p(true label | input) under the model's distribution; minimized when model = data distribution.
- **Decision trees**: split on the feature that maximizes information gain = H(parent) - Σ H(children).

## Read the Source

- *Information Theory, Inference, and Learning Algorithms* by David MacKay — free online; the best intro.
- Shannon's original 1948 paper "A Mathematical Theory of Communication" — readable; foundational.
- *Elements of Information Theory* by Cover & Thomas — graduate-level reference.

## Ship It

This lesson ships **`outputs/coding.py`** — `entropy`, `huffman_codes`, `hamming74_encode`, `hamming74_decode`. Useful in any later lesson with compression or coding.

## Exercises

1. **Easy.** Compute the entropy of the English-letter distribution (A=8.17%, B=1.49%, …). Compare with naive 4.7 bits per character (log₂ 26).
2. **Medium.** Build a Huffman code for the English distribution above. Average codeword length should be ~ 4.22 bits — within 1 bit of entropy (~ 4.17).
3. **Hard.** Implement Hamming (15,11): 11 data bits + 4 parity = 15 transmitted, corrects any single-bit error. Verify by flipping each bit of 1000 random codewords and confirming decode recovers the original.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Entropy H(X) | "Information content" | -Σ p log₂ p; expected bits to optimally encode one sample of X |
| Mutual information I(X;Y) | "How much X tells you about Y" | H(X) - H(X|Y); bits about X revealed by knowing Y |
| Prefix code | "No codeword is a prefix of another" | The decode-without-delimiters property; Huffman codes are prefix codes |
| Hamming distance | "Bit-difference count" | Number of positions where two equal-length strings differ |
| Channel capacity | "Max reliable bits/second" | Supremum of mutual information over all input distributions |

## Further Reading

- *Coding Theory: A First Course* by San Ling, Chaoping Xing — modern, undergrad-friendly textbook.
- [Polar codes (Arikan 2008)](https://arxiv.org/abs/0807.3917) — capacity-achieving codes used in 5G.
- *Information Theory: A Tutorial Introduction* by James Stone — short, focused, very visual.
