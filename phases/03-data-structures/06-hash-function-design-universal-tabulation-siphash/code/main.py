"""main.py — explore Python's hash() and demonstrate PYTHONHASHSEED randomization.

Python's hash() uses SipHash-2-4 for str/bytes with a per-process random seed.
This blocks hash-flooding attacks against frameworks that hash HTTP params.
"""
from __future__ import annotations
import os, hashlib


def fnv1a(data: bytes) -> int:
    h = 0xcbf29ce484222325
    for b in data:
        h = ((h ^ b) * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return h


def main() -> None:
    seed = os.environ.get("PYTHONHASHSEED", "(unset)")
    print(f"PYTHONHASHSEED = {seed!r}")
    print(f"  hash('hello') = {hash('hello'):#x}  (varies per process unless seed is fixed)")
    print(f"  hash(42)      = {hash(42)}  (integers hash to themselves; not SipHashed)")

    print(f"\nFNV-1a('foobar') = {fnv1a(b'foobar'):#x}  (expected 0x85944171f73967e8)")
    assert fnv1a(b"foobar") == 0x85944171f73967e8

    print(f"\nSHA-256('hello') = {hashlib.sha256(b'hello').hexdigest()[:16]}...  (cryptographic)")

    print("\nTo block hash flooding in your own code:")
    print("  - Use random.SystemRandom for any user-facing dict key derived from untrusted data.")
    print("  - Don't disable PYTHONHASHSEED in production.")
    print("  - For HMAC-style use, use hmac.new() with a secret key, not hash().")


if __name__ == "__main__":
    main()
