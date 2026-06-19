import struct
from collections import Counter


def otp_encrypt(plaintext: bytes, key: bytes) -> bytes:
    assert len(key) >= len(plaintext), "Key must be at least as long as plaintext"
    return bytes(p ^ k for p, k in zip(plaintext, key[:len(plaintext)]))


def otp_decrypt(ciphertext: bytes, key: bytes) -> bytes:
    return otp_encrypt(ciphertext, key)


def two_time_pad_attack(c1: bytes, c2: bytes) -> bytes:
    return bytes(a ^ b for a, b in zip(c1, c2))


class LFSR:
    def __init__(self, degree: int, taps: list[int], seed: int):
        self.degree = degree
        self.taps = taps
        self.state = seed & ((1 << degree) - 1)
        if self.state == 0:
            self.state = 1

    def step(self) -> int:
        feedback = 0
        for t in self.taps:
            feedback ^= (self.state >> (t - 1)) & 1
        output = self.state & 1
        self.state = (self.state >> 1) | (feedback << (self.degree - 1))
        return output

    def generate(self, n: int) -> list[int]:
        return [self.step() for _ in range(n)]

    def generate_bytes(self, n: int) -> bytes:
        bits = self.generate(n * 8)
        result = bytearray()
        for i in range(0, len(bits), 8):
            byte = 0
            for j in range(8):
                byte |= (bits[i + j] << (7 - j))
            result.append(byte)
        return bytes(result)


def berlekamp_massey_gf2(s: list[int]) -> tuple[int, list[int]]:
    n = len(s)
    c = [1]
    b = [1]
    l = 0
    m = 1
    for N in range(n):
        d = s[N]
        for i in range(1, l + 1):
            d ^= c[i] & s[N - i]
        if d == 0:
            m += 1
        else:
            t = c[:]
            pad = [0] * (1 + max(0, len(b) + m - len(c)))
            c.extend(pad)
            for i in range(len(b)):
                c[i + m] ^= b[i]
            if 2 * l <= N:
                l = N + 1 - l
                b = t
                m = 1
            else:
                m += 1
    return l, c



class RC4:
    def __init__(self, key: bytes):
        self.S = list(range(256))
        j = 0
        for i in range(256):
            j = (j + self.S[i] + key[i % len(key)]) % 256
            self.S[i], self.S[j] = self.S[j], self.S[i]
        self.i = 0
        self.j = 0

    def generate_byte(self) -> int:
        self.i = (self.i + 1) % 256
        self.j = (self.j + self.S[self.i]) % 256
        self.S[self.i], self.S[self.j] = self.S[self.j], self.S[self.i]
        k = (self.S[self.i] + self.S[self.j]) % 256
        return self.S[k]

    def generate(self, n: int) -> bytes:
        return bytes(self.generate_byte() for _ in range(n))


def rc4_bias_analysis(num_keys: int = 10000, key_length: int = 16) -> dict:
    position_counts = {}
    for pos in range(256):
        position_counts[pos] = Counter()

    for _ in range(num_keys):
        key = bytes([_ % 256] * key_length)
        rc4 = RC4(key)
        stream = rc4.generate(256)
        for pos, byte_val in enumerate(stream):
            position_counts[pos][byte_val] += 1

    biases = {}
    for pos in range(4):
        zero_count = position_counts[pos][0]
        expected = num_keys / 256.0
        biases[pos] = {
            "zero_count": zero_count,
            "expected": expected,
            "ratio": zero_count / expected,
        }
    return biases


def chacha20_quarter_round(a: int, b: int, c: int, d: int) -> tuple[int, int, int, int]:
    def rotl32(v: int, n: int) -> int:
        return ((v << n) | (v >> (32 - n))) & 0xFFFFFFFF

    a = (a + b) & 0xFFFFFFFF; d = rotl32(d ^ a, 16)
    c = (c + d) & 0xFFFFFFFF; b = rotl32(b ^ c, 12)
    a = (a + b) & 0xFFFFFFFF; d = rotl32(d ^ a, 8)
    c = (c + d) & 0xFFFFFFFF; b = rotl32(b ^ c, 7)
    return a, b, c, d


def chacha20_quarter_round_trace(a: int, b: int, c: int, d: int) -> list[tuple[str, tuple[int, int, int, int]]]:
    def rotl32(v: int, n: int) -> int:
        return ((v << n) | (v >> (32 - n))) & 0xFFFFFFFF

    trace = []
    trace.append(("initial", (a, b, c, d)))

    a = (a + b) & 0xFFFFFFFF
    d = rotl32(d ^ a, 16)
    trace.append(("a+=b; d^=a; d<<<=16", (a, b, c, d)))

    c = (c + d) & 0xFFFFFFFF
    b = rotl32(b ^ c, 12)
    trace.append(("c+=d; b^=c; b<<<=12", (a, b, c, d)))

    a = (a + b) & 0xFFFFFFFF
    d = rotl32(d ^ a, 8)
    trace.append(("a+=b; d^=a; d<<<=8", (a, b, c, d)))

    c = (c + d) & 0xFFFFFFFF
    b = rotl32(b ^ c, 7)
    trace.append(("c+=d; b^=c; b<<<=7", (a, b, c, d)))

    return trace


def demo_otp():
    print("=" * 60)
    print("ONE-TIME PAD")
    print("=" * 60)

    plaintext = b"HELLO WORLD!!!!"
    key = bytes([0x4A, 0x3B, 0x2C, 0x1D, 0x0E, 0xFF, 0xEE, 0xDD,
                 0xCC, 0xBB, 0xAA, 0x99, 0x88, 0x77, 0x66, 0x55])
    ciphertext = otp_encrypt(plaintext, key)
    decrypted = otp_decrypt(ciphertext, key)

    print(f"  Plaintext:  {plaintext}")
    print(f"  Key:        {key.hex()}")
    print(f"  Ciphertext: {ciphertext.hex()}")
    print(f"  Decrypted:  {decrypted}")
    print(f"  Roundtrip:   {decrypted == plaintext}")

    print()
    print("  TWO-TIME PAD ATTACK")
    p1 = b"Attack at dawn!!"
    p2 = b"Defend at dusk!!"
    k = bytes(range(16))
    c1 = otp_encrypt(p1, k)
    c2 = otp_encrypt(p2, k)
    xored = two_time_pad_attack(c1, c2)
    print(f"  P1:  {p1}")
    print(f"  P2:  {p2}")
    print(f"  C1:  {c1.hex()}")
    print(f"  C2:  {c2.hex()}")
    print(f"  C1^C2 = P1^P2: {xored.hex()}")
    p1_xor_p2 = bytes(a ^ b for a, b in zip(p1, p2))
    print(f"  P1^P2 (direct): {p1_xor_p2.hex()}")
    print(f"  Match: {xored == p1_xor_p2}")


def demo_lfsr():
    print()
    print("=" * 60)
    print("LFSR")
    print("=" * 60)

    degree = 8
    taps = [8, 6, 5, 4]
    seed = 0b10110110
    lfsr = LFSR(degree, taps, seed)

    output = lfsr.generate(24)
    print(f"  Degree:         {degree}")
    print(f"  Taps:           x^{taps[0]} + x^{taps[1]} + x^{taps[2]} + x^{taps[3]} + 1")
    print(f"  Seed:           {seed:0{degree}b}")
    print(f"  First 24 bits:  {''.join(str(b) for b in output)}")

    expected_period = (1 << degree) - 1
    lfsr2 = LFSR(degree, taps, seed)
    long_output = lfsr2.generate(expected_period + 10)
    period = len(long_output) - len(long_output[10:])
    state = seed
    count = 0
    lfsr3 = LFSR(degree, taps, seed)
    for _ in range(expected_period + 1):
        lfsr3.step()
        count += 1
    lfsr4 = LFSR(degree, taps, seed)
    observed = lfsr4.generate(expected_period)
    lfsr5 = LFSR(degree, taps, seed)
    next_bits = lfsr5.generate(expected_period + 4)
    repeats = next_bits[0:4] == observed[0:4]
    print(f"  Period:         {expected_period} (m-sequence verified: {repeats})")

    print()
    print("  BERLEKAMP-MASSEY ATTACK")
    lfsr_bm = LFSR(degree, taps, seed)
    observed_bits = lfsr_bm.generate(2 * degree)

    l_recovered, c_poly = berlekamp_massey_gf2(observed_bits)
    recurrence_taps = [i for i in range(1, len(c_poly)) if c_poly[i]]
    print(f"  Observed:           {2 * degree} bits")
    print(f"  Original LFSR:      degree={degree}, taps={taps}")
    print(f"  Recovered L:        {l_recovered}")
    print(f"  Recurrence taps:    {recurrence_taps}")
    print(f"  (s[n] = s[n-1] + s[n-3] + s[n-4] + s[n-5]  mod 2)")

    predicted = list(observed_bits)
    test_len = 64
    for n in range(len(predicted), test_len):
        bit = 0
        for i in range(1, l_recovered + 1):
            if i < len(c_poly):
                bit ^= c_poly[i] & predicted[n - i]
        predicted.append(bit)

    lfsr_verify = LFSR(degree, taps, seed)
    actual = lfsr_verify.generate(test_len)
    match_after = predicted[2*degree:test_len] == actual[2*degree:test_len]
    print(f"  Prediction beyond observation window: {match_after}")
    print(f"  ({2*degree} bits observed, correctly predicted bits {2*degree}..{test_len-1})")


def demo_rc4():
    print()
    print("=" * 60)
    print("RC4")
    print("=" * 60)

    key = b"SecretKey"
    rc4 = RC4(key)
    stream = rc4.generate(32)
    print(f"  Key:            {key}")
    print(f"  First 32 bytes: {stream.hex()}")

    plaintext = b"Hello, RC4 cipher!"
    rc4_enc = RC4(key)
    ciphertext = bytes(p ^ k for p, k in zip(plaintext, rc4_enc.generate(len(plaintext))))
    rc4_dec = RC4(key)
    decrypted = bytes(c ^ k for c, k in zip(ciphertext, rc4_dec.generate(len(ciphertext))))
    print(f"  Plaintext:      {plaintext}")
    print(f"  Ciphertext:     {ciphertext.hex()}")
    print(f"  Decrypted:      {decrypted}")
    print(f"  Roundtrip:      {decrypted == plaintext}")

    print()
    print("  RC4 BIAS ANALYSIS (sampling first 4 positions)")
    print("  Using sequential keys for deterministic demo...")
    num_trials = 10000
    key_len = 16
    pos_counts = [Counter() for _ in range(4)]

    for trial in range(num_trials):
        key = bytes([(trial >> i) & 0xFF for i in range(key_len)])
        rc4 = RC4(key)
        stream = rc4.generate(4)
        for pos in range(4):
            pos_counts[pos][stream[pos]] += 1

    for pos in range(4):
        zero_count = pos_counts[pos][0]
        expected = num_trials / 256.0
        ratio = zero_count / expected
        indicator = " *** BIAS DETECTED ***" if ratio > 1.5 else ""
        print(f"  Position {pos}: zero_count={zero_count}, expected={expected:.1f}, ratio={ratio:.3f}{indicator}")

    print("  (Position 1 ratio is typically ~2.0 with random keys)")
    print("  (Fluhrer-Mantin-Shamir proved position 1 bias = 2/256)")


def demo_chacha20_quarter_round():
    print()
    print("=" * 60)
    print("CHACHA20 QUARTER ROUND")
    print("=" * 60)

    a, b, c, d = 0x11111111, 0x01020304, 0x9b8d6f43, 0x01234567
    print(f"  Input:  a=0x{a:08X}  b=0x{b:08X}  c=0x{c:08X}  d=0x{d:08X}")

    trace = chacha20_quarter_round_trace(a, b, c, d)
    for label, (va, vb, vc, vd) in trace:
        print(f"  {label:30s}  a=0x{va:08X}  b=0x{vb:08X}  c=0x{vc:08X}  d=0x{vd:08X}")

    result = chacha20_quarter_round(a, b, c, d)
    print(f"  Output: a=0x{result[0]:08X}  b=0x{result[1]:08X}  c=0x{result[2]:08X}  d=0x{result[3]:08X}")


def demo_ctr_mode():
    print()
    print("=" * 60)
    print("CTR MODE (conceptual)")
    print("=" * 60)

    print("  CTR mode turns a block cipher E_K into a stream cipher:")
    print("  keystream[i] = E_K(counter + i)")
    print("  ciphertext  = plaintext XOR keystream")
    print()
    print("  Properties:")
    print("  - Parallelizable: each block is independent")
    print("  - Seekable: decrypt block N without blocks 0..N-1")
    print("  - No authentication: needs MAC (or use ChaCha20-Poly1305)")
    print()
    print("  Python pseudo-code:")
    print("    def ctr_encrypt(block_cipher, key, nonce, plaintext):")
    print("        keystream = b''")
    print("        for counter in range(ceil(len(plaintext) / block_size)):")
    print("            block = nonce + counter.to_bytes(8, 'big')")
    print("            keystream += block_cipher(key, block)")
    print("        return xor(plaintext, keystream[:len(plaintext)])")


def main():
    demo_otp()
    demo_lfsr()
    demo_rc4()
    demo_chacha20_quarter_round()
    demo_ctr_mode()


if __name__ == "__main__":
    main()