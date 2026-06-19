# Threat Modeling Reference

## STRIDE Per Element Cheat Sheet

| Element Type | Spoofing | Tampering | Repudiation | Info Disclosure | DoS | EoP |
|-------------|----------|-----------|-------------|-----------------|-----|-----|
| External Entity | Authentication required | N/A | Non-repudiation (signing) | N/A | N/A | N/A |
| Process | Authenticate callers | Integrity of code/data | Audit logging | Encrypt output | Resource limits | Authorization checks |
| Data Store | Authenticate access | Integrity checks (checksums) | Audit trail (WAL) | Encrypt at rest | Redundancy, backup | N/A |
| Data Flow | Mutual auth (mTLS) | Tamper detection in transit | Origin proof (signatures) | Encrypt channel (TLS) | Connection resilience | N/A |

## DREAD Score Sheet Template

| Threat | D | R | E | A | D | Total | Priority |
|--------|---|---|---|---|---|-------|----------|
| (threat description) | 1-3 | 1-3 | 1-3 | 1-3 | 1-3 | /15 | HIGH/MED/LOW |

### Scoring Guide

| Score | Damage Potential | Reproducibility | Exploitability | Affected Users | Discoverability |
|-------|-----------------|-----------------|----------------|----------------|-----------------|
| 1 | Limited data exposure | Very hard, race condition | Local access / extensive resources | None or single user | Requires source access |
| 2 | Major data loss / degradation | Complex multi-step | Network access, moderate skill | Subset of users | Scanning tools detect |
| 3 | Full compromise / destruction | One click, always works | Unauthenticated, low skill | All users | Publicly known CVE |

### Priority Thresholds

| Score | Priority | Action |
|-------|----------|--------|
| 12-15 | CRITICAL | Fix immediately, block release |
| 9-11 | HIGH | Fix in current sprint |
| 5-8 | MEDIUM | Fix in next iteration |
| Below 5 | LOW | Accept risk or backlog |

## Attack Tree Template (ASCII)

```
GOAL: [describe attacker's ultimate goal — one imperative sentence]
├── OR: [method 1 — any one sub-path is sufficient]
│   ├── [specific action 1]
│   │   ├── AND: [sub-goal — all children must hold]
│   │   └── AND: [sub-goal]
│   └── [specific action 2]
├── OR: [method 2]
│   └── AND: [sub-goal]
│       ├── [leaf node — concrete action or condition]
│       └── [leaf node]
└── OR: [method 3]
    └── [leaf node]

Legend:
├── OR branch = any one child satisfies the parent (logical disjunction)
├── AND branch = all children must be satisfied (logical conjunction)
└── Leaf = primitive action or condition (no further decomposition)
```

## DFD Notation Reference

```
+----------+     +----------+     +----------+
| External |---->| Process  |---->|   Data   |
|  Entity  |     |          |     |   Store  |
+----------+     +----------+     +----------+

Shapes:
  Rectangle   = External Entity (user, third-party system)
  Circle/ellipse = Process (web server, worker, service)
  Parallel lines = Data Store (database, cache, filesystem)
  Arrow       = Data Flow (direction of movement)
  Dashed line = Trust Boundary

Trust boundaries:
  Every dashed line is a security invariant boundary.
  Data crossing a boundary changes trust level.
  Apply STRIDE more aggressively at boundary crossings.
```

## Completed DFD — Note-Taking Web App

```
+----------+     HTTPS (TLS 1.3)   +------------+     SQL (TCP/3306)   +----------+
|  Browser |──────────────────────>| Web Server |─────────────────────>| Database |
|  (User)  |<──────────────────────|  (Node.js) |<─────────────────────| (Postgres)|
+----------+                       +------------+                      +----------+
                                       |    ^
                                       |    |
                                       |    | SQL (TCP/6379)
                                       v    |
                                    +-----------+
                                    |  Redis    |
                                    |  Session  |
                                    |  Store    |
                                    +-----------+

Trust Boundary ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
  [Public internet]                          [Private subnet]
```

## Completed STRIDE Worksheet — Note-Taking Web App

| Element | Threat Type | Threat Description | Mitigation |
|---------|-------------|-------------------|------------|
| User (External Entity) | Spoofing | Attacker impersonates user via credential stuffing or session hijacking | MFA, rate-limit login attempts, HttpOnly+Secure cookies |
| Web Server (Process) | Spoofing | MITM attacker presents rogue cert, impersonates server | Certificate pinning, HSTS preload, valid CA chain |
| Web Server (Process) | Tampering | Attacker intercepts and modifies request body (downgrade to HTTP) | Enforce HSTS, redirect HTTP→301→HTTPS, CSP upgrade-insecure-requests |
| Web Server (Process) | Repudiation | User performs action and denies it; no audit trail | Structured audit log (user_id, action, timestamp, IP), immutable log store |
| Web Server (Process) | Info Disclosure | Server leaks stack trace with DB credentials in error response | Structured error responses, no debug info in production, secrets in vault |
| Web Server (Process) | DoS | Attacker sends 100k req/s, exhausts connection pool | Rate limiter (token bucket), auto-scaling group, Cloudflare/WAF |
| Web Server (Process) | EoP | Authenticated user accesses /admin/users without admin role | RBAC middleware on every protected route, deny by default |
| Database (Data Store) | Tampering | SQL injection in /api/notes?id=1 UNION SELECT ... | Parameterized queries (PreparedStatement), ORM with input sanitization |
| Database (Data Store) | Info Disclosure | Database credentials in .env file committed to GitHub | .gitignore, secret scanning pre-commit hook, vault-injected env vars |
| Database (Data Store) | DoS | Attacker runs SELECT * FROM notes (full table scan, billions of rows) | Query timeout, row limits, read replicas for heavy queries |
| Session Store (Data Store) | Tampering | Session fixation: attacker sets victim's session ID to known value | Regenerate session ID on login, validate session origin |
| Session Store (Data Store) | Info Disclosure | Session token logged in access log via Referer header | HttpOnly flag, no sensitive data in URL, strip session from logs |
| HTTPS (Data Flow) | Spoofing | DNS poisoning directs traffic to attacker's server | DNSSEC, CAA records, certificate transparency monitoring |
| HTTPS (Data Flow) | Tampering | Downgrade attack via SSLstrip | HSTS preload list, HPKP (deprecated — use Expect-CT instead) |
| SQL (Data Flow) | Spoofing | Attacker MiTM between web server and DB | mTLS between app and database, VPC peering, no public DB endpoint |
| SQL (Data Flow) | Info Disclosure | Query returns rows from other tenants (no tenant_id filter) | Row-level security (RLS) in PostgreSQL, tenant isolation via schema |

## Completed Attack Tree — "Read another user's notes"

```
GOAL: Read another user's notes
├── OR: SQL injection on notes endpoint
│   ├── AND: Find injectable parameter (notes GET parameter 'id')
│   ├── AND: Bypass input filter (WAF, parameterized query check)
│   └── AND: Craft UNION SELECT to extract other users' note content
├── OR: Session hijacking
│   ├── AND: Steal session cookie
│   │   ├── OR: Cross-Site Scripting (XSS) — inject script that reads document.cookie
│   │   │   ├── AND: Find stored XSS in note content
│   │   │   └── AND: Victim visits note page with injected script
│   │   ├── OR: Intercept unencrypted connection (FAIL — HTTPS mandatory)
│   │   └── OR: Read cookie from access log (FAIL — HttpOnly flag set)
│   └── AND: Use stolen cookie to impersonate victim session
├── OR: Insecure Direct Object Reference (IDOR)
│   └── AND: Guess or enumerate note IDs in /notes/{id}
│       └── AND: Endpoint returns note without ownership check
├── OR: SSRF to access database directly
│   ├── AND: Find server-side endpoint that makes requests to user-supplied URLs
│   └── AND: Point SSRF to internal database API or network share
└── OR: Database backup leak
    ├── AND: Backup file exposed on public web server (.sql, .dump)
    └── AND: Read backup containing plaintext note data
```

## Completed DREAD Scoring — Note-Taking Web App

| Threat Path | D | R | E | A | D | Total | Priority |
|-------------|---|---|---|---|---|-------|----------|
| SQL injection | 3 | 2 | 2 | 1 | 3 | 11/15 | HIGH |
| XSS → cookie theft → session hijack | 3 | 3 | 2 | 1 | 2 | 11/15 | HIGH |
| IDOR — guess note IDs without ownership check | 3 | 2 | 2 | 1 | 2 | 10/15 | HIGH |
| Session fixation | 2 | 3 | 2 | 1 | 2 | 10/15 | HIGH |
| SSRF → internal database | 3 | 1 | 1 | 1 | 1 | 7/15 | MEDIUM |
| Database backup leak | 3 | 1 | 1 | 3 | 1 | 9/15 | MEDIUM |
| DoS via request flood | 2 | 3 | 2 | 3 | 2 | 12/15 | CRITICAL |

## Action Plan (Priority-Ordered)

1. **CRITICAL (DoS — 12/15):** Rate limiting, auto-scaling, WAF, DDoS protection (Cloudflare/AWS Shield)
2. **HIGH (SQL injection — 11/15):** Parameterized queries everywhere, ORM with safe defaults, SQL linter in CI
3. **HIGH (XSS — 11/15):** Output encoding (context-aware), Content-Security-Policy header, DOMPurify on user HTML
4. **HIGH (IDOR — 10/15):** Ownership check middleware on all user-scoped endpoints, unit test for access control
5. **HIGH (Session fixation — 10/15):** Regenerate session ID on login, validate session at each privileged action
6. **MEDIUM (Backup leak — 9/15):** Restrict backup directory with .htaccess/nftables, encrypt backup files, monitor public access
7. **MEDIUM (SSRF — 7/15):** URL allowlist in HTTP client, block private IP ranges (RFC 1918), no internal service access via user input

## Trust Boundary Checklist

Use this checklist when drawing DFDs to make sure you have not missed any trust boundaries:

- [ ] Internet ↔ Internal network (the classic boundary)
- [ ] DMZ ↔ Internal services (web server ↔ database)
- [ ] Web tier ↔ Data tier (app ↔ database)
- [ ] App tier ↔ Caching tier (app ↔ redis/memcached)
- [ ] Container ↔ Host (namespace boundaries)
- [ ] Internal network ↔ Third-party API (outbound)
- [ ] Third-party ↔ Internal network (inbound webhook)
- [ ] User ↔ Application (every user input is a crossing)
- [ ] Admin interface ↔ Regular interface (separate trust zones)
- [ ] Build/CI pipeline ↔ Production environment
- [ ] Development ↔ Production (credentials, data)
- [ ] Encrypted ↔ Unencrypted (in transit, at rest)

## Security Requirements Checklist (by STRIDE Category)

### Authentication (Anti-Spoofing)
- [ ] Every entry point authenticates the caller
- [ ] Credentials are never transmitted in plaintext
- [ ] Session tokens are cryptographically random and tied to IP/user-agent
- [ ] MFA is enforced for privileged actions

### Integrity (Anti-Tampering)
- [ ] All data in transit is TLS-encrypted
- [ ] Database writes use parameterized queries (no string concatenation)
- [ ] Configuration files and binaries have checksums/signatures
- [ ] Immutable audit log for all state-changing operations

### Non-Repudiation (Anti-Repudiation)
- [ ] Every user action is logged with user_id, timestamp, and action
- [ ] Logs are append-only and cannot be modified
- [ ] Cryptographic signing for sensitive operations

### Confidentiality (Anti-Info Disclosure)
- [ ] All data at rest is encrypted (AES-256)
- [ ] All data in transit is encrypted (TLS 1.3)
- [ ] Access control prevents unauthorized data access
- [ ] Secrets are not in source code, environment files, or error messages

### Availability (Anti-DoS)
- [ ] Rate limiting on all public endpoints
- [ ] Connection pooling and timeout configuration
- [ ] Auto-scaling for load spikes
- [ ] Redundant infrastructure (multi-AZ, multi-region)

### Authorization (Anti-EoP)
- [ ] Least privilege principle for all roles
- [ ] RBAC/ABAC enforced at every protected endpoint
- [ ] Admin interfaces are on separate networks or require separate authentication
- [ ] No direct object references without ownership verification

## Common Security Requirements (Quick Reference)

| Requirement | Prevents | Implementation |
|------------|----------|----------------|
| Authentication | Spoofing | OAuth 2.0, OIDC, SAML, client certs, MFA |
| Authorization | EoP | RBAC (Casbin, OPA), ABAC, ACL |
| Integrity | Tampering | HMAC, digital signatures, checksums |
| Confidentiality | Info Disclosure | TLS, AES-256-GCM, envelope encryption |
| Non-repudiation | Repudiation | Signed audit logs, blockchain-based logging |
| Availability | DoS | Rate limiting (token bucket), circuit breaker, redundancy |

## Glossary of Threat Modeling Terms

| Term | Definition |
|------|------------|
| Asset | Something of value that needs protection (data, system, reputation) |
| Threat | A potential cause of an unwanted incident (the "what if") |
| Vulnerability | A weakness that can be exploited by a threat |
| Risk | The potential for loss when a threat exploits a vulnerability |
| Control | A safeguard that mitigates risk (technical, administrative, physical) |
| Mitigation | A specific action or control that reduces risk to an acceptable level |
| Countermeasure | A control that directly opposes a specific threat |
| Attack surface | The sum of all points where an attacker can enter or extract data |
| Attack vector | The specific path or method used by an attacker |
| Zero-day | A vulnerability unknown to the vendor with no available patch |
| Threat actor | The entity behind an attack (nation-state, criminal, insider, hacktivist) |
| Security requirement | A statement of what the system must guarantee to be secure |
| Trust zone | A region of the DFD where all entities share the same trust level |
| Threat model | The complete output: DFD + STRIDE list + attack trees + risk scores |
