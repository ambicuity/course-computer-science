"""TLS 1.3 Handshake Simulation.

This is an educational simulation — not real cryptography.
It models the message flow, key derivation, and certificate verification
without actual mathematical operations.
"""

import hashlib
import hmac
import os
import time
from dataclasses import dataclass, field
from typing import Optional


# --- Simulated HKDF ---

def hkdf_extract(salt: bytes, input_key_material: bytes) -> bytes:
    """HKDF-Extract: PRK = HMAC(salt, IKM)."""
    if not salt:
        salt = b"\x00" * 32
    return hmac.new(salt, input_key_material, hashlib.sha256).digest()


def hkdf_expand(prk: bytes, info: bytes, length: int) -> bytes:
    """HKDF-Expand: generate output key material from PRK."""
    hash_len = 32
    n = (length + hash_len - 1) // hash_len
    okm = b""
    t = b""
    for i in range(1, n + 1):
        t = hmac.new(prk, t + info + bytes([i]), hashlib.sha256).digest()
        okm += t
    return okm[:length]


def hkdf_expand_label(prk: bytes, label: str, context: bytes, length: int) -> bytes:
    """Derive a key using TLS 1.3 label format."""
    hkdf_label = (
        length.to_bytes(2, "big")
        + bytes([len("tls13 " + label)])
        + ("tls13 " + label).encode()
        + bytes([len(context)])
        + context
    )
    return hkdf_expand(prk, hkdf_label, length)


# --- Certificate Simulation ---

@dataclass
class Certificate:
    subject: str
    issuer: str
    public_key: bytes
    not_before: float
    not_after: float
    signature: bytes = b"valid"
    is_ca: bool = False

    def is_valid_at(self, timestamp: float) -> bool:
        return self.not_before <= timestamp <= self.not_after


@dataclass
class CertificateAuthority:
    name: str
    private_key: bytes
    certificate: Certificate


def create_ca(name: str) -> CertificateAuthority:
    """Create a simulated CA."""
    private_key = hashlib.sha256(f"ca-{name}-private".encode()).digest()
    public_key = hashlib.sha256(f"ca-{name}-public".encode()).digest()
    cert = Certificate(
        subject=name,
        issuer=name,
        public_key=public_key,
        not_before=0,
        not_after=9999999999,
        is_ca=True,
    )
    return CertificateAuthority(name=name, private_key=private_key, certificate=cert)


def issue_certificate(ca: CertificateAuthority, subject: str, hostname: str, valid_days: int = 365) -> Certificate:
    """Issue a certificate signed by the CA."""
    public_key = hashlib.sha256(f"{subject}-public".encode()).digest()
    now = time.time()
    cert = Certificate(
        subject=subject,
        issuer=ca.name,
        public_key=public_key,
        not_before=now - 86400,
        not_after=now + valid_days * 86400,
        signature=hmac.new(ca.private_key, subject.encode(), hashlib.sha256).digest(),
    )
    return cert


def verify_certificate_chain(chain: list[Certificate], root_cas: list[Certificate], hostname: str, timestamp: float) -> tuple[bool, str]:
    """Verify a certificate chain against trusted root CAs."""
    if not chain:
        return False, "Empty certificate chain"

    leaf = chain[0]
    if not leaf.is_valid_at(timestamp):
        return False, f"Leaf certificate expired or not yet valid"

    if hostname not in leaf.subject and hostname != leaf.subject:
        return False, f"Hostname '{hostname}' does not match certificate subject '{leaf.subject}'"

    for i in range(len(chain) - 1):
        current = chain[i]
        issuer = chain[i + 1]
        if current.issuer != issuer.subject:
            return False, f"Chain break at index {i}: issuer '{current.issuer}' != subject '{issuer.subject}'"

    root = chain[-1]
    trusted = any(r.subject == root.subject for r in root_cas)
    if not trusted:
        return False, f"Root CA '{root.subject}' not in trust store"

    return True, "Certificate chain valid"


# --- Handshake Messages ---

@dataclass
class ClientHello:
    supported_versions: list[str]
    cipher_suites: list[str]
    key_share: bytes
    random: bytes
    sni: str
    alpn: list[str]


@dataclass
class ServerHello:
    selected_cipher: str
    key_share: bytes
    random: bytes


@dataclass
class ServerCertificate:
    cert_chain: list[Certificate]


@dataclass
class FinishedMessage:
    verify_data: bytes


# --- TLS Handshake Simulation ---

class TLSHandshake:
    """Simulates a TLS 1.3 handshake."""

    CIPHER_SUITES = [
        "TLS_AES_256_GCM_SHA384",
        "TLS_AES_128_GCM_SHA256",
        "TLS_CHACHA20_POLY1305_SHA256",
    ]

    def __init__(self, server_name: str, ca: CertificateAuthority):
        self.server_name = server_name
        self.ca = ca
        self.client_random = os.urandom(32)
        self.server_random = os.urandom(32)
        self.client_key_share = os.urandom(32)
        self.server_key_share = os.urandom(32)
        self.shared_secret = os.urandom(32)

        # Derived keys
        self.handshake_secret: Optional[bytes] = None
        self.master_secret: Optional[bytes] = None
        self.client_handshake_key: Optional[bytes] = None
        self.server_handshake_key: Optional[bytes] = None
        self.client_app_key: Optional[bytes] = None
        self.server_app_key: Optional[bytes] = None
        self.transcript: list[bytes] = []

    def client_hello(self) -> ClientHello:
        """Client sends ClientHello."""
        hello = ClientHello(
            supported_versions=["TLS 1.3"],
            cipher_suites=self.CIPHER_SUITES,
            key_share=self.client_key_share,
            random=self.client_random,
            sni=self.server_name,
            alpn=["h2", "http/1.1"],
        )
        self.transcript.append(f"ClientHello:{self.client_random.hex()}".encode())
        return hello

    def server_hello(self, client_hello: ClientHello) -> ServerHello:
        """Server processes ClientHello and sends ServerHello."""
        selected = None
        for suite in client_hello.cipher_suites:
            if suite in self.CIPHER_SUITES:
                selected = suite
                break

        if not selected:
            raise ValueError("No common cipher suite")

        hello = ServerHello(
            selected_cipher=selected,
            key_share=self.server_key_share,
            random=self.server_random,
        )
        self.transcript.append(f"ServerHello:{self.server_random.hex()}".encode())
        return hello

    def derive_handshake_keys(self):
        """Derive handshake traffic keys from ECDHE shared secret."""
        transcript_hash = hashlib.sha256(b"".join(self.transcript)).digest()

        # Early secret (no PSK)
        early_secret = hkdf_extract(b"", b"\x00" * 32)

        # Handshake secret from ECDHE
        self.handshake_secret = hkdf_extract(
            hkdf_expand_label(early_secret, "derived", b"", 32),
            self.shared_secret,
        )

        self.client_handshake_key = hkdf_expand_label(
            self.handshake_secret, "c hs traffic", transcript_hash, 32
        )
        self.server_handshake_key = hkdf_expand_label(
            self.handshake_secret, "s hs traffic", transcript_hash, 32
        )

    def derive_application_keys(self):
        """Derive application traffic keys after handshake completion."""
        transcript_hash = hashlib.sha256(b"".join(self.transcript)).digest()

        # Master secret
        self.master_secret = hkdf_extract(
            hkdf_expand_label(self.handshake_secret, "derived", b"", 32),
            b"\x00" * 32,
        )

        self.client_app_key = hkdf_expand_label(
            self.master_secret, "c ap traffic", transcript_hash, 32
        )
        self.server_app_key = hkdf_expand_label(
            self.master_secret, "s ap traffic", transcript_hash, 32
        )

    def server_send_certificate(self) -> ServerCertificate:
        """Server sends its certificate chain."""
        leaf = issue_certificate(self.ca, self.server_name, self.server_name)
        return ServerCertificate(cert_chain=[leaf, self.ca.certificate])

    def compute_finished(self, key: bytes) -> FinishedMessage:
        """Compute a finished message (HMAC of transcript)."""
        transcript_hash = hashlib.sha256(b"".join(self.transcript)).digest()
        verify_data = hmac.new(key, transcript_hash, hashlib.sha256).digest()[:12]
        return FinishedMessage(verify_data=verify_data)

    def simulate(self) -> dict:
        """Run the full 1-RTT handshake simulation."""
        print(f"Starting TLS 1.3 handshake with {self.server_name}\n")

        # Client -> Server: ClientHello
        client_hello = self.client_hello()
        print(f"1. ClientHello")
        print(f"   Supported versions: {client_hello.supported_versions}")
        print(f"   Cipher suites: {client_hello.cipher_suites}")
        print(f"   SNI: {client_hello.sni}")
        print(f"   ALPN: {client_hello.alpn}")
        print(f"   Key share: {client_hello.key_share.hex()[:32]}...")

        # Server -> Client: ServerHello
        server_hello = self.server_hello(client_hello)
        print(f"\n2. ServerHello")
        print(f"   Selected cipher: {server_hello.selected_cipher}")
        print(f"   Key share: {server_hello.key_share.hex()[:32]}...")

        # Derive handshake keys
        self.derive_handshake_keys()
        print(f"\n   [Handshake keys derived]")
        print(f"   Client handshake key: {self.client_handshake_key.hex()[:32]}...")
        print(f"   Server handshake key: {self.server_handshake_key.hex()[:32]}...")

        # Server sends encrypted extensions + certificate + finished
        cert_msg = self.server_send_certificate()
        print(f"\n3. EncryptedExtensions + Certificate + Finished (encrypted)")
        print(f"   Certificate chain:")
        for i, cert in enumerate(cert_msg.cert_chain):
            print(f"     [{i}] {cert.subject} (issued by {cert.issuer})")

        server_finished = self.compute_finished(self.server_handshake_key)
        self.transcript.append(b"ServerFinished")
        print(f"   Server finished: {server_finished.verify_data.hex()}")

        # Client verifies and sends finished
        client_finished = self.compute_finished(self.client_handshake_key)
        self.transcript.append(b"ClientFinished")
        print(f"\n4. ClientFinished")
        print(f"   Client finished: {client_finished.verify_data.hex()}")

        # Derive application keys
        self.derive_application_keys()
        print(f"\n   [Application keys derived]")
        print(f"   Client app key: {self.client_app_key.hex()[:32]}...")
        print(f"   Server app key: {self.server_app_key.hex()[:32]}...")

        print(f"\n   Handshake complete. Application data can now flow.")

        return {
            "cipher": server_hello.selected_cipher,
            "client_app_key": self.client_app_key,
            "server_app_key": self.server_app_key,
        }


def main():
    print("=== TLS 1.3 Handshake Simulation ===\n")

    # Set up a CA
    ca = create_ca("Demo Root CA")
    print(f"Root CA: {ca.name}")
    print(f"  Cert subject: {ca.certificate.subject}\n")

    # Run handshake
    handshake = TLSHandshake("example.com", ca)
    result = handshake.simulate()

    # Certificate verification demo
    print("\n=== Certificate Verification ===")
    leaf = issue_certificate(ca, "example.com", "example.com")
    valid, msg = verify_certificate_chain(
        [leaf, ca.certificate],
        [ca.certificate],
        "example.com",
        time.time(),
    )
    print(f"Chain valid: {valid} — {msg}")

    # Wrong hostname
    valid2, msg2 = verify_certificate_chain(
        [leaf, ca.certificate],
        [ca.certificate],
        "evil.com",
        time.time(),
    )
    print(f"Wrong hostname: {valid2} — {msg2}")

    # Expired cert
    expired = Certificate(
        subject="expired.com",
        issuer=ca.name,
        public_key=b"expired",
        not_before=0,
        not_after=1,
    )
    valid3, msg3 = verify_certificate_chain(
        [expired, ca.certificate],
        [ca.certificate],
        "expired.com",
        time.time(),
    )
    print(f"Expired cert: {valid3} — {msg3}")


if __name__ == "__main__":
    main()
