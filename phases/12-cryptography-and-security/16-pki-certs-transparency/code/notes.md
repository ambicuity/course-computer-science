# PKI Reference Notes

## X.509 Certificate Structure (RFC 5280)

```
Certificate
├── TBSCertificate (To Be Signed)
│   ├── Version (1, 2, or 3)
│   ├── Serial Number (unique per CA)
│   ├── Signature Algorithm (e.g., sha256WithRSAEncryption)
│   ├── Issuer (DN of signing CA)
│   ├── Validity
│   │   ├── notBefore
│   │   └── notAfter
│   ├── Subject (DN of certificate owner)
│   ├── Subject Public Key Info
│   │   ├── Algorithm (RSA, EC, etc.)
│   │   └── Key (modulus+exponent or curve point)
│   └── Extensions [v3 only]
│       ├── Authority Key Identifier
│       ├── Subject Key Identifier
│       ├── Key Usage
│       ├── Extended Key Usage
│       ├── Basic Constraints
│       ├── Subject Alternative Name (SAN)
│       ├── CRL Distribution Points
│       ├── Authority Information Access (AIA)
│       ├── Signed Certificate Timestamp (SCT)
│       └── ...
├── Signature Algorithm
└── Signature Value
```

## Common X.509 OIDs

| OID | Name | Purpose |
|-----|------|---------|
| 2.5.4.3 | commonName (CN) | Subject name (deprecated for hostname matching) |
| 2.5.4.6 | countryName (C) | Two-letter country code |
| 2.5.4.7 | localityName (L) | City/locality |
| 2.5.4.8 | stateOrProvinceName (ST) | State/province |
| 2.5.4.10 | organizationName (O) | Organization name |
| 2.5.4.11 | organizationalUnitName (OU) | Department/unit |
| 1.2.840.113549.1.1.1 | rsaEncryption | RSA public key |
| 1.2.840.10045.2.1 | ecPublicKey | EC public key |
| 1.2.840.10045.4.3.2 | ecdsa-with-SHA256 | ECDSA with SHA-256 |
| 1.2.840.113549.1.1.11 | sha256WithRSAEncryption | RSA with SHA-256 |
| 1.3.6.1.5.5.7.3.1 | serverAuth | TLS server auth |
| 1.3.6.1.5.5.7.3.2 | clientAuth | TLS client auth |
| 2.5.29.14 | subjectKeyIdentifier | Unique ID for this key |
| 2.5.29.15 | keyUsage | Digital sig, encipherment, etc. |
| 2.5.29.17 | subjectAltName | SAN — domain names |
| 2.5.29.19 | basicConstraints | CA:TRUE/FALSE |
| 2.5.29.31 | cRLDistributionPoints | Where to fetch CRL |
| 1.3.6.1.5.5.7.1.1 | authorityInfoAccess | Issuer cert URL + OCSP URL |
| 1.3.6.1.4.1.11129.2.4.2 | signedCertificateTimestampList | CT SCTs (Google) |

## Extension OID Meanings

| Extension OID | Name | Critical? | Meaning |
|---------------|------|-----------|---------|
| 2.5.29.15 | Key Usage | Usually yes | Restricts key to specific operations |
| 2.5.29.19 | Basic Constraints | Yes | CA:TRUE means can sign other certs |
| 2.5.29.17 | SAN | No | Lists valid DNS names/IPs |
| 2.5.29.37 | Extended Key Usage | No | Server auth, client auth, code signing |
| 2.5.29.31 | CRL Distribution Points | No | URLs for CRL download |
| 1.3.6.1.5.5.7.1.1 | Authority Info Access | No | AIA: issuer cert + OCSP URLs |

## Trust Store Locations

| OS | Location | Format |
|----|----------|--------|
| macOS | System Keychain (`/System/Library/Keychains/`) | .keychain |
| macOS | User Keychain (`~/Library/Keychains/`) | .keychain |
| Linux (Debian/Ubuntu) | `/etc/ssl/certs/` | PEM symlinks |
| Linux (Debian/Ubuntu) | `/usr/share/ca-certificates/` | PEM |
| Linux (Fedora/RHEL) | `/etc/pki/tls/certs/` | PEM |
| Linux (Arch) | `/etc/ca-certificates/extracted/` | PEM |
| Windows | `cert:\LocalMachine\Root` | Certificate Store |
| certifi (Python) | `certifi.where()` | PEM file path |

## Useful openssl Commands

```bash
# Inspect a PEM certificate
openssl x509 -in cert.pem -text -noout

# Inspect a DER certificate
openssl x509 -in cert.der -inform der -text -noout

# Fetch and inspect a server's certificate
openssl s_client -connect google.com:443 -showcerts </dev/null

# Get the full chain from a server
openssl s_client -connect google.com:443 -showcerts </dev/null 2>/dev/null

# Verify a certificate chain against a trust store
openssl verify -CAfile /etc/ssl/certs/ca-certificates.crt cert.pem

# Extract the public key from a certificate
openssl x509 -in cert.pem -pubkey -noout

# Convert PEM to DER
openssl x509 -in cert.pem -outform der -out cert.der

# Convert DER to PEM
openssl x509 -in cert.der -inform der -out cert.pem

# Generate a self-signed certificate (for testing)
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 365 -nodes

# Check certificate expiry dates
openssl x509 -in cert.pem -dates -noout

# Print the subject and issuer only
openssl x509 -in cert.pem -subject -issuer -noout

# Show certificate fingerprint
openssl x509 -in cert.pem -fingerprint -sha256 -noout

# Show ASN.1 structure (raw)
openssl asn1parse -in cert.pem

# Query OCSP responder
openssl ocsp -issuer issuer.pem -cert cert.pem -url "$(openssl x509 -in cert.pem -ocsp_uri -noout)"

# Decode a CSR
openssl req -in request.csr -text -noout
```

## Chain Validation Checklist

1. [ ] Each certificate (except root) has signature verified by next cert's public key
2. [ ] Each issuer has `CA:TRUE` in Basic Constraints
3. [ ] Each issuer's `pathLength` is not exceeded
4. [ ] All certificates are within their validity period
5. [ ] Leaf certificate has `CA:FALSE` in Basic Constraints
6. [ ] Leaf certificate's SAN matches the hostname
7. [ ] Key Usage is appropriate (digitalSignature for TLS, keyCertSign for CAs)
8. [ ] Certificate has sufficient SCTs (Chrome: 2 from different logs)
9. [ ] Certificate is not revoked (CRL or OCSP check — when possible)

## CT Log Structure (RFC 6962)

```
CT Log = append-only Merkle tree of certificates + SCTs

Log Entry = { leaf_cert, sct }
Leaf Hash = SHA-256(0x00 | MerkleTreeLeaf)
Node Hash = SHA-256(0x01 | left_hash | right_hash)

Audit:  Verify inclusion proof  (leaf → root)
Monitor: Watch for mis-issued certs
Gossip:  Cross-check root hashes between logs
```

## Certificate Lifecycle

```
1. CSR generation     (key pair created, CSR signed)
2. Validation          (CA verifies domain control / identity)
3. Issuance            (CA signs certificate, submits to CT logs)
4. Deployment          (cert installed on server)
5. TLS handshake       (cert presented to clients)
6. Revocation (if needed) — CRL + OCSP
7. Expiry              (cert auto-expires; must be renewed)
```
