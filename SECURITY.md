# Security Policy

This is an educational repository: a computer-science curriculum plus a static
website. It has no backend, accounts, or user data. Even so, we take security
reports seriously — most notably for the website, the build tooling, and any
runnable lesson code.

## Supported versions

The `main` branch (and the deployed site at
`https://course-computer-science.riteshrana.engineer`) is the only supported
version. Fixes land on `main`; there are no long-lived release branches.

## Reporting a vulnerability

**Please do not open a public issue for a security problem.**

Preferred: use GitHub's private reporting —
**Security → Advisories → "Report a vulnerability"** on this repository
([new advisory](https://github.com/ambicuity/course-computer-science/security/advisories/new)).

Alternatively, email **contact@riteshrana.engineer** with:

- a description of the issue and its impact,
- steps to reproduce (or a proof of concept),
- the affected page, file, or lesson path.

## What to expect

- We aim to acknowledge a report within **5 business days**.
- We'll confirm the issue, agree on a fix and timeline, and credit you in the
  fix (unless you prefer to remain anonymous).
- Please give us a reasonable window to remediate before any public disclosure.

## In scope

- The static site under `site/` (XSS, unsafe `innerHTML`, injection via
  rendered Markdown, dependency issues).
- Build and CI tooling (`scripts/`, `site/build.js`, GitHub Actions workflows).
- Runnable lesson code that could harm a reader running it locally.

## Out of scope

- Vulnerabilities in third-party services (GitHub Pages, the DNS/CDN provider).
- Findings that require a already-compromised machine or browser.
- Missing security headers that are intentionally delegated to the host/CDN.
