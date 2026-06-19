import os
import struct
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
from cryptography.hazmat.primitives import padding as sym_padding

BLOCK_SIZE = 16


def aes_encrypt_block(key: bytes, block: bytes) -> bytes:
    assert len(key) in (16, 24, 32)
    assert len(block) == BLOCK_SIZE
    cipher = Cipher(algorithms.AES(key), modes.ECB())
    enc = cipher.encryptor()
    return enc.update(block) + enc.finalize()


def aes_decrypt_block(key: bytes, block: bytes) -> bytes:
    assert len(key) in (16, 24, 32)
    assert len(block) == BLOCK_SIZE
    cipher = Cipher(algorithms.AES(key), modes.ECB())
    dec = cipher.decryptor()
    return dec.update(block) + dec.finalize()


def pkcs7_pad(data: bytes) -> bytes:
    padder = sym_padding.PKCS7(BLOCK_SIZE * 8).padder()
    return padder.update(data) + padder.finalize()


def pkcs7_unpad(data: bytes) -> bytes:
    unpadder = sym_padding.PKCS7(BLOCK_SIZE * 8).unpadder()
    return unpadder.update(data) + unpadder.finalize()


def xor_bytes(a: bytes, b: bytes) -> bytes:
    return bytes(x ^ y for x, y in zip(a, b))


# --- ECB Mode ---

def ecb_encrypt(key: bytes, plaintext: bytes) -> bytes:
    padded = pkcs7_pad(plaintext)
    ciphertext = b""
    for i in range(0, len(padded), BLOCK_SIZE):
        ciphertext += aes_encrypt_block(key, padded[i:i + BLOCK_SIZE])
    return ciphertext


def ecb_decrypt(key: bytes, ciphertext: bytes) -> bytes:
    plaintext = b""
    for i in range(0, len(ciphertext), BLOCK_SIZE):
        plaintext += aes_decrypt_block(key, ciphertext[i:i + BLOCK_SIZE])
    return pkcs7_unpad(plaintext)


# --- CBC Mode ---

def cbc_encrypt(key: bytes, iv: bytes, plaintext: bytes) -> bytes:
    assert len(iv) == BLOCK_SIZE
    padded = pkcs7_pad(plaintext)
    ciphertext = b""
    prev = iv
    for i in range(0, len(padded), BLOCK_SIZE):
        block = xor_bytes(padded[i:i + BLOCK_SIZE], prev)
        encrypted = aes_encrypt_block(key, block)
        ciphertext += encrypted
        prev = encrypted
    return ciphertext


def cbc_decrypt(key: bytes, iv: bytes, ciphertext: bytes) -> bytes:
    assert len(iv) == BLOCK_SIZE
    plaintext = b""
    prev = iv
    for i in range(0, len(ciphertext), BLOCK_SIZE):
        decrypted = aes_decrypt_block(key, ciphertext[i:i + BLOCK_SIZE])
        plaintext += xor_bytes(decrypted, prev)
        prev = ciphertext[i:i + BLOCK_SIZE]
    return pkcs7_unpad(plaintext)


# --- CTR Mode ---

def ctr_encrypt(key: bytes, nonce: bytes, plaintext: bytes) -> bytes:
    assert len(nonce) == 12
    ciphertext = b""
    counter = 0
    for i in range(0, len(plaintext), BLOCK_SIZE):
        ctr_block = nonce + struct.pack(">I", counter)
        keystream = aes_encrypt_block(key, ctr_block)
        chunk = plaintext[i:i + BLOCK_SIZE]
        ciphertext += xor_bytes(chunk, keystream[:len(chunk)])
        counter += 1
    return ciphertext


def ctr_decrypt(key: bytes, nonce: bytes, ciphertext: bytes) -> bytes:
    return ctr_encrypt(key, nonce, ciphertext)


# --- GCM Mode ---

def gcm_ctr_counter(nonce: bytes) -> bytes:
    return nonce + b"\x00\x00\x00\x01"


def inc32(block: bytes) -> bytes:
    val = int.from_bytes(block[12:], "big")
    return block[:12] + (val + 1).to_bytes(4, "big")


def gcm_ctr_encrypt(key: bytes, nonce: bytes, plaintext: bytes) -> bytes:
    counter_block = gcm_ctr_counter(nonce)
    ciphertext = b""
    for i in range(0, len(plaintext), BLOCK_SIZE):
        keystream = aes_encrypt_block(key, counter_block)
        chunk = plaintext[i:i + BLOCK_SIZE]
        ciphertext += xor_bytes(chunk, keystream[:len(chunk)])
        counter_block = inc32(counter_block)
    return ciphertext


def gf128_mul(x: int, y: int) -> int:
    R = 0xe1 << 120
    z = 0
    v = y
    for i in range(127, -1, -1):
        if (x >> i) & 1:
            z ^= v
        carry = v & 1
        v >>= 1
        if carry:
            v ^= R
    return z


def gh(h: int, aad: bytes, ciphertext: bytes) -> int:
    def to_blocks(data: bytes) -> list[int]:
        if len(data) == 0:
            return []
        blocks = []
        for i in range(0, len(data), BLOCK_SIZE):
            chunk = data[i:i + BLOCK_SIZE]
            if len(chunk) < BLOCK_SIZE:
                chunk = chunk + b"\x00" * (BLOCK_SIZE - len(chunk))
            blocks.append(int.from_bytes(chunk, "big"))
        return blocks

    y = 0
    for block in to_blocks(aad):
        y = gf128_mul(y ^ block, h)
    for block in to_blocks(ciphertext):
        y = gf128_mul(y ^ block, h)
    len_block = (len(aad) * 8).to_bytes(8, "big") + (len(ciphertext) * 8).to_bytes(8, "big")
    y = gf128_mul(y ^ int.from_bytes(len_block, "big"), h)
    return y


def gcm_encrypt(key: bytes, nonce: bytes, plaintext: bytes, aad: bytes = b"") -> tuple[bytes, bytes]:
    assert len(nonce) == 12
    h = int.from_bytes(aes_encrypt_block(key, b"\x00" * BLOCK_SIZE), "big")
    ciphertext = gcm_ctr_encrypt(key, nonce, plaintext)
    j0 = int.from_bytes(gcm_ctr_counter(nonce), "big")
    s = gh(h, aad, ciphertext)
    e_j0 = int.from_bytes(aes_encrypt_block(key, gcm_ctr_counter(nonce)), "big")
    tag = (s ^ e_j0).to_bytes(BLOCK_SIZE, "big")
    return ciphertext, tag


def gcm_decrypt(key: bytes, nonce: bytes, ciphertext: bytes, tag: bytes, aad: bytes = b"") -> bytes | None:
    assert len(nonce) == 12
    assert len(tag) == BLOCK_SIZE
    h = int.from_bytes(aes_encrypt_block(key, b"\x00" * BLOCK_SIZE), "big")
    j0 = int.from_bytes(gcm_ctr_counter(nonce), "big")
    s = gh(h, aad, ciphertext)
    e_j0 = int.from_bytes(aes_encrypt_block(key, gcm_ctr_counter(nonce)), "big")
    computed_tag = (s ^ e_j0).to_bytes(BLOCK_SIZE, "big")
    if computed_tag != tag:
        return None
    return gcm_ctr_encrypt(key, nonce, ciphertext)


# --- ECB Penguin Demo ---

def demo_ecb_penguin():
    print("=" * 60)
    print("ECB Penguin Demo: Pattern Preservation")
    print("=" * 60)
    key = os.urandom(16)
    width, height = 8, 8
    black = b"\x00" * BLOCK_SIZE
    white = b"\xFF" * BLOCK_SIZE
    orange = b"\x80" * BLOCK_SIZE
    tux = [
        [black, black, black, white, white, black, black, black],
        [black, black, white, white, white, white, black, black],
        [black, white, white, orange, orange, white, white, black],
        [black, white, orange, white, white, orange, white, black],
        [black, white, white, white, white, white, white, black],
        [black, black, white, white, white, white, black, black],
        [black, black, black, white, white, black, black, black],
        [black, black, black, black, black, black, black, black],
    ]
    color_map = {0: "B", 1: "W", 2: "O"}
    ecb_encrypted = {}
    ecb_grid = []
    for row in tux:
        encrypted_row = []
        for pixel in row:
            ct = aes_encrypt_block(key, pixel)
            if ct not in ecb_encrypted:
                ecb_encrypted[pixel] = ct
            short = ct[:2].hex()
            encrypted_row.append(short)
        ecb_grid.append(encrypted_row)
    ecb_lookup = {}
    reverse_lookup = {}
    idx = 0
    symbols = "XYZABCDEFGHIJKLMNOPQRSTUVW"
    for pixel_val, name in [(black, "B"), (white, "W"), (orange, "O")]:
        ct = ecb_encrypted[pixel_val]
        if ct not in reverse_lookup:
            reverse_lookup[ct] = symbols[idx]
            idx += 1
    print("\nOriginal (3 colors: B=black, W=white, O=orange):")
    for row in tux:
        line = " ".join("B" if p == black else "W" if p == white else "O" for p in row)
        print(f"  {line}")
    print(f"\nECB encrypted (same plaintext block -> same ciphertext):")
    for row in tux:
        line = " ".join(reverse_lookup[aes_encrypt_block(key, p)] for p in row)
        print(f"  {line}")
    print(f"\nPattern preserved! {symbols[0]}=black, {symbols[1]}=white, {symbols[2]}=orange")
    print("ECB reveals the image structure without decrypting.")


# --- Padding Oracle Attack ---

def padding_oracle_attack(key: bytes, iv: bytes, ciphertext: bytes) -> bytes:
    def has_valid_padding(prev_block: bytes, target_block: bytes) -> bool:
        decrypted = aes_decrypt_block(key, target_block)
        plaintext = xor_bytes(decrypted, prev_block)
        try:
            pkcs7_unpad(plaintext + b"\x00")
            return plaintext[-1] != 0 or all(
                plaintext[-(i + 1)] == len(plaintext) - (len(plaintext) - plaintext[-1])
                for i in range(plaintext[-1])
            ) if plaintext[-1] != 0 else False
        except Exception:
            return False

    def oracle(prev_block: bytes, target_block: bytes) -> bool:
        decrypted = aes_decrypt_block(key, target_block)
        plaintext = xor_bytes(decrypted, prev_block)
        try:
            pkcs7_unpad(plaintext)
            return True
        except Exception:
            return False

    num_blocks = len(ciphertext) // BLOCK_SIZE
    blocks = [iv] + [ciphertext[i:i + BLOCK_SIZE] for i in range(0, len(ciphertext), BLOCK_SIZE)]
    recovered = b""
    for block_idx in range(1, len(blocks)):
        prev = blocks[block_idx - 1]
        target = blocks[block_idx]
        intermediate = bytearray(BLOCK_SIZE)
        for byte_pos in range(BLOCK_SIZE - 1, -1, -1):
            padding_val = BLOCK_SIZE - byte_pos
            crafted = bytearray(prev)
            for k in range(byte_pos + 1, BLOCK_SIZE):
                crafted[k] = intermediate[k] ^ padding_val
            found = False
            for guess in range(256):
                crafted[byte_pos] = guess
                if oracle(bytes(crafted), target):
                    if byte_pos == BLOCK_SIZE - 1:
                        test = bytearray(crafted)
                        test[byte_pos - 1] ^= 1
                        if not oracle(bytes(test), target):
                            continue
                    intermediate[byte_pos] = guess ^ padding_val
                    found = True
                    break
            if not found:
                raise RuntimeError(f"Padding oracle attack failed at byte {byte_pos}")
        recovered += xor_bytes(aes_decrypt_block(key, target), prev)
    return recovered


def demo_padding_oracle():
    print("\n" + "=" * 60)
    print("Padding Oracle Attack Simulation")
    print("=" * 60)
    key = os.urandom(16)
    iv = os.urandom(16)
    plaintext = b"Secret message!!"
    assert len(plaintext) == 16
    ciphertext = cbc_encrypt(key, iv, plaintext)
    print(f"\nOriginal plaintext: {plaintext}")
    print(f"Ciphertext: {ciphertext.hex()}")
    print(f"Attacking (no knowledge of key, only padding oracle)...")
    recovered = padding_oracle_attack(key, iv, ciphertext)
    print(f"Recovered plaintext: {recovered}")
    print("Attack succeeded without ever knowing the key!")


# --- Nonce Reuse Demo ---

def demo_nonce_reuse():
    print("\n" + "=" * 60)
    print("CTR Nonce Reuse: Two-Time Pad")
    print("=" * 60)
    key = os.urandom(16)
    nonce = b"\x00" * 12
    msg1 = b"Attack at dawn!!" + b"\x10" * 16
    msg2 = b"Defend at noon!!" + b"\x10" * 16
    ct1 = ctr_encrypt(key, nonce, pkcs7_pad(b"Attack at dawn!!"))
    ct2 = ctr_encrypt(key, nonce, pkcs7_pad(b"Defend at noon!!"))
    xor_ct = xor_bytes(ct1, ct2)
    xor_pt = xor_bytes(b"Attack at dawn!!" + b"\x10" * 16, b"Defend at noon!!" + b"\x10" * 16)
    print(f"\nMessage 1: {msg1!r}")
    print(f"Message 2: {msg2!r}")
    print(f"C1 XOR C2 = {xor_ct.hex()[:32]}...")
    print(f"P1 XOR P2 = {xor_pt.hex()[:32]}...")
    print(f"XOR of ciphertexts == XOR of plaintexts: {xor_ct == xor_pt}")
    print("Keystream cancelled! Attacker can crib-drag to recover both messages.")


# --- Mode Comparison Demo ---

def demo_mode_comparison():
    print("\n" + "=" * 60)
    print("Mode Comparison: ECB vs CBC vs CTR vs GCM")
    print("=" * 60)
    key = os.urandom(16)
    iv = os.urandom(16)
    nonce = os.urandom(12)
    message = b"Repeat pattern: " + b"Repeat pattern: " + b"AAAA AAAA AAAA "
    print(f"\nMessage: {message}")
    print(f"Message contains repeated patterns (16-byte block repeated)\n")
    ecb_ct = ecb_encrypt(key, message)
    print(f"ECB ciphertext:")
    for i in range(0, len(ecb_ct), BLOCK_SIZE):
        print(f"  Block {i // BLOCK_SIZE}: {ecb_ct[i:i + BLOCK_SIZE].hex()}")
    identical_blocks = 0
    for i in range(0, len(ecb_ct) - BLOCK_SIZE, BLOCK_SIZE):
        if ecb_ct[i:i + BLOCK_SIZE] == ecb_ct[i + BLOCK_SIZE:i + 2 * BLOCK_SIZE]:
            identical_blocks += 1
    print(f"  Identical consecutive blocks: {identical_blocks} (patterns leaked!)")
    cbc_ct = cbc_encrypt(key, iv, message)
    print(f"\nCBC ciphertext:")
    for i in range(0, len(cbc_ct), BLOCK_SIZE):
        print(f"  Block {i // BLOCK_SIZE}: {cbc_ct[i:i + BLOCK_SIZE].hex()}")
    identical_blocks_cbc = 0
    for i in range(0, len(cbc_ct) - BLOCK_SIZE, BLOCK_SIZE):
        if cbc_ct[i:i + BLOCK_SIZE] == cbc_ct[i + BLOCK_SIZE:i + 2 * BLOCK_SIZE]:
            identical_blocks_cbc += 1
    print(f"  Identical consecutive blocks: {identical_blocks_cbc} (patterns hidden)")
    ct_ct = ctr_encrypt(key, nonce, message)
    print(f"\nCTR ciphertext:")
    for i in range(0, len(ct_ct), BLOCK_SIZE):
        print(f"  Block {i // BLOCK_SIZE}: {ct_ct[i:i + BLOCK_SIZE].hex()}")
    identical_blocks_ctr = 0
    for i in range(0, len(ct_ct) - BLOCK_SIZE, BLOCK_SIZE):
        if ct_ct[i:i + BLOCK_SIZE] == ct_ct[i + BLOCK_SIZE:i + 2 * BLOCK_SIZE]:
            identical_blocks_ctr += 1
    print(f"  Identical consecutive blocks: {identical_blocks_ctr} (patterns hidden)")
    gcm_ct, gcm_tag = gcm_encrypt(key, nonce, message)
    print(f"\nGCM ciphertext:")
    for i in range(0, len(gcm_ct), BLOCK_SIZE):
        print(f"  Block {i // BLOCK_SIZE}: {gcm_ct[i:i + BLOCK_SIZE].hex()}")
    print(f"  Auth tag: {gcm_tag.hex()}")
    identical_blocks_gcm = 0
    for i in range(0, len(gcm_ct) - BLOCK_SIZE, BLOCK_SIZE):
        if gcm_ct[i:i + BLOCK_SIZE] == gcm_ct[i + BLOCK_SIZE:i + 2 * BLOCK_SIZE]:
            identical_blocks_gcm += 1
    print(f"  Identical consecutive blocks: {identical_blocks_gcm} (patterns hidden)")
    print(f"\nEncryption + decryption verification:")
    print(f"  ECB decrypt: {ecb_decrypt(key, ecb_ct)[:len(message)]}")
    print(f"  CBC decrypt: {cbc_decrypt(key, iv, cbc_ct)[:len(message)]}")
    print(f"  CTR decrypt: {ctr_decrypt(key, nonce, ct_ct)[:len(message)]}")
    gcm_pt = gcm_decrypt(key, nonce, gcm_ct, gcm_tag)
    print(f"  GCM decrypt: {gcm_pt[:len(message)] if gcm_pt else 'TAG MISMATCH'}")
    print(f"\nGCM tag verification (tampered ciphertext):")
    tampered = bytearray(gcm_ct)
    tampered[0] ^= 0xFF
    result = gcm_decrypt(key, nonce, bytes(tampered), gcm_tag)
    print(f"  Result: {'REJECTED (tag mismatch)' if result is None else 'ACCEPTED (BUG!)'}")


# --- IV Effect Demo ---

def demo_iv_effect():
    print("\n" + "=" * 60)
    print("CBC IV Effect: Same Message, Different IV")
    print("=" * 60)
    key = os.urandom(16)
    message = b"Hello, world! Th"
    assert len(message) == 16
    iv1 = b"\x00" * 16
    iv2 = b"\x00" * 15 + b"\x01"
    ct1 = cbc_encrypt(key, iv1, message)
    ct2 = cbc_encrypt(key, iv2, message)
    print(f"\nMessage:  {message!r}")
    print(f"IV 1:    {iv1.hex()} (all zeros)")
    print(f"IV 2:    {iv2.hex()} (last byte differs)")
    print(f"CT 1:    {ct1.hex()}")
    print(f"CT 2:    {ct2.hex()}")
    print(f"Same?    {ct1 == ct2}")
    print("Even a 1-bit IV change produces completely different ciphertext.")


def main():
    print("Modes of Operation: ECB, CBC, CTR, GCM")
    print("=" * 60)
    demo_ecb_penguin()
    demo_iv_effect()
    demo_nonce_reuse()
    demo_mode_comparison()
    demo_padding_oracle()


if __name__ == "__main__":
    main()