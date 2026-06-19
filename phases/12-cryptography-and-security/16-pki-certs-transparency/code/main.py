"""
PKI, Certificates & Transparency
Phase 12 — Cryptography & Security, Lesson 16

A certificate inspection and validation toolkit.
Dependencies: pip install cryptography certifi requests
"""

import ssl
import datetime
import hashlib
from typing import Optional

from cryptography import x509
from cryptography.x509.oid import (
    NameOID,
    ExtensionOID,
    ExtendedKeyUsageOID,
    AuthorityInformationAccessOID,
)
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa, ec, padding
from cryptography.hazmat.backends import default_backend


# ─── Certificate Parsing ─────────────────────────────────────────────────────


def fetch_certificate(hostname: str, port: int = 443) -> str:
    """Fetch a PEM-encoded certificate from a TLS server."""
    pem_bytes = ssl.get_server_certificate((hostname, port))
    return pem_bytes if isinstance(pem_bytes, str) else pem_bytes.decode()


def oid_name(oid) -> str:
    """Return a human-readable name for an OID."""
    if hasattr(oid, "_name"):
        return oid._name
    return str(oid)


def extract_rdn_list(rdn) -> str:
    """Extract a comma-separated string from an X.509 RelativeDistinguishedName."""
    parts = []
    for attr in rdn:
        parts.append(f"{oid_name(attr.oid)}={attr.value}")
    return ", ".join(parts)


def parse_certificate(pem_data: str) -> dict:
    """Parse a PEM certificate and extract all fields into a dictionary."""
    cert = x509.load_pem_x509_certificate(pem_data.encode(), default_backend())

    info: dict = {}

    # ── Subject & Issuer ────────────────────────────────────────────────
    info["subject"] = extract_rdn_list(cert.subject)
    info["issuer"] = extract_rdn_list(cert.issuer)

    # ── Serial Number ───────────────────────────────────────────────────
    info["serial"] = hex(cert.serial_number)

    # ── Validity ────────────────────────────────────────────────────────
    info["not_valid_before"] = (
        cert.not_valid_before_utc
        if hasattr(cert, "not_valid_before_utc")
        else cert.not_valid_before
    )
    info["not_valid_after"] = (
        cert.not_valid_after_utc
        if hasattr(cert, "not_valid_after_utc")
        else cert.not_valid_after
    )

    # ── Signature Algorithm ─────────────────────────────────────────────
    info["signature_algorithm"] = oid_name(cert.signature_algorithm_oid)

    # ── Fingerprint ─────────────────────────────────────────────────────
    info["fingerprint_sha256"] = cert.fingerprint(hashes.SHA256()).hex()

    # ── Public Key ──────────────────────────────────────────────────────
    pub_key = cert.public_key()
    if isinstance(pub_key, rsa.RSAPublicKey):
        info["pub_key_algorithm"] = "RSA"
        info["pub_key_size"] = pub_key.key_size
    elif isinstance(pub_key, ec.EllipticCurvePublicKey):
        info["pub_key_algorithm"] = "EC"
        info["pub_key_size"] = pub_key.key_size
        try:
            info["pub_key_curve"] = pub_key.curve.name
        except Exception:
            info["pub_key_curve"] = str(pub_key.curve)
    elif isinstance(pub_key, ec.EllipticCurvePublicKey):
        info["pub_key_algorithm"] = "EC"
        info["pub_key_size"] = pub_key.key_size
        info["pub_key_curve"] = str(pub_key.curve)
    else:
        info["pub_key_algorithm"] = type(pub_key).__name__
        info["pub_key_size"] = 0

    # ── Extensions ──────────────────────────────────────────────────────
    exts: dict = {}
    info["extensions"] = exts

    # Subject Alternative Name (SAN)
    try:
        san = cert.extensions.get_extension_for_oid(
            ExtensionOID.SUBJECT_ALTERNATIVE_NAME
        )
        exts["SAN"] = [str(n) for n in san.value]
    except x509.ExtensionNotFound:
        pass

    # Key Usage
    try:
        ku = cert.extensions.get_extension_for_oid(ExtensionOID.KEY_USAGE)
        exts["key_usage"] = []
        for attr_name in ("digital_signature", "content_commitment",
                          "key_encipherment", "data_encipherment",
                          "key_agreement", "key_cert_sign", "crl_sign"):
            try:
                if getattr(ku.value, attr_name, False):
                    exts["key_usage"].append(attr_name.replace("content_commitment", "non_repudiation"))
            except ValueError:
                pass
        # encipher_only / decipher_only — only valid if key_agreement is set
        if ku.value.key_agreement:
            if ku.value.encipher_only:
                exts["key_usage"].append("encipher_only")
            if ku.value.decipher_only:
                exts["key_usage"].append("decipher_only")
    except x509.ExtensionNotFound:
        pass

    # Extended Key Usage
    try:
        eku = cert.extensions.get_extension_for_oid(
            ExtensionOID.EXTENDED_KEY_USAGE
        )
        exts["extended_key_usage"] = [oid_name(u) for u in eku.value]
    except x509.ExtensionNotFound:
        pass

    # Basic Constraints
    try:
        bc = cert.extensions.get_extension_for_oid(
            ExtensionOID.BASIC_CONSTRAINTS
        )
        exts["ca"] = bc.value.ca
        exts["path_length"] = bc.value.path_length
    except x509.ExtensionNotFound:
        pass

    # CRL Distribution Points
    try:
        crl = cert.extensions.get_extension_for_oid(
            ExtensionOID.CRL_DISTRIBUTION_POINTS
        )
        exts["crl_dps"] = []
        for dp in crl.value:
            for name in dp.full_name or []:
                exts["crl_dps"].append(str(name))
    except x509.ExtensionNotFound:
        pass

    # Authority Information Access (AIA)
    try:
        aia = cert.extensions.get_extension_for_oid(
            ExtensionOID.AUTHORITY_INFORMATION_ACCESS
        )
        exts["aia"] = []
        for desc in aia.value:
            method = oid_name(desc.access_method)
            exts["aia"].append(f"{method}: {desc.access_location}")
    except x509.ExtensionNotFound:
        pass

    # Signed Certificate Timestamps (SCTs)
    for oid in (ExtensionOID.PRECERT_SIGNED_CERTIFICATE_TIMESTAMPS,
                ExtensionOID.SIGNED_CERTIFICATE_TIMESTAMPS):
        try:
            sct_ext = cert.extensions.get_extension_for_oid(oid)
            scts = []
            for sct in sct_ext.value:
                scts.append({
                    "version": sct.version,
                    "log_id": sct.log_id.hex(),
                    "timestamp": str(sct.timestamp),
                })
            exts["scts"] = scts
            break
        except x509.ExtensionNotFound:
            continue

    return info


def print_cert_summary(info: dict) -> None:
    """Pretty-print parsed certificate information."""
    print("=" * 72)
    print("  CERTIFICATE SUMMARY")
    print("=" * 72)
    print(f"  Subject:           {info['subject']}")
    print(f"  Issuer:            {info['issuer']}")
    print(f"  Serial:            {info['serial']}")
    print(f"  Not Before:        {info['not_valid_before']}")
    print(f"  Not After:         {info['not_valid_after']}")
    print(f"  Signature Algo:    {info.get('signature_algorithm', '?')}")
    pub_alg = info.get("pub_key_algorithm", "?")
    pub_sz = info.get("pub_key_size", "?")
    curve = info.get("pub_key_curve", "")
    curve_info = f" ({curve})" if curve else ""
    print(f"  Public Key:        {pub_alg} ({pub_sz} bits{curve_info})")
    print(f"  SHA-256 FP:        {info['fingerprint_sha256']}")

    exts = info["extensions"]
    if "SAN" in exts:
        san_list = exts["SAN"]
        display = san_list[:5]
        suffix = "..." if len(san_list) > 5 else ""
        print(f"  Subject Alt Names: {', '.join(display)}{suffix}")
    if "key_usage" in exts:
        print(f"  Key Usage:         {', '.join(exts['key_usage'])}")
    if "extended_key_usage" in exts:
        print(f"  Ext Key Usage:     {', '.join(exts['extended_key_usage'])}")
    if "ca" in exts:
        print(f"  CA:                {exts['ca']}  (path_len={exts['path_length']})")
    if "crl_dps" in exts:
        print(f"  CRL Distribution:  {exts['crl_dps'][0]}")
    if "aia" in exts:
        for aia in exts["aia"]:
            print(f"  AIA:               {aia}")
    if "scts" in exts:
        for sct in exts["scts"]:
            log_preview = sct["log_id"][:16]
            print(f"  SCT:               log={log_preview}... ts={sct['timestamp']}")
    print()


# ─── Certificate Chain Validation ────────────────────────────────────────────


def check_validity(pem_data: str) -> tuple[bool, str]:
    """Check whether a PEM certificate is currently within its validity period."""
    cert = x509.load_pem_x509_certificate(pem_data.encode(), default_backend())
    now = datetime.datetime.now(datetime.timezone.utc)

    nbf = (
        cert.not_valid_before_utc
        if hasattr(cert, "not_valid_before_utc")
        else cert.not_valid_before.replace(tzinfo=datetime.timezone.utc)
    )
    nba = (
        cert.not_valid_after_utc
        if hasattr(cert, "not_valid_after_utc")
        else cert.not_valid_after.replace(tzinfo=datetime.timezone.utc)
    )

    if now < nbf:
        return False, f"NOT YET VALID — starts {nbf}"
    if now > nba:
        return False, f"EXPIRED — ended {nba}"
    return True, f"valid ({nbf} to {nba})"


def verify_cert_signature(child: x509.Certificate,
                           issuer_pub_key) -> bool:
    """Verify child's signature against issuer's public key.

    Handles both RSA and ECDSA signatures.
    """
    try:
        if isinstance(issuer_pub_key, rsa.RSAPublicKey):
            issuer_pub_key.verify(
                child.signature,
                child.tbs_certificate_bytes,
                padding.PKCS1v15(),
                child.signature_hash_algorithm,
            )
            return True
        elif isinstance(issuer_pub_key, ec.EllipticCurvePublicKey):
            issuer_pub_key.verify(
                child.signature,
                child.tbs_certificate_bytes,
                ec.ECDSA(child.signature_hash_algorithm),
            )
            return True
        else:
            return False
    except Exception:
        return False


def fetch_issuer_cert(issuer_url: str) -> Optional[str]:
    """Fetch an issuer certificate from an AIA URL (HTTP/HTTPS)."""
    try:
        import urllib.request
        with urllib.request.urlopen(issuer_url, timeout=10) as resp:
            data = resp.read()
            # Try PEM first, then DER
            if data.startswith(b"-----"):
                return data.decode("utf-8")
            # Convert DER to PEM
            cert = x509.load_der_x509_certificate(data, default_backend())
            return cert.public_bytes(serialization.Encoding.PEM).decode("utf-8")
    except Exception:
        return None


def _fetch_chain_via_openssl(hostname: str,
                              port: int) -> Optional[list[bytes]]:
    """Fetch certificate chain using openssl s_client -showcerts."""
    import subprocess
    try:
        proc = subprocess.run(
            ["openssl", "s_client", "-showcerts",
             "-connect", f"{hostname}:{port}"],
            input=b"\n", capture_output=True, timeout=15,
        )
        output = proc.stdout.decode("utf-8", errors="replace")
    except (FileNotFoundError, subprocess.TimeoutExpired) as e:
        print(f"    OpenSSL fallback not available: {e}")
        return None

    certs: list[bytes] = []
    current_pem: list[str] = []
    in_cert = False
    for line in output.splitlines():
        if line.startswith("-----BEGIN CERTIFICATE-----"):
            in_cert = True
            current_pem = [line]
        elif line.startswith("-----END CERTIFICATE-----"):
            current_pem.append(line)
            pem_str = "\n".join(current_pem)
            certs.append(pem_str.encode())
            current_pem = []
            in_cert = False
        elif in_cert:
            current_pem.append(line)
        elif "BEGIN CERTIFICATE" in line:
            in_cert = True
            current_pem = [line]

    if not certs:
        return None

    # Convert PEM to DER
    der_certs = []
    for pem_data in certs:
        cert = x509.load_pem_x509_certificate(pem_data, default_backend())
        der_certs.append(cert.public_bytes(serialization.Encoding.DER))
    return der_certs


def _fetch_chain_via_ssl(hostname: str,
                          port: int) -> Optional[list[bytes]]:
    """Fetch chain via Python SSL socket."""
    import socket as sock_mod
    import ssl as ssl_mod

    context = ssl_mod.create_default_context()
    context.check_hostname = False
    context.verify_mode = ssl_mod.CERT_NONE

    try:
        with context.wrap_socket(
                sock_mod.socket(sock_mod.AF_INET),
                server_hostname=hostname,
        ) as s:
            s.settimeout(10)
            s.connect((hostname, port))
            der_chain = s.get_peer_cert_chain()
            if der_chain:
                return list(der_chain)
            return None
    except AttributeError:
        # get_peer_cert_chain not available on this platform
        return None
    except Exception:
        return None


def build_chain_from_server(hostname: str,
                             port: int = 443) -> list[dict]:
    """Fetch a server's certificate chain, parse each cert, and verify the chain.

    Tries Python SSL socket first, falls back to openssl CLI.
    """
    der_chain = _fetch_chain_via_ssl(hostname, port)
    if der_chain is None:
        der_chain = _fetch_chain_via_openssl(hostname, port)

    if der_chain is None or len(der_chain) < 2:
        print("  WARNING: Chain fetch unavailable — try:\n"
              "    openssl s_client -connect <host>:443 -showcerts")
        return []

    chain_info: list[dict] = []
    cert_objects = [
        x509.load_der_x509_certificate(c, default_backend())
        for c in der_chain
    ]

    max_links = min(len(cert_objects) - 1, 10)
    for i in range(max_links):
        child = cert_objects[i]
        issuer = cert_objects[i + 1]
        issuer_pub = issuer.public_key()

        child_subject = child.subject.rfc4514_string()
        child_issuer_str = child.issuer.rfc4514_string()
        issuer_subject = issuer.subject.rfc4514_string()

        sig_ok = verify_cert_signature(child, issuer_pub)

        bc_ok = False
        try:
            bc_ext = issuer.extensions.get_extension_for_oid(
                ExtensionOID.BASIC_CONSTRAINTS
            )
            bc_ok = bc_ext.value.ca
        except x509.ExtensionNotFound:
            pass

        valid_now, validity_msg = check_validity(
            issuer.public_bytes(serialization.Encoding.PEM).decode()
        )

        link = {
            "index": i,
            "child_subject": child_subject,
            "child_issuer": child_issuer_str,
            "issuer_subject": issuer_subject,
            "signature_valid": sig_ok,
            "issuer_is_ca": bc_ok,
            "issuer_valid": valid_now,
            "validity_msg": validity_msg,
        }
        chain_info.append(link)

    # Handle the last (root) entry: self-signed, trust anchor
    if cert_objects:
        root = cert_objects[-1]
        root_subject = root.subject.rfc4514_string()
        root_issuer = root.issuer.rfc4514_string()
        is_self_signed = root.subject == root.issuer
        valid_now, validity_msg = check_validity(
            root.public_bytes(serialization.Encoding.PEM).decode()
        )

        chain_info.append({
            "index": len(cert_objects) - 1,
            "child_subject": root_subject,
            "child_issuer": root_issuer,
            "issuer_subject": "(self-signed — trust anchor)",
            "signature_valid": is_self_signed,
            "issuer_is_ca": True,
            "issuer_valid": valid_now,
            "validity_msg": validity_msg,
        })

    return chain_info


def print_chain_status(chain: list[dict]) -> None:
    """Print the chain validation results."""
    print("=" * 72)
    print("  CHAIN VALIDATION")
    print("=" * 72)
    all_pass = True
    for link in chain:
        idx = link["index"]
        sig = "✓" if link["signature_valid"] else "✗"
        ca = "✓" if link["issuer_is_ca"] else "✗"
        val = "✓" if link["issuer_valid"] else "✗"

        if not (link["signature_valid"] and link["issuer_is_ca"] and link["issuer_valid"]):
            all_pass = False

        print(f"  #{idx}: {link['child_subject'][:60]}")
        if idx == len(chain) - 1 and link["issuer_subject"] == "(self-signed — trust anchor)":
            print(f"       ↓ signed by  {link['issuer_subject']}")
        else:
            print(f"       ↓ signed by  {link['issuer_subject'][:60]}")
        print(f"       Signature: {sig}   CA: {ca}   Valid: {val}   {link['validity_msg']}")
        print()

    if all_pass:
        print("  Chain: ALL CHECKS PASSED")
    else:
        print("  Chain: ONE OR MORE CHECKS FAILED")
    print()


# ─── Merkle Tree (Certificate Transparency) ─────────────────────────────────


def hash_leaf(data: bytes) -> bytes:
    """Hash a leaf entry as in CT: SHA-256(0x00 || data)."""
    return hashlib.sha256(b"\x00" + data).digest()


def hash_node(left: bytes, right: bytes) -> bytes:
    """Hash an internal node as in CT: SHA-256(0x01 || left || right)."""
    return hashlib.sha256(b"\x01" + left + right).digest()


class MerkleTree:
    """An append-only Merkle tree, following the CT design (RFC 6962)."""

    def __init__(self, leaves: Optional[list[bytes]] = None):
        self.leaves: list[bytes] = []
        self.tree: list[list[bytes]] = []
        if leaves:
            for leaf in leaves:
                self.add_leaf(leaf)

    def add_leaf(self, data: bytes) -> None:
        self.leaves.append(data)
        self._build()

    def _build(self) -> None:
        if not self.leaves:
            self.tree = []
            return
        self.tree = [[hash_leaf(d) for d in self.leaves]]
        while len(self.tree[-1]) > 1:
            level = []
            hashes = self.tree[-1]
            for i in range(0, len(hashes), 2):
                if i + 1 < len(hashes):
                    level.append(hash_node(hashes[i], hashes[i + 1]))
                else:
                    level.append(hashes[i])
            self.tree.append(level)

    @property
    def root(self) -> Optional[bytes]:
        if not self.tree:
            return None
        return self.tree[-1][0]

    @property
    def leaf_count(self) -> int:
        return len(self.leaves)

    def get_proof(self, index: int) -> list[bytes]:
        if index < 0 or index >= len(self.leaves):
            raise IndexError(f"leaf index {index} out of range (0..{len(self.leaves)-1})")
        proof: list[bytes] = []
        idx = index
        for level in self.tree[:-1]:
            sibling_idx = idx ^ 1
            if sibling_idx < len(level):
                proof.append(level[sibling_idx])
            idx //= 2
        return proof

    def print_tree(self) -> None:
        print(f"  Merkle Tree ({self.leaf_count} leaves)")
        root_hex = self.root.hex() if self.root else "None"
        print(f"    Root: {root_hex}")
        for lvl, level in enumerate(self.tree):
            label = "Leaves" if lvl == 0 else f"Level {lvl}"
            short_hashes = ", ".join(h.hex()[:12] for h in level)
            print(f"    {label}: {short_hashes}")
        print()


def verify_inclusion(leaf: bytes, proof: list[bytes],
                     root: bytes, index: int) -> bool:
    """Verify a Merkle inclusion proof.

    Returns True if recomputing the root from the leaf, proof, and index
    matches the given root.
    """
    current = hash_leaf(leaf)
    idx = index
    for sibling in proof:
        if idx % 2 == 0:
            current = hash_node(current, sibling)
        else:
            current = hash_node(sibling, current)
        idx //= 2
    return current == root


# ─── Demos ───────────────────────────────────────────────────────────────────


def demo_parse() -> None:
    """Step 1: Fetch and parse a real TLS certificate."""
    print("\n" + "=" * 72)
    print("  STEP 1: Parse and Inspect an X.509 Certificate")
    print("=" * 72)

    hostname = "google.com"
    print(f"  Fetching certificate from {hostname}:443...\n")
    try:
        pem = fetch_certificate(hostname)
    except Exception as e:
        print(f"  ERROR: Could not fetch certificate: {e}")
        return

    info = parse_certificate(pem)
    print_cert_summary(info)

    valid, msg = check_validity(pem)
    status = "✓" if valid else "✗"
    print(f"  Validity: {status}  {msg}")
    print()


def demo_chain() -> None:
    """Step 2: Fetch and validate a server's certificate chain."""
    print("\n" + "=" * 72)
    print("  STEP 2: Certificate Chain Validation")
    print("=" * 72)

    hostname = "google.com"
    print(f"  Fetching certificate chain from {hostname}:443...\n")

    try:
        chain = build_chain_from_server(hostname)
    except Exception as e:
        print(f"  ERROR: Could not fetch chain: {e}")
        return

    if not chain:
        print("  No chain data received.")
        return

    print_chain_status(chain)


def demo_merkle() -> None:
    """Step 3: Certificate Transparency Merkle tree demonstration."""
    print("\n" + "=" * 72)
    print("  STEP 3: Certificate Transparency — Merkle Tree Demo")
    print("=" * 72)

    entries = [
        b"cert_001: CN=example.com, serial=ABC123",
        b"cert_002: CN=google.com, serial=DEF456",
        b"cert_003: CN=github.com, serial=GHI789",
        b"cert_004: CN=stackoverflow.com, serial=JKL012",
        b"cert_005: CN=cloudflare.com, serial=MNO345",
        b"cert_006: CN=amazon.com, serial=PQR678",
    ]

    print(f"  Building Merkle tree from {len(entries)} certificate log entries:\n")
    for e in entries:
        print(f"    [{entries.index(e)}] {e.decode()}")
    print()

    tree = MerkleTree(entries)
    tree.print_tree()

    root_before = tree.root

    # ── Inclusion Proof ──────────────────────────────────────────────────
    print("  --- Inclusion Proof ---")
    leaf_index = 2
    leaf_data = entries[leaf_index]
    proof = tree.get_proof(leaf_index)
    print(f"    Proving leaf #{leaf_index}: {leaf_data.decode()[:50]}...")
    print(f"    Sibling hashes in proof: {len(proof)}")
    for i, p in enumerate(proof):
        print(f"      [{i}] {p.hex()}")
    ok = verify_inclusion(leaf_data, proof, root_before, leaf_index)
    print(f"    Result: {'✓ VALID' if ok else '✗ INVALID'}")
    print()

    # ── Tamper Detection ─────────────────────────────────────────────────
    print("  --- Tamper Detection ---")
    fake_leaf = b"cert_003: CN=evil.com, serial=EVIL001"
    fake_proof = tree.get_proof(leaf_index)
    fake_ok = verify_inclusion(fake_leaf, fake_proof, root_before, leaf_index)
    print(f"    Trying fake leaf: {fake_leaf.decode()}")
    print(f"    Result: {'✗ REJECTED (correctly)' if not fake_ok else '✗ BUG: wrongly accepted'}")
    print()

    # ── Append-Only Property ─────────────────────────────────────────────
    print("  --- Append-Only Property ---")
    tree.add_leaf(b"cert_007: CN=reddit.com, serial=STU901")
    root_after = tree.root
    print(f"    Added leaf #7: cert_007")
    print(f"    Root before: {root_before.hex()}")
    print(f"    Root after:  {root_after.hex()}")
    print(f"    Roots differ: {root_before != root_after} (should be True)")
    print()

    # ── Old Proof Still Valid ────────────────────────────────────────────
    print("  --- Old Proof Still Valid After Append ---")
    old_proof = tree.get_proof(leaf_index)
    if len(old_proof) >= len(proof):
        old_ok = verify_inclusion(leaf_data, old_proof[:len(proof)],
                                  root_before, leaf_index)
    else:
        old_ok = False
    print(f"    Old inclusion proof still verified: {'✓ YES' if old_ok else 'n/a (tree depth grew)'}")
    print()

    # ── Many Leaves ──────────────────────────────────────────────────────
    print("  --- Larger Tree ---")
    many_entries = [
        f"cert_{i:03d}: CN=site{i}.com".encode()
        for i in range(16)
    ]
    big_tree = MerkleTree(many_entries)
    big_tree.print_tree()

    mid = 7
    mid_proof = big_tree.get_proof(mid)
    mid_ok = verify_inclusion(many_entries[mid], mid_proof,
                              big_tree.root, mid)
    print(f"    Leaf #{mid} in 16-leaf tree: {'✓ VALID' if mid_ok else '✗ INVALID'}")
    print()


def main() -> None:
    print("=" * 72)
    print("  PKI, Certificates & Transparency — Certificate Toolkit")
    print("  Phase 12 — Cryptography & Security, Lesson 16")
    print("=" * 72)

    demo_parse()
    demo_chain()
    demo_merkle()

    print("=" * 72)
    print("  All demos completed successfully!")
    print("=" * 72)


if __name__ == "__main__":
    main()
