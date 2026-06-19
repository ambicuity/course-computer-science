#!/usr/bin/env bash
set -euo pipefail

LESSON="Dependency Management & SemVer"
PHASE=16
LESSON_NUM=16

echo "============================================================"
echo "  $LESSON (Phase $PHASE, Lesson $LESSON_NUM)"
echo "============================================================"
echo ""

# ─── Section 1: SemVer Parsing & Comparison ────────────────────
echo "── Section 1: SemVer Parsing & Comparison ──────────────────"
echo ""

semver_parse() {
    local version="$1"
    local major minor patch prerelease
    
    if [[ "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-[a-zA-Z0-9._]+)?(\+[a-zA-Z0-9._]+)?$ ]]; then
        major="${BASH_REMATCH[1]}"
        minor="${BASH_REMATCH[2]}"
        patch="${BASH_REMATCH[3]}"
        prerelease="${BASH_REMATCH[4]:-}"
        echo "Version: $version"
        echo "  Major: $major"
        echo "  Minor: $minor"
        echo "  Patch: $patch"
        echo "  Pre-release: ${prerelease:-none}"
    else
        echo "ERROR: '$version' is not valid SemVer" >&2
        return 1
    fi
}

semver_compare() {
    local v1="$1" v2="$2"
    local m1 n1 p1 m2 n2 p2
    
    IFS='.' read -r m1 n1 p1 <<< "$(echo "$v1" | sed 's/-.*//')"
    IFS='.' read -r m2 n2 p2 <<< "$(echo "$v2" | sed 's/-.*//')"
    
    if (( m1 > m2 )); then echo "$v1 > $v2"; return
    elif (( m1 < m2 )); then echo "$v1 < $v2"; return; fi
    
    if (( n1 > n2 )); then echo "$v1 > $v2"; return
    elif (( n1 < n2 )); then echo "$v1 < $v2"; return; fi
    
    if (( p1 > p2 )); then echo "$v1 > $v2"; return
    elif (( p1 < p2 )); then echo "$v1 < $v2"; return; fi
    
    echo "$v1 == $v2"
}

echo "Parsing SemVer versions:"
for v in "2.3.1" "0.4.0" "1.0.0-alpha.1" "3.12.456-beta+build.99"; do
    semver_parse "$v"
    echo ""
done

echo "Comparing versions:"
semver_compare "2.3.1" "2.3.0"
semver_compare "1.0.0" "2.0.0"
semver_compare "3.5.10" "3.5.10"
echo ""

# ─── Section 2: Version Range Checking ─────────────────────────
echo "── Section 2: Version Range Checking ──────────────────────"
echo ""

check_caret() {
    local range_major range_minor range_patch version="$2"
    IFS='.' read -r range_major range_minor range_patch <<< "$(echo "$1" | tr -d '^')"
    local v_major v_minor v_patch
    IFS='.' read -r v_major v_minor v_patch <<< "$(echo "$version" | sed 's/-.*//')"
    
    if (( range_major > 0 )); then
        (( v_major == range_major )) && (( v_minor > range_minor || (v_minor == range_minor && v_patch >= range_patch) ))
    elif (( range_minor > 0 )); then
        (( v_major == 0 )) && (( v_minor == range_minor )) && (( v_patch >= range_patch ))
    else
        (( v_major == 0 )) && (( v_minor == 0 )) && (( v_patch == range_patch ))
    fi
}

check_tilde() {
    local range_major range_minor range_patch version="$2"
    IFS='.' read -r range_major range_minor range_patch <<< "$(echo "$1" | tr -d '~')"
    local v_major v_minor v_patch
    IFS='.' read -r v_major v_minor v_patch <<< "$(echo "$version" | sed 's/-.*//')"
    
    (( v_major == range_major )) && (( v_minor == range_minor )) && (( v_patch >= range_patch ))
}

check_gte() {
    local range_version="${1#>=}"
    local version="$2"
    local rm rn rp vm vn vp
    IFS='.' read -r rm rn rp <<< "$range_version"
    IFS='.' read -r vm vn vp <<< "$(echo "$version" | sed 's/-.*//')"
    
    (( vm > rm )) || (( vm == rm && vn > rn )) || (( vm == rm && vn == rn && vp >= rp ))
}

versions=("2.3.1" "2.3.5" "2.4.0" "3.0.0" "1.9.9")

echo "Which versions match ^2.3.1?"
for v in "${versions[@]}"; do
    if check_caret "^2.3.1" "$v"; then
        echo "  ✓ $v"
    else
        echo "  ✗ $v"
    fi
done
echo ""

echo "Which versions match ~2.3.1?"
for v in "${versions[@]}"; do
    if check_tilde "~2.3.1" "$v"; then
        echo "  ✓ $v"
    else
        echo "  ✗ $v"
    fi
done
echo ""

echo "Which versions match >=2.3.1?"
for v in "${versions[@]}"; do
    if check_gte ">=2.3.1" "$v"; then
        echo "  ✓ $v"
    else
        echo "  ✗ $v"
    fi
done
echo ""

# ─── Section 3: Dependency Tree & Diamond Problem ──────────────
echo "── Section 3: Dependency Tree & Diamond Problem ───────────"
echo ""

build_dep_tree() {
    echo "Your App"
    echo "├── express@^4.18.0"
    echo "│   ├── accepts@^1.3.8"
    echo "│   ├── body-parser@^1.20.1"
    echo "│   ├── cookie@^0.5.0"
    echo "│   └── qs@^6.11.0"
    echo "├── lodash@^4.17.21"
    echo "└── axios@^1.6.0"
    echo "    ├── follow-redirects@^1.15.0"
    echo "    └── form-data@^4.0.0"
}

echo "Sample dependency tree:"
build_dep_tree
echo ""

echo "Diamond problem example:"
echo ""
echo "  Your App"
echo "  ├── A@^1.0.0 ─── requires C@^2.0.0"
echo "  └── B@^1.0.0 ─── requires C@~2.5.0"
echo ""
echo "  C constraints from both paths:"
echo "    ^2.0.0  →  >=2.0.0 <3.0.0"
echo "    ~2.5.0  →  >=2.5.0 <2.6.0"
echo ""
echo "  Intersection: >=2.5.0 <2.6.0"
echo "  Latest in range: C@2.5.9"
echo ""

echo "Unresolvable diamond:"
echo ""
echo "  Your App"
echo "  ├── X@^1.0.0 ─── requires D@^1.0.0"
echo "  └── Y@^1.0.0 ─── requires D@^2.0.0"
echo ""
echo "  ^1.0.0 intersects ^2.0.0? NO — no single version matches both"
echo "  Resolution: ERROR (unresolvable conflict)"
echo ""

# ─── Section 4: Lockfile Generation Simulation ─────────────────
echo "── Section 4: Lockfile Generation Simulation ─────────────"
echo ""

generate_npm_lockfile() {
    cat <<'LOCKFILE'
{
  "name": "demo-app",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": {
      "name": "demo-app",
      "version": "1.0.0",
      "dependencies": {
        "express": "^4.18.0",
        "lodash": "^4.17.21"
      }
    },
    "node_modules/express": {
      "version": "4.18.2",
      "resolved": "https://registry.npmjs.org/express/-/express-4.18.2.tgz",
      "integrity": "sha512-5apI8lR4W7INhzJM6njIiDvF3OqrL/4U8rm6BAIvP3KN3oOEd3jVoG6S+3Y0mS3xYySAYI0G0g3y3JxWGS7dw==",
      "dependencies": {
        "accepts": "~1.3.8",
        "qs": "6.11.0"
      }
    },
    "node_modules/lodash": {
      "version": "4.17.21",
      "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
      "integrity": "sha512-veL2lEdG3P1Ja5o0gZ9ZxJ2E4N9L0O0LJLa2xZ5OmJycVL8WNWJiG2aL7aG7nFL1P3x03U9jH8pv9XJk7YpW7A=="
    },
    "node_modules/qs": {
      "version": "6.11.0",
      "resolved": "https://registry.npmjs.org/qs/-/qs-6.11.0.tgz",
      "integrity": "sha512-M5Ylj6zB68QGH8ApLSY3K0g1EDSOT8lE2YgiY7B0CGUYZzX+Z6t3y4UjoR5dOZ3i0Edzy0hpnC2pG7NaIj3/YA=="
    }
  }
}
LOCKFILE
}

generate_cargo_lock() {
    cat <<'LOCKFILE'
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 3

[[package]]
name = "demo-app"
version = "0.1.0"
dependencies = [
    "serde",
    "tokio",
]

[[package]]
name = "serde"
version = "1.0.171"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a8ab9af32671192d6a49dac173f00c4a5c2e3a55de8e7e1a9a3a7ff5c6a6bc52"

[[package]]
name = "tokio"
version = "1.29.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a8af4e416c43811ab5b00907682b44e6be3ab7f5b3d79d2e4f09d5e6fdd5afe3"
dependencies = [
    "bytes",
]

[[package]]
name = "bytes"
version = "1.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "89b5398110b0b78d4bbe0f3e7ab277e4aa3f8eb11c0a4c0e9ea8a6791e8f0de0"
LOCKFILE
}

generate_go_sum() {
    cat <<'LOCKFILE'
github.com/gin-gonic/gin v1.9.0 h1:4+fr/el88TOO3ewCmQr8cx/SnFR9P2K9Ej5Cg6g8eYc=
github.com/gin-gonic/gin v1.9.0/go.mod h1:h8k0z8eQ6+U8m4L9K7E3nH3Of4S3GIb3T7f5h5lj5A=
golang.org/x/crypto v0.9.0 h1:LU8kQ0vM9J0k6b7yjp7v+9gkCfUu3P3c0gI0k1L8Jj0=
golang.org/x/crypto v0.9.0/go.mod h1:0gI0bJ8b1U9K2r0T3r8a0qFzG9A8L8a0U0kqR5W0qE=
golang.org/x/net v0.10.0 h1:z0Q0Q8x8W5r5rr6CtTTIKSbD5rr6JxU5rr6N5o2xX8=
golang.org/x/net v0.10.0/go.mod h1:0pQ0Q0QZz0Z0Z0Z0Z0Z0=0pQ0Q0QZz0Z0Z0Z0Z0Z0=
LOCKFILE
}

echo "Simulated package-lock.json:"
echo "---"
generate_npm_lockfile | head -25
echo "..."
echo ""

echo "Simulated Cargo.lock:"
echo "---"
generate_cargo_lock
echo ""

echo "Simulated go.sum:"
echo "---"
generate_go_sum
echo ""

# ─── Section 5: Vulnerability Audit Simulation ────────────────
echo "── Section 5: Vulnerability Audit Simulation ─────────────"
echo ""

audit_package() {
    local name="$1" version="$2" severity="$3" advisory="$4"
    echo "[$severity] $name@$version — $advisory"
}

echo "Simulated npm audit output:"
echo ""
audit_package "lodash" "4.17.20" "HIGH" "Prototype Pollution in lodash (CVE-2020-8203)"
audit_package "express" "4.17.0" "MEDIUM" "Open Redirect in express (CVE-2020-8891)"
audit_package "minimist" "1.2.5" "LOW" "Prototype Pollution in minimist (CVE-2020-7598)"
echo ""

SEVERITY_COUNTS="9 Regular, 4 Informational, 0 Low, 1 Moderate, 2 High, 0 Critical"
echo "Severity summary: $SEVERITY_COUNTS"
echo ""

echo "Resolution commands:"
echo "  npm audit fix                # Apply safe fixes automatically"
echo "  npm audit fix --force        # Apply fixes (may break compat)"
echo "  npm update lodash            # Update specific package"
echo "  cargo audit                  # Rust equivalent"
echo ""

# ─── Section 6: License Scanning ───────────────────────────────
echo "── Section 6: License Scanning ──────────────────────────────"
echo ""

scan_license() {
    local pkg="$1" license="$2" copyleft="$3"
    printf "  %-25s %-12s %s\n" "$pkg" "$license" "$copyleft"
}

echo "Dependency license summary:"
printf "  %-25s %-12s %s\n" "PACKAGE" "LICENSE" "COPYLEFT?"
echo "  ─────────────────────────────────────────────────────"
scan_license "express" "MIT" "No"
scan_license "lodash" "MIT" "No"
scan_license "axios" "MIT" "No"
scan_license "sequelize" "MIT" "No"
scan_license "some-gpl-lib" "GPL-3.0" "YES ⚠"
scan_license "another-agpl" "AGPL-3.0" "YES ⚠"
echo ""
echo "⚠  GPL and AGPL packages require careful review."
echo "   GPL-3.0: Must distribute source if you distribute the binary."
echo "   AGPL-3.0: Must distribute source if users interact over network."
echo ""

# ─── Section 7: Resolution Strategy Demo ───────────────────────
echo "── Section 7: Resolution Strategy Comparison ──────────────"
echo ""

resolve_latest_compatible() {
    echo "Latest Compatible (npm/Yarn):"
    echo "  Constraints: C@^2.0.0 (from A), C@~2.5.0 (from B)"
    echo "  Intersection: >=2.5.0 <2.6.0"
    echo "  Available: 2.5.0, 2.5.1, 2.5.3, 2.5.9"
    echo "  Selected: C@2.5.9 (latest in intersection)"
}

resolve_minimal_version() {
    echo "Minimal Version Selection (Go MVS):"
    echo "  Requirements: A requires C@>=2.0.0, B requires C@>=2.5.0"
    echo "  Max of minimums: max(2.0.0, 2.5.0) = 2.5.0"
    echo "  Selected: C@2.5.0 (even though 2.5.9 exists)"
}

resolve_sat_backtrack() {
    echo "SAT-Based with Backtracking (Cargo):"
    echo "  A@1.5.0 requires D@^2.0.0"
    echo "  A@1.3.0 requires D@^1.0.0"
    echo "  B@1.0.0 requires D@~1.5.0"
    echo ""
    echo "  Try A@1.5.0 → D needs ^2.0.0 → conflicts with B's ~1.5.0 ✗"
    echo "  Backtrack → try A@1.3.0 → D needs ^1.0.0 → intersects B's ~1.5.0"
    echo "  Intersection: >=1.5.0 <1.6.0 → D@1.5.x ✓"
}

resolve_latest_compatible
echo ""
resolve_minimal_version
echo ""
resolve_sat_backtrack
echo ""

# ─── Section 8: npm ci vs npm install ──────────────────────────
echo "── Section 8: npm ci vs npm install ────────────────────────"
echo ""

printf "  %-22s %-30s %-30s\n" "Property" "npm install" "npm ci"
echo "  ────────────────────────────────────────────────────────────────────────"
printf "  %-22s %-30s %-30s\n" "Uses lockfile?" "Optional (may create)" "Required (must exist)"
printf "  %-22s %-30s %-30s\n" "Modifies lockfile?" "Yes" "No (fails if mismatch)"
printf "  %-22s %-30s %-30s\n" "Resolution" "From ranges" "From lockfile exactly"
printf "  %-22s %-30s %-30s\n" "Deterministic?" "Not guaranteed" "Yes"
printf "  %-22s %-30s %-30s\n" "Deletes node_modules?" "No" "Yes (clean slate)"
printf "  %-22s %-30s %-30s\n" "Speed (cold)" "Slower" "Faster (no resolution)"
printf "  %-22s %-30s %-30s\n" "Use in CI?" "No" "Yes"
echo ""

echo "Recommended workflow:"
echo "  1. Local dev:       npm install           # resolve + update lockfile"
echo "  2. Commit:          git add package-lock.json"
echo "  3. CI:              npm ci               # install from lockfile exactly"
echo ""

# ─── Section 9: Vendoring vs Registry ─────────────────────────
echo "── Section 9: Vendoring vs Registry ────────────────────────"
echo ""

echo "Registry-based (default):"
echo "  npm install lodash          # downloads from npmjs.com"
echo "  cargo add serde             # downloads from crates.io"
echo "  go get github.com/gin-gonic/gin  # downloads from proxy.golang.org"
echo ""

echo "Vendoring commands:"
echo "  go mod vendor               # copies all deps into ./vendor/"
echo "  cargo vendor > .cargo/config.toml  # copies into ./vendor/"
echo "  npm config set prefer-offline true  # cache, don't re-download"
echo ""

echo "When to vendor:"
echo "  ✓ Air-gapped/offline environments"
echo "  ✓ Embedded systems with no runtime network"
echo "  ✓ Auditing all source code requirement"
echo "  ✓ Forking a dependency with local modifications"
echo "  ✗ Normal web development with reliable registries"
echo "  ✗ Published libraries (let consumers resolve)"
echo ""

# ─── Summary ───────────────────────────────────────────────────
echo "============================================================"
echo "  Summary"
echo "============================================================"
echo ""
echo "  1. SemVer encodes compatibility: MAJOR.MINOR.PATCH"
echo "  2. Caret (^) trusts major; Tilde (~) trusts major+minor"
echo "  3. Lockfiles pin exact versions for reproducibility"
echo "  4. npm ci > npm install for CI/CD determinism"
echo "  5. Resolution is constraint satisfaction (SAT/MVS/latest)"
echo "  6. Diamond problem: incompatible transitive deps"
echo "  7. Always audit for vulnerabilities (npm audit, cargo audit)"
echo "  8. Always check licenses (GPL is a copyleft virus)"
echo "  9. Vendor when you need offline/audit builds"
echo " 10. Pin in lockfiles, float in manifests, update via Dependabot"
echo ""
echo "Done."