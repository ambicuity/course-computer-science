"""
Crypto Primitives Quick Reference
Phase 12, Lesson 01 — What Cryptography Actually Promises

Runnable reference: python3 crypto_primitives_demo.py

Shows each primitive category, what it promises, and what it doesn't.
Reused in later phases when you need a quick refresher on "which primitive do I reach for?"
"""

import hashlib
import hmac
import os


def demo_symmetric_xor(plaintext: bytes, key: bytes) -> bytes:
    return bytes(p ^ k for p, k in zip(plaintext, key))


def demo_hmac(key: bytes, message: bytes) -> str:
    return hmac.new(key, message, hashlib.sha256).hexdigest()


def demo_sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def demo_pbkdf2(password: bytes, salt: bytes, iterations: int = 600000) -> bytes:
    return hashlib.pbkdf2_hmac('sha256', password, salt, iterations)


def main() -> None:
    print("=== Crypto Primitives Quick Reference ===\n")

    print("1. Symmetric (XOR demo):")
    key = os.urandom(5)
    ct = demo_symmetric_xor(b"hello", key)
    pt = demo_symmetric_xor(ct, key)
    print(f"   encrypt('hello') = {ct.hex()}")
    print(f"   decrypt           = {pt}")
    print(f"   PROMISES: Confidentiality (with secret key)")
    print(f"   DOES NOT: Integrity, Authenticity\n")

    print("2. Hash (SHA-256):")
    h1 = demo_sha256(b"hello")
    h2 = demo_sha256(b"hellp")
    print(f"   SHA-256('hello') = {h1[:32]}...")
    print(f"   SHA-256('hellp') = {h2[:32]}...")
    print(f"   PROMISES: Integrity, preimage resistance, collision resistance")
    print(f"   DOES NOT: Authenticity, Confidentiality\n")

    print("3. MAC (HMAC-SHA256):")
    k = b"shared-secret"
    tag = demo_hmac(k, b"message")
    print(f"   HMAC(key, 'message') = {tag[:32]}...")
    print(f"   PROMISES: Integrity + Authenticity (with shared key)")
    print(f"   DOES NOT: Non-repudiation, Confidentiality\n")

    print("4. KDF (PBKDF2):")
    salt = os.urandom(16)
    dk = demo_pbkdf2(b"password", salt)
    print(f"   PBKDF2('password', salt) = {dk.hex()[:32]}...")
    print(f"   PROMISES: Slow, salted key derivation")
    print(f"   DOES NOT: Make weak passwords strong\n")

    print("5. Asymmetric (RSA toy, see main.py for full demo)")
    print("   PROMISES: Confidentiality (pub→priv), Auth + Non-repudiation (priv→pub)")
    print("   DOES NOT: Efficiency, quantum resistance\n")

    print("Rule of thumb:")
    print("  Need secrecy?           → Symmetric encryption (AES-GCM)")
    print("  Need integrity?        → Hash (SHA-256)")
    print("  Need integrity+auth?   → MAC (HMAC) or AEAD (AES-GCM tag)")
    print("  Need non-repudiation?  → Digital signature (RSA-PSS, EdDSA)")
    print("  Need password→key?    → KDF (PBKDF2, Argon2)")


if __name__ == "__main__":
    main()