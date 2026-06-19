# PKI, Certificates & Transparency

> Without PKI, your browser is just taking the server's word for it — and that's how MITM attacks happen.

**Type:** Learn (with Build elements)
**Languages:** Markdown, Python
**Prerequisites:** Phase 12 lessons 01–15 (especially 09–11 for public key crypto, 14–15 for TLS)
**Time:** ~60 minutes

## Learning Objectives

- Understand how X.509 certificates and the CA hierarchy bind public keys to identities.
- Parse and inspect a real TLS certificate to extract subject, issuer, SANs, and extensions.
- Implement certificate chain validation — verify signatures from leaf through intermediate to a trusted root.
- Explain why Certificate Transparency matters and how Merkle trees make logs append-only and auditable.
- Distinguish the CA trust model from TOFU (SSH) and name constraints (DNS, DANE).

## The Problem

You connect to `https://google.com`. Your browser and the server negotiate a TLS session using the handshake you studied in lesson 14. The server sends its certificate. But how does your browser know that certificate actually belongs to Google?

Without PKI, an attacker on the same network could intercept the connection, present *their own* certificate, and your browser would have no way to tell the difference. This is a **man-in-the-middle (MITM)** attack — the attacker forwards traffic between you and Google while decrypting and re-encrypting everything in between.

The core challenge is **key distribution**: how do you securely associate a public key with an identity over an untrusted network? You cannot ask the server itself — the attacker is the one answering. You need a trusted third party.

PKI solves this with a global system of **Certificate Authorities (CAs)**, **X.509 certificates**, and — more recently — **Certificate Transparency** logs. The system is far from perfect, but it is what secures every HTTPS connection on the internet.

## The Concept

### X.509 Certificates

An X.509 certificate is a signed binding between a **subject** (domain, organization, or person) and a **public key**. Its structure, defined in RFC 5280:

| Field | Meaning | Example |
|-------|---------|---------|
| Subject | Who this cert belongs to | `CN=google.com` |
| Issuer | Who signed this cert | `CN=GTS CA 1O1, O=Google Trust Services` |
| Validity | Date range | `2025-01-15` → `2026-04-15` |
| Subject Public Key Info | The public key | RSA-2048 or EC P-256 |
| Signature Algorithm | How it was signed | `sha256WithRSAEncryption` |
| Extensions | Constraints and metadata | SAN, KU, EKU, Basic Constraints |

Extensions are where the real policy lives:

- **Subject Alternative Name (SAN)** — Lists the domains/IPs this cert is valid for. Modern browsers IGNORE the Common Name (CN) and only check SANs. If `evil.com` is in the SAN of a cert issued for `google.com`, that cert is mis-issued.
- **Key Usage (KU)** — What the key can do: `digitalSignature`, `keyEncipherment`, `keyCertSign`, `cRLSign`.
- **Extended Key Usage (EKU)** — Higher-level purpose: `serverAuth`, `clientAuth`, `codeSigning`.
- **Basic Constraints** — `CA:TRUE` means this cert can sign other certs. `CA:FALSE` means it cannot. An end-entity (leaf) cert must have `CA:FALSE`.
- **CRL Distribution Points** — Where to find the Certificate Revocation List.
- **Authority Information Access (AIA)** — Where to fetch the issuer's certificate (the one that signed this one).
- **Signed Certificate Timestamp (SCT)** — Proof that this cert was submitted to a CT log.

### The CA Hierarchy

```
┌─────────────────────────────────┐
│   Root CA (offline / HSM)      │  Self-signed, in every trust store
│   CA:TRUE, path_length: 2      │  ~20-year validity
└────────────┬────────────────────┘
             │ signs
┌────────────▼────────────────────┐
│   Intermediate CA               │  Signed by root
│   CA:TRUE, path_length: 1      │  5–10 year validity
└────────────┬────────────────────┘
    ┌────────┼────────┐
    │ signs │ signs  │ signs
┌───▼───┐ ┌─▼───┐ ┌─▼───┐
│ Leaf  │ │Leaf │ │Leaf │  End-entity certs
│(your  │ │(SMTP│ │(code │  CA:FALSE
│ site) │ │ TLS)│ │sign) │  90-day–2yr validity
└───────┘ └─────┘ └─────┘
```

Your device ships with ~100–150 **trust anchors** — root CA certificates pre-installed in the operating system or browser. On macOS: `Security.framework` / Keychain. On Linux: `/etc/ssl/certs/` (symlinks to PEM files from `ca-certificates`). On Windows: the Certificate Store.

When a server presents its leaf certificate during TLS, it SHOULD also send the intermediate(s) that chain it to a root. The browser builds the chain:

1. Leaf cert → (signed by) Intermediate → (signed by) Root
2. Verifies each signature
3. Checks validity dates
4. Checks Basic Constraints (each issuer must be CA:TRUE)
5. Checks SAN against the hostname in the URL
6. Checks revocation (CRL or OCSP)
7. **Soft-fail** if revocation check can't reach the server (this is a known weakness)

### Revocation: CRL vs OCSP

A certificate must be revoked before its expiry if the private key is compromised or the CA mis-issued it.

- **Certificate Revocation List (CRL)** — A signed, timestamped list of revoked cert serial numbers, published by the CA at a URL in the `CRL Distribution Points` extension. Problem: CRLs grow unboundedly. Browsers often skip CRL checks because downloading a multi-megabyte list on every TLS connection is impractical.

- **Online Certificate Status Protocol (OCSP)** — Real-time check: the browser asks the CA "is cert serial X revoked?" and gets a signed response. Problem: the OCSP responder must be reachable (privacy concern, and a blocking check adds latency). More critically, if the attacker blocks OCSP traffic, most browsers **soft-fail** — they proceed without revocation info.

- **OCSP Stapling** — The server fetches an OCSP response *before* the TLS handshake and staples it to the certificate. The browser can verify the stapled response without contacting the CA. This fixes both the latency and the blocking problem, but adoption is incomplete.

Neither CRL nor OCSP is a complete solution. This is why Certificate Transparency exists.

### Certificate Transparency (CT)

CT is a Google-led system (RFC 6962) that makes certificate issuance **publicly auditable**. Every CA must submit every certificate they issue to one or more **CT logs** before the certificate is trusted by Chrome/Safari.

**How it works:**

1. CA prepares a **pre-certificate** (almost identical to the real cert).
2. CA sends the pre-certificate to one or more CT logs.
3. Each log appends it to an append-only Merkle tree and returns a **Signed Certificate Timestamp (SCT)** — a signed promise to include it.
4. CA embeds the SCTs in the final certificate (via the `signed_certificate_timestamp` extension) or delivers them via TLS extension or OCSP stapling.
5. The browser verifies the SCTs and only trusts the certificate if it has enough valid SCTs from known logs.

**Merkle Trees in CT:**

A Merkle tree is a binary hash tree where:
- Each leaf is `SHA-256(0x00 || certificate_entry)`.
- Each internal node is `SHA-256(0x01 || left_hash || right_hash)`.
- The root commits to the entire set of entries.
- A log can prove inclusion by showing the sibling hashes along the path from leaf to root.
- A log can prove append-only consistency: that the old root is a prefix of the new tree — without revealing the new entries.

Because CT logs are public, anyone can monitor them. If a CA mis-issues a certificate for `google.com`, a monitor spots it within hours and the CA gets called out (or their root is removed from trust stores).

**DigiNotar (2011)** — A Dutch CA was compromised and issued fake certificates for google.com, Yahoo, and others. The fraud was detected only after the certs were used in the wild (Iranian MITM). The CA went bankrupt and its root was distrusted. CT would have caught this within hours of issuance.

### TOFU vs PKI

SSH uses a completely different model: **Trust On First Use (TOFU)**. The first time you connect to a server, SSH records its host key. If the key changes later, SSH warns you. This works well for small-scale deployments but is vulnerable on the first connection (no initial trust).

### Let's Encrypt and ACME

Let's Encrypt automated certificate issuance via the **ACME** (Automatic Certificate Management Environment) protocol (RFC 8555). The CA verifies domain control by:
- **HTTP-01**: Place a token at `http://domain/.well-known/acme-challenge/<token>`
- **DNS-01**: Create a TXT record with the token
- **TLS-ALPN-01**: Respond to a TLS handshake on port 443 with the token

ACME made TLS certificates free and automatic, removing the cost and friction that previously blocked widespread HTTPS adoption. As of 2025, Let's Encrypt issues over 3 million certificates per day.

## Build It

We will build a certificate inspection and validation toolkit in Python using the `cryptography` library.

```python
# Dependencies: pip install cryptography certifi requests
```

### Step 1: Parse and Inspect an X.509 Certificate

We fetch a real certificate from `google.com` using Python's `ssl` module, then parse it with `cryptography.x509` to extract every field.

```python
import ssl
from cryptography import x509
from cryptography.hazmat.backends import default_backend

pem_data = ssl.get_server_certificate(('google.com', 443))
cert = x509.load_pem_x509_certificate(pem_data.encode(), default_backend())

# Extract fields
subject = cert.subject.rfc4514_string()
issuer = cert.issuer.rfc4514_string()
serial = hex(cert.serial_number)
not_before = cert.not_valid_before_utc
not_after = cert.not_valid_after_utc
```

The full implementation in `code/main.py` prints a formatted summary with SANs, key usage, extended key usage, basic constraints, CRL distribution points, authority info access, and any embedded SCTs.

### Step 2: Certificate Chain Validation

Validation is signature verification up the chain:

1. For each certificate (except the root), verify that its signature was made by the next certificate's public key.
2. Check that each issuer (except the leaf) has `CA:TRUE` in Basic Constraints.
3. Check that all certificates are within their validity period.

The signature verification differs by key type:

```python
def verify_cert_signature(cert, issuer_pub_key):
    if isinstance(issuer_pub_key, rsa.RSAPublicKey):
        issuer_pub_key.verify(
            cert.signature, cert.tbs_certificate_bytes,
            padding.PKCS1v15(), cert.signature_hash_algorithm
        )
    elif isinstance(issuer_pub_key, ec.EllipticCurvePublicKey):
        issuer_pub_key.verify(
            cert.signature, cert.tbs_certificate_bytes,
            ec.ECDSA(cert.signature_hash_algorithm)
        )
```

The chain is valid only if every link in the chain passes. The code in `main.py` performs this check on a server's certificate chain by fetching intermediate certs via AIA if needed.

### Step 3: Certificate Transparency — Merkle Tree Operations

We implement an append-only Merkle tree as used by CT logs:

```python
import hashlib

def hash_leaf(data: bytes) -> bytes:
    return hashlib.sha256(b'\x00' + data).digest()

def hash_node(left: bytes, right: bytes) -> bytes:
    return hashlib.sha256(b'\x01' + left + right).digest()

class MerkleTree:
    def __init__(self):
        self.leaves: list[bytes] = []
        self.tree: list[list[bytes]] = []
```

The tree supports:
- **Adding a leaf** — appends to the log and rebuilds the tree
- **Root hash** — the Merkle root commits to all entries
- **Inclusion proof** — the sibling hashes along the leaf-to-root path
- **Verification** — recompute the root from leaf + proof and compare

This demonstrates the key CT property: anyone can verify that a certificate was included in the log without downloading the entire log.

## Use It

Every HTTPS connection uses PKI. When you visit a website:

1. The server presents its certificate chain during the TLS handshake.
2. Your browser verifies each signature up to a built-in trust anchor.
3. It checks the SAN against the domain in the URL bar.
4. It checks for valid SCTs (Chrome requires 2 SCTs from qualifying logs).
5. Optionally checks revocation via CRL/OCSP (soft-fail if unreachable).
6. The green padlock means chain validated + domain matches + CT satisfied.

Real-world CA governance is defined by the **CA/Browser Forum Baseline Requirements** — a set of rules CAs must follow to remain trusted in browsers. These cover verification of certificate applicants, key protection, and maximum validity periods (currently 90 days for TLS certificates as of 2025).

Notable CA failures:
- **DigiNotar (2011)** — Breach led to fraudulent google.com certs; CA dissolved.
- **WoSign (2016)** — Issued certs with backdated timestamps to bypass SHA-1 deprecation; root distrusted by Mozilla and Apple.
- **TrustCor (2022)** — Revealed ties to a company producing spyware; major browsers distrusted.

CT log monitoring is now essential: services like **crt.sh** and **Censys** continuously monitor CT logs and email domain owners when certificates for their domains appear.

## Read the Source

- **RFC 5280** — Internet X.509 Public Key Infrastructure Certificate and Certificate Revocation List Profile: the foundational specification for X.509 certificates, CRLs, and path validation.
- **RFC 6962** — Certificate Transparency: defines the Merkle tree structure, SCT format, and audit protocol for CT logs.
- **RFC 8555** — Automatic Certificate Management Environment (ACME): the protocol that powers Let's Encrypt's automated certificate issuance.
- **Let's Encrypt Boulder source** — `github.com/letsencrypt/boulder`: the Go implementation of the ACME server; see `web/`, `ca/`, `sa/` for challenge validation, certificate issuance, and storage.
- **crt.sh** — `github.com/crtsh`: the code behind the Certificate Transparency log search engine. Look at how they ingest, index, and search millions of certificates from CT logs.

## Ship It

The reusable artifact is a **certificate inspection and validation toolkit** in `outputs/`. It parses X.509 certificates, validates chains against system trust stores, and demonstrates the Merkle tree data structure at the heart of Certificate Transparency. This toolkit feeds into the phase capstone (TLS 1.3 library) by providing the certificate validation logic needed for the TLS handshake.

## Exercises

1. **Easy** — Run the toolkit against `github.com` and `expired.badssl.com`. Compare the certificate fields. What differences do you notice in the validity dates, issuer, and SANs?
2. **Medium** — Extend the Merkle tree implementation to support **consistency proofs**: given the old root and the new tree, prove that the old root is a prefix. This is what CT auditors use to verify that a log has not been tampered with.
3. **Hard** — Implement a minimal ACME client in Python that registers with Let's Encrypt's staging server and obtains a certificate for a domain you control. Handle HTTP-01 challenge verification and CSR generation.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| X.509 | A certificate format | The ITU-T standard defining the structure of public-key certificates: subject, issuer, validity, public key, and extensions |
| Certificate Authority (CA) | A company that issues certificates | A trusted entity that verifies identities and signs certificates binding public keys to those identities |
| Intermediate CA | A middleman CA | A certificate with CA:TRUE signed by a root CA or another intermediate, used to issue leaf certificates without exposing the root |
| Leaf certificate | The server's cert | An end-entity certificate (CA:FALSE) that binds a domain/organization to a public key; presented during TLS |
| Subject Alternative Name (SAN) | Domains a cert covers | The X.509 extension listing all DNS names/IPs for which the certificate is valid; the only field browsers check for hostname matching |
| Certificate Revocation List (CRL) | A list of bad certs | A signed, timestamped list of revoked certificate serial numbers published periodically by a CA |
| OCSP | Real-time cert status check | Online Certificate Status Protocol: query a CA's responder for the current revocation status of a specific certificate |
| Certificate Transparency (CT) | Public log of certs | An open auditing system using Merkle trees to make all certificate issuance publicly visible and cryptographically verifiable |
| SCT | A promise to log | Signed Certificate Timestamp: a signed assurance from a CT log that it will include the certificate in its Merkle tree within a certain time |
| ACME | Auto-cert issuance | Automated Certificate Management Environment: the protocol (RFC 8555) used by Let's Encrypt for domain-validated certificate issuance |
| Chain validation | Verifying a cert chain | Building the path from leaf to a trusted root and verifying each link's signature, constraints, and validity period |
| Trust anchor | A root you trust | A self-signed root CA certificate pre-installed in the OS/browser that serves as the starting point for chain validation |

## Further Reading

- "The First Few Milliseconds of an HTTPS Connection" (Jeff Moser, 2009) — a visual walkthrough of TLS with certificate chain verification, still the best intuition-builder written.
- "Bulletproof TLS and PKI" (Ivan Ristić, 2nd edition) — the comprehensive reference on TLS and PKI: protocol details, attacks, deployment best practices, and CT.
- "Certificate Transparency" (Ben Laurie et al., RFC 6962) — the original paper and standard; covers the Merkle tree design, log audit protocol, and threat model.
- "Let's Encrypt: An Automated Certificate Authority to Encrypt the Entire Web" (Aas et al., 2019, ACM CCS) — the paper describing Let's Encrypt's architecture, ACME protocol, and operational experience.
- **crt.sh** and **Censys Certificate Search** — search engines that aggregate CT log data; explore what certificates exist for your own domains.
