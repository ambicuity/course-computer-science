# Classical Ciphers and Why They Fail

> Every cipher that leaked patterns died — and the patterns always leak.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 12 lesson 01
**Time:** ~45 minutes

## Learning Objectives

- Implement Caesar, substitution, and Vigenère ciphers from scratch and break each one with the corresponding cryptanalytic attack.
- Explain why the one-time pad is provably unbreakable (Shannon 1949) and why it is impractical for most real use cases.
- Define Shannon's confusion and diffusion properties and explain how their absence in classical ciphers leads to systematic breaks.
- Analyze the Enigma machine as a case study in key-management failure and operator-error exploitation.

## The Problem

You need to send a message that an eavesdropper cannot read. For thousands of years, the answer was "scramble the letters." Caesar shifted them. Monarchs substituted them. Generals used polyalphabetic keywords. Every single one was broken.

This is not a historical curiosity. The *same structural flaws* that broke Caesar's cipher — small key space, statistical leakage, no diffusion — reappear in every weak cryptosystem. When a modern protocol uses a 40-bit key or a linear feedback shift register, it is making the exact same mistake Julius Caesar made, just with more bits. Understanding *why* classical ciphers fail gives you the mental model to spot weak cryptography anywhere.

The phase capstone (a TLS 1.3 implementation) requires you to understand what a cipher must achieve — confusion and diffusion — so that when you evaluate AES-GCM later, you know exactly what problems it solves and what would happen if it didn't.

## The Concept

### The Caesar Cipher

Shift every letter by a fixed amount *k*:

```
Plaintext:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
Ciphertext: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C   (k=3)
```

Key space: 25 possible keys (shifts 1–25). An attacker tries all of them in seconds. Even if you don't know the language, only one shift produces readable text. Frequency analysis makes it even easier — the most common ciphertext letter maps to E.

### The Substitution Cipher

Map each letter to an arbitrary different letter. The key is a permutation of the alphabet — 26! ≈ 4 × 10²⁶ possibilities. Exhaustive search is infeasible, but the cipher is still trivially broken because each plaintext letter always maps to the *same* ciphertext letter. The frequency distribution of English (ETAOIN SHRDLU) is a fingerprint:

```
Letter frequency in English:   E≈12.7%  T≈9.1%  A≈8.2%  O≈7.5%  I≈7.0%  N≈6.7%  S≈6.3%  H≈6.1%
Letter frequency in ciphertext: identical shape, just relabeled.
```

An attacker counts letter frequencies, matches the peaks, and recovers most of the key. A hill-climbing search finishes the rest.

### The Vigenère Cipher

Use a keyword to select a *different* Caesar shift for each position:

```
Keyword:    K E Y K E Y K E Y K
Plaintext:  A T T A C K A T D A W N
Shift:      10 4 24 10 4 24 10 4 24 10
Ciphertext: K X T K G M K X B K
```

This *almost* works — each letter gets shifted differently, so the frequency fingerprint is smeared across multiple columns. But the keyword repeats, so if the key length is *L*, every *L*-th letter uses the same shift. The **Kasiski examination** (1863) exploits this:

1. Find repeated sequences in the ciphertext (e.g., "KXT" at positions 0 and 6).
2. The distance between repeats is a multiple of the key length.
3. The greatest common divisor of these distances narrows down *L*.
4. Once *L* is known, split the ciphertext into *L* columns and solve each as a Caesar cipher.

The Vigenère cipher held for 300 years until Kasiski broke it. The lesson: repeating keys create periodic structure that leaks the key length.

### The One-Time Pad

XOR the plaintext with a truly random key that is exactly as long as the message:

```
Plaintext: 0 1 1 0 1 0 0 1
Key:       1 0 0 1 1 1 0 0
Ciphertext:1 1 1 1 0 1 0 1
```

Shannon proved in 1949 that this is *perfectly* secret: every ciphertext is equally likely under every key, so ciphertext reveals zero information about plaintext. No amount of computation can break it.

The catch: the key must be truly random, as long as the message, never reused, and pre-shared over a secure channel. For a 1 GB message, you need to securely distribute a 1 GB key — at which point you could have just sent the message itself. The OTP works for diplomatic hotlines (where pre-shared key material exists) but not for the internet.

### Why Classical Ciphers Fail: Three Fundamental Flaws

| Flaw | Caesar | Substitution | Vigenère | OTP |
|------|--------|-------------|----------|-----|
| **Small key space** | 25 keys | Broken via frequency | Key repeats | Infinite (random) |
| **Statistical leaks** | Frequency match | Frequency match | Frequency per column | None |
| **No diffusion** | One letter → one letter | One letter → one letter | Local to key period | Full XOR |
| **No confusion** | Shift is trivial | Simple 1:1 mapping | Simple shift per column | Key XOR plaintext |

**Diffusion** means: changing one bit of plaintext should change ~50% of ciphertext bits. In classical ciphers, changing one plaintext letter changes exactly one ciphertext letter (or one position within the key period). This locality is what frequency analysis exploits.

**Confusion** means: the relationship between key and ciphertext should be complex. In classical ciphers, the key-message equation is trivially invertible (subtract the shift, look up the table). AES uses multiple rounds of substitution and mixing so that knowing a ciphertext bit tells you almost nothing about which key bit produced it.

### Enigma: A Case Study in System-Level Failure

The Enigma machine (WWII) was a polyalphabetic cipher with a plugboard — far stronger than a Vigenère. The Poles (Rejewski, 1932) and later the British (Turing, Welchman, 1939–45) broke it through a cascade of flaws:

- **Predictable key schedule**: Each day's key was transmitted in a predictable format. Operators enciphered the message key twice (e.g., "ABC ABC"), creating a known-plaintext structure.
- **Operator errors**: Operators chose predictable keys (girlfriend initials, keyboard patterns like "QWE"), used stereotyped message headers ("WEATHER"), and retransmitted with minor changes.
- **No diffusion within a letter**: Enigma never encrypted a letter as itself (a design property that reduced the search space), and the plugboard affected letters individually rather than mixing across positions.
- **Key distribution was the bottleneck**: Daily key sheets had to be physically distributed and captured.

The Bletchley Park breaks were not brute force against the machine's key space — they were exploitation of *system-level* weaknesses: protocol choices, operator behavior, and physical key distribution. Modern attacks on TLS follow the same pattern: the cipher (AES) is fine, but the *protocol around it* (certificate validation, session resumption, padding) is where the bugs live.

## Build It

Open `code/main.py` alongside this lesson. We will implement each classical cipher, then break it, then see why the one-time pad resists the same attacks.

### Step 1: Caesar Cipher — Encrypt and Decrypt

The shift cipher. Each letter is moved forward by *k* positions in the alphabet:

```python
def caesar_encrypt(text: str, shift: int) -> str:
    result = []
    for ch in text:
        if ch.isalpha():
            base = ord('A') if ch.isupper() else ord('a')
            result.append(chr((ord(ch) - base + shift) % 26 + base))
        else:
            result.append(ch)
    return ''.join(result)

def caesar_decrypt(text: str, shift: int) -> str:
    return caesar_encrypt(text, -shift)
```

Test it:

```python
>>> caesar_encrypt("ATTACK AT DAWN", 3)
'DWWDFN DW GDZQ'
>>> caesar_decrypt("DWWDFN DW GDZQ", 3)
'ATTACK AT DAWN'
```

### Step 2: Break Caesar — Brute Force and Frequency Analysis

Only 25 possible shifts. Try all of them and score each against English letter frequencies:

```python
def frequency_analysis(ciphertext: str) -> dict[str, float]:
    cleaned = [c.upper() for c in ciphertext if c.isalpha()]
    total = len(cleaned)
    if total == 0:
        return {}
    counts = {}
    for c in cleaned:
        counts[c] = counts.get(c, 0) + 1
    return {c: count / total for c, count in counts.items()}

ENGLISH_FREQ = {
    'E': 0.127, 'T': 0.091, 'A': 0.082, 'O': 0.075, 'I': 0.070,
    'N': 0.067, 'S': 0.063, 'H': 0.061, 'R': 0.060, 'D': 0.043,
    'L': 0.040, 'C': 0.028, 'U': 0.028, 'M': 0.024, 'W': 0.024,
    'F': 0.022, 'G': 0.020, 'Y': 0.020, 'P': 0.019, 'B': 0.015,
    'V': 0.010, 'K': 0.008, 'J': 0.002, 'X': 0.002, 'Q': 0.001,
    'Z': 0.001,
}

def _frequency_score(text: str) -> float:
    freq = frequency_analysis(text)
    return sum(abs(freq.get(c, 0) - ENGLISH_FREQ.get(c, 0)) for c in 'ABCDEFGHIJKLMNOPQRSTUVWXYZ')

def break_caesar(ciphertext: str) -> tuple[str, int]:
    best = (ciphertext, 0)
    best_score = float('inf')
    for shift in range(26):
        candidate = caesar_encrypt(ciphertext, -shift)
        score = _frequency_score(candidate)
        if score < best_score:
            best_score = score
            best = (candidate, shift)
    return best
```

```python
>>> ct = caesar_encrypt("THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG", 7)
>>> break_caesar(ct)
('THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG', 7)
```

25 tries. That's it. Caesar is broken.

### Step 3: Substitution Cipher — Encrypt and Decrypt

Map each letter to a different letter via a key dictionary:

```python
import string, random

def substitution_encrypt(text: str, key_map: dict[str, str]) -> str:
    return ''.join(key_map.get(c.upper(), c) if c.isalpha() else c for c in text)

def substitution_decrypt(text: str, key_map: dict[str, str]) -> str:
    inv = {v: k for k, v in key_map.items()}
    return ''.join(inv.get(c.upper(), c) if c.isalpha() else c for c in text)
```

26! ≈ 4 × 10²⁶ possible keys — but the frequency fingerprint is identical, just relabeled. An attacker reads the frequency chart, matches peaks, and recovers most of the mapping.

### Step 4: Break Substitution — Frequency Analysis + Hill Climbing

We score candidate keys by how well their decryption's frequency distribution matches English, then iteratively swap letters in the key to improve the score:

```python
import random

BIGRAM_FREQ = {
    'TH': 0.0356, 'HE': 0.0307, 'IN': 0.0243, 'ER': 0.0205, 'AN': 0.0198,
    'RE': 0.0185, 'ON': 0.0176, 'AT': 0.0149, 'EN': 0.0145, 'ND': 0.0135,
    'TI': 0.0134, 'ES': 0.0134, 'OR': 0.0128, 'TE': 0.0127, 'OF': 0.0117,
    'ED': 0.0117, 'IS': 0.0113, 'IT': 0.0112, 'AL': 0.0109, 'AR': 0.0107,
    'ST': 0.0105, 'TO': 0.0104, 'NT': 0.0104, 'NG': 0.0095, 'SE': 0.0093,
    'HA': 0.0092, 'AS': 0.0087, 'OU': 0.0087, 'IO': 0.0083, 'LE': 0.0083,
    'VE': 0.0083, 'CO': 0.0079, 'ME': 0.0079, 'DE': 0.0076, 'HI': 0.0076,
    'RI': 0.0073, 'RO': 0.0073, 'IC': 0.0071, 'NE': 0.0069, 'EA': 0.0069,
    'RA': 0.0062, 'CE': 0.0062,
}

def _bigram_score(text: str) -> float:
    cleaned = ''.join(c for c in text.upper() if c.isalpha())
    bigrams = [cleaned[i:i+2] for i in range(len(cleaned) - 1)]
    if not bigrams:
        return 0.0
    total = 0.0
    for b in bigrams:
        total += BIGRAM_FREQ.get(b, 0.0)
    return total / len(bigrams)

def break_substitution(ciphertext: str, iterations: int = 20000) -> tuple[str, dict[str, str]]:
    freq = frequency_analysis(ciphertext)
    sorted_ct = sorted(freq.keys(), key=lambda c: freq.get(c, 0), reverse=True)
    sorted_en = sorted(ENGLISH_FREQ.keys(), key=lambda c: ENGLISH_FREQ[c], reverse=True)
    best_key = {}
    for i, c in enumerate(sorted_ct):
        if i < len(sorted_en):
            best_key[c] = sorted_en[i]
        else:
            best_key[c] = sorted_en[i % 26]
    for c in string.ascii_uppercase:
        if c not in best_key:
            best_key[c] = c
    remaining = [c for c in string.ascii_uppercase if c not in best_key.values()]
    for c in string.ascii_uppercase:
        if c not in best_key:
            best_key[c] = remaining.pop()
    best_text = substitution_decrypt(ciphertext, best_key).upper()
    best_score = _bigram_score(best_text) + _frequency_score(best_text) * -0.5
    for _ in range(iterations):
        a, b = random.sample(string.ascii_uppercase, 2)
        new_key = dict(best_key)
        new_key[a], new_key[b] = best_key[b], best_key[a]
        try_text = substitution_decrypt(ciphertext, new_key).upper()
        score = _bigram_score(try_text) + _frequency_score(try_text) * -0.5
        if score > best_score:
            best_score = score
            best_key = new_key
            best_text = try_text
    return best_text, best_key
```

This works well on texts longer than ~100 characters. Shorter texts don't have enough statistical signal.

### Step 5: Vigenère Cipher — Encrypt and Decrypt

A polyalphabetic cipher: the keyword selects which Caesar shift applies at each position:

```python
def vigenere_encrypt(text: str, keyword: str) -> str:
    result = []
    ki = 0
    for ch in text:
        if ch.isalpha():
            shift = ord(keyword[ki % len(keyword)].upper()) - ord('A')
            base = ord('A') if ch.isupper() else ord('a')
            result.append(chr((ord(ch.upper()) - ord('A') + shift) % 26 + ord('A')))
            ki += 1
        else:
            result.append(ch)
    return ''.join(result)

def vigenere_decrypt(text: str, keyword: str) -> str:
    inv_key = ''.join(chr((26 - (ord(c.upper()) - ord('A'))) % 26 + ord('A')) for c in keyword)
    return vigenere_encrypt(text, inv_key)
```

```python
>>> vigenere_encrypt("ATTACK AT DAWN", "KEY")
'KXTKGMKXTKAWQ'
>>> vigenere_decrypt("KXTKGMKXTKAWQ", "KEY")
'ATTACKATDAWN'
```

### Step 6: Break Vigenère — Kasiski Examination

The Kasiski examination finds the key length by searching for repeated substrings and computing their distances:

```python
from math import gcd
from collections import Counter

def kasiski_examination(ciphertext: str, min_len: int = 3) -> list[tuple[int, int]]:
    cleaned = ''.join(c for c in ciphertext.upper() if c.isalpha())
    distances = []
    for length in range(min_len, min(len(cleaned) // 2, 6)):
        seen: dict[str, int] = {}
        for i in range(len(cleaned) - length + 1):
            sub = cleaned[i:i + length]
            if sub in seen:
                dist = i - seen[sub]
                if dist > 0:
                    distances.append(dist)
            seen[sub] = i
    if not distances:
        return [(1, 1)]
    factor_counts: dict[int, int] = Counter()
    for d in distances:
        for f in range(2, d + 1):
            if d % f == 0:
                factor_counts[f] += 1
    candidates = sorted(factor_counts.items(), key=lambda x: -x[1])
    return candidates[:5]

def break_vigenere(ciphertext: str) -> tuple[str, str]:
    key_lengths = kasiski_examination(ciphertext)
    best_overall = (ciphertext, "")
    best_overall_score = float('inf')
    for kl, _ in key_lengths[:5]:
        if kl < 1 or kl > 20:
            continue
        cleaned = ''.join(c for c in ciphertext.upper() if c.isalpha())
        columns = ['' for _ in range(kl)]
        for i, ch in enumerate(cleaned):
            columns[i % kl] += ch
        keyword = []
        for col in columns:
            _, shift = break_caesar(col)
            keyword.append(chr(shift + ord('A')))
        key = ''.join(keyword)
        plaintext = vigenere_decrypt(ciphertext, key)
        score = _frequency_score(plaintext)
        if score < best_overall_score:
            best_overall_score = score
            best_overall = (plaintext, key)
    return best_overall
```

The key length leaks through repeated substrings. Once you know the key length, each column is just a Caesar cipher.

### Step 7: One-Time Pad — Perfect Secrecy

XOR the plaintext with a random key. Every bit of the key is independently random:

```python
import os

def one_time_pad_encrypt(plaintext: bytes, key: bytes) -> bytes:
    assert len(key) >= len(plaintext), "Key must be at least as long as plaintext"
    return bytes(p ^ k for p, k in zip(plaintext, key))

def one_time_pad_decrypt(ciphertext: bytes, key: bytes) -> bytes:
    return one_time_pad_encrypt(ciphertext, key)
```

```python
>>> key = os.urandom(32)
>>> ct = one_time_pad_encrypt(b"ATTACK AT DAWN SUBMARINE", key)
>>> one_time_pad_decrypt(ct, key)
b'ATTACK AT DAWN SUBMARINE'
```

Try to brute-force it: for a 25-byte ciphertext, there are 2²⁵⁰ possible keys — each produces a different "plaintext," including every possible 25-byte message. There is no way to distinguish the real one. That is Shannon's proof: the ciphertext gives you zero information about the plaintext.

### Step 8: Run the Full Demo

The complete `code/main.py` encrypts sample text with each cipher, then breaks it, then demonstrates that the one-time pad is unbreakable. Run it with `python3 main.py`.

## Use It

In production, you never use classical ciphers. Python's `cryptography` library wraps OpenSSL, which implements AES-GCM — a cipher with both confusion (8 substitution boxes per round) and diffusion (ShiftRows and MixColumns spread each input byte across all output bytes). Compare:

| Property | Caesar | Substitution | Vigenère | OTP | AES-GCM |
|----------|--------|-------------|----------|-----|----------|
| Key space | 25 | 26! (but leaks) | 26^L (leaks period) | ∞ | 2¹²⁸ |
| Statistical leak | Full | Full | Per-column | None | None |
| Diffusion | None | None | None (local to period) | XOR | Full (128-bit block) |
| Confusion | None | None (simple map) | None (simple shift) | XOR (linear) | Yes (S-boxes) |

The `cryptography` library:

```python
from cryptography.hazmat.primitives.ciphers.aead import AESGCM
key = AESGCM.generate_key(bit_length=256)
aes = AESGCM(key)
nonce = os.urandom(12)
ct = aes.encrypt(nonce, b"secret message", None)
pt = aes.decrypt(nonce, ct, None)
```

AES-GCM gives you what the one-time pad promises (secrecy) without requiring key pre-distribution of equal length — the 256-bit key expands into a keystream via a PRNG, which is a stream cipher (covered in lesson 03).

## Read the Source

- **PyCryptodome** `Cipher/PKCS1_OAEP.py` — shows how a production library wraps a cipher with padding and OAEP, adding confusion beyond the raw algorithm.
- **OpenSSL** `crypto/evp/e_aes.c` — the C implementation of AES in the most widely deployed TLS library. Notice the rounds: SubBytes (confusion) → ShiftRows → MixColumns (diffusion) → AddRoundKey, repeated 14 times for AES-256.

## Ship It

The reusable artifact for this lesson is in `outputs/`:

- **A classical cipher toolkit** — `code/main.py` can be imported as a module (`caesar_encrypt`, `break_substitution`, `break_vigenere`, `kasiski_examination`, etc.) for use in later crypto lessons and the phase capstone CTF challenges.

## Exercises

1. **Easy** — Encrypt "HELLO WORLD" with Caesar shift 17, then use `break_caesar` to recover the plaintext. Verify it works.
2. **Medium** — Modify `break_substitution` to use trigram frequencies (THE, AND, ING) in addition to bigrams. Measure how many fewer iterations it needs on a 500-character English text.
3. **Hard** — Implement a known-plaintext attack on the one-time pad *when the key is reused*: given two ciphertexts encrypted with the same key, recover both plaintexts without knowing the key. (This is what the Venona project did.)

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Frequency analysis | "Count the letters" | Statistical attack exploiting the non-uniform distribution of letters/bigrams in natural language — works on any cipher where plaintext statistics survive encryption |
| Kasiski examination | "Find repeating patterns" | Method to determine Vigenère key length by measuring distances between repeated substrings; their GCD reveals the period |
| Perfect secrecy | "Can't be broken" | Shannon's 1949 proof that OTP is unbreakable: every plaintext is equally likely given any ciphertext, meaning ciphertext leaks zero information about plaintext |
| Confusion | "Make it complicated" | The relationship between key and ciphertext must be complex — knowing a ciphertext bit should reveal almost nothing about which key bit produced it |
| Diffusion | "Spread things around" | Changing one plaintext bit should change ~50% of ciphertext bits; prevents statistical attacks that exploit local patterns |
| Key space | "Number of possible keys" | The set of all valid keys; large key space is necessary but not sufficient (substitution cipher has 26! keys and still breaks) |

## Further Reading

- [Shannon, "Communication Theory of Secrecy Systems" (1949)](https://www.cs.miami.edu/home/burt/learning/Csc609.062/doc/shannon1949.pdf) — the original proof of perfect secrecy for the one-time pad.
- [Singh, *The Code Book* (1999)](https://www.simonsingh.net/The_Code_Book.html) — accessible history of cryptography from Caesar to Enigma.
- [Kasiski, *Die Geheimschriften und die Dechiffrir-Kunst* (1863)](https://en.wikipedia.org/wiki/Kasiski_examination) — the original paper breaking the Vigenère cipher.
- [Copeland, *Colossus: The Secrets of Bletchley Park's Code-Breaking Computers* (2006)](https://www.google.com/books/edition/Colossus/cWmMDwAAQBAJ) — detailed account of the Enigma breaks and how operator errors and protocol flaws doomed the machine.