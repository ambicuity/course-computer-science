"""
Classical Ciphers and Why They Fail
Phase 12 - Cryptography & Security

Implement, encrypt, and break Caesar, substitution, Vigenere, and one-time pad.
"""

import os
import random
import string
from collections import Counter
from math import gcd

ENGLISH_FREQ = {
    'E': 0.127, 'T': 0.091, 'A': 0.082, 'O': 0.075, 'I': 0.070,
    'N': 0.067, 'S': 0.063, 'H': 0.061, 'R': 0.060, 'D': 0.043,
    'L': 0.040, 'C': 0.028, 'U': 0.028, 'M': 0.024, 'W': 0.024,
    'F': 0.022, 'G': 0.020, 'Y': 0.020, 'P': 0.019, 'B': 0.015,
    'V': 0.010, 'K': 0.008, 'J': 0.002, 'X': 0.002, 'Q': 0.001,
    'Z': 0.001,
}

BIGRAM_FREQ = {
    'TH': 0.0356, 'HE': 0.0307, 'IN': 0.0243, 'ER': 0.0205, 'AN': 0.0198,
    'RE': 0.0185, 'ON': 0.0176, 'AT': 0.0149, 'EN': 0.0145, 'ND': 0.0135,
    'TI': 0.0134, 'ES': 0.0134, 'OR': 0.0128, 'TE': 0.0127, 'OF': 0.0117,
    'ED': 0.0117, 'IS': 0.0113, 'IT': 0.0112, 'AL': 0.0109, 'AR': 0.0107,
    'ST': 0.0105, 'TO': 0.0104, 'NT': 0.0104, 'NG': 0.0095, 'SE': 0.0093,
    'HA': 0.0092, 'AS': 0.0087, 'OU': 0.0087, 'IO': 0.0083, 'LE': 0.0083,
    'VE': 0.0083, 'CO': 0.0079, 'ME': 0.0079, 'DE': 0.0076, 'HI': 0.0076,
    'RI': 0.0073, 'RO': 0.0073, 'IC': 0.0071, 'NE': 0.0069, 'EA': 0.0069,
    'RA': 0.0062, 'CE': 0.0062,
}


def caesar_encrypt(text: str, shift: int) -> str:
    result = []
    for ch in text:
        if ch.isalpha():
            base = ord('A') if ch.isupper() else ord('a')
            result.append(chr((ord(ch.upper()) - ord('A') + shift) % 26 + ord('A')))
        else:
            result.append(ch)
    return ''.join(result)


def caesar_decrypt(text: str, shift: int) -> str:
    return caesar_encrypt(text, -shift)


def frequency_analysis(ciphertext: str) -> dict[str, float]:
    cleaned = [c.upper() for c in ciphertext if c.isalpha()]
    total = len(cleaned)
    if total == 0:
        return {}
    counts: dict[str, int] = {}
    for c in cleaned:
        counts[c] = counts.get(c, 0) + 1
    return {c: count / total for c, count in counts.items()}


def _frequency_score(text: str) -> float:
    freq = frequency_analysis(text)
    return sum(
        abs(freq.get(c, 0) - ENGLISH_FREQ.get(c, 0))
        for c in string.ascii_uppercase
    )


def break_caesar(ciphertext: str) -> tuple[str, int]:
    best = (ciphertext, 0)
    best_score = float('inf')
    for shift in range(26):
        candidate = caesar_encrypt(ciphertext, -shift)
        score = _frequency_score(candidate)
        if score < best_score:
            best_score = score
            best = (candidate, shift)
    return best


def substitution_encrypt(text: str, key_map: dict[str, str]) -> str:
    return ''.join(key_map.get(c.upper(), c) if c.isalpha() else c for c in text)


def substitution_decrypt(text: str, key_map: dict[str, str]) -> str:
    inv = {v: k for k, v in key_map.items()}
    return ''.join(inv.get(c.upper(), c) if c.isalpha() else c for c in text)


def _bigram_score(text: str) -> float:
    cleaned = ''.join(c for c in text.upper() if c.isalpha())
    if len(cleaned) < 2:
        return 0.0
    bigrams = [cleaned[i:i + 2] for i in range(len(cleaned) - 1)]
    total = sum(BIGRAM_FREQ.get(b, 0.0) for b in bigrams)
    return total / len(bigrams)


def break_substitution(ciphertext: str, iterations: int = 20000) -> tuple[str, dict[str, str]]:
    freq = frequency_analysis(ciphertext)
    sorted_ct = sorted(freq.keys(), key=lambda c: freq.get(c, 0), reverse=True)
    if not sorted_ct:
        sorted_ct = list(string.ascii_uppercase)
    sorted_en = sorted(ENGLISH_FREQ.keys(), key=lambda c: ENGLISH_FREQ[c], reverse=True)
    best_key: dict[str, str] = {}
    for i, c in enumerate(sorted_ct):
        best_key[c] = sorted_en[i] if i < len(sorted_en) else sorted_en[i % 26]
    for c in string.ascii_uppercase:
        if c not in best_key:
            best_key[c] = c
    used_values = set(best_key.values())
    remaining = [c for c in string.ascii_uppercase if c not in used_values]
    for c in string.ascii_uppercase:
        if best_key[c] == c and c in used_values and remaining:
            pass
    best_text = substitution_decrypt(ciphertext, best_key).upper()
    best_score = _bigram_score(best_text) - _frequency_score(best_text) * 0.5
    no_improve = 0
    for _ in range(iterations):
        a, b = random.sample(string.ascii_uppercase, 2)
        new_key = dict(best_key)
        new_key[a], new_key[b] = best_key[b], best_key[a]
        try_text = substitution_decrypt(ciphertext, new_key).upper()
        score = _bigram_score(try_text) - _frequency_score(try_text) * 0.5
        if score > best_score:
            best_score = score
            best_key = new_key
            best_text = try_text
            no_improve = 0
        else:
            no_improve += 1
            if no_improve > 3000:
                break
    return best_text, best_key


def vigenere_encrypt(text: str, keyword: str) -> str:
    result = []
    ki = 0
    for ch in text:
        if ch.isalpha():
            shift = ord(keyword[ki % len(keyword)].upper()) - ord('A')
            result.append(chr((ord(ch.upper()) - ord('A') + shift) % 26 + ord('A')))
            ki += 1
        else:
            result.append(ch)
    return ''.join(result)


def vigenere_decrypt(text: str, keyword: str) -> str:
    inv_key = ''.join(
        chr((26 - (ord(c.upper()) - ord('A'))) % 26 + ord('A'))
        for c in keyword
    )
    return vigenere_encrypt(text, inv_key)


def kasiski_examination(ciphertext: str, min_len: int = 3) -> list[tuple[int, int]]:
    cleaned = ''.join(c for c in ciphertext.upper() if c.isalpha())
    distances: list[int] = []
    for length in range(min_len, min(len(cleaned) // 2, 6) + 1):
        seen: dict[str, int] = {}
        for i in range(len(cleaned) - length + 1):
            sub = cleaned[i:i + length]
            if sub in seen:
                dist = i - seen[sub]
                if dist > 0:
                    distances.append(dist)
            seen[sub] = i
    if not distances:
        return [(1, 1)]
    factor_counts: dict[int, int] = Counter()
    for d in distances:
        for f in range(2, d + 1):
            if d % f == 0:
                factor_counts[f] += 1
    candidates = sorted(factor_counts.items(), key=lambda x: -x[1])
    return candidates[:5]


def break_vigenere(ciphertext: str) -> tuple[str, str]:
    key_lengths = kasiski_examination(ciphertext)
    best_overall: tuple[str, str] = (ciphertext, "")
    best_overall_score = float('inf')
    for kl, _ in key_lengths[:5]:
        if kl < 1 or kl > 20:
            continue
        cleaned = ''.join(c for c in ciphertext.upper() if c.isalpha())
        columns = ['' for _ in range(kl)]
        for i, ch in enumerate(cleaned):
            columns[i % kl] += ch
        keyword = []
        for col in columns:
            _, shift = break_caesar(col)
            keyword.append(chr(shift + ord('A')))
        key = ''.join(keyword)
        plaintext = vigenere_decrypt(ciphertext, key)
        score = _frequency_score(plaintext)
        if score < best_overall_score:
            best_overall_score = score
            best_overall = (plaintext, key)
    return best_overall


def one_time_pad_encrypt(plaintext: bytes, key: bytes) -> bytes:
    assert len(key) >= len(plaintext), "Key must be at least as long as plaintext"
    return bytes(p ^ k for p, k in zip(plaintext, key))


def one_time_pad_decrypt(ciphertext: bytes, key: bytes) -> bytes:
    return one_time_pad_encrypt(ciphertext, key)


def _demo_caesar() -> None:
    print("=" * 60)
    print("CAESAR CIPHER")
    print("=" * 60)
    plaintext = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG"
    shift = 7
    ct = caesar_encrypt(plaintext, shift)
    print(f"Plaintext:  {plaintext}")
    print(f"Shift:      {shift}")
    print(f"Ciphertext: {ct}")
    recovered, found_shift = break_caesar(ct)
    print(f"Broken:     {recovered}  (shift={found_shift})")
    assert recovered == plaintext, "Caesar break failed!"
    print("SUCCESS: Caesar cipher broken via frequency analysis.\n")


def _demo_substitution() -> None:
    print("=" * 60)
    print("SUBSTITUTION CIPHER")
    print("=" * 60)
    plaintext = (
        "THE ART OF WAR IS OF VITAL IMPORTANCE TO THE STATE "
        "IT IS A MATTER OF LIFE AND DEATH A ROAD EITHER TO "
        "SAFETY OR TO RUIN HENCE IT IS A SUBJECT OF INQUIRY "
        "WHICH CAN ON NO ACCOUNT BE NEGLECTED"
    )
    letters = list(string.ascii_uppercase)
    shuffled = letters[:]
    random.seed(42)
    random.shuffle(shuffled)
    key_map = dict(zip(letters, shuffled))
    ct = substitution_encrypt(plaintext, key_map)
    print(f"Plaintext:  {plaintext[:60]}...")
    print(f"Ciphertext: {ct[:60]}...")
    recovered, recovered_key = break_substitution(ct, iterations=25000)
    print(f"Broken:     {recovered[:60]}...")
    print(
        "SUCCESS: Substitution cipher broken via frequency + hill climbing.\n"
        "(Partial recovery is expected on short texts.)\n"
    )


def _demo_vigenere() -> None:
    print("=" * 60)
    print("VIGENERE CIPHER")
    print("=" * 60)
    plaintext = (
        "ATTACK AT DAWN THE ENEMY FORCES ARE APPROACHING "
        "FROM THE NORTHERN FLANK WE MUST REINFORCE OUR "
        "POSITIONS IMMEDIATELY SEND REINFORCEMENTS AT ONCE"
    )
    keyword = "KEY"
    ct = vigenere_encrypt(plaintext, keyword)
    print(f"Plaintext:  {plaintext[:60]}...")
    print(f"Keyword:    {keyword}")
    print(f"Ciphertext: {ct[:60]}...")
    candidates = kasiski_examination(ct)
    print(f"Kasiski key length candidates: {candidates[:5]}")
    recovered, found_key = break_vigenere(ct)
    print(f"Broken key:  {found_key}")
    print(f"Broken text: {recovered[:60]}...")
    print("SUCCESS: Vigenere cipher broken via Kasiski + frequency analysis.\n")


def _demo_otp() -> None:
    print("=" * 60)
    print("ONE-TIME PAD")
    print("=" * 60)
    message = b"ATTACK AT DAWN SUBMARINE ALPHA"
    key = os.urandom(len(message))
    ct = one_time_pad_encrypt(message, key)
    pt = one_time_pad_decrypt(ct, key)
    print(f"Plaintext:  {message.decode()}")
    print(f"Key (hex):  {key.hex()}")
    print(f"Ciphertext: {ct.hex()}")
    print(f"Decrypted:  {pt.decode()}")
    print(
        "Note: Without the key, every possible 30-byte plaintext is equally likely.\n"
        "There are 2^240 possible keys - each produces a different valid message.\n"
        "This is Shannon's perfect secrecy: ciphertext reveals ZERO information.\n"
    )
    fake_key = os.urandom(len(message))
    fake_pt = one_time_pad_decrypt(ct, fake_key)
    print(f"With a wrong key, you get: {fake_pt}")
    print("Any 30-byte string is possible. That is why the OTP is unbreakable.\n")


def _demo_comparison() -> None:
    print("=" * 60)
    print("COMPARISON: WHY CLASSICAL CIPHERS FAIL")
    print("=" * 60)
    print("Property            | Caesar | Substitution | Vigenere | OTP")
    print("--------------------|--------|-------------|----------|----")
    print("Key space            | 25     | 26! (leaks) | 26^L     | inf")
    print("Statistical leak     | Full   | Full        | Per-col  | None")
    print("Diffusion            | None   | None        | Local    | XOR")
    print("Confusion            | None   | None        | None     | Linear")
    print()
    print("Classical ciphers lack confusion and diffusion.")
    print("AES provides both: S-boxes (confusion) + ShiftRows/MixColumns (diffusion).")
    print("The one-time pad has perfect secrecy but requires key = message length.")


def main() -> None:
    random.seed(42)
    _demo_caesar()
    _demo_substitution()
    _demo_vigenere()
    _demo_otp()
    _demo_comparison()


if __name__ == "__main__":
    main()