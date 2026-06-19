# Phase Capstone — A TLS 1.3 Library + a Mini-CTF

> Phase Capstone — A TLS 1.3 Library + a Mini-CTF — the part of CS you can't skip.

**Type:** Build
**Languages:** Rust, Python
**Phase:** 12 — Cryptography & Security
**Prerequisites:** Phase 12 lessons 01–23
**Time:** ~150 minutes

## Learning Objectives

- Integrate cryptographic primitives (symmetric ciphers, AEAD, key exchange, signatures, KDFs) into a working TLS 1.3 protocol implementation.
- Construct a modular TLS 1.3 library in Rust with record layer, handshake state machine, and key schedule.
- Implement and exploit five classic cryptography/security vulnerabilities in a mini-CTF framework: ECB byte-at-a-time, nonce reuse, weak RSA, timing side-channels, and padding oracle attacks.
- Compare your hand-built implementation against a production TLS library (rustls) to understand what production adds: session resumption, 0-RTT, certificate validation, key update, and middlebox compatibility.
- Design a grading system that validates challenge completion, simulating real CTF competition infrastructure.

## The Problem

You've spent the entire phase learning cryptographic primitives, protocols, attacks, and defenses. Now you must combine everything into a real TLS 1.3 library and prove you can both build and break it.

This capstone integrates every prior Phase 12 lesson:

| Lesson | Topic | Where It Fits |
|--------|-------|---------------|
| 03–05 | Symmetric ciphers (AES) | Cipher suite: AES-128-GCM |
| 06 | Hash functions (SHA-256) | Transcript hash, HKDF |
| 07 | MACs (HMAC) | HKDF building block |
| 08 | AEAD (GCM) | Record encryption |
| 09 | Key exchange (DH) | X25519 ECDHE |
| 10 | RSA | Weak RSA challenge (CTF) |
| 11 | ECC | X25519, Ed25519 |
| 12 | Digital signatures | CertificateVerify with Ed25519 |
| 13 | KDFs (HKDF) | TLS 1.3 key schedule |
| 14 | TLS handshake | Protocol flow |
| 15 | TLS client | Connection state machine |
| 16 | PKI | Certificate handling (simplified) |
| 19 | Side-channels | Timing oracle challenge (CTF) |
| 20 | Memory safety | Rust's ownership model |
| 21 | Web security | Real-world TLS context |

Without this capstone, all those lessons remain isolated facts. The capstone forces you to wire them together: the key exchange feeds the KDF, which feeds the cipher, which protects records whose integrity is verified by the transcript hash, which chains the handshake messages signed by the server's private key.

## The Concept — Capstone Architecture

The capstone has **two deliverables**:

### Part 1: TLS 1.3 Library (Rust)

A modular Rust library implementing the core TLS 1.3 protocol. The library reuses and integrates components from lessons 09–15:

```
tls13_lib/
├── record.rs       — TLS record layer framing
├── handshake.rs    — Handshake message types
├── key_schedule.rs — HKDF-based key derivation
├── cipher.rs       — AES-128-GCM encrypt/decrypt
├── transcript.rs   — Rolling SHA-256 transcript hash
└── lib.rs          — Public API: TlsConnection
```

**Scope:**
- X25519 key exchange (ECDHE)
- AES-128-GCM AEAD encryption
- HKDF-SHA256 key derivation (early secret → handshake secret → master secret → traffic keys)
- Ed25519 digital signatures (CertificateVerify)
- Full handshake: ClientHello ↔ ServerHello ↔ Certificate ↔ CertificateVerify ↔ Finished
- Encrypted application data records
- close_notify alert

**Non-goals (simplified for the capstone):**
- Certificate chain validation (accepts self-signed)
- 0-RTT data
- Key update
- Session tickets / PSK resumption
- Multiple cipher suites (hard-coded to TLS_AES_128_GCM_SHA256)

### Part 2: Mini-CTF (Python)

A capture-the-flag framework with 5 cryptography/security challenges. Each challenge presents a vulnerable implementation; participants must exploit the vulnerability to recover a flag.

| # | Challenge | Vulnerability | Phase 12 Lesson |
|---|-----------|---------------|-----------------|
| 1 | Broken AES-ECB | Byte-at-a-time ECB decryption | 03 (AES modes) |
| 2 | Nonce Reuse | AES-CTR two-time pad | 04 (CTR mode) |
| 3 | Weak RSA | Small exponent (e=3) cube root attack | 10 (RSA attacks) |
| 4 | Timing Oracle | Variable-time comparison leaks password | 19 (side-channels) |
| 5 | Padding Oracle | CBC padding oracle decryption | 03 (CBC mode), 19 |

Each challenge includes: a vulnerable oracle/challenge setup, an auto-solver demonstrating the exploit, and a flag validation system.

## Build It — Part 1: TLS 1.3 Library (Rust)

### Step 1: Library Architecture

Define the module structure. Each module encapsulates one TLS 1.3 sub-protocol:

```text
Cargo.toml           — Dependencies: x25519-dalek, sha2, hmac, hkdf, aes-gcm, ed25519-dalek, rand, hex
main.rs              — Library + binary with echo demo
```

**Module interfaces:**

- **record**: `encode_record(content_type, payload) → Vec<u8>` and `decode_record(data) → (content_type, payload)`
- **key_schedule**: `derive_early_secret`, `derive_handshake_secret`, `derive_master_secret`, `derive_traffic_keys`
- **cipher**: `encrypt(key, iv, seq, content_type, plaintext) → ciphertext` and `decrypt(...) → plaintext`
- **transcript**: `update(data)` and `current_hash() → [u8; 32]`
- **handshake**: `build_client_hello(key_share)`, `parse_server_hello(data)`, `build_finished(verify_data)`, `build_certificate_verify(signing_key, transcript_hash)`, `parse_certificate_verify(verify_key, data, transcript_hash)`

### Step 2: Key Schedule (HKDF-SHA256)

TLS 1.3 derives all keys from the ECDHE shared secret using a chain of HKDF-Extract and HKDF-Expand-Label:

```text
0                 → PSK (pre-shared key = 0 for non-PSK)
                   ↓ HKDF-Extract(salt=0, ikm=PSK)
early_secret      → derive_secret("derived", "")
                   ↓ HKDF-Extract(salt=derived, ikm=shared_secret)
handshake_secret  → derive_secret("c hs traffic", hello_hash)   → client_handshake_traffic_secret
                  → derive_secret("s hs traffic", hello_hash)   → server_handshake_traffic_secret
                   ↓ HKDF-Extract(salt=derived("derived"), ikm="")
master_secret     → derive_secret("c ap traffic", handshake_hash) → client_application_traffic_secret
                  → derive_secret("s ap traffic", handshake_hash) → server_application_traffic_secret
```

Each `derive_secret(PRK, label, context)` calls HKDF-Expand-Label which is HKDF-Expand with the TLS 1.3 HkdfLabel struct:
```
HkdfLabel {
    uint16 length;              // Output length in bytes
    opaque label<7..255>;       // "tls13 " + label
    opaque context<0..255>;     // Typically a transcript hash
}
```

The traffic keys (key + IV) come from expanding each traffic secret:
```
key = HKDF-Expand-Label(traffic_secret, "key", "", 16)
iv  = HKDF-Expand-Label(traffic_secret, "iv", "", 12)
```

### Step 3: Record Layer + AEAD Encryption

Each TLS record is framed as:
```
struct {
    ContentType type;       // 1 byte: 22=handshake, 23=application_data, 21=alert
    ProtocolVersion legacy; // 2 bytes: 0x0303 (TLS 1.2 legacy)
    uint16 length;          // 2 bytes: encrypted payload length
    opaque encrypted;       // Encrypted content + auth tag (16 bytes GCM tag)
} TLSPlaintext;
```

For AES-128-GCM, the nonce is constructed from the sequence number and the derived IV:
```
nonce = IV XOR (pad(sequence_number, 12))
```

Where `pad(seq, 12)` is the 8-byte sequence number prepended with 4 zero bytes.

The AEAD additional authenticated data (AAD) is:
```
aad = content_type || legacy_protocol_version || length
```

### Step 4: Handshake State Machine

The simplified TLS 1.3 handshake flow:

```text
Client                                    Server
------                                    ------
ClientHello (X25519 public key)  ──────→
                                      Compute shared secret
                                      Derive handshake keys
                                      Build transcript hash
                              ←────── ServerHello (X25519 public key)
                              ←────── EncryptedExtensions
                              ←────── Certificate (Ed25519 public key)
                              ←────── CertificateVerify (signature)
                              ←────── Finished (verify_data)
Compute shared secret
Derive handshake keys
Verify CertificateVerify
Verify Finished
Build transcript hash
Finished (verify_data)         ──────→
Derive application keys                 Verify Finished
                              ←────── [Application Data]
[Application Data]            ──────→   Derive application keys
close_notify                   ──────→
```

### Step 5: TlsConnection Public API

The `TlsConnection` struct wraps the full state machine:

```rust
pub struct TlsConnection {
    state: TlsState,                    // Handshake | Connected | Closed
    is_server: bool,
    // Key schedule state
    client_hs_key: [u8; 16],
    client_hs_iv: [u8; 12],
    server_hs_key: [u8; 16],
    server_hs_iv: [u8; 12],
    client_ap_key: [u8; 16],
    client_ap_iv: [u8; 12],
    server_ap_key: [u8; 16],
    server_ap_iv: [u8; 12],
    // Secrets
    shared_secret: [u8; 32],
    // Sequence numbers
    client_seq: u64,
    server_seq: u64,
    // Transcript
    transcript: Transcript,
    // Peer's public key
    peer_public: Option<[u8; 32]>,
}
```

Public methods:
- `connect()` — perform full handshake as client
- `accept()` — perform full handshake as server
- `send_data(data)` — encrypt and queue application data
- `receive_data()` — decrypt and return application data
- `close()` — send and verify close_notify

### Step 6: Echo Server Demo

The binary wraps the library in a TCP echo server:

```text
$ cargo run
[TLS 1.3 Library Demo]
Mode: In-process handshake simulation

Step 1: Client generates X25519 keypair
Step 2: Server generates X25519 keypair
Step 3: ClientHello → ServerHello exchange
Step 4: Shared secret computed (both sides)
Step 5: Handshake secrets derived
Step 6: Server signs transcript (Ed25519)
Step 7: Client verifies signature
Step 8: Both derive application keys
Step 9: Encrypted application data exchange
Step 10: close_notify

✓ TLS 1.3 handshake complete
✓ Encrypted data verified
```

## Build It — Part 2: Mini-CTF (Python)

### Challenge 1: AES-ECB Byte-at-a-Time

**Setup:** A server encrypts `user_input + secret_flag` with AES-ECB using a fixed unknown key. The participant controls the input.

**Attack:** The participant feeds incrementally longer inputs to find the block size (16), confirms ECB mode (identical plaintext blocks → identical ciphertext blocks), then recovers the flag one byte at a time by crafting inputs that push one unknown byte into a known position.

**Algorithm:**
1. Determine block size by measuring ciphertext length growth
2. Detect ECB by encrypting 32+ identical bytes
3. For byte position i, craft prefix of length `(blocksize - 1) - (i % blocksize)` to isolate the ith unknown byte as the last byte of a block
4. Brute-force all 256 possible values for that byte (only 1–2 needed in practice with the oracle)
5. Repeat until the full flag is recovered

### Challenge 2: AES-CTR Nonce Reuse

**Setup:** Two messages are encrypted with AES-CTR using the **same key and same nonce**. The first message is known; the second contains the flag.

**Attack:** CTR mode generates a keystream `KS = AES_CTR(key, nonce)` and computes `ct = pt XOR KS`. With the same nonce, both ciphertexts share the same keystream: `ct1 XOR ct2 = pt1 XOR pt2`. Given known `pt1`, recover `pt2 = ct1 XOR ct2 XOR pt1`.

```python
def recover(ct1, ct2, pt1):
    return bytes(a ^ b ^ c for a, b, c in zip(ct1, ct2, pt1))
```

### Challenge 3: Weak RSA (Small Exponent)

**Setup:** An RSA public key with `e = 3` and a 2048-bit modulus encrypts a short flag without padding.

**Attack:** If `m^e < n` (true when the message is short, e.g., a flag < 256 bits), the ciphertext is exactly `m^3` with no modular reduction. Recover `m` by computing the integer cube root.

```python
def integer_cube_root(n):
    lo, hi = 0, 1 << (n.bit_length() // 3 + 1)
    while lo < hi:
        mid = (lo + hi) // 2
        if mid**3 < n:
            lo = mid + 1
        else:
            hi = mid
    return lo

m = integer_cube_root(ct)
flag = long_to_bytes(m).decode()
```

### Challenge 4: Timing Oracle

**Setup:** A password checker compares the user's guess against the secret flag byte-by-byte with a `time.sleep(0.05)` after each correct character. Incorrect characters cause an early return.

**Attack:** The total comparison time leaks the position of the first wrong character. Recover the password byte-by-byte by measuring response times for each possible character choice.

```python
def recover_password(oracle, length):
    known = ""
    for pos in range(length):
        best_char = None
        best_time = 0
        for c in string.printable:
            guess = known + c + "A" * (length - pos - 1)
            start = time.time()
            oracle(guess)
            elapsed = time.time() - start
            if elapsed > best_time:
                best_time = elapsed
                best_char = c
        known += best_char
    return known
```

### Challenge 5: Padding Oracle

**Setup:** An AES-CBC decryption oracle reveals whether the padding of a decrypted ciphertext is valid (PKCS7). The participant has the IV and ciphertext of an encrypted flag.

**Attack:** The padding oracle attack decrypts each byte of the ciphertext by manipulating the previous ciphertext block:

1. For the last byte of a block, craft a fake previous block whose last byte forces the decrypted plaintext's last byte to 0x01 (valid padding)
2. Brute-force the last byte of the crafted block until the oracle returns valid
3. From this, compute `intermediate_byte = crafted_byte XOR 0x01`, then `plaintext_byte = intermediate_byte XOR original_iv_byte`
4. Repeat for each position, padding with the appropriate value (0x02 for the next-to-last byte, etc.)

```python
def padding_oracle_decrypt(oracle, iv, ct, block_size=16):
    blocks = [iv] + [ct[i:i+block_size] for i in range(0, len(ct), block_size)]
    result = b""
    for idx in range(1, len(blocks)):
        intermediate = [0] * block_size
        for byte_pos in range(block_size - 1, -1, -1):
            pad_val = block_size - byte_pos
            for guess in range(256):
                crafted = [0] * block_size
                for j in range(block_size - 1, byte_pos, -1):
                    crafted[j] = intermediate[j] ^ pad_val
                crafted[byte_pos] = guess
                test = bytes(crafted) + blocks[idx]
                if oracle(test):
                    intermediate[byte_pos] = guess ^ pad_val
                    break
        for i in range(block_size):
            result += bytes([intermediate[i] ^ blocks[idx - 1][i]])
    return unpad(result, block_size)
```

## Use It

Compare your TLS 1.3 library against **rustls** — the production Rust TLS library used by Firefox, curl, and many Rust projects:

| Feature | Your Library | rustls |
|---------|-------------|--------|
| Cipher suites | 1 (TLS_AES_128_GCM_SHA256) | 10+ |
| Key exchange | X25519 only | X25519, P-256, P-384 |
| Certificate validation | Self-signed only | Full chain validation + CRL/OCSP |
| Session resumption | None | PSK + ticket-based |
| 0-RTT | None | Supported |
| Key update | None | Periodic key rotation |
| Middlebox compatibility | None | DTLS, downgrade prevention |
| Performance | Single-thread, no optimizations | AES-NI, multi-threaded |

Your library captures the **core protocol logic** — the handshake state machine, key schedule, and record encryption are all correct. rustls adds the **production hardening**: certificate validation, multiple cipher suites, session resumption, and defense-in-depth.

The mini-CTF is modeled on real CTF competitions. Similar challenges appear in:

- **PicoCTF** (CMU): Beginner-friendly crypto challenges with ECB byte-at-a-time and padding oracle attacks
- **DEF CON Quals**: Advanced crypto challenges including nonce reuse and side-channel exploitation
- **CryptoHack**: Interactive platform with the exact same challenge types

Real CTFs require the same skills you just demonstrated: understanding the primitive deeply enough to find and exploit the gap between the specification and the implementation.

## Read the Source

- **[RFC 8446 — The Transport Layer Security (TLS) Protocol Version 1.3](https://datatracker.ietf.org/doc/html/rfc8446):** Sections 4 (handshake), 5 (record protocol), 7 (key schedule). This is the definitive reference. Read the key schedule diagram in Section 7.1 and the HkdfLabel struct in Section 7.1.
- **[rustls source](https://github.com/rustls/rustls):** `rustls/src/tls13/` — the production implementation of the TLS 1.3 handshake, key schedule, and record encryption. Compare your `key_schedule.rs` with `rustls/src/tls13/key_schedule.rs`.
- **[CryptoHack — ECB Byte-at-a-Time](https://cryptohack.org/challenges/cryptopals/):** CryptoHack's implementation of the byte-at-a-time ECB attack, based on the Cryptopals challenges (Challenge 12).
- **[Padding Oracle Attack — Classic Paper](https://www.usenix.org/legacy/event/woot10/tech/full_papers/Rizzo.pdf):** "Practical Padding Oracle Attacks" by Rizzo and Duong — the paper that showed how padding oracles break real-world CBC implementations.
- **[PicoCTF Crypto Challenges](https://picoctf.org):** CTF problems spanning ECB, padding oracle, weak RSA, and side-channel attacks.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A TLS 1.3 library** (`outputs/tls13-lib/`) — copy of the Rust implementation with Cargo.toml, ready to build with `cargo build`.
- **A Mini-CTF framework** (`outputs/mini-ctf/`) — copy of the Python implementation with all 5 challenges and auto-grader, ready to run with `python3 main.py`.

Both artifacts are standalone and can be extended, forked, or reused in future projects.

## Exercises

1. **Easy** — Extend the TLS 1.3 library with Pre-Shared Key (PSK) resumption. Implement `derive_early_secret` with a non-empty PSK, store session state after the first handshake, and allow a second connection to use PSK-mode (skipping the ECDHE exchange). Test by running two sequential handshakes and verifying the second uses fewer round-trips.

2. **Medium** — Add a 6th challenge to the mini-CTF: "Hash Length Extension" attack. Given `H(secret || message)` for a SHA-256-based MAC, compute `H(secret || message || padding || extension)` without knowing the secret (SHA-256 is vulnerable to length extension). Write the challenge setup, solver, and flag file.

3. **Hard** — Break your own TLS 1.3 implementation. Intentionally introduce a vulnerability (e.g., fixed nonce, disabled Finished verification, reused sequence numbers) and write a CTF challenge around it. Your challenge must include: the vulnerable Rust code, a Python exploit script, and documentation describing the vulnerability and fix. This exercise teaches the full cycle: build → break → fix.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Capstone | The final project that integrates everything | A build task requiring synthesis of all prior lessons in a phase, demonstrating holistic understanding |
| TLS record | A framed message in the TLS protocol | The wire format: content type (1B) + legacy version (2B) + length (2B) + encrypted payload, the unit of data exchange |
| Handshake | The initial negotiation between TLS client and server | A multi-message protocol exchange establishing cipher suite, keys, and peer identity before application data flows |
| Key schedule | The algorithm that derives traffic keys | A chain of HKDF-Extract and HKDF-Expand-Label operations producing separate traffic secrets and keys for each direction |
| ECB byte-at-a-time | An attack on deterministic ECB encryption | Recovering plaintext by crafting inputs that align unknown bytes into predictable block positions and comparing ciphertexts |
| Nonce reuse | Using the same nonce for multiple encryptions with the same key | In CTR/GCM modes, nonce reuse causes keystream collision, allowing an attacker to XOR ciphertexts and cancel the keystream |
| Padding oracle | A server that reveals valid vs. invalid padding | When a server's response differs based on padding validity, an attacker can iteratively decrypt any ciphertext by manipulating CBC blocks |
| Timing oracle | A side-channel that leaks secrets through response timing | Variable-time operations (e.g., string comparison, branching on secret data) leak information through measurable execution time differences |
| Weak RSA | RSA with a small exponent or modulus | Small public exponent (e=3) with unpadded messages allows plaintext recovery via integer root when m^e < n |
| Integration | Combining separate components into a working system | Wiring key exchange → KDF → cipher → record layer → handshake state machine so they function correctly as a protocol implementation |

## Further Reading

1. **RFC 8446 — TLS 1.3 Specification.** The authoritative protocol specification. Essential reading for the key schedule (Section 7.1), handshake messages (Section 4), and record layer (Section 5).
2. **"Implementing TLS 1.3" — A. Langley (ImperialViolet blog).** Series of blog posts by the author of BoringSSL, walking through the design decisions in TLS 1.3.
3. **"Serious Cryptography" — J-P. Aumasson.** Chapters on block cipher modes, AEAD, and key exchange provide the mathematical foundation the capstone builds on.
4. **"The Padding Oracle Attack — Why CBC is Dangerous" — B. Preneel.** Foundational paper explaining why CBC mode with padding oracles is broken, and why AEAD modes like GCM replaced it.
5. **CryptoPals / Cryptopals Crypto Challenges (Set 1–2).** The original challenges that inspired the mini-CTF format. Challenges 7 (AES-ECB), 8 (detect ECB), 12 (byte-at-a-time), and 13 (ECB cut-and-paste) directly parallel the CTF challenges in this capstone.
