"""
Public Key II — RSA Internals & Padding
Phase 12 — Cryptography & Security

Working RSA implementation with key generation (Miller-Rabin),
textbook encryption/decryption, and OAEP padding.
"""

import hashlib
import os
import random


def is_prime(n: int, k: int = 10) -> bool:
    if n < 2:
        return False
    if n < 4:
        return True
    if n % 2 == 0:
        return False
    r, d = 0, n - 1
    while d % 2 == 0:
        r += 1
        d //= 2
    for _ in range(k):
        a = random.randrange(2, n - 2)
        x = pow(a, d, n)
        if x == 1 or x == n - 1:
            continue
        for _ in range(r - 1):
            x = pow(x, 2, n)
            if x == n - 1:
                break
        else:
            return False
    return True


def generate_prime(bits: int) -> int:
    while True:
        n = int.from_bytes(os.urandom(bits // 8), "big")
        n |= (1 << (bits - 1)) | 1
        if is_prime(n, k=20):
            return n


def generate_keypair(bits: int = 512):
    p = generate_prime(bits // 2)
    q = generate_prime(bits // 2)
    n = p * q
    phi = (p - 1) * (q - 1)
    e = 65537
    d = pow(e, -1, phi)
    return (n, e, p, q), (n, d)


def encrypt(pub_key: tuple, plaintext: int) -> int:
    n, e = pub_key[0], pub_key[1]
    return pow(plaintext, e, n)


def decrypt(priv_key: tuple, ciphertext: int) -> int:
    n, d = priv_key
    return pow(ciphertext, d, n)


def mgf1(seed: bytes, length: int, hash_func=hashlib.sha1) -> bytes:
    hlen = hash_func().digest_size
    result = b""
    counter = 0
    while len(result) < length:
        c = counter.to_bytes(4, "big")
        result += hash_func(seed + c).digest()
        counter += 1
    return result[:length]


def oaep_pad(message: bytes, n_bytes: int) -> bytes:
    import hashlib
    hash_func = hashlib.sha1
    hlen = hash_func().digest_size
    max_msg_len = n_bytes - 2 * hlen - 2
    if len(message) > max_msg_len:
        raise ValueError(f"Message too long (max {max_msg_len} bytes)")

    label_hash = hash_func(b"").digest()
    ps_len = n_bytes - len(message) - 2 * hlen - 2
    ps = b"\x00" * ps_len
    db = label_hash + ps + b"\x01" + message

    seed = os.urandom(hlen)
    db_mask = mgf1(seed, n_bytes - hlen - 1)
    masked_db = bytes(a ^ b for a, b in zip(db, db_mask))
    seed_mask = mgf1(masked_db, hlen)
    masked_seed = bytes(a ^ b for a, b in zip(seed, seed_mask))
    return b"\x00" + masked_seed + masked_db


def oaep_unpad(padded: bytes, n_bytes: int) -> bytes:
    import hashlib
    hash_func = hashlib.sha1
    hlen = hash_func().digest_size

    if len(padded) != n_bytes:
        raise ValueError("Decryption error")
    if padded[0] != 0:
        raise ValueError("Decryption error")

    masked_seed = padded[1:1 + hlen]
    masked_db = padded[1 + hlen:]

    seed_mask = mgf1(masked_db, hlen)
    seed = bytes(a ^ b for a, b in zip(masked_seed, seed_mask))
    db_mask = mgf1(seed, n_bytes - hlen - 1)
    db = bytes(a ^ b for a, b in zip(masked_db, db_mask))

    label_hash = hash_func(b"").digest()
    if db[:hlen] != label_hash:
        raise ValueError("Decryption error")

    i = hlen
    while i < len(db) and db[i] == 0:
        i += 1
    if i >= len(db) or db[i] != 1:
        raise ValueError("Decryption error")

    return db[i + 1:]


def encrypt_oaep(pub_key: tuple, message: bytes) -> int:
    n = pub_key[0]
    n_bytes = (n.bit_length() + 7) // 8
    padded = oaep_pad(message, n_bytes)
    m = int.from_bytes(padded, "big")
    return encrypt(pub_key, m)


def decrypt_oaep(priv_key: tuple, ciphertext: int) -> bytes:
    n = priv_key[0]
    n_bytes = (n.bit_length() + 7) // 8
    padded_int = decrypt(priv_key, ciphertext)
    padded = padded_int.to_bytes(n_bytes, "big")
    return oaep_unpad(padded, n_bytes)


def main() -> None:
    print("=== RSA Key Generation (512-bit demo) ===")
    pub_key, priv_key = generate_keypair(bits=512)
    n, e, p, q = pub_key
    print(f"p (prime 1):        {p}")
    print(f"q (prime 2):        {q}")
    print(f"n (modulus):        {n}")
    print(f"e (public exp):     {e}")
    print(f"d (private exp):    {priv_key[1]}")
    print(f"Bit length:         {n.bit_length()}")

    print("\n=== Textbook RSA Encryption/Decryption ===")
    message_int = 42
    cipher = encrypt(pub_key, message_int)
    plain = decrypt(priv_key, cipher)
    print(f"Message:              {message_int}")
    print(f"Ciphertext:           {cipher}")
    print(f"Decrypted:            {plain}")
    print(f"Match:                {message_int == plain}")

    print("\n=== Textbook RSA is Deterministic ===")
    c1 = encrypt(pub_key, 42)
    c2 = encrypt(pub_key, 42)
    print(f"Encrypt(42) again:    {c2}")
    print(f"Same ciphertext:      {c1 == c2}  (BAD!)")

    print("\n=== Textbook RSA is Malleable ===")
    c = encrypt(pub_key, 100)
    modified = (c * pow(2, e, n)) % n
    decrypted = decrypt(priv_key, modified)
    print(f"Encrypt(100):         {c}")
    print(f"Modified ciphertext:  {modified}")
    print(f"Decrypted modified:   {decrypted} (should be 200)")

    print("\n=== RSA-OAEP Encryption ===")
    message = b"Hi, OAEP!"
    ct = encrypt_oaep(pub_key, message)
    pt = decrypt_oaep(priv_key, ct)
    print(f"Original message:     {message}")
    print(f"Ciphertext (int):     {ct}")
    print(f"Decrypted message:    {pt}")
    print(f"Match:                {message == pt}")

    print("\n=== RSA-OAEP is Non-Deterministic ===")
    ct2 = encrypt_oaep(pub_key, message)
    print(f"Same message:         {ct == ct2}  (should be False — OAEP uses random seed)")

    print("\n=== RSA-OAEP Tamper Detection ===")
    tampered = ct ^ 1
    try:
        decrypt_oaep(priv_key, tampered)
        print("Tampered ciphertext decrypted (BAD!)")
    except ValueError as e:
        print(f"Tampered ciphertext rejected: {e}")


if __name__ == "__main__":
    main()
