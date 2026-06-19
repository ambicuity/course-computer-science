# Notes — Reading Large Codebases

## Reading Strategies by Codebase Size

### Small Codebase (< 1,000 lines)

- Read the entire thing. Seriously. Open every file.
- You can hold the entire model in your head. Do it.
- Start with the entry point, read linearly.
- Sketch the call graph on paper — it will fit.

### Medium Codebase (1,000 – 50,000 lines)

**Orientation (30 minutes):**

1. Read the README end-to-end.
2. Skim the directory structure. Note the top-level modules.
3. Find and read the build file (`Makefile`, `Cargo.toml`, `package.json`).
4. Read the dependency file to understand external choices.
5. Run the test suite and read the test output — it maps the module surface.

**Deep dive (2–4 hours):**

1. Pick one module. Read its tests, then its interface, then its implementation.
2. Trace one complete flow: input → processing → output.
3. Sketch the module dependency diagram.
4. Read `git log --oneline -30` to understand recent activity.

### Large Codebase (50,000 – 500,000 lines)

**Orientation (1–2 hours):**

1. Read the README, CONTRIBUTING guide, and architecture docs (if they exist).
2. Skim the top-level directory structure. Identify the "neighborhoods."
3. Find the entry point. Read only its first 50 lines.
4. Run `git log --stat | head -100` to see what files change most often.
5. Identify the core module (it is usually the one with the most importers).

**Deep dive (per module, 2–4 hours each):**

1. Read the module's public interface — exported types, functions, constants.
2. Read the module's tests.
3. Trace one flow through the module.
4. Find who else imports this module and why.
5. Check `git log -- path/to/module/` for recent changes.

**Scaling strategy:**

- Never try to read the entire codebase. You cannot.
- Build a mental model of the architecture first, then read modules on-demand.
- Keep a running notes file: "Module X does Y. Key file: Z."

### Very Large Codebase (500,000+ lines — e.g., Linux kernel, Chromium)

**Orientation (1 day):**

1. Read the architecture documentation. Large projects always have it.
2. Skim the directory structure at two levels deep.
3. Find the entry point. Do not read it — just note its location.
4. Read the build system to understand what subsystems exist.
5. Pick one subsystem. Treat it as its own codebase.

**Deep dive (per subsystem):**

- Treat each subsystem as an independent medium-to-large codebase.
- Read the subsystem's documentation, then its API, then its internals.
- Cross subsystem boundaries only when necessary.

---

## Tool Cheat Sheet

### Search and Navigation

| Tool          | Install                    | Use Case                           | Example                                        |
|---------------|----------------------------|------------------------------------|-------------------------------------------------|
| `grep`        | Built-in                   | Basic pattern search               | `grep -rn 'pattern' --include='*.go' .`        |
| `ripgrep`     | `cargo install ripgrep`    | Fast code search, respects gitignore | `rg 'pattern' -t go`                           |
| `ctags`       | `brew install ctags`       | Symbol index for editor navigation | `ctags -R .` then `:tag SymbolName`           |
| `LSP`         | Varies by language         | Go-to-def, find refs, rename      | Configured per-editor (VS Code has it built-in)|
| `GitHub search`| github.com               | Cross-repo code search             | `gh search code 'pattern' --repo=owner/repo`  |
| `Sourcegraph`  | sourcegraph.com          | Multi-repo code intelligence       | Web-based, supports regex + symbol search      |

### Git Investigation

| Command                           | Purpose                                          |
|-----------------------------------|--------------------------------------------------|
| `git log --oneline -20`          | Recent commit summary                            |
| `git log --oneline --graph`      | Branch topology visualization                    |
| `git log --follow -- path`       | History of a file including renames              |
| `git log --stat`                  | Which files changed in each commit               |
| `git blame path`                  | Who wrote each line and when                     |
| `git blame -L 10,20 path`        | Blame a specific line range                      |
| `git show <hash>`                 | Full diff of a specific commit                   |
| `git diff HEAD~1`                 | What changed in the last commit                  |
| `git log --author="name"`        | Commits by a specific author                     |
| `git log -S "functionName"`      | Find commits that added/removed a string         |

### Build and Dependency Files

| File                | Read Command                  | Key Information                                   |
|---------------------|-------------------------------|---------------------------------------------------|
| `Makefile`          | `cat Makefile`                | Targets, dependencies, phony tasks               |
| `Cargo.toml`        | `cat Cargo.toml`              | Workspace, features, dependencies                |
| `package.json`      | `jq . package.json`           | Scripts, deps, workspaces                        |
| `go.mod`            | `cat go.mod`                  | Module path, Go version, requires                |
| `go.sum`            | `wc -l go.sum`                | Number of locked dependencies                     |
| `requirements.txt`  | `cat requirements.txt`        | Direct Python dependencies                        |
| `pyproject.toml`    | `cat pyproject.toml`          | Build system, project metadata                   |

### Dependency Graph Tools

| Tool            | Language | Command                    | Output                                    |
|-----------------|----------|----------------------------|--------------------------------------------|
| `go mod graph`  | Go       | `go mod graph`             | Module dependency tree                    |
| `cargo tree`    | Rust     | `cargo tree`               | Crate dependency tree                     |
| `npm ls`        | JS       | `npm ls`                   | Package dependency tree                   |
| `pipdeptree`    | Python   | `pipdeptree`               | Package dependency tree                   |
| `madge`         | JS       | `madge --image graph.svg .`| Visual module dependency graph             |

---

## Common Patterns Guide

### Directory Layout Conventions

#### Go

```
cmd/            # Entry points (one directory per binary)
internal/       # Private packages (not importable externally)
pkg/            # Public packages (importable by external projects)
api/            # API definitions (protobuf, OpenAPI)
web/            # Frontend assets
configs/        # Configuration files
scripts/         # Build and deploy scripts
test/           # Integration tests
docs/           # Documentation
```

#### Rust

```
src/
  bin/          # Entry points for multiple binaries
  lib.rs        # Library root
  main.rs       # Binary entry point
  modules/      # Submodules
Cargo.toml      # Package manifest
tests/          # Integration tests
benches/         # Benchmarks
examples/        # Example programs
```

#### JavaScript / TypeScript

```
src/
  index.ts      # Entry point
  routes/       # Route handlers
  services/     # Business logic
  models/       # Data models
  utils/        # Utility functions
tests/          # Test files
package.json    # Manifest
tsconfig.json   # TypeScript config
```

#### Python

```
package_name/
  __init__.py   # Package root
  core.py       # Core logic
  models.py     # Data models
  api.py        # API handlers
  utils.py      # Utilities
tests/          # Test directory
pyproject.toml  # Build config
requirements.txt # Dependencies
```

### Architecture Pattern Indicators

| Pattern           | Directory Indicators                           | Entry Point Pattern              |
|-------------------|------------------------------------------------|----------------------------------|
| Layered           | `models/`, `views/`, `controllers/`            | Request → Controller → Service   |
| Hexagonal         | `ports/`, `adapters/`, `domain/`               | Port → Adapter → Domain          |
| Event-driven      | `events/`, `handlers/`, `queue/`               | Event → Handler → Side effects   |
| Microservices     | Multiple `Dockerfile`s, `docker-compose.yml`  | Service → HTTP/gRPC              |
| Plugin-based      | `plugins/`, `extensions/`                       | App → Plugin loader → Plugin     |
| Pipeline          | `stages/`, `steps/`, `transforms/`             | Input → Stage1 → Stage2 → Output|

### Test Naming Conventions

| Language | Test File Pattern   | Test Function Pattern               |
|----------|---------------------|--------------------------------------|
| Go       | `*_test.go`         | `func TestXxx(t *testing.T)`        |
| Rust     | `#[test]` in any file| `fn test_xxx()`                     |
| Python   | `test_*.py`         | `def test_xxx():`                    |
| JS/TS    | `*.test.ts/.spec.ts`| `test('xxx', () => {})`             |
| Java     | `*Test.java`        | `@Test void testXxx()`              |

### Quick Diagnostic Commands

```bash
# How big is this codebase?
find . -name '*.go' | xargs wc -l | tail -1

# What are the biggest files?
find . -name '*.go' -exec wc -l {} + | sort -rn | head -20

# What modules exist?
find . -name 'go.mod' -exec dirname {} \;

# What's the most-changed file? (likely the core)
git log --format=format: --name-only | sort | uniq -c | sort -rn | head -20

# What functions are exported from this package?
grep '^func [A-Z]' *.go

# What types are defined here?
grep '^type.*struct' *.go

# What interfaces are defined?
grep '^type.*interface' *.go

# Find TODO/FIXME/HACK comments
rg 'TODO|FIXME|HACK' -t go

# Find all error types
rg 'type.*Error struct' -t go

# Find all HTTP endpoints
rg 'r\.(Get|Post|Put|Delete|Handle)\(' -t go
```