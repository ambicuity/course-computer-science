"""
MACs and HMAC — Phase 12, Lesson 07
HMAC-SHA256 from scratch, HKDF, length extension attack, timing attack demo.
"""

import hashlib
import hmac as hmac_stdlib
import struct
import time


# ── SHA-256 (minimal, for HMAC internals) ──────────────────────────────────

SHA256_K = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9fc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
]

SHA256_H0 = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
]

MASK32 = 0xFFFFFFFF


def _rotr32(x, n):
    return ((x >> n) | (x << (32 - n))) & MASK32


def sha256(data: bytes) -> bytes:
    h = list(SHA256_H0)
    msg = bytearray(data)
    msg_len_bits = len(data) * 8
    msg.append(0x80)
    while len(msg) % 64 != 56:
        msg.append(0x00)
    msg += struct.pack(">Q", msg_len_bits)

    for chunk_start in range(0, len(msg), 64):
        chunk = msg[chunk_start:chunk_start + 64]
        w = list(struct.unpack(">16L", chunk))
        for i in range(16, 64):
            s0 = _rotr32(w[i-15], 7) ^ _rotr32(w[i-15], 18) ^ (w[i-15] >> 3)
            s1 = _rotr32(w[i-2], 17) ^ _rotr32(w[i-2], 19) ^ (w[i-2] >> 10)
            w.append((w[i-16] + s0 + w[i-7] + s1) & MASK32)
        a, b, c, d, e, f, g, hh = h
        for i in range(64):
            S1 = _rotr32(e, 6) ^ _rotr32(e, 11) ^ _rotr32(e, 25)
            ch = (e & f) ^ ((~e & MASK32) & g)
            temp1 = (hh + S1 + ch + SHA256_K[i] + w[i]) & MASK32
            S0 = _rotr32(a, 2) ^ _rotr32(a, 13) ^ _rotr32(a, 22)
            maj = (a & b) ^ (a & c) ^ (b & c)
            temp2 = (S0 + maj) & MASK32
            hh = g
            g = f
            f = e
            e = (d + temp1) & MASK32
            d = c
            c = b
            b = a
            a = (temp1 + temp2) & MASK32
        h[0] = (h[0] + a) & MASK32
        h[1] = (h[1] + b) & MASK32
        h[2] = (h[2] + c) & MASK32
        h[3] = (h[3] + d) & MASK32
        h[4] = (h[4] + e) & MASK32
        h[5] = (h[5] + f) & MASK32
        h[6] = (h[6] + g) & MASK32
        h[7] = (h[7] + hh) & MASK32

    return struct.pack(">8L", *h)


# ── HMAC-SHA256 from scratch ─────────────────────────────────────────────

SHA256_BLOCK_SIZE = 64
SHA256_DIGEST_SIZE = 32


def hmac_sha256(key: bytes, message: bytes) -> bytes:
    if len(key) > SHA256_BLOCK_SIZE:
        key = sha256(key)
    key = key + b'\x00' * (SHA256_BLOCK_SIZE - len(key))

    ipad = bytes(k ^ 0x36 for k in key)
    opad = bytes(k ^ 0x5C for k in key)

    inner = sha256(ipad + message)
    return sha256(opad + inner)


# ── HKDF (HMAC-based Extract-and-Expand) ───────────────────────────────

def hkdf_extract(salt: bytes, ikm: bytes) -> bytes:
    return hmac_sha256(salt, ikm)


def hkdf_expand(prk: bytes, info: bytes, length: int) -> bytes:
    n = (length + SHA256_DIGEST_SIZE - 1) // SHA256_DIGEST_SIZE
    okm = b""
    t = b""
    for i in range(1, n + 1):
        t = hmac_sha256(prk, t + info + bytes([i]))
        okm += t
    return okm[:length]


# ── Naive MAC (vulnerable to length extension) ──────────────────────────

def naive_mac(key: bytes, message: bytes) -> bytes:
    return sha256(key + message)


def sha256_length_extension(original_hash: bytes, original_len: int, suffix: bytes) -> bytes:
    """
    Given H(K || M) and len(K || M), compute H(K || M || padding || suffix)
    without knowing K or M.
    """
    original_data_len = original_len
    padded_len = ((original_data_len + 9 + 63) // 64) * 64

    padding = b'\x80'
    padding += b'\x00' * (padded_len - original_data_len - 1 - 8)
    padding += struct.pack(">Q", original_data_len * 8)

    glue_padding = padding[0:(padded_len - original_data_len)]

    total_len = padded_len + len(suffix)
    new_msg = bytearray(len(suffix))
    for i in range(64, len(suffix) + 64):
        pass  # we just need the suffix data

    h = list(struct.unpack(">8L", original_hash))
    extended = suffix
    extended_len = total_len

    new_msg = bytearray(extended)

    msg = bytearray(new_msg)
    msg_len_bits = extended_len + len(suffix)
    msg.append(0x80)
    while len(msg) % 64 != 56:
        msg.append(0x00)
    msg += struct.pack(">Q", extended_len * 8 + len(suffix) * 8)

    result = h
    for chunk_start in range(0, len(msg), 64):
        chunk = msg[chunk_start:chunk_start + 64]
        w = list(struct.unpack(">16L", chunk)) if len(chunk) == 64 else [0]*16
        for i in range(16, 64):
            s0 = _rotr32(w[i-15], 7) ^ _rotr32(w[i-15], 18) ^ (w[i-15] >> 3)
            s1 = _rotr32(w[i-2], 17) ^ _rotr32(w[i-2], 19) ^ (w[i-2] >> 10)
            w.append((w[i-16] + s0 + w[i-7] + s1) & MASK32)
        a, b, c, d, e, f, g, hh = result
        for i in range(64):
            S1 = _rotr32(e, 6) ^ _rotr32(e, 11) ^ _rotr32(e, 25)
            ch = (e & f) ^ ((~e & MASK32) & g)
            temp1 = (hh + S1 + ch + SHA256_K[i] + w[i]) & MASK32
            S0 = _rotr32(a, 2) ^ _rotr32(a, 13) ^ _rotr32(a, 22)
            maj = (a & b) ^ (a & c) ^ (b & c)
            temp2 = (S0 + maj) & MASK32
            hh = g
            g = f
            f = e
            e = (d + temp1) & MASK32
            d = c
            c = b
            b = a
            a = (temp1 + temp2) & MASK32
        result[0] = (result[0] + a) & MASK32
        result[1] = (result[1] + b) & MASK32
        result[2] = (result[2] + c) & MASK32
        result[3] = (result[3] + d) & MASK32
        result[4] = (result[4] + e) & MASK32
        result[5] = (result[5] + f) & MASK32
        result[6] = (result[6] + g) & MASK32
        result[7] = (result[7] + hh) & MASK32

    return struct.pack(">8L", *result)


def sha256_continue_from_state(state_bytes: bytes, remaining_data: bytes, total_processed: int) -> bytes:
    """
    Continue SHA-256 from a known state (the hash digest) with new data,
    given total_processed bytes were already hashed. This is the core
    of the length extension attack.
    """
    h = list(struct.unpack(">8L", state_bytes))
    data = bytearray(remaining_data)
    data.append(0x80)
    total_bits = (total_processed + len(remaining_data)) * 8
    while len(data) % 64 != 56:
        data.append(0x00)
    data += struct.pack(">Q", total_bits)

    for chunk_start in range(0, len(data), 64):
        chunk = data[chunk_start:chunk_start + 64]
        w = list(struct.unpack(">16L", chunk))
        for i in range(16, 64):
            s0 = _rotr32(w[i-15], 7) ^ _rotr32(w[i-15], 18) ^ (w[i-15] >> 3)
            s1 = _rotr32(w[i-2], 17) ^ _rotr32(w[i-2], 19) ^ (w[i-2] >> 10)
            w.append((w[i-16] + s0 + w[i-7] + s1) & MASK32)
        a, b, c, d, e, f, g, hh = h
        for i in range(64):
            S1 = _rotr32(e, 6) ^ _rotr32(e, 11) ^ _rotr32(e, 25)
            ch = (e & f) ^ ((~e & MASK32) & g)
            temp1 = (hh + S1 + ch + SHA256_K[i] + w[i]) & MASK32
            S0 = _rotr32(a, 2) ^ _rotr32(a, 13) ^ _rotr32(a, 22)
            maj = (a & b) ^ (a & c) ^ (b & c)
            temp2 = (S0 + maj) & MASK32
            hh = g
            g = f
            f = e
            e = (d + temp1) & MASK32
            d = c
            c = b
            b = a
            a = (temp1 + temp2) & MASK32
        h[0] = (h[0] + a) & MASK32
        h[1] = (h[1] + b) & MASK32
        h[2] = (h[2] + c) & MASK32
        h[3] = (h[3] + d) & MASK32
        h[4] = (h[4] + e) & MASK32
        h[5] = (h[5] + f) & MASK32
        h[6] = (h[6] + g) & MASK32
        h[7] = (h[7] + hh) & MASK32

    return struct.pack(">8L", *h)


def compute_glue_padding(msg_len: int) -> bytes:
    padded_len = ((msg_len + 1 + 8 + 63) // 64) * 64
    padding = b'\x80'
    padding += b'\x00' * (padded_len - msg_len - 1 - 8)
    padding += struct.pack(">Q", msg_len * 8)
    return padding


def demonstrate_length_extension_attack():
    """
    Demonstrate that H(K || M) is vulnerable to length extension,
    but HMAC(K, M) is NOT.
    """
    key = b'secret_key_1234'
    original_message = b'transfer $100 to Alice'
    extension = b' and $50 to Eve'

    print("=" * 60)
    print("LENGTH EXTENSION ATTACK DEMONSTRATION")
    print("=" * 60)

    naive_tag = naive_mac(key, original_message)
    print(f"\nOriginal message: {original_message}")
    print(f"Naive MAC tag:    {naive_tag.hex()}")

    # Compute glue padding for K || M
    km_len = len(key) + len(original_message)
    glue_padding = compute_glue_padding(km_len)
    total_processed = km_len + len(glue_padding)

    # The attacker sees the tag and knows len(K||M) but NOT K or M
    # They compute H(K || M || glue_padding || extension) from H(K||M)
    forged_tag = sha256_continue_from_state(naive_tag, extension, total_processed)

    # Verify: compute the actual hash of K || M || glue_padding || extension
    full_message = original_message + glue_padding + extension
    actual_hash = sha256(key + full_message)

    print(f"\nForged tag (via length extension):  {forged_tag.hex()}")
    print(f"Actual hash K||M||pad||ext:        {actual_hash.hex()}")
    print(f"Length extension attack works:     {forged_tag == actual_hash}")
    print(f"The attacker forged a valid tag WITHOUT knowing the key!")

    # Now show HMAC resists this
    hmac_tag = hmac_sha256(key, original_message)
    print(f"\nHMAC tag:                          {hmac_tag.hex()}")
    print(f"HMAC resists length extension:     the inner hash hides state")

    # Try the same trick on HMAC — it fails
    hmac_forged = sha256_continue_from_state(hmac_tag, extension, SHA256_BLOCK_SIZE + SHA256_DIGEST_SIZE)
    hmac_actual = sha256(bytes(k ^ 0x5C for k in (key + b'\x00' * (SHA256_BLOCK_SIZE - len(key)))) +
                         sha256(bytes(k ^ 0x36 for k in (key + b'\x00' * (SHA256_BLOCK_SIZE - len(key)))) +
                                original_message + extension))
    print(f"Forcing length extension on HMAC forged: {hmac_forged.hex()}")
    print(f"Actual HMAC of extended message:          {hmac_actual.hex()}")
    print(f"Length extension on HMAC works:           {hmac_forged == hmac_actual}")
    print("HMAC is NOT vulnerable to length extension attacks!")


# ── HKDF Test Vectors (RFC 5869) ───────────────────────────────────────

def test_hkdf():
    print("\n" + "=" * 60)
    print("HKDF TEST VECTORS (RFC 5869, Test Case 1)")
    print("=" * 60)

    ikm = bytes.fromhex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b")
    salt = bytes.fromhex("000102030405060708090a0b0c")
    info = bytes.fromhex("f0f1f2f3f4f5f6f7f8f9")
    l = 42

    prk = hkdf_extract(salt, ikm)
    okm = hkdf_expand(prk, info, l)

    expected_prk = bytes.fromhex(
        "077709362c2e32df0ddc3f0dc47bba63"
        "90b6c73bb50f9c3122ca8e4f3df5970"
    )
    expected_okm = bytes.fromhex(
        "3cb25f25faacd57a90434f64d0362f2a"
        "2d2d0a90cf1a5a4c5db5418eac4aee2"
        "3d1a29b1a4d2f439de3ffbe3f4af705"
        "cf5d7bba"
    )

    print(f"PRK computed:  {prk.hex()}")
    print(f"PRK expected:  {expected_prk.hex()}")
    print(f"PRK matches:   {prk == expected_prk}")
    print(f"\nOKM computed:  {okm.hex()}")
    print(f"OKM expected:  {expected_okm.hex()}")
    print(f"OKM matches:   {okm == expected_okm}")


# ── HMAC-SHA256 Test Vectors (RFC 4231) ──────────────────────────────────

def test_hmac_vectors():
    print("\n" + "=" * 60)
    print("HMAC-SHA256 TEST VECTORS (RFC 4231)")
    print("=" * 60)

    test_cases = [
        {
            "key": bytes.fromhex("0b" * 20),
            "data": b"Hi There",
            "expected": "b0344c703b8ec6cf82e8b4d0394b4b0a3b6f85d7b0f0303f7afab2403f063029",
        },
        {
            "key": b"Jefe",
            "data": b"what do ya want for nothing?",
            "expected": "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
        },
        {
            "key": bytes.fromhex("aa" * 20),
            "data": bytes.fromhex("dd" * 50),
            "expected": "773ea91e36800e46854db8ebd09181a72986033167f883d0289f18758fdb3822",
        },
    ]

    all_pass = True
    for i, tc in enumerate(test_cases, 1):
        result = hmac_sha256(tc["key"], tc["data"])
        expected = bytes.fromhex(tc["expected"])
        match = result == expected
        all_pass = all_pass and match
        print(f"\n  Test {i}: key={tc['key'][:8].hex()}..., "
              f"data={tc['data'][:16]}..., match={match}")
        if not match:
            print(f"    Got:      {result.hex()}")
            print(f"    Expected: {tc['expected']}")

    # Cross-verify with stdlib
    key = b"test_key_12345678"
    msg = b"Hello, HMAC world!"
    ours = hmac_sha256(key, msg)
    theirs = hmac_stdlib.new(key, msg, hashlib.sha256).digest()
    print(f"\n  Cross-verify with hashlib: {ours == theirs}")
    all_pass = all_pass and (ours == theirs)

    print(f"\n  All HMAC test vectors pass: {all_pass}")


# ── Timing Attack on MAC Comparison ────────────────────────────────────────

def insecure_compare(a: bytes, b: bytes) -> bool:
    """VULNERABLE: returns early on first mismatch, leaking timing."""
    if len(a) != len(b):
        return False
    for x, y in zip(a, b):
        if x != y:
            return False
    return True


def constant_time_compare(a: bytes, b: bytes) -> bool:
    """SAFE: always scans all bytes regardless of match position."""
    if len(a) != len(b):
        return False
    result = 0
    for x, y in zip(a, b):
        result |= x ^ y
    return result == 0


def demonstrate_timing_attack():
    """
    Show that byte-by-byte comparison leaks information through timing.
    An attacker can recover the tag byte by byte by measuring response times.
    """
    print("\n" + "=" * 60)
    print("TIMING ATTACK ON MAC COMPARISON")
    print("=" * 60)

    key = b"my_secret_key_for_demo_12345"
    message = b"Important financial transaction"
    correct_tag = hmac_sha256(key, message)

    print(f"\nCorrect tag: {correct_tag.hex()[:32]}...")

    # Measure timing for tags with increasing prefix matches
    print("\n  Insecure comparison — timing leaks prefix match length:")
    print(f"  {'Prefix match':<16} {'Avg time (µs)':<16} {'Insecure result'}")

    for prefix_len in range(0, 33, 4):
        forged = bytearray(correct_tag[:prefix_len]) + bytearray([0xFF] * (32 - prefix_len))
        forged = bytes(forged)

        times = []
        for _ in range(10000):
            start = time.perf_counter_ns()
            insecure_compare(correct_tag, forged)
            elapsed = time.perf_counter_ns() - start
            times.append(elapsed)

        avg_ns = sum(times) / len(times)
        result = insecure_compare(correct_tag, forged)
        print(f"  {prefix_len:>3} bytes       {avg_ns:>10.0f}       {result}")

    print("\n  Note: timing increases with prefix match length — attacker can")
    print("  recover the tag byte by byte by observing which猜测 maximizes delay.")

    print("\n  Constant-time comparison — no timing leak:")
    for prefix_len in [0, 16, 32]:
        forged = bytearray(correct_tag[:prefix_len]) + bytearray([0xFF] * (32 - prefix_len))
        forged = bytes(forged)

        times = []
        for _ in range(10000):
            start = time.perf_counter_ns()
            constant_time_compare(correct_tag, forged)
            elapsed = time.perf_counter_ns() - start
            times.append(elapsed)

        avg_ns = sum(times) / len(times)
        result = constant_time_compare(correct_tag, forged)
        print(f"  {prefix_len:>3} bytes match   {avg_ns:>10.0f}       {result}")

    print("\n  Python hmac.compare_digest() provides constant-time comparison.")
    print("  Rust: use subtle::ConstantTimeEq.")


# ── Message Integrity Demo ────────────────────────────────────────────────

def demonstrate_message_integrity():
    print("\n" + "=" * 60)
    print("MESSAGE INTEGRITY AND AUTHENTICATION DEMO")
    print("=" * 60)

    key = b"shared_secret_between_alice_bob"
    original_message = b"Transfer $500 to account #1234"
    tampered_message = b"Transfer $500 to account #5678"

    original_tag = hmac_sha256(key, original_message)
    tampered_tag = hmac_sha256(key, tampered_message)

    print(f"\nOriginal message: {original_message}")
    print(f"Original tag:      {original_tag.hex()}")
    print(f"\nTampered message:  {tampered_message}")
    print(f"Tampered tag:      {tampered_tag.hex()}")

    print(f"\nVerify original (tag matches): {hmac_sha256(key, original_message) == original_tag}")
    print(f"Verify tampered (tag differs): {hmac_sha256(key, tampered_message) == original_tag}")

    print(f"\nAttack scenario: Eve intercepts and modifies the message.")
    print(f"She cannot produce a valid tag for her modified message without the key.")
    print(f"Bob detects the forgery: tag mismatch.")


# ── Main ──────────────────────────────────────────────────────────────────

def main():
    print("MACs and HMAC — Phase 12, Lesson 07")
    print("=" * 60)

    test_hmac_vectors()
    test_hkdf()
    demonstrate_length_extension_attack()
    demonstrate_message_integrity()
    demonstrate_timing_attack()

    print("\n" + "=" * 60)
    print("All demonstrations complete.")
    print("=" * 60)


if __name__ == "__main__":
    main()