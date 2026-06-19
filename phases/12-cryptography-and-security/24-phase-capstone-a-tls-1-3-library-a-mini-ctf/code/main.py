#!/usr/bin/env python3
"""
Phase 12 Mini-CTF — 5 Cryptography & Security Challenges
TLS 1.3 Library + a Mini-CTF — Phase Capstone
"""

import os
import sys
import time
import struct
import string

from Crypto.Cipher import AES
from Crypto.Util.Padding import pad, unpad
from Crypto.Util.number import getPrime, bytes_to_long, long_to_bytes
from Crypto.Random import get_random_bytes


FLAGS = {
    1: "FLAG{ecb_leaks_like_a_sieve}",
    2: "FLAG{never_reuse_nonces_in_ctr_mode}",
    3: "FLAG{small_e_is_a_big_problem}",
    4: "FLAG{tim}",
    5: "FLAG{padding_oracles_break_cbc}",
}

SOLVED = set()


# ── Helper Utilities ──────────────────────────────────────────────────────────

BLOCK_SIZE = 16


def aes_ecb_encrypt(key: bytes, plaintext: bytes) -> bytes:
    cipher = AES.new(key, AES.MODE_ECB)
    return cipher.encrypt(plaintext)


def aes_ctr_encrypt(key: bytes, nonce: int, plaintext: bytes) -> bytes:
    nonce_bytes = nonce.to_bytes(8, "big")
    cipher = AES.new(key, AES.MODE_CTR, nonce=nonce_bytes)
    return cipher.encrypt(plaintext)


def aes_cbc_encrypt(key: bytes, iv: bytes, plaintext: bytes) -> bytes:
    cipher = AES.new(key, AES.MODE_CBC, iv)
    return cipher.encrypt(plaintext)


def aes_cbc_decrypt(key: bytes, iv: bytes, ciphertext: bytes) -> bytes:
    cipher = AES.new(key, AES.MODE_CBC, iv)
    return cipher.decrypt(ciphertext)


def check_pkcs7_padding(data: bytes) -> bool:
    if len(data) == 0:
        return False
    pad_len = data[-1]
    if pad_len < 1 or pad_len > BLOCK_SIZE:
        return False
    if len(data) < pad_len:
        return False
    return all(data[-i] == pad_len for i in range(1, pad_len + 1))


def remove_pkcs7_padding(data: bytes) -> bytes:
    pad_len = data[-1]
    if pad_len < 1 or pad_len > BLOCK_SIZE:
        raise ValueError("Invalid padding")
    if not all(data[-i] == pad_len for i in range(1, pad_len + 1)):
        raise ValueError("Invalid padding")
    return data[:-pad_len]


def integer_cube_root(n: int) -> int:
    lo, hi = 0, 1 << (n.bit_length() // 3 + 1)
    while lo < hi:
        mid = (lo + hi) // 2
        if mid**3 < n:
            lo = mid + 1
        else:
            hi = mid
    return lo


# ── Challenge 1: AES-ECB Byte-at-a-Time ──────────────────────────────────────

def challenge1():
    print("=" * 60)
    print("CHALLENGE 1: AES-ECB Byte-at-a-Time Decryption")
    print("=" * 60)
    print("An oracle encrypts (your_input + SECRET_FLAG) with AES-ECB.")
    print("Recover the flag byte-by-byte using ECB's determinism.\n")

    key = get_random_bytes(16)
    secret_flag = FLAGS[1].encode()

    def oracle(plaintext: bytes) -> bytes:
        return aes_ecb_encrypt(key, pad(plaintext + secret_flag, BLOCK_SIZE))

    base_len = len(oracle(b""))
    block_size = 0
    for i in range(1, 33):
        if len(oracle(b"A" * i)) != base_len:
            block_size = len(oracle(b"A" * i)) - base_len
            break
    print(f"  [detect] Block size: {block_size}")

    test_input = b"A" * (block_size * 3)
    ct = oracle(test_input)
    blocks = [ct[i : i + block_size] for i in range(0, len(ct), block_size)]
    is_ecb = len(blocks) != len(set(blocks))
    print(f"  [detect] ECB mode: {is_ecb}")

    if not is_ecb:
        print("  [FAIL] Cannot detect ECB mode; aborting.")
        return

    recovered = b""
    total_unknown = len(oracle(b""))
    for i in range(total_unknown):
        prefix_len = block_size - 1 - (i % block_size)
        prefix = b"A" * prefix_len
        reference_ct = oracle(prefix)
        block_num = i // block_size
        ref_block = reference_ct[
            block_num * block_size : (block_num + 1) * block_size
        ]
        found = False
        for guess in range(256):
            test_input = prefix + recovered + bytes([guess])
            test_ct = oracle(test_input)
            test_block = test_ct[
                block_num * block_size : (block_num + 1) * block_size
            ]
            if test_block == ref_block:
                recovered += bytes([guess])
                found = True
                break
        if not found:
            break

    recovered_str = remove_pkcs7_padding(recovered).decode("utf-8", errors="replace")
    print(f"  [result] Recovered: {recovered_str}")

    if FLAGS[1] in recovered_str:
        print(f"  [PASS] Challenge 1 solved! Flag: {FLAGS[1]}")
        SOLVED.add(1)
    else:
        print(f"  [FAIL] Could not recover the flag correctly.")
    print()


# ── Challenge 2: AES-CTR Nonce Reuse ─────────────────────────────────────────

def challenge2():
    print("=" * 60)
    print("CHALLENGE 2: AES-CTR Nonce Reuse (Two-Time Pad)")
    print("=" * 60)
    print("Two plaintexts encrypted with the same key and nonce.")
    print("XOR the ciphertexts to cancel the keystream.\n")

    key = get_random_bytes(16)
    nonce = 0

    known_plaintext = b"Known plaintext: X" * 4
    flag = FLAGS[2].encode()

    ct_known = aes_ctr_encrypt(key, nonce, known_plaintext)
    ct_flag = aes_ctr_encrypt(key, nonce, flag)

    print(f"  Known plaintext:  {known_plaintext}")
    print(f"  CT known (hex):   {ct_known.hex()[:48]}...")
    print(f"  CT flag (hex):    {ct_flag.hex()[:48]}...")

    min_len = min(len(ct_known), len(ct_flag), len(known_plaintext))
    recovered_keystream = bytes(
        a ^ b for a, b in zip(ct_known[:min_len], known_plaintext[:min_len])
    )
    recovered_flag = bytes(
        c ^ k for c, k in zip(ct_flag[: len(recovered_keystream)], recovered_keystream)
    )
    recovered_flag_str = recovered_flag.decode("utf-8", errors="replace")
    print(f"  [result] Recovered: {recovered_flag_str}")

    if FLAGS[2] in recovered_flag_str:
        print(f"  [PASS] Challenge 2 solved! Flag: {FLAGS[2]}")
        SOLVED.add(2)
    else:
        print(f"  [FAIL] Could not recover the flag correctly.")
    print()


# ── Challenge 3: Weak RSA (Small Exponent) ───────────────────────────────────

def challenge3():
    print("=" * 60)
    print("CHALLENGE 3: Weak RSA (Small Exponent e=3)")
    print("=" * 60)
    print("RSA with e=3 and no padding. If m^3 < n, just take cube root.\n")

    while True:
        p = getPrime(512)
        q = getPrime(512)
        if (p - 1) % 3 != 0 and (q - 1) % 3 != 0:
            break
    n = p * q
    e = 3

    flag_int = bytes_to_long(FLAGS[3].encode())
    if flag_int**3 >= n:
        print("  [WARN] Flag too large; using smaller test message")
        flag_int = bytes_to_long(b"test_flag_123")
    assert flag_int**3 < n, "Message must have m^3 < n for cube root attack"

    ct = pow(flag_int, e, n)

    print(f"  RSA Modulus n (bits): {n.bit_length()}")
    print(f"  Public exponent e:     {e}")
    print(f"  Ciphertext (hex):      {hex(ct)[:64]}...")
    print(f"  Flag integer (bits):   {flag_int.bit_length()}")
    print(f"  flag^3 (bits):         {(flag_int**3).bit_length()}  (< {n.bit_length()}, so no reduction)\n")

    recovered_int = integer_cube_root(ct)
    recovered_bytes = long_to_bytes(recovered_int)
    recovered_str = recovered_bytes.decode("utf-8", errors="replace")
    print(f"  [result] Recovered: {recovered_str}")

    if FLAGS[3] in recovered_str:
        print(f"  [PASS] Challenge 3 solved! Flag: {FLAGS[3]}")
        SOLVED.add(3)
    else:
        print(f"  [FAIL] Could not recover the flag correctly.")
    print()


# ── Challenge 4: Timing Oracle ───────────────────────────────────────────────

def challenge4():
    print("=" * 60)
    print("CHALLENGE 4: Timing Oracle Side-Channel")
    print("=" * 60)
    print("Password checker leaks character position via timing.")
    print("Measure response time to recover secret byte-by-byte.\n")

    password = FLAGS[4]
    delay_per_char = 0.02

    def check_password(guess: str) -> float:
        start = time.perf_counter()
        for i in range(min(len(guess), len(password))):
            if guess[i] != password[i]:
                return time.perf_counter() - start
            time.sleep(delay_per_char)
        return time.perf_counter() - start

    known = ""
    charset = string.ascii_letters + "_{}"
    password_len = len(password)
    print(f"  Password length: {password_len}")
    print(f"  Character set:   {len(charset)} chars")
    print(f"  Delay per char:  {delay_per_char * 1000:.0f}ms\n")

    for pos in range(password_len):
        first_pass = {}
        for ch in charset:
            first_pass[ch] = check_password(known + ch)
        candidates = sorted(first_pass, key=first_pass.get, reverse=True)[:3]

        second_pass = {}
        for ch in candidates:
            samples = [check_password(known + ch) for _ in range(10)]
            second_pass[ch] = sorted(samples)[5]

        best_char = max(second_pass, key=second_pass.get)
        known += best_char
        expected = password[pos]
        match = "✓" if best_char == expected else "✗"
        print(f"    [{match}] Position {pos:2d}: '{best_char}' (expected '{expected}') "
              f"t={second_pass[best_char]*1000:.0f}ms")

    print(f"\n  [result] Recovered: {known}")

    if known == password:
        print(f"  [PASS] Challenge 4 solved! Flag: {FLAGS[4]}")
        SOLVED.add(4)
    else:
        print(f"  [FAIL] Could not recover the password correctly.")
    print()


# ── Challenge 5: Padding Oracle ──────────────────────────────────────────────

def challenge5():
    print("=" * 60)
    print("CHALLENGE 5: Padding Oracle Attack on AES-CBC")
    print("=" * 60)
    print("Server reveals padding validity. Decrypt ciphertext by")
    print("manipulating CBC blocks and observing oracle responses.\n")

    key = get_random_bytes(16)
    iv = get_random_bytes(16)
    flag_bytes = pad(FLAGS[5].encode(), BLOCK_SIZE)
    ct = aes_cbc_encrypt(key, iv, flag_bytes)

    ct_blocks = [ct[i : i + BLOCK_SIZE] for i in range(0, len(ct), BLOCK_SIZE)]

    def padding_oracle(ciphertext: bytes) -> bool:
        try:
            plain = aes_cbc_decrypt(key, iv, ciphertext)
            return check_pkcs7_padding(plain)
        except Exception:
            return False

    print(f"  IV (hex):      {iv.hex()}")
    print(f"  Ciphertext:    {ct.hex()}")
    print(f"  Blocks:        {len(ct_blocks)}")
    print(f"  Block size:    {BLOCK_SIZE}")

    all_blocks = [iv] + ct_blocks
    result = b""

    for block_idx in range(1, len(all_blocks)):
        intermediate = [0] * BLOCK_SIZE
        decrypted_block = [0] * BLOCK_SIZE
        for byte_pos in range(BLOCK_SIZE - 1, -1, -1):
            pad_val = BLOCK_SIZE - byte_pos
            found_byte = False
            for guess in range(256):
                crafted = bytearray(BLOCK_SIZE)
                for j in range(BLOCK_SIZE - 1, byte_pos, -1):
                    crafted[j] = intermediate[j] ^ pad_val
                crafted[byte_pos] = guess
                test_ct = bytes(crafted) + all_blocks[block_idx]
                if padding_oracle(test_ct):
                    intermediate[byte_pos] = guess ^ pad_val
                    decrypted_block[byte_pos] = (
                        intermediate[byte_pos] ^ all_blocks[block_idx - 1][byte_pos]
                    )
                    found_byte = True
                    break
            if not found_byte:
                print(f"  [WARN] Could not decrypt byte {byte_pos} in block {block_idx - 1}")
                decrypted_block[byte_pos] = 0

        result += bytes(decrypted_block)

    try:
        result = remove_pkcs7_padding(result)
    except ValueError:
        result = result.rstrip(b"\x00")

    result_str = result.decode("utf-8", errors="replace")
    print(f"\n  [result] Decrypted: {result_str}")

    if FLAGS[5] in result_str:
        print(f"  [PASS] Challenge 5 solved! Flag: {FLAGS[5]}")
        SOLVED.add(5)
    else:
        print(f"  [FAIL] Could not decrypt the flag correctly.")
    print()


# ── Grader ────────────────────────────────────────────────────────────────────

def grader():
    print("\n" + "=" * 60)
    print("GRADER REPORT")
    print("=" * 60)
    total = len(FLAGS)
    solved = len(SOLVED)
    for i in range(1, total + 1):
        status = "PASS" if i in SOLVED else "FAIL"
        print(f"  Challenge {i}: [{status}] {FLAGS[i]}")
    print(f"\n  Score: {solved}/{total}")
    if solved == total:
        print("  ★ All challenges solved! You've mastered Phase 12!")
    else:
        print(f"  {total - solved} challenge(s) remaining.")
    print()


# ── Main Menu ─────────────────────────────────────────────────────────────────

def main():
    print("╔══════════════════════════════════════════════════════════╗")
    print("║     Phase 12 Capstone — Mini-CTF                       ║")
    print("║     5 Cryptography & Security Challenges               ║")
    print("╚══════════════════════════════════════════════════════════╝")
    print()
    print("Challenges:")
    print("  1. AES-ECB Byte-at-a-Time Decryption")
    print("  2. AES-CTR Nonce Reuse (Two-Time Pad)")
    print("  3. Weak RSA (Small Exponent e=3)")
    print("  4. Timing Oracle Side-Channel")
    print("  5. Padding Oracle Attack on AES-CBC")
    print()

    while True:
        print("Options:")
        print("  [1-5]  Run a specific challenge")
        print("  [a]    Run all challenges")
        print("  [g]    Show grader report")
        print("  [q]    Quit")
        choice = input("> ").strip().lower()

        if choice == "q":
            print("Goodbye!")
            break
        elif choice == "a":
            for i in range(1, 6):
                globals()[f"challenge{i}"]()
            grader()
        elif choice == "g":
            grader()
        elif choice in "12345":
            globals()[f"challenge{choice}"]()
            grader()
        else:
            print("Invalid choice.\n")


if __name__ == "__main__":
    main()
