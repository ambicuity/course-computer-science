"""
What Cryptography Actually Promises
Phase 12 — Cryptography & Security

Demonstrates five primitive categories and what each promises (and doesn't).
Run: python3 main.py
Requires: cryptography (pip install cryptography)
"""

import hashlib
import hmac
import os
import struct


def _separate(title: str) -> None:
    print(f"\n{'=' * 60}")
    print(f"  {title}")
    print('=' * 60)


def demo_symmetric() -> None:
    _separate("SYMMETRIC ENCRYPTION (AES-256-GCM)")
    print()
    print("PROMISES: Confidentiality, Integrity, Authenticity")
    print("DOES NOT: Key management, side-channel resistance, availability")
    print()

    try:
        from cryptography.hazmat.primitives.ciphers.aead import AESGCM
    except ImportError:
        print("Falling back to simple XOR demo (install 'cryptography' for AES-GCM)")
        _demo_symmetric_xor()
        return

    key = AESGCM.generate_key(bit_length=256)
    aesgcm = AESGCM(key)
    nonce = os.urandom(12)
    plaintext = b"The attack begins at dawn."
    associated_data = b"header-visible-but-authenticated"

    ciphertext = aesgcm.encrypt(nonce, plaintext, associated_data)
    print(f"Plaintext:  {plaintext}")
    print(f"Key:        {key.hex()[:32]}... ({len(key) * 8}-bit)")
    print(f"Nonce:      {nonce.hex()}")
    print(f"Ciphertext: {ciphertext.hex()}")
    print()

    decrypted = aesgcm.decrypt(nonce, ciphertext, associated_data)
    print(f"Decrypted:  {decrypted}")
    print()

    print("--- What happens if we tamper with the ciphertext? ---")
    tampered = bytearray(ciphertext)
    tampered[-1] ^= 0xFF
    try:
        aesgcm.decrypt(nonce, bytes(tampered), associated_data)
        print("UNEXPECTED: decryption succeeded on tampered data!")
    except Exception:
        print("Tampered ciphertext REJECTED — authentication tag invalid.")
        print("This is integrity + authenticity working correctly.")
    print()

    print("--- What happens if we tamper with associated data? ---")
    try:
        aesgcm.decrypt(nonce, ciphertext, b"tampered-header")
        print("UNEXPECTED: decryption succeeded with wrong header!")
    except Exception:
        print("Tampered associated data REJECTED — authentication tag invalid.")
    print()

    print("What this DOES NOT promise:")
    print("  - If the key leaks, confidentiality is gone.")
    print("  - If nonce is reused with same key, all confidentiality is lost.")
    print("  - If side channels leak timing, key recovery may be possible.")


def _demo_symmetric_xor() -> None:
    plaintext = b"The attack begins at dawn."
    key = os.urandom(len(plaintext))

    ciphertext = bytes(p ^ k for p, k in zip(plaintext, key))
    decrypted = bytes(c ^ k for c, k in zip(ciphertext, key))

    print(f"Plaintext:  {plaintext}")
    print(f"Key (hex):  {key.hex()[:32]}...")
    print(f"XOR ciphertext: {ciphertext.hex()}")
    print(f"Decrypted:  {decrypted}")
    print()

    print("--- XOR has no integrity or authenticity ---")
    tampered = bytearray(ciphertext)
    tampered[0] ^= 0x01
    broken = bytes(t ^ k for t, k in zip(tampered, key))
    print(f"Tampered decryption (no error!): {broken}")
    print("XOR encryption silently produces garbled plaintext — no detection.")
    print()

    print("What AES-GCM adds that XOR doesn't:")
    print("  - Authentication tag: any tampering is detected")
    print("  - Associated data: authenticated but not encrypted")
    print("  - Standardized, analyzed, constant-time implementations")


def demo_asymmetric() -> None:
    _separate("ASYMMETRIC ENCRYPTION (Toy RSA)")
    print()
    print("PROMISES: Confidentiality (encrypt with public key),")
    print("          Authenticity + Non-repudiation (sign with private key)")
    print("DOES NOT: Efficiency, side-channel safety, quantum resistance")
    print()

    print("Generating small RSA-style key pair (61-bit primes for demo)...")
    p, q = 61, 53
    n = p * q
    phi = (p - 1) * (q - 1)
    e = 17
    d = pow(e, -1, phi)

    print(f"p = {p}, q = {q}, n = {n}, phi(n) = {phi}")
    print(f"Public key:  (n={n}, e={e})")
    print(f"Private key: (n={n}, d={d})")
    print()

    message = 42
    print(f"Original message (as number): {message}")

    ciphertext = pow(message, e, n)
    print(f"Encrypted with public key: pow({message}, {e}, {n}) = {ciphertext}")

    decrypted = pow(ciphertext, d, n)
    print(f"Decrypted with private key: pow({ciphertext}, {d}, {n}) = {decrypted}")
    print()

    print("--- Signing: encrypt with PRIVATE key (anyone verifies with public key) ---")
    signature = pow(message, d, n)
    print(f"Signature (message signed with private key d): {signature}")
    verified = pow(signature, e, n)
    print(f"Verification (pow({signature}, {e}, {n})): {verified}")
    print(f"Signature valid: {verified == message}")
    print()

    print("--- What this toy demo skips ---")
    print("  - Real RSA uses 2048+ bit primes (n > 2^2048)")
    print("  - Real RSA uses OAEP padding (PKCS#1 v1.5 is broken)")
    print("  - Real RSA signing uses PSS padding")
    print("  - Raw RSA (textbook RSA) is vulnerable to many attacks")
    print("  - Key generation must use provably random primes")
    print()
    print("What asymmetric encryption DOES NOT promise:")
    print("  - Efficiency: RSA is 100-1000x slower than AES")
    print("  - Post-quantum security: Shor's algorithm breaks RSA")
    print("  - Implementation safety: constant-time exponentiation is hard")


def demo_hash() -> None:
    _separate("HASH FUNCTIONS (SHA-256)")
    print()
    print("PROMISES: Integrity, preimage resistance, collision resistance")
    print("DOES NOT: Authenticity (anyone can compute the hash),")
    print("          confidentiality (hashes don't hide data)")
    print()

    msg_a = b"The quick brown fox jumps over the lazy dog"
    msg_b = b"The quick brown fox jumps over the lazy dog."
    msg_c = b"The quick brown fox jumps over the lazy dof"

    hash_a = hashlib.sha256(msg_a).hexdigest()
    hash_b = hashlib.sha256(msg_b).hexdigest()
    hash_c = hashlib.sha256(msg_c).hexdigest()

    print(f"Message A: {msg_a}")
    print(f"SHA-256:   {hash_a}")
    print()
    print(f"Message B: {msg_b}  (added a period)")
    print(f"SHA-256:   {hash_b}")
    print()
    print(f"Message C: {msg_c}  (changed 'dog' to 'dof')")
    print(f"SHA-256:   {hash_c}")
    print()

    print("--- Avalanche effect ---")
    diff_ab = sum(c1 != c2 for c1, c2 in zip(hash_a, hash_b))
    diff_ac = sum(c1 != c2 for c1, c2 in zip(hash_a, hash_c))
    print(f"Hamming distance A→B (period): {diff_ab}/64 hex chars differ")
    print(f"Hamming distance A→C (1 letter): {diff_ac}/64 hex chars differ")
    print("1-bit change in input → ~50% of output bits change (avalanche)")
    print()

    print("--- Preimage resistance ---")
    target = hashlib.sha256(b"password").hexdigest()
    print(f"SHA-256('password') = {target}")
    print("Given this hash, finding the input is computationally infeasible.")
    print("(But dictionary attacks on common passwords are easy — that's")
    print(" a key management problem, not a hash problem.)")
    print()

    print("What hashes DO NOT promise:")
    print("  - Anyone can compute SHA-256, so a hash alone proves nothing")
    print("    about who sent the data (no authenticity)")
    print("  - SHA-1 and MD5 have broken collision resistance")
    print("  - A hash is a fixed-size fingerprint — different inputs can")
    print("    theoretically produce the same hash (collision), though")
    print("    finding one is computationally infeasible for SHA-256")


def demo_mac() -> None:
    _separate("MESSAGE AUTHENTICATION CODE (HMAC-SHA256)")
    print()
    print("PROMISES: Integrity + Authenticity (verified by shared key)")
    print("DOES NOT: Non-repudiation (either party can produce the tag),")
    print("          Confidentiality (messages are in plaintext)")
    print()

    shared_key = b"super-secret-key-shared-by-both-parties"
    message = b"Transfer $5000 to account 789"

    tag = hmac.new(shared_key, message, hashlib.sha256).hexdigest()
    print(f"Key:      {shared_key.decode()}")
    print(f"Message:  {message.decode()}")
    print(f"HMAC tag: {tag}")
    print()

    print("--- Verifier checks the tag ---")
    recomputed = hmac.new(shared_key, message, hashlib.sha256).hexdigest()
    print(f"Recomputed tag matches: {hmac.compare_digest(tag, recomputed)}")
    print()

    print("--- Attacker tampers with the message ---")
    tampered_msg = b"Transfer $50000 to account 789"
    tampered_tag = hmac.new(shared_key, tampered_msg, hashlib.sha256).hexdigest()
    print(f"Tampered message: {tampered_msg.decode()}")
    print(f"HMAC tag for tampered message: {tampered_tag}")
    print(f"Original tag still valid? {hmac.compare_digest(tag, tampered_tag)}")
    print()

    print("--- Attacker forges a tag without the key ---")
    forged_tag = hashlib.sha256(tampered_msg).hexdigest()
    print(f"SHA-256 hash (no key) of tampered message: {forged_tag}")
    print(f"This is NOT a valid HMAC — verifier rejects it.")
    print()

    print("What HMAC DOES NOT promise:")
    print("  - Non-repudiation: both sender and receiver know the key,")
    print("    so either could have produced the tag. In a dispute, you")
    print("    can't prove WHO sent it. (That's what signatures are for.)")
    print("  - Confidentiality: the message is sent in plaintext.")
    print("    (Pair HMAC with encryption for confidentiality.)")


def demo_kdf() -> None:
    _separate("KEY DERIVATION FUNCTION (PBKDF2-HMAC-SHA256)")
    print()
    print("PROMISES: Slow, salted derivation from low-entropy password →")
    print("          high-entropy key; resistance to dictionary attacks")
    print("DOES NOT: Make weak passwords strong, protect against")
    print("          GPU/ASIC brute force (use Argon2 for that)")
    print()

    password = b"correct-horse-battery-staple"
    weak_password = b"password123"
    salt = os.urandom(16)

    print("--- Strong password derivation ---")
    key = hashlib.pbkdf2_hmac('sha256', password, salt, iterations=600000)
    print(f"Password: {password.decode()}")
    print(f"Salt:     {salt.hex()}")
    print(f"Derived key (600k iterations): {key.hex()}")
    print()

    print("--- Same password, different salt → different key ---")
    salt2 = os.urandom(16)
    key2 = hashlib.pbkdf2_hmac('sha256', password, salt2, iterations=600000)
    print(f"Same password, different salt: {key2.hex()}")
    print(f"Keys are different: {key != key2}")
    print("Salt prevents rainbow table attacks on password hashes.")
    print()

    print("--- Weak password: KDF doesn't fix the underlying weakness ---")
    weak_key = hashlib.pbkdf2_hmac('sha256', weak_password, salt, iterations=600000)
    print(f"Password: {weak_password.decode()}")
    print(f"Derived key: {weak_key.hex()}")
    print("The key looks random, but 'password123' is in every dictionary.")
    print("An attacker tries common passwords with the known salt and iterations.")
    print("600k iterations slow them down, but don't stop a dedicated attacker.")
    print()

    print("--- What the iteration count gives you ---")
    print(f"At 600,000 iterations, deriving one key takes ~0.3s on a modern CPU.")
    print("An attacker trying 10 billion guesses on a GPU cluster:")
    print(f"  - Without KDF: ~seconds")
    print(f"  - With PBKDF2 @ 600k iterations: ~months")
    print("But Argon2 with memory hardness (64MB per guess) forces the")
    print("attacker to invest RAM, not just time, making GPU attacks far less efficient.")
    print()

    print("What KDFs DO NOT promise:")
    print("  - A derived key is only as strong as the password.");
    print("    'password123' → 64 bytes of random-looking bytes is still breakable.")
    print("  - PBKDF2 is parallelizable on GPUs. Use Argon2 or scrypt")
    print("    for memory-hard resistance.")


def main() -> None:
    print("PHASE 12, LESSON 01 — What Cryptography Actually Promises")
    print()
    print("This program demonstrates five cryptographic primitive categories.")
    print("Each shows what the primitive PROMISES and what it DOES NOT promise.")

    demo_symmetric()
    demo_asymmetric()
    demo_hash()
    demo_mac()
    demo_kdf()

    _separate("SUMMARY")
    print()
    print("Primitive          | Promises                                        | Does NOT promise")
    print("-------------------|-------------------------------------------------|-------------------------------------------")
    print("Symmetric (AES)    | Confidentiality, Integrity, Authenticity (AEAD) | Key management, side channels, availability")
    print("Asymmetric (RSA)   | Confidentiality (pub→priv), Auth + NR (priv→pub)| Efficiency, quantum safety, impl correctness")
    print("Hash (SHA-256)     | Integrity, preimage + collision resistance      | Authenticity, confidentiality")
    print("MAC (HMAC)         | Integrity + Authenticity (with shared key)      | Non-repudiation, confidentiality")
    print("KDF (PBKDF2)       | Slow, salted derivation from passwords           | Can't make weak passwords strong")
    print()
    print("Core lesson: Know what each primitive gives you, and")
    print("equally important, know what it doesn't.")


if __name__ == "__main__":
    main()