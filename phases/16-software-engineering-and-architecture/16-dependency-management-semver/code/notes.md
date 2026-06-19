# Notes — Dependency Management & SemVer

**_meta:** { "phase": 16, "lesson": 16 }

---

## 1. SemVer Version Range Syntax

### Core Ranges

| Range | Expands To | Accepts |
|-------|-----------|---------|
| `1.2.3` | `1.2.3` | Exactly 1.2.3 only |
| `^1.2.3` | `>=1.2.3 <2.0.0` | 1.2.3, 1.2.4, 1.3.0, 1.99.99 |
| `^0.2.3` | `>=0.2.3 <0.3.0` | 0.2.3, 0.2.4, 0.2.99 |
| `^0.0.3` | `>=0.0.3 <0.0.4` | Only 0.0.3 |
| `~1.2.3` | `>=1.2.3 <1.3.0` | 1.2.3, 1.2.4, 1.2.99 |
| `~1.2` | `>=1.2.0 <1.3.0` | 1.2.0, 1.2.1, ..., 1.2.99 |
| `~1` | `>=1.0.0 <2.0.0` | 1.0.0 through 1.99.99 |
| `*` | `>=0.0.0` | Any version at all |
| `1.x` | `>=1.0.0 <2.0.0` | Same as `^1.0.0` |
| `1.2.x` | `>=1.2.0 <1.3.0` | Same as `~1.2.0` |

### Comparison Ranges

| Range | Accepts |
|-------|---------|
| `>=1.2.3` | 1.2.3 or newer |
| `>1.2.3` | Anything strictly above 1.2.3 |
| `<=1.2.3` | 1.2.3 or older |
| `<2.0.0` | Anything below 2.0.0 |
| `>=1.0.0 <2.0.0` | Explicit range (shorthand for ^1.0.0) |

### Hyphen Ranges

| Range | Expands To |
|-------|-----------|
| `1.2.3 - 2.3.4` | `>=1.2.3 <=2.3.4` |
| `1.2 - 2.3.4` | `>=1.2.0 <=2.3.4` |
| `1.2.3 - 2.3` | `>=1.2.3 <2.4.0` |

### Pre-release Ordering

```
1.0.0-alpha < 1.0.0-alpha.1 < 1.0.0-alpha.beta
< 1.0.0-beta < 1.0.0-beta.2 < 1.0.0-beta.11
< 1.0.0-rc.1 < 1.0.0
```

Pre-release versions are ordered by comparing each dot-separated identifier left to right: numeric identifiers compared numerically, alphanumeric compared lexicographically (ASCII sort). Numeric always has lower precedence than alphanumeric.

### Caret Rules Summary (npm)

```
^1.2.3  :=  >=1.2.3 <2.0.0    (leftmost non-zero is major=1)
^0.2.3  :=  >=0.2.3 <0.3.0    (leftmost non-zero is minor=2)
^0.0.3  :=  >=0.0.3 <0.0.4    (leftmost non-zero is patch=3)
```

The `^` freezes everything to the left of the leftmost non-zero digit and allows changes to the right.

---

## 2. Lockfile Comparison Table

| Property | package-lock.json | Cargo.lock | go.sum | Pipfile.lock | yarn.lock |
|----------|-------------------|------------|--------|-------------|-----------|
| Ecosystem | npm/Node.js | Rust | Go | Python | Node.js |
| Format | JSON | TOML | Text | JSON | YAML-like |
| Hashes | SHA-512 (integrity) | SHA-256 (checksum) | SHA-256 | SHA-256 | Integrity hash |
| Committable? | Yes (apps only) | Yes (apps only) | Always | Always | Yes |
| Updated on | `npm install` | `cargo build` | `go mod tidy` | `pipenv install` | `yarn install` |
| CI command | `npm ci` | `cargo build --locked` | `go mod download` | `pipenv install --deploy` | `yarn install --frozen-lockfile` |
| Supports nested? | Yes (node_modules tree) | No (flat graph) | No (flat) | No (flat) | Yes |
| Records registry? | Yes (resolved URL) | Yes (source) | N/A (module path) | Yes (source URL) | Yes (resolved) |
| Human-mergeable? | Difficult | Moderate | Easy | Difficult | Moderate |

### When to Commit Lockfiles

**Always commit for:**
- Applications (web servers, CLI tools, desktop apps)
- Monorepo roots
- CI/CD pipelines

**Generally don't commit for:**
- Published libraries (let consumers resolve their own versions)
- Unless the library has a test suite that needs reproducible installs

**Always commit go.sum** — Go requires it for integrity verification.

---

## 3. Dependency Resolution Strategies

### Strategy: Latest Compatible (npm, Yarn, Pip)

```
Goal: Pick the newest version satisfying all constraints.

Algorithm:
1. Start at root package
2. For each dependency, find the latest version matching the range
3. For each transitive dependency, find the version that satisfies
   the intersection of all parent constraints
4. If no intersection exists → error
5. If multiple versions allowed → pick latest

Example:
  App → A@^1.0.0, A → C@^2.0.0
  App → B@^1.0.0, B → C@~2.3.0
  
  C constraint: ^2.0.0 ∩ ~2.3.0 = >=2.3.0 <2.4.0
  Pick latest in range → C@2.3.9
```

### Strategy: Minimal Version Selection (Go MVS)

```
Goal: Pick the maximum of all minimum versions.

Algorithm:
1. For each dependency, collect all minimum version requirements
2. Pick the maximum of those minimums
3. Never upgrade beyond what's explicitly required

Example:
  App → A requiring C@v1.2
  App → B requiring C@v1.5
  
  MVS picks C@v1.5 (max of {1.2, 1.5})
  Even if v1.9 exists, it won't be selected
  Unless someone in the graph requires >=v1.9
```

### Strategy: SAT-Based (Cargo, Pub)

```
Goal: Find a valid assignment of versions, preferring newer.

Algorithm (Cargo):
1. Build a constraint graph
2. Use unit propagation to reduce constraints
3. Use backtracking search when conflicts arise
4. Prefer newer versions as heuristic
5. Return the first valid assignment, or error

Example of backtracking:
  App → A@^1.0.0, A@^1.5.0 requires C@^2.0.0
  App → A@^1.0.0, A@^1.3.0 requires C@^1.0.0  (older A)
  App → B@^1.0.0, B requires C@^1.5.0
  
  Try A@1.5.0 → C needs ^2.0.0 → conflicts with B's ^1.5.0
  Backtrack → try A@1.3.0 → C needs ^1.0.0 → intersects with B's ^1.5.0
  Resolve C@1.5.x ✓
```

### Strategy: Nearest Wins (Maven)

```
Goal: The nearest definition in the dependency tree wins.

Algorithm:
1. Traverse the dependency tree depth-first
2. If a dependency appears at multiple depths, use the one
   at the shallowest depth (closest to root)
3. If at same depth, first declaration wins

Example:
  App → A (depth 1) → C@1.0 (depth 2)
  App → B (depth 1) → D (depth 2) → C@2.0 (depth 3)
  
  C@1.0 is at depth 2, C@2.0 is at depth 3
  Nearest wins → C@1.0 is selected
  
  Risk: D might expect C@2.0 features → runtime errors
```

### Resolution Decision Matrix

| Scenario | Strategy | Resolution |
|----------|----------|-----------|
| Ranges overlap | Any | Pick best version in intersection |
| Ranges don't overlap, versions flexible | SAT/MVS | Backtrack to find compatible combo |
| Ranges don't overlap, versions pinned | None | Error: unresolvable conflict |
| Diamond with compatible ranges | Any | Single version in intersection |
| Diamond with incompatible ranges | npm: nest; Cargo: error; Go: MVS; Maven: nearest | Ecosystem-dependent |