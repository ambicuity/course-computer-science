import hashlib
import hmac
import struct
import time

# ============================================================
# SHA-256 from scratch (educational)
# ============================================================

_K = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
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

_H0 = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
]

MASK32 = 0xFFFFFFFF


def _rotr32(x, n):
    return ((x >> n) | (x << (32 - n))) & MASK32


def _shr32(x, n):
    return x >> n


def _ch(x, y, z):
    return (x & y) ^ ((~x) & z) & MASK32


def _maj(x, y, z):
    return (x & y) ^ (x & z) ^ (y & z)


def _big_sigma0(x):
    return _rotr32(x, 2) ^ _rotr32(x, 13) ^ _rotr32(x, 22)


def _big_sigma1(x):
    return _rotr32(x, 6) ^ _rotr32(x, 11) ^ _rotr32(x, 25)


def _small_sigma0(x):
    return _rotr32(x, 7) ^ _rotr32(x, 18) ^ _shr32(x, 3)


def _small_sigma1(x):
    return _rotr32(x, 17) ^ _rotr32(x, 19) ^ _shr32(x, 10)


def _sha256_compress(state, block_bytes):
    W = list(struct.unpack('>16I', block_bytes))
    for i in range(16, 64):
        W.append((_small_sigma1(W[i - 2]) + W[i - 7] + _small_sigma0(W[i - 15]) + W[i - 16]) & MASK32)

    a, b, c, d, e, f, g, h = state

    for i in range(64):
        T1 = (h + _big_sigma1(e) + _ch(e, f, g) + _K[i] + W[i]) & MASK32
        T2 = (_big_sigma0(a) + _maj(a, b, c)) & MASK32
        h = g
        g = f
        f = e
        e = (d + T1) & MASK32
        d = c
        c = b
        b = a
        a = (T1 + T2) & MASK32

    return [(state[i] + v) & MASK32 for i, v in enumerate([a, b, c, d, e, f, g, h])]


def sha256_manual(data: bytes) -> bytes:
    if data is None:
        data = b""
    msg = bytearray(data)
    msg_len_bits = len(data) * 8

    msg.append(0x80)
    while len(msg) % 64 != 56:
        msg.append(0x00)
    msg += struct.pack('>Q', msg_len_bits)

    state = list(_H0)
    for i in range(0, len(msg), 64):
        state = _sha256_compress(state, bytes(msg[i:i + 64]))

    return b''.join(struct.pack('>I', s) for s in state)


# ============================================================
# SHA-3 / Keccak sponge (educational)
# ============================================================

_KECCAK_RC = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808A,
    0x8000000080008000, 0x000000000000808B, 0x0000000080000001,
    0x8000000080008081, 0x8000000000008009, 0x000000000000008A,
    0x0000000000000088, 0x0000000080008009, 0x000000008000000A,
    0x000000008000808B, 0x800000000000008B, 0x8000000000008089,
    0x8000000000008003, 0x8000000000008002, 0x8000000000000080,
    0x000000000000800A, 0x800000008000000A, 0x8000000080008081,
    0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
]

_KECCAK_RHO = [
    [0, 1, 62, 28, 27],
    [36, 44, 6, 55, 20],
    [3, 10, 43, 25, 39],
    [41, 45, 15, 21, 8],
    [18, 2, 61, 56, 14],
]

M64 = 0xFFFFFFFFFFFFFFFF


def _rot64(x, n):
    if n == 0:
        return x & M64
    return ((x << n) | (x >> (64 - n))) & M64


def _keccak_f(state):
    A = [[state[5 * y + x] for x in range(5)] for y in range(5)]

    for rnd in range(24):
        C = [A[0][x] ^ A[1][x] ^ A[2][x] ^ A[3][x] ^ A[4][x] for x in range(5)]
        D = [(C[(x - 1) % 5] ^ _rot64(C[(x + 1) % 5], 1)) & M64 for x in range(5)]
        for y in range(5):
            for x in range(5):
                A[y][x] = (A[y][x] ^ D[x]) & M64

        B = [[0] * 5 for _ in range(5)]
        for y in range(5):
            for x in range(5):
                B[(2 * x + 3 * y) % 5][y] = _rot64(A[y][x], _KECCAK_RHO[y][x])

        for y in range(5):
            for x in range(5):
                A[y][x] = (B[y][x] ^ ((~B[y][(x + 1) % 5]) & B[y][(x + 2) % 5])) & M64

        A[0][0] = (A[0][0] ^ _KECCAK_RC[rnd]) & M64

    return [A[y][x] for y in range(5) for x in range(5)]


def sha3_256_manual(data: bytes) -> bytes:
    rate = 1088 // 8
    state = [0] * 25

    msg = bytearray(data)
    msg.append(0x06)
    while len(msg) % rate != 0:
        msg.append(0x00)
    msg[-1] |= 0x80

    for i in range(0, len(msg), rate):
        block = msg[i:i + rate]
        for j in range(rate // 8):
            state[j] ^= struct.unpack('<Q', bytes(block[j * 8:j * 8 + 8]))[0]
        state = _keccak_f(state)

    return b''.join(struct.pack('<Q', state[i]) for i in range(4))


# ============================================================
# Birthday attack simulation — find collision in 24-bit hash
# ============================================================

def birthday_attack_simulation(bits=24, max_attempts=2000000):
    print(f"\nBirthday Attack Simulation ({bits}-bit truncated SHA-256)")
    print("=" * 55)
    mask = (1 << bits) - 1
    seen = {}
    for i in range(max_attempts):
        msg = f"message_{i}".encode()
        h = int(hashlib.sha256(msg).hexdigest(), 16)
        truncated = h >> (256 - bits)
        if truncated in seen:
            msg1 = seen[truncated]
            h1 = hashlib.sha256(msg1.encode()).hexdigest()
            h2 = hashlib.sha256(msg).hexdigest()
            print(f"  Collision found after {i + 1} evaluations!")
            print(f"  Expected (birthday bound): ~2^({bits // 2}) = {2 ** (bits // 2)}")
            print(f"  Messages:  m1=\"{msg1}\"  m2=\"message_{i}\"")
            print(f"  Full SHA-256(m1) = {h1}")
            print(f"  Full SHA-256(m2) = {h2}")
            print(f"  Top {bits} bits both = {truncated:0{bits // 4}x}")
            return i + 1
        seen[truncated] = f"message_{i}"
    print(f"  No collision found in {max_attempts} attempts")
    return None


# ============================================================
# Length extension attack demo
# ============================================================

def length_extension_demo():
    print("\nLength Extension Attack Demo")
    print("=" * 55)
    secret = b"SECRET_KEY"
    message = b"data_to_authenticate"
    extension = b"malicious_payload"

    h_secret_msg = hashlib.sha256(secret + message).digest()
    original_len = len(secret) + len(message)

    print(f"  Server computes H(secret || message):")
    print(f"    secret   = {secret}")
    print(f"    message  = {message}")
    print(f"    H(secret||message) = {h_secret_msg.hex()}")

    glue_padding_len = 64 - (original_len % 64)
    if glue_padding_len < 9:
        glue_padding_len += 64
    bit_len = original_len * 8
    glue = b'\x80' + b'\x00' * (glue_padding_len - 9) + struct.pack('>Q', bit_len)

    forged_msg = message + glue + extension
    h_forged = hashlib.sha256(secret + forged_msg).digest()

    print(f"\n  Attacker knows H(secret||message) and len(secret||message) = {original_len}")
    print(f"  Attacker constructs: message || glue_padding || extension")
    print(f"  Attacker uses H(secret||message) as IV and continues hashing")
    print(f"\n  Server would compute H(secret || message || glue || extension):")
    print(f"    = {h_forged.hex()}")
    print(f"\n  This is why raw H(key||message) is INSECURE for authentication.")
    print(f"  Use HMAC(key, message) instead — it defeats length extension.")
    h_mac = hmac.new(secret, message, 'sha256').hexdigest()
    print(f"  HMAC(key, message) = {h_mac}")
    print(f"  HMAC is immune to length extension because of the double-hash construction.")


# ============================================================
# BLAKE2b demo
# ============================================================

def blake2b_demo():
    print("\nBLAKE2b Demo")
    print("=" * 55)
    msg = b"BLAKE2 is faster than MD5 and more secure than SHA-2"
    h = hashlib.blake2b(msg).hexdigest()
    print(f"  Message: \"{msg.decode()}\"")
    print(f"  BLAKE2b: {h}")
    print(f"  Output: {len(h) * 4} bits ({len(h)} hex chars)")

    h_keyed = hashlib.blake2b(msg, key=b"my-secret-key").hexdigest()
    print(f"\n  Keyed BLAKE2b: {h_keyed}")
    print(f"  BLAKE2b acts as a MAC when a key is provided — built-in,")
    print(f"  unlike SHA-256 which requires HMAC wrapper for authentication.")

    h_short = hashlib.blake2b(msg, digest_size=16).hexdigest()
    print(f"\n  BLAKE2b-128 (16 bytes): {h_short}")
    print(f"  BLAKE2b supports variable output (1-64 bytes) without truncation")


# ============================================================
# Benchmark comparison
# ============================================================

def benchmark_comparison():
    print("\nBenchmark: SHA-256 vs SHA-3-256 vs BLAKE2b (1 MB)")
    print("=" * 55)
    data = b'\x00' * (1024 * 1024)
    iterations = 50

    for name, func in [
        ("SHA-256", lambda d: hashlib.sha256(d).digest()),
        ("SHA-3-256", lambda d: hashlib.sha3_256(d).digest()),
        ("BLAKE2b", lambda d: hashlib.blake2b(d).digest()),
    ]:
        start = time.perf_counter()
        for _ in range(iterations):
            func(data)
        elapsed = time.perf_counter() - start
        mb_per_sec = (iterations * len(data)) / (1024 * 1024) / elapsed
        print(f"  {name:12s}: {elapsed:.3f}s for {iterations}x1MB  ({mb_per_sec:.0f} MB/s)")

    print(f"\n  Tip: BLAKE2b is typically 2-3x faster than SHA-256 on modern CPUs")
    print(f"  because it uses ARX (Add-Rotate-XOR) operations that map well to")
    print(f"  modern SIMD instruction sets.")


# ============================================================
# SHA-256 and SHA-3 manual test vector verification
# ============================================================

def verify_manual_implementations():
    print("Manual Implementation Verification")
    print("=" * 55)

    h_empty = sha256_manual(b"").hex()
    expected_empty = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    print(f"  sha256_manual(\"\")    = {h_empty}")
    print(f"  expected              = {expected_empty}")
    print(f"  Match: {h_empty == expected_empty}")

    h_abc = sha256_manual(b"abc").hex()
    expected_abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    print(f"\n  sha256_manual(\"abc\") = {h_abc}")
    print(f"  expected              = {expected_abc}")
    print(f"  Match: {h_abc == expected_abc}")

    print(f"\n  hashlib.sha256(\"abc\").hexdigest() = {hashlib.sha256(b'abc').hexdigest()}")

    h_sha3 = sha3_256_manual(b"abc").hex()
    expected_sha3 = hashlib.sha3_256(b"abc").hexdigest()
    print(f"\n  sha3_256_manual(\"abc\") = {h_sha3}")
    print(f"  expected                = {expected_sha3}")
    print(f"  Match: {h_sha3 == expected_sha3}")


# ============================================================
# Hash function properties demonstration
# ============================================================

def demonstrate_properties():
    print("\nHash Function Properties")
    print("=" * 55)

    h1 = hashlib.sha256(b"hello").hexdigest()
    h2 = hashlib.sha256(b"hello!").hexdigest()
    print(f"  SHA-256('hello')  = {h1}")
    print(f"  SHA-256('hello!') = {h2}")
    print(f"  One character change → completely different hash (avalanche effect)")

    print(f"\n  Preimage resistance: given {h1[:16]}..., hard to find any input")
    print(f"  that hashes to this value (requires ~2^256 brute-force attempts)")

    print(f"\n  Second preimage resistance: given 'hello' and its hash,")
    print(f"  hard to find a DIFFERENT input with the SAME hash (requires ~2^256)")

    print(f"\n  Collision resistance: hard to find ANY two inputs with the same hash")
    print(f"  Birthday paradox reduces this to ~2^128 for SHA-256")
    print(f"  This is why SHA-256 has '128-bit security' for collisions, not 256-bit")


# ============================================================
# Main
# ============================================================

if __name__ == "__main__":
    print("=" * 55)
    print("  Hash Functions — SHA-2, SHA-3, BLAKE")
    print("  Phase 12 Lesson 06")
    print("=" * 55)

    verify_manual_implementations()
    demonstrate_properties()
    birthday_attack_simulation(bits=24)
    length_extension_demo()
    blake2b_demo()
    benchmark_comparison()

    print("\n" + "=" * 55)
    print("  Done. All demos complete.")
    print("=" * 55)