# Dependency Reference Card

**Phase 16, Lesson 16 — Dependency Management & SemVer**

---

## SemVer Quick Reference

```
MAJOR.MINOR.PATCH
  │     │     └── Backward-compatible bug fixes
  │     └──────── Backward-compatible new features
  └────────────── Breaking changes / incompatible API changes
```

### Pre-release & Build Metadata

```
1.2.3-alpha.1    ← pre-release (unstable, < 1.2.3)
1.2.3+build.456  ← build metadata (ignored for ordering)
1.2.3-alpha.1+build.456  ← combined
```

---

## Version Range Syntax

| Range | Expands To | Accepts | Rejects |
|-------|-----------|---------|---------|
| `1.2.3` | `1.2.3` | Only 1.2.3 | Everything else |
| `^1.2.3` | `>=1.2.3 <2.0.0` | 1.2.3–1.99.99 | 2.0.0+ |
| `^0.2.3` | `>=0.2.3 <0.3.0` | 0.2.3–0.2.99 | 0.3.0+ |
| `^0.0.3` | `>=0.0.3 <0.0.4` | Only 0.0.3 | 0.0.4+ |
| `~1.2.3` | `>=1.2.3 <1.3.0` | 1.2.3–1.2.99 | 1.3.0+ |
| `~1.2` | `>=1.2.0 <1.3.0` | 1.2.0–1.2.99 | 1.3.0+ |
| `>=1.2.3` | `>=1.2.3` | 1.2.3+ | 1.2.2 and below |
| `<2.0.0` | `<2.0.0` | 0.x.x, 1.x.x | 2.0.0+ |
| `*` | `>=0.0.0` | Everything | Nothing |
| `1.x` | `>=1.0.0 <2.0.0` | 1.0.0–1.99.99 | 2.0.0+ |

### Caret vs Tilde — The Key Difference

```
^1.2.3  →  >=1.2.3  <2.0.0   (allows minor + patch bumps)
~1.2.3  →  >=1.2.3  <1.3.0   (allows only patch bumps)
```

Caret trusts MAJOR won't break you. Tilde trusts MAJOR + MINOR won't break you.

---

## Lockfile Comparison

| | package-lock.json | Cargo.lock | go.sum |
|---|---|---|---|
| **Ecosystem** | npm/Node.js | Rust/Cargo | Go |
| **Format** | JSON | TOML | Text |
| **Hashes** | SHA-512 | SHA-256 | SHA-256 |
| **Commit for apps?** | Yes | Yes | Yes |
| **Commit for libs?** | Debated | Usually no | Always |
| **CI install command** | `npm ci` | `cargo build --locked` | `go mod download` |

---

## CI Best Practices

```
# npm — always use ci in CI
npm ci                    # install from lockfile exactly

# Cargo — always use locked in CI
cargo build --locked       # fail if Cargo.lock is outdated

# Go — verify module checksums
go mod verify              # verify downloaded modules match go.sum
```

---

## Dependency Resolution Strategies

| Strategy | Ecosystem | Approach | Key Trait |
|----------|-----------|---------|-----------|
| Latest compatible | npm, Yarn, Pip | Pick newest in range | May be non-deterministic |
| Minimal version | Go (MVS) | Max of all minimums | Most conservative |
| SAT solving | Cargo, Pub (Dart) | Constraint satisfaction, backtrack | Thorough but can be slow |
| Nearest wins | Maven | Shallowest dependency wins | Silent downgrade risk |

---

## Diamond Problem Resolution

```
Scenario: App → A (needs C@^2.0.0), App → B (needs C@~2.5.0)

Step 1: Compute intersections
  ^2.0.0  →  >=2.0.0 <3.0.0
  ~2.5.0  →  >=2.5.0 <2.6.0

Step 2: Intersect the ranges
  >=2.5.0 <2.6.0  (= intersection)

Step 3: Resolve to latest in intersection
  C@2.5.9

Unresolvable: App → X (needs D@^1.0.0), App → Y (needs D@^2.0.0)
  ^1.0.0 ∩ ^2.0.0 = ∅ → ERROR
```

---

## Vulnerability Management Commands

```bash
# npm
npm audit                  # list vulnerabilities
npm audit fix              # auto-fix where possible
npm audit fix --force      # fix (may break compatibility)

# Cargo
cargo audit                # check RustSec advisory database

# Go
go mod graph | nancy sleuth # check Go vulnerabilities

# General: Dependabot (GitHub-native)
# Enable in repo Settings → Code security → Dependabot
```

---

## License Quick Reference

| License | Copyleft? | Use in Proprietary? | Key Restriction |
|---------|-----------|---------------------|-----------------|
| MIT | No | Yes | Include license text |
| Apache 2.0 | No | Yes | Include license + NOTICE + patent grant |
| BSD-3 | No | Yes | Include license, don't use author's name |
| GPL-3.0 | **Yes** | Only if your code is also GPL | Must share source on distribution |
| AGPL-3.0 | **Yes** (strong) | Only if your code is also AGPL | Must share source on network use |
| LGPL-3.0 | Weak | Yes (with conditions) | Must share modifications to LGPL code |
| Unlicense | No | Yes | Public domain, no restrictions |

---

## Vendoring Decision Tree

```
Need offline builds?  ──Yes──→  Vendor
Need to audit all source?  ──Yes──→  Vendor
Forking a dependency?  ──Yes──→  Vendor
Dependency is tiny & stable?  ──Yes──→  Consider vendoring
Normal web dev with good registry?  ──Yes──→  Use registry + lockfile
Building a library?  ──Yes──→  Use registry (don't vendor)
```

---

## Key Commands Cheat Sheet

```bash
# npm
npm install <pkg>           # install + update lockfile
npm ci                      # install from lockfile exactly
npm ls --depth=0            # list direct dependencies
npm outdated                # check for updates
npm audit                   # check for vulnerabilities
npx license-checker         # scan dependency licenses

# Cargo
cargo add <crate>           # add dependency
cargo build --locked        # build using Cargo.lock exactly
cargo tree                  # show dependency tree
cargo update                # update Cargo.lock within ranges
cargo audit                 # check for vulnerabilities
cargo vendor                # vendor all dependencies

# Go
go get <module>@v1.2.3      # add/upgrade dependency
go mod tidy                 # clean up go.mod and go.sum
go mod vendor                # vendor all dependencies
go mod verify                # verify checksums
govulncheck ./...           # check for Go vulnerabilities
```