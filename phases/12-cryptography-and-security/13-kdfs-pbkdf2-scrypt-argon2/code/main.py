"""
KDFs, PBKDF2, scrypt, Argon2
Phase 12 — Cryptography & Security

Demonstrates Argon2id key derivation, password hashing comparison,
and benchmarks across PBKDF2, scrypt, and Argon2id.
"""

import hashlib
import secrets
import time
from typing import Callable


# ---------------------------------------------------------------------------
# Argon2id: state-of-the-art KDF
# ---------------------------------------------------------------------------

def argon2id_demo() -> None:
    """Demonstrate Argon2id key derivation and verification."""
    print("=== Argon2id Key Derivation ===\n")

    try:
        from argon2 import PasswordHasher, Type
        from argon2.exceptions import VerifyMismatchError
    except ImportError:
        print("  argon2-cffi not installed. Install with: pip install argon2-cffi")
        print("  Skipping Argon2id demo.\n")
        return

    # Derive a key from a password
    password = b"correct horse battery staple"
    salt = secrets.token_bytes(16)
    key = hashlib.scrypt(password, salt=salt, n=2**14, r=8, p=1, dklen=32)
    print(f"  Password: {password!r}")
    print(f"  Salt:     {salt.hex()}")
    print(f"  Key (32B): {key.hex()}")
    print()

    # Parameter selection matters
    print("  --- Argon2id Parameter Selection ---")
    params = [
        ("Interactive  ", 2, 15, 2),
        ("Moderate     ", 3, 18, 2),
        ("Sensitive    ", 4, 21, 2),
    ]
    for label, t_cost, m_cost, parallelism in params:
        ph = PasswordHasher(
            time_cost=t_cost,
            memory_cost=2**m_cost // 1024,  # in KiB
            parallelism=parallelism,
            hash_len=32,
            type=Type.ID,
        )
        start = time.perf_counter()
        h = ph.hash(password.decode())
        elapsed = time.perf_counter() - start
        valid = ph.verify(h, password.decode())
        print(f"  {label}: t={t_cost}, m=2^{m_cost}, p={parallelism}")
        print(f"    Hash: {h[:60]}...")
        print(f"    Time: {elapsed:.3f}s   Valid: {valid}")
        print()

    # Verification with wrong password is rejected
    ph = PasswordHasher(type=Type.ID)
    h = ph.hash(password.decode())
    try:
        ph.verify(h, "wrong password")
        print("  ERROR: wrong password was accepted!")
    except VerifyMismatchError:
        print("  ✓ Wrong password correctly rejected")
    print()


# ---------------------------------------------------------------------------
# KDF benchmarks: PBKDF2 vs scrypt vs Argon2id
# ---------------------------------------------------------------------------

def benchmark_pbkdf2(password: bytes, salt: bytes, iterations: int) -> float:
    start = time.perf_counter()
    hashlib.pbkdf2_hmac("sha256", password, salt, iterations, dklen=32)
    return time.perf_counter() - start


def benchmark_scrypt(password: bytes, salt: bytes, n: int) -> float:
    start = time.perf_counter()
    hashlib.scrypt(password, salt=salt, n=n, r=8, p=1, dklen=32)
    return time.perf_counter() - start


def benchmark_argon2id(password: bytes, t_cost: int, m_kib: int) -> float:
    from argon2 import PasswordHasher, Type
    ph = PasswordHasher(
        time_cost=t_cost,
        memory_cost=m_kib,
        parallelism=1,
        hash_len=32,
        type=Type.ID,
    )
    start = time.perf_counter()
    ph.hash(password.decode())
    return time.perf_counter() - start


def run_benchmarks() -> None:
    """Compare PBKDF2, scrypt, and Argon2id with different work factors."""
    print("=== KDF Performance Benchmarks ===\n")

    password = b"correct horse battery staple"
    salt = secrets.token_bytes(16)

    print("--- PBKDF2-HMAC-SHA256 ---")
    for iterations in [100_000, 300_000, 600_000]:
        elapsed = benchmark_pbkdf2(password, salt, iterations)
        print(f"  {iterations:>6,} iterations: {elapsed:.3f}s")

    print()
    print("--- scrypt (r=8, p=1) ---")
    for n in [2**14, 2**15, 2**16]:
        mem_kb = 128 * 8 * n / 1024
        elapsed = benchmark_scrypt(password, salt, n)
        print(f"  n=2^{n.bit_length()-1:<3} (mem: ~{mem_kb:,.0f} KB): {elapsed:.3f}s")

    print()
    print("--- Argon2id (p=1) ---")
    for t_cost, m_cost_log2 in [(2, 18), (3, 19), (4, 20)]:
        m_kib = 2**m_cost_log2 // 1024
        elapsed = benchmark_argon2id(password, t_cost, m_kib)
        print(f"  t={t_cost}, m=2^{m_cost_log2:<3} ({m_kib:>6,} KiB): {elapsed:.3f}s")

    print()


# ---------------------------------------------------------------------------
# Password storage demo: salt + verify
# ---------------------------------------------------------------------------

def password_storage_demo() -> None:
    """Demonstrate salted password hashing and verification."""
    print("=== Password Storage with Unique Salts ===\n")

    passwords = [
        b"let-me-in",
        b"password123",
        b"Tr0ub4dor&3",
    ]

    print("  Same password → different hash (different salt per hash)\n")
    for pw in passwords:
        salt = secrets.token_bytes(16)
        h = hashlib.pbkdf2_hmac("sha256", pw, salt, 100_000, dklen=32)
        print(f"  Password: {pw!r}")
        print(f"    Salt: {salt.hex()}")
        print(f"    Hash: {h.hex()}")
        print()

    # Verification: re-derive key with stored salt and compare
    print("  --- Verification Check ---")
    stored_salt = secrets.token_bytes(16)
    stored_hash = hashlib.pbkdf2_hmac("sha256", b"let-me-in", stored_salt, 100_000, dklen=32)

    def verify(attempt: bytes) -> bool:
        h = hashlib.pbkdf2_hmac("sha256", attempt, stored_salt, 100_000, dklen=32)
        return h == stored_hash

    print(f"  Stored salt: {stored_salt.hex()}")
    print(f"  Stored hash: {stored_hash.hex()}")
    print(f"  Verify 'let-me-in':    {verify(b'let-me-in')}")
    print(f"  Verify 'wrong-pw':     {verify(b'wrong-pw')}")
    print()


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def main() -> None:
    print()
    print("╔══════════════════════════════════════════════════════╗")
    print("║   KDFs: PBKDF2, scrypt & Argon2id                  ║")
    print("║   Phase 12 — Cryptography & Security               ║")
    print("╚══════════════════════════════════════════════════════╝")
    print()

    password_storage_demo()
    argon2id_demo()
    run_benchmarks()

    print("=== Why This Matters ===")
    print("  PBKDF2:  CPU-bound iteration, no memory-hardness → vulnerable to GPU/ASIC")
    print("  scrypt:  Memory-hard via large ROMix buffer → resists GPU/ASIC better")
    print("  Argon2id: Memory-hard + data-dependent access + side-channel resistant")
    print("            → the OWASP-recommended choice for new applications")
    print()


if __name__ == "__main__":
    main()
