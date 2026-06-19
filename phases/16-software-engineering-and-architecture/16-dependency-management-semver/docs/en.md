# Dependency Management & SemVer

> Your project is only as reliable as your weakest dependency — and you have more dependencies than you think.

**Type:** Learn
**Languages:** Markdown, Shell
**Prerequisites:** Phase 16 lessons 01–15
**Time:** ~45 minutes
**_meta:** { "phase": 16, "lesson": 16 }

## Learning Objectives

- Explain what dependencies are (and aren't) and why versioning matters for reproducibility.
- Decode semantic version numbers (major.minor.patch) and predict compatibility from version changes.
- Read and write version range syntax (^, ~, >=, *, etc.) and explain what each accepts.
- Explain why lockfiles exist and compare package-lock.json, Cargo.lock, and go.sum.
- Describe dependency resolution as constraint satisfaction (SAT solving, backtracking).
- Diagnose the diamond problem in transitive dependency graphs.
- Apply vulnerability management tools (npm audit, cargo audit, Dependabot, Snyk).
- Compare vendoring vs registry-based dependency management.
- Make informed decisions about pinning vs floating dependencies.
- Identify license compliance risks (GPL, MIT, Apache) in your dependency tree.

## The Problem

You build an app. It works on your machine. You ship it. It breaks in production. Why?

The app depends on library A version 2.3.0. Library A depends on library B version ^1.2.0. Meanwhile, library C — also a dependency — requires library B version ~1.2.5. If B 1.3.0 was just published with a breaking change, your `npm install` on Tuesday might resolve a different set of versions than it did on Monday. Your build is no longer reproducible. Your tests pass locally but fail in CI. A vulnerability is discovered in B 1.2.7 and you have no idea which version you're actually running.

The next sections build the mental model, then the code, then the production equivalent.

## What Dependencies Are (And Aren't)

A **dependency** is any external code your project references at build time, runtime, or test time that is not part of your project's own source tree.

**What counts as a dependency:**
- An npm package you `import` from node_modules
- A Cargo crate listed in your `Cargo.toml`
- A Go module in your `go.mod`
- A system library you link against (e.g., libc, openssl)
- A build tool or compiler version (gcc 12, rustc 1.70)
- A Docker base image your CI pulls down

**What is NOT a dependency:**
- Code you wrote yourself inside your project
- Language standard library functions (those are part of the language runtime, though the runtime version IS a dependency)
- Configuration files or environment variables (those are parameters, not dependencies)

Dependencies form a **directed acyclic graph (DAG)**. Your project is the root. Each node is a package. Each edge points from the consumer to the provider. The graph is acyclic because circular dependencies are forbidden by all mainstream package managers — if A requires B and B requires A, neither can be installed.

## Semantic Versioning (SemVer)

SemVer is a versioning scheme that communicates compatibility through three numbers:

```
MAJOR.MINOR.PATCH
  2   .  3  .  1
```

**MAJOR** — Incremented when you make incompatible API changes. A project depending on v2.x will break on v3.x. This is the "breaking change" signal.

**MINOR** — Incremented when you add functionality in a backward-compatible way. A project depending on v2.3.x is safe to update to v2.4.x. New features, same contract.

**PATCH** — Incremented when you make backward-compatible bug fixes. v2.3.1 to v2.3.2 fixes bugs without changing the API at all.

### The SemVer Contract

SemVer is a **promise**, not a guarantee. The version number tells the *intent*:
- `1.0.0` → "I consider this stable. The public API won't break in 1.x.x"
- `0.x.x` → "I'm still exploring. Anything might change."
- `2.0.0` → "Something changed compared to 1.x.x and it might break your code"

### Pre-release and Build Metadata

```
1.2.3-alpha.1    ← pre-release (unstable, comes before 1.2.3)
1.2.3+build.456  ← build metadata (ignored for precedence)
1.2.3-alpha.1+build.456  ← combined
```

Pre-release versions have lower precedence than the associated normal version: `1.2.3-alpha.1 < 1.2.3`. Build metadata is ignored for version ordering.

### Worked Example

Your project depends on `express@^4.18.0`.

The `^` means "compatible with" — it allows any version that doesn't bump the leftmost non-zero digit. Since 4 is non-zero, `^4.18.0` means `>=4.18.0 <5.0.0`.

When Express publishes 4.19.0 (a minor bump), your `npm install` will pick it up. That's fine — SemVer promises backward compatibility. When Express publishes 5.0.0 (a major bump), your `^4.18.0` will NOT pick it up. You must explicitly upgrade.

## Version Range Syntax

Version ranges tell the package manager which versions are acceptable:

| Syntax | Name | Meaning | Example Match |
|--------|------|---------|---------------|
| `1.2.3` | Exact | Only this exact version | Only 1.2.3 |
| `^1.2.3` | Caret | Compatible with (same major) | 1.2.3 up to <2.0.0 |
| `^0.2.3` | Caret (0.x) | Compatible with (same minor) | 0.2.3 up to <0.3.0 |
| `^0.0.3` | Caret (0.0.x) | Compatible with (same patch) | Only 0.0.3 |
| `~1.2.3` | Tilde | Approximately (same minor) | 1.2.3 up to <1.3.0 |
| `~1.2` | Tilde | Approximately (same minor) | 1.2.0 up to <1.3.0 |
| `>=1.2.3` | GTE | Greater than or equal | 1.2.3 or newer |
| `>1.2.3` | GT | Strictly greater | Anything above 1.2.3 |
| `<=1.2.3` | LTE | Less than or equal | 1.2.3 or older |
| `<2.0.0` | LT | Strictly less | Anything below 2.0.0 |
| `1.2.3 - 2.0.0` | Hyphen range | Inclusive range | 1.2.3 up through 2.0.0 |
| `*` | Wildcard | Any version | Everything |
| `1.x` | X-range | Any patch/minor | 1.0.0 up to <2.0.0 |
| `1.2.x` | X-range | Any patch | 1.2.0 up to <1.3.0 |

### Caret vs Tilde — The Key Distinction

```
^1.2.3  →  >=1.2.3  <2.0.0   (allows minor + patch bumps)
~1.2.3  →  >=1.2.3  <1.3.0   (allows only patch bumps)
```

Caret trusts the MAJOR number won't break you. Tilde trusts both MAJOR and MINOR won't break you. Tilde is more conservative; caret is more common.

### Ranges in Different Ecosystems

**npm (package.json):** Uses caret and tilde as shown above. Default `npm install` adds `^`.

**Cargo (Cargo.toml):** Uses caret by default. `1.2.3` in Cargo means `^1.2.3`. Uses SemVer Version Requirements spec: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html

**Go (go.mod):** Uses minimum version matching. `require foo v1.2.3` means exactly v1.2.3 OR a newer version if indirectly required. Go's approach is: use the maximum of all minimum versions required — "minimal version selection."

**Python (requirements.txt):** Pip supports `==`, `>=`, `<=`, `~=`, `!=`. `~=1.2.3` is roughly equivalent to `~1.2.3`.

## Lockfiles

A lockfile records the **exact versions** of every dependency (including transitive ones) that were resolved and installed. Without it, two `npm install` runs can produce different results.

### Why Lockfiles Exist

```
Day 1: You run `npm install` → resolves lodash@4.17.20
Day 2: lodash publishes 4.17.21 (a patch)
Day 3: CI runs `npm install` → resolves lodash@4.17.21
Day 4: 4.17.21 has a regression — your CI breaks, but you can't reproduce locally
```

The lockfile pins the exact version so every install reproduces the same dependency tree.

### Lockfile Comparison

| Feature | package-lock.json | Cargo.lock | go.sum |
|----------|-------------------|------------|--------|
| **Ecosystem** | npm/Node.js | Rust/Cargo | Go |
| **Format** | JSON | TOML | text (module hash + version) |
| **Committed to VCS?** | Yes (for apps), debated (for libs) | Yes (for apps), no (for libs) | Yes always |
| **Contains hashes?** | Integrity field (SHA-512) | checksum field (SHA-256) | SHA-256 hashes |
| **Human-readable?** | Somewhat | Yes | Somewhat |
| **Resolved by** | npm resolver | Cargo resolver (uses SAT) | Go minimal version selection |
| **Regenerated on** | `npm install` | `cargo build` | `go mod tidy` |

### What's Inside a Lockfile

```jsonc
// package-lock.json (simplified)
{
  "name": "my-app",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "node_modules/lodash": {
      "version": "4.17.21",
      "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
      "integrity": "sha512-...sha512hash..."
    }
  }
}
```

```toml
# Cargo.lock (simplified)
[[package]]
name = "serde"
version = "1.0.171"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a8ab9af..."
```

```
// go.sum (simplified)
github.com/gin-gonic/gin v1.9.0 h1:abc123...
github.com/gin-gonic/gin v1.9.0/go.mod h1:def456...
```

## Dependency Resolution

When you run `npm install`, the package manager must find a set of versions that satisfies **all** constraints simultaneously. This is a constraint satisfaction problem — specifically a form of SAT solving.

### SAT Solving and Backtracking

Consider this dependency graph:

```
Your App
├── A@^1.2.0
│   └── C@^2.0.0
└── B@^3.1.0
    └── C@^2.3.0
```

The resolver must find a version of C that satisfies both `^2.0.0` AND `^2.3.0`. The intersection is `>=2.3.0 <3.0.0`. The resolver picks the latest version in that range, say C@2.5.0.

Now consider a harder case:

```
Your App
├── A@^1.0.0
│   ├── C@^2.0.0
│   └── D@^1.0.0
└── B@^1.0.0
    ├── C@~2.1.0      ← conflicts with A's ^2.0.0?
    └── D@^2.0.0      ← conflicts with A's ^1.0.0!
```

B requires D@^2.0.0 but A requires D@^1.0.0. These ranges don't intersect — no single version of D satisfies both. The resolver must **backtrack**: try a different version of A or B that might have different D requirements, or report an error (unresolvable conflict).

### Why npm install Can Be Non-Deterministic

Without a lockfile, `npm install` is non-deterministic because:

1. **Time of install matters.** New versions are published between installs. `^1.2.0` resolves to the latest 1.x.x at the time of install, which changes.
2. **Registry mirrors can lag.** A mirror might not have the latest patch yet, so different registries resolve differently.
3. **Resolution algorithm order matters.** If multiple valid solutions exist, the one picked depends on traversal order.
4. **Flaky registries.** If the registry is temporarily unreachable, npm might fall back to cache, producing a different tree.
5. **Post-install scripts.** Some packages run scripts that affect resolution (rare but possible).

**The fix:** Always commit your lockfile. Always use `npm ci` (not `npm install`) in CI, which installs from the lockfile exactly.

### `npm ci` vs `npm install`

| `npm install` | `npm ci` |
|----------------|----------|
| May update package-lock.json | Requires package-lock.json to exist |
| Resolves from ranges | Installs exact versions from lockfile |
| Faster for incremental updates | Faster for clean installs |
| Can be non-deterministic | Deterministic |

## Transitive Dependencies and the Diamond Problem

A **transitive dependency** is a dependency of your dependency. If you depend on A, and A depends on B, then B is a transitive dependency of yours.

The **diamond problem** occurs when two different paths through the dependency tree require different versions of the same package:

```
         Your App
          /    \
         A      B
        / \    / \
       C   D  C   E
       |       |
   v1.x   v2.x   ← DIAMOND: two versions of C needed
```

### How Ecosystems Handle the Diamond Problem

**npm (Node.js):** Allows multiple versions of the same package in different `node_modules` subtrees via **nested dependencies**. Package A gets its own C@1.x, package B gets C@2.x. This works but bloats disk and memory. Deduplication (hoisting) tries to share when versions are compatible.

**Cargo (Rust):** Tries to find a single version that satisfies all constraints. If the ranges intersect, it picks one. If they don't, **compilation fails** — Rust forces you to resolve the conflict. This is stricter but safer.

**Go:** Uses **minimal version selection** — picks the maximum of all minimum versions required. All importers share one version. If A needs C>=1.2 and B needs C>=1.5, Go picks C>=1.5. Go modules are designed around the assumption that newer versions are backward-compatible within a major version.

**Maven (Java):** Picks the **nearest definition** — if the same dependency appears at different depths, the one closest to the root wins. First declaration wins for same-depth conflicts. This can silently pick an older version that lacks features a deeper dependency expected.

## Vulnerability Management

Dependencies are attack surface. Every package you add is code you didn't write running in your context.

### Tools by Ecosystem

**npm audit** — Built into npm. Checks your dependencies against a known vulnerability database.
```bash
npm audit           # list vulnerabilities
npm audit fix       # auto-fix where possible
```

**cargo audit** — Checks Rust crates against the RustSec Advisory Database.
```bash
cargo install cargo-audit
cargo audit          # list vulnerabilities
```

**Dependabot** — GitHub-native. Automatically creates pull requests to update dependencies when vulnerabilities are found. Works across npm, Cargo, pip, Maven, Go, and more.

**Snyk** — Commercial tool. Scans for vulnerabilities, license issues, and misconfigurations. Integrates with CI/CD.

### Vulnerability Severity Systems

Most tools use CVSS (Common Vulnerability Scoring System):
- **Critical** (9.0–10.0): Remote code execution, authentication bypass
- **High** (7.0–8.9): Privilege escalation, significant data exposure
- **Medium** (4.0–6.9): Limited data exposure, DoS
- **Low** (0.0–3.9): Minor information disclosure

### Practical Vulnerability Management Workflow

1. Run `npm audit` / `cargo audit` in CI on every push.
2. Block merges that introduce critical or high vulnerabilities.
3. Use Dependabot to auto-create PRs for security patches.
4. Review and merge security PRs within SLA timelines (e.g., 48h for critical).
5. For vulnerabilities without patches: evaluate if vulnerable code path is reachable, apply workarounds, or remove the dependency.

## Vendoring vs Registries

### Registry-Based (Default)

Most package managers pull dependencies from a central registry:
- **npm** → npmjs.com
- **Crates.io** → crates.io
- **PyPI** → pypi.org
- **Maven Central** → search.maven.org

**Pros:** Easy to update, small repo size, standardized metadata.
**Cons:** Requires network access, registries can go down, availability depends on third parties.

### Vendoring

**Vendoring** means copying dependency source code into your project's repository.

```bash
# Go vendor
go mod vendor          # copies all deps into ./vendor/

# Cargo vendor
cargo vendor > .cargo/config.toml   # copies into ./vendor/
```

**Pros:** Fully offline builds, immutable, auditable, no registry dependency.
**Cons:** Bloats repo size, manual updates, must track upstream patches.

### When to Vendor

Vendor when:
- You need deterministic offline builds (embedded systems, air-gapped environments).
- The dependency is tiny and unlikely to change (the "left-pad" scenario).
- You're modifying the dependency locally (forking).
- Security or compliance requires you to audit all source code.

Don't vendor when:
- The dependency has frequent updates and security patches.
- Your project is a library — consumers should get your transitive deps from the registry.
- You're in a normal web development context with reliable npm access.

## Monorepo Dependency Management

In a monorepo, multiple packages live in one repository, and they often share or depend on each other.

### Approaches

**npm workspaces:**
```json
// package.json (root)
{
  "workspaces": ["packages/*"]
}
```
All packages share one lockfile. `npm install` at root hoists common deps. Linking between workspace packages is automatic.

**Cargo workspace:**
```toml
# Cargo.toml (root)
[workspace]
members = ["crates/*"]
```
All crates share one `Cargo.lock`. Cross-crate dependencies use path + version dependencies.

**Go modules:** No native monorepo workspace. Use `go.work` files (Go 1.18+) or a multi-module approach with `replace` directives.

### Monorepo Gotchas

- **Phantom dependencies:** Package A can accidentally import Package B's dependencies if they're hoisted to the root `node_modules`.
- **Circular dependencies:** Package A imports B, B imports A. The build system may silently allow this but it creates tight coupling.
- **Version drift:** Without a shared lockfile, packages can drift to different versions of the same dependency.
- **Tooling:** Use Turborepo, Nx, or Bazel to manage build ordering and caching.

## The Left-Pad Incident and Lessons

On March 22, 2016, developer Azer Koçulu unpublished over 250 of his npm packages, including `left-pad` — an 11-line function that pads strings. Thousands of projects (including React and Babel) depended on it transitively. Builds broke worldwide.

### Key Lessons

1. **The micro-package risk.** An 11-line package can be a single point of failure for the entire JavaScript ecosystem. Before depending on something, ask: could I write this myself in under 5 minutes?

2. **Namespace squatting.** Anyone can publish to npm with any name. Left-pad took a generic name that many might assume is "owned" by a larger project.

3. **Transitive dependency risk.** You don't just depend on your direct dependencies — you depend on their maintainers' decisions, even for packages you've never heard of.

4. **Registry governance.** npm changed its policy after left-pad: packages can no longer be unpublished if other packages depend on them (within 24 hours, any package can be unpublished; after that, it stays).

5. **Supply chain awareness.** Every `npm install` trusts not just the package author, but also the registry infrastructure, the CDN, and the TLS certificate chain.

## Dependency Pinning vs Floating

### Pinning (Exact Versions)

```json
"lodash": "4.17.21"     // EXACT — only 4.17.21
```

**Pros:** Fully reproducible builds, predictable, safe.
**Cons:** You miss security patches, you must manually update.

### Floating (Ranges)

```json
"lodash": "^4.17.21"    // RANGE — 4.17.21 up to <5.0.0
```

**Pros:** Automatic patches and features within the range.
**Cons:** Non-reproducible, surprising behavior, risk of supply chain attacks.

### Best Practice: Pin in Lockfiles, Float in Manifests

- **Package manifest** (package.json, Cargo.toml): Use ranges (`^`, `~`) so your project can accept updates.
- **Lockfile** (package-lock.json, Cargo.lock): Always commit it. This pins the exact versions for reproducibility.
- **CI:** Use `npm ci` / `cargo build --locked` to install from the lockfile exactly.
- **Updates:** Use Dependabot or Renovate to create PRs that update the lockfile and test changes. This gives you floating with safety rails.

## Resolution Strategies Compared

| Strategy | Used By | Approach | Pros | Cons |
|----------|---------|---------|------|------|
| **Latest compatible** | npm, Yarn, Pip | Pick the newest version in range | Get latest bug fixes | May be non-deterministic |
| **Minimal version** | Go (via MVS) | Pick the oldest version that satisfies all constraints | Maximum stability | May miss patches |
| **SAT solving** | Cargo, Pub (Dart) | Find any solution; prefer newer if multiple | Optimal or near-optimal | Can be slow for large graphs |
| **Nearest wins** | Maven | First/nearest declaration wins | Simple rule | Silent downgrade risk |
| **Lockfile-first** | All (with lockfile) | Use lockfile if it exists, else resolve | Reproducible | Can fall behind on patches |

## Dependency Licensing

Every dependency carries a license. Using a dependency means accepting its license terms.

### Common Open Source Licenses

| License | Copyleft? | Can Use in Proprietary? | Notes |
|---------|-----------|------------------------|-------|
| **MIT** | No | Yes | Most permissive. "Do whatever, just include the license." |
| **Apache 2.0** | No | Yes | Like MIT but with patent grants and clearer terms. |
| **BSD (2/3 clause)** | No | Yes | Similar to MIT. "Do whatever, don't use our name." |
| **GPL v2/v3** | **Yes** | Only if your code is also GPL | "If you distribute, you must share source." |
| **AGPL** | **Yes** | Only if your code is also AGPL | Like GPL but triggered by network use, not just distribution. |
| **LGPL** | Weak | Yes (with conditions) | Linking is OK; modifying the LGPL code requires sharing. |
| **Unlicense / CC0** | No | Yes | Public domain dedication. |
| **SSPL** | **Yes** (strong) | Essentially no for SaaS | Not considered open source by OSI. |

### The GPL Virus

If you include GPL-licensed code in your project and distribute it, your project must also be GPL. This "infects" your code. This is fine for open source projects but a major problem for proprietary software.

**Transitive licensing risk:** You might import an MIT library that depends on a GPL library. The GPL terms still apply to your project. Always check the full dependency tree, not just direct dependencies.

### License Scanning Tools

- **`licensee`** — Detects license from repo files.
- **`SPDX`** — Standardized license identifier format.
- **`FOSSA`** — Commercial license compliance platform.
- **`npm`** — `npm ls --json` lists packages; pair with license-checker.

## When to Vendor vs. When to Internal-Source

**Vendor** when:
- The dependency is small and stable (you could rewrite it, but it's not worth the effort).
- You need deterministic, offline builds.
- You've patched the dependency and can't upstream the change.

**Internal-source** when:
- You need a capability that no open-source package provides.
- The available packages are poor quality, unmaintained, or carry unacceptable licenses.
- The dependency is core to your business logic and you need full control.

**Avoid** when:
- A well-maintained, permissively-licensed package does exactly what you need.
- You'd just be reinventing the wheel (e.g., writing another left-pad).

## Exercises

1. **Easy** — Given `^2.3.1`, `~2.3.1`, and `>=2.3.1`, list all versions from the set {2.3.1, 2.3.5, 2.4.0, 3.0.0} that each range accepts.
2. **Medium** — Your app depends on A@^1.0.0 and B@^1.0.0. A depends on C@^2.0.0, B depends on C@~2.5.0. Explain the resolution. What version of C will be selected? What happens if C publishes 3.0.0?
3. **Hard** — Implement a minimal SAT solver for dependency resolution: given a set of packages with version ranges, determine if a valid resolution exists, or report the conflict. Handle at least one backtrack scenario.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| SemVer | "Just version numbers" | A backward-compatibility contract encoded in three numbers |
| Lockfile | "That lock thing" | An immutable snapshot of exactly which versions were installed |
| Transitive dependency | "My dependency's dependency" | Any package reachable through the dependency graph, not just direct edges |
| Diamond problem | "Version conflict" | Two paths require incompatible versions of the same package |
| Vendoring | "Copying the code" | Duplicating dependency source into your repo for offline/deterministic builds |
| Caret range (^) | "The little hat" | Accept any version that doesn't change the leftmost non-zero digit |
| Tilde range (~) | "The squiggly" | Accept only patch-level changes (same major.minor) |
| Resolution | "Figuring out versions" | Solving a constraint satisfaction problem to find compatible versions |
| MVS | "Go's version thing" | Minimal Version Selection — pick the oldest version satisfying all constraints |
| npm ci | "Clean install" | Install exactly from lockfile, fail if it doesn't match package.json |

## Further Reading

- [SemVer Specification 2.0.0](https://semver.org/)
- [npm SemVer Calculator](https://semver.npmjs.com/)
- [Go Minimal Version Selection](https://research.swtch.com/vgo-mvs)
- [Cargo Resolver Documentation](https://doc.rust-lang.org/cargo/reference/resolver.html)
- [The left-pad incident (2016)](https://www.theregister.com/2016/03/23/npm_left_pad_chaos/)
- [The Diamond Dependency Problem](https://jlbp.dev/what-is-the-diamond-dependency-problem)
- [Choosing an OSS License](https://choosealicense.com/)
- [SPDX License List](https://spdx.org/licenses/)