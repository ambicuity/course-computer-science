# Web Security Testing Suite

**Author:** Phase 12 — Cryptography & Security, Lesson 21

## What It Is

A deliberately vulnerable web application (Express.js/TypeScript) and matching Python exploit scripts demonstrating five critical web vulnerability classes:

| # | Vulnerability | App Endpoint | Python Exploit |
|---|---------------|-------------|----------------|
| 1 | **Reflected XSS** | `GET /search-vuln?q=<script>...` | `xss_steal_cookie()` |
| 2 | **Stored XSS** | `POST /comment-vuln` → `GET /comments-vuln` | `xss_stored_comment()` |
| 3 | **SQL Injection** | `POST /login-vuln` (string concat) | `sqli_extract_table()`, `sqli_blind_extract()` |
| 4 | **CSRF** | `POST /change-password-vuln` (no token) | `csrf_attack()`, `csrf_serve_and_exploit()` |
| 5 | **SSRF** | `POST /fetch-url-vuln` (no validation) | `ssrf_probe()`, `ssrf_metadata()` |
| 6 | **Insecure Deserialization** | `POST /deserialize-vuln` | `pickle_rce_demo()` |

Each vulnerability is paired with a defended counterpart (`/search`, `/login`, `/change-password`, `/fetch-url`, `/deserialize`) showing proper mitigations.

## How to Use

### 1. Start the Vulnerable App

```bash
cd code
npm install
npx tsx main.ts
# Listening on http://localhost:4001
```

### 2. Run Exploit Scripts

```bash
cd code
pip install requests

# Run all exploits against the vulnerable server
python3 main.py all

# Or run individual exploits:
python3 main.py xss
python3 main.py csrf
python3 main.py sqli
python3 main.py ssrf
python3 main.py pickle

# Verify defenses (these should all PASS):
python3 main.py defend
```

### 3. Manual Exploration

- Reflected XSS: `http://localhost:4001/search-vuln?q=<script>alert(1)</script>`
- Stored XSS: `POST` a comment with `<script>...</script>` to `/comment-vuln`, then visit `/comments-vuln`
- SQLi: `POST` to `/login-vuln` with `username=admin' OR '1'='1' --`
- CSRF: Visit `http://localhost:8888/` (after running `python3 main.py csrf`) while logged in
- SSRF: `POST` to `/fetch-url-vuln` with JSON `{"url": "http://169.254.169.254/latest/meta-data/"}`

## What Each Exploit Demonstrates

| Exploit | Core technique | Defense |
|---------|---------------|---------|
| XSS steal cookie | Inject `<script>` that reads `document.cookie` | CSP + HTML entity encoding |
| CSRF auto-submit | Cross-origin form auto-submit steals cookie auth | CSRF token + SameSite=Strict |
| SQLi UNION extract | Break out of string, UNION SELECT other tables | Parameterized queries |
| SSRF metadata probe | Server fetches internal cloud metadata URLs | URL allowlist + IP validation |
| Pickle RCE | `__reduce__` returns `os.system()` | JSON only + signed data |

## Connection to Capstone

The Phase 12 capstone (Lesson 24: TLS 1.3 Library + Mini-CTF) includes web exploitation challenges. This suite provides:
- A reference vulnerable application that can serve as a CTF target.
- Exploit scripts that can be adapted to other challenge servers.
- Defended implementations that demonstrate proper secure coding patterns.

## Limitations

- The vulnerable app runs on localhost only. Do not expose it to a network — the intentional vulnerabilities make it dangerous in untrusted environments.
- The CSRF demo requires manual browser interaction (the victim must be logged in and visit the CSRF page).
- Cloud metadata SSRF probes will only succeed on actual cloud instances (AWS EC2, GCP Compute Engine, Azure VM). On a local machine, the requests will time out or be rejected.
- The pickle RCE demo demonstrates the concept on the local Python process; it does not target the Express.js server.