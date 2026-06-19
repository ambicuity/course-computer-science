"""
Public Key I — Diffie-Hellman
Phase 12 — Cryptography & Security

Working Diffie-Hellman key exchange implementation with
modular exponentiation, keypair generation, shared secret
computation, and MITM attack simulation.
"""

import hashlib
import os


def mod_exp(base: int, exp: int, mod: int) -> int:
    result = 1
    base = base % mod
    while exp > 0:
        if exp & 1:
            result = (result * base) % mod
        base = (base * base) % mod
        exp >>= 1
    return result


def generate_private(p: int) -> int:
    return int.from_bytes(os.urandom(32), "big") % (p - 2) + 2


def generate_keypair(p: int, g: int) -> tuple[int, int]:
    private = generate_private(p)
    public = mod_exp(g, private, p)
    return private, public


def compute_shared_secret(their_pub: int, my_priv: int, p: int) -> int:
    return mod_exp(their_pub, my_priv, p)


def derive_key(shared_secret: int) -> bytes:
    return hashlib.sha256(str(shared_secret).encode()).digest()[:16]


def simulate_dh(p: int, g: int) -> None:
    print("=== Diffie-Hellman Key Exchange ===")
    print(f"Public parameters: p = {p}, g = {g}")

    alice_priv, alice_pub = generate_keypair(p, g)
    bob_priv, bob_pub = generate_keypair(p, g)

    print(f"\nAlice's private key: {alice_priv}")
    print(f"Alice's public key:  {alice_pub}")
    print(f"Bob's private key:   {bob_priv}")
    print(f"Bob's public key:    {bob_pub}")

    alice_shared = compute_shared_secret(bob_pub, alice_priv, p)
    bob_shared = compute_shared_secret(alice_pub, bob_priv, p)

    print(f"\nAlice's shared secret: {alice_shared}")
    print(f"Bob's shared secret:   {bob_shared}")
    print(f"Shared secrets match:  {alice_shared == bob_shared}")

    alice_key = derive_key(alice_shared)
    bob_key = derive_key(bob_shared)
    print(f"Derived AES-128 key:   {alice_key.hex()}")
    print(f"Bob's key matches:     {alice_key == bob_key}\n")


def simulate_mitm(p: int, g: int, message: str) -> None:
    print("=== Man-in-the-Middle Attack ===")
    print(f"Original message: \"{message}\"\n")

    alice_priv, alice_pub = generate_keypair(p, g)
    bob_priv, bob_pub = generate_keypair(p, g)
    mallory_priv, mallory_pub = generate_keypair(p, g)

    print(f"Alice sends public key:    {alice_pub}")
    print(f"Mallory intercepts, sends: {mallory_pub}")
    print(f"Bob sends public key:      {bob_pub}")
    print(f"Mallory intercepts, sends: {mallory_pub}\n")

    alice_mallory_shared = compute_shared_secret(mallory_pub, alice_priv, p)
    mallory_alice_shared = compute_shared_secret(alice_pub, mallory_priv, p)
    bob_mallory_shared = compute_shared_secret(mallory_pub, bob_priv, p)
    mallory_bob_shared = compute_shared_secret(bob_pub, mallory_priv, p)

    assert alice_mallory_shared == mallory_alice_shared
    assert bob_mallory_shared == mallory_bob_shared

    alice_key = derive_key(alice_mallory_shared)
    mallory_alice_key = derive_key(mallory_alice_shared)
    bob_key = derive_key(bob_mallory_shared)
    mallory_bob_key = derive_key(mallory_bob_shared)

    msg_bytes = message.encode()
    key_len = min(len(alice_key), len(msg_bytes))
    cipher_alice = bytes(a ^ b for a, b in zip(msg_bytes[:key_len], alice_key[:key_len]))
    intercepted = bytes(a ^ b for a, b in zip(cipher_alice, mallory_alice_key[:key_len]))
    cipher_to_bob = bytes(a ^ b for a, b in zip(intercepted, bob_key[:key_len]))

    print(f"Alice encrypts:         {cipher_alice.hex()}")
    print(f"Mallory decrypts, sees: \"{intercepted.decode()}\"")
    print(f"Mallory re-encrypts:    {cipher_to_bob.hex()}")
    print("Neither Alice nor Bob detects the interception!\n")


def main() -> None:
    # RFC 7919 2048-bit safe prime (MODP group 14)
    p_hex = (
        "FFFFFFFFFFFFFFFFC90FDAA22168C234C4C6628B80DC1CD1"
        "29024E088A67CC74020BBEA63B139B22514A08798E3404DD"
        "EF9519B3CD3A431B302B0A6DF25F14374FE1356D6D51C245"
        "E485B576625E7EC6F44C42E9A637ED6B0BFF5CB6F406B7ED"
        "EE386BFB5A899FA5AE9F24117C4B1FE649286651ECE45B3D"
        "C2007CB8A163BF0598DA48361C55D39A69163FA8FD24CF5F"
        "83655D23DCA3AD961C62F356208552BB9ED529077096966D"
        "670C354E4ABC9804F1746C08CA18217C32905E462E36CE3B"
        "E39E772C180E86039B2783A2EC07A28FB5C55DF06F4C52C9"
        "DE2BCBF6955817183995497CEA956AE515D2261898FA0510"
        "15728E5A8AACAA68FFFFFFFFFFFFFFFF"
    )
    p = int(p_hex, 16)
    g = 2

    simulate_dh(p, g)
    simulate_mitm(p, g, "Transfer $1000000 to Mallory")


if __name__ == "__main__":
    main()
