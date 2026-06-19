# Reading Large Codebases — Reference Card

A practical guide for approaching any unfamiliar codebase.

---

## The 8-Phase Reading Strategy

### Phase 1: Orient Yourself (20 min)

- [ ] Read the README
- [ ] Skim top-level directory structure
- [ ] Find the entry point (`main()`, `index.js`, etc.)
- [ ] Read the build file (`Makefile`, `Cargo.toml`, `package.json`)
- [ ] Read the dependency file (`go.mod`, `Cargo.lock`, `package-lock.json`)

### Phase 2: Trace the Main Flow (40 min)

- [ ] Follow the call chain from entry point to first output
- [ ] Map the module structure
- [ ] Identify the architecture pattern

### Phase 3: Read Tests First

- [ ] Find all test files
- [ ] Read tests for the module you care about
- [ ] Use tests as documentation of intent

### Phase 4: Navigation Tools

- [ ] Set up `ripgrep` for fast search
- [ ] Set up `ctags` for symbol navigation
- [ ] Set up LSP for your editor
- [ ] Use `GitHub search` or `Sourcegraph` for open-source projects

### Phase 5: Read Commit History

- [ ] `git log --oneline -20` — recent activity
- [ ] `git log --follow -- path` — file history with renames
- [ ] `git blame path` — who wrote each line and when
- [ ] `git log -S "string"` — find when a string was introduced

### Phase 6: Read the Architecture Onion

- [ ] Outer layer: README, directory structure
- [ ] Interface layer: Public APIs, type signatures
- [ ] Module layer: Directory structure, dependency graph
- [ ] Implementation layer: Internal functions, algorithms
- [ ] Detail layer: Line-by-line logic, edge cases

### Phase 7: Draw the Map

- [ ] Sketch module dependencies
- [ ] Trace data flow for the main path
- [ ] Build a call graph for critical functions

### Phase 8: Read for Purpose

- [ ] Bug fixing → trace backwards from failure
- [ ] Feature adding → find similar features, copy the pattern
- [ ] Refactoring → map all callers and dependencies first

---

## Tool Quick Reference

### Search

```
# ripgrep (preferred)
rg 'pattern' -t go              # Search in Go files
rg 'func [A-Z]' -t go           # Find exported Go functions
rg 'TODO|FIXME' --type-add      # Find TODO comments

# grep (fallback)
grep -rn 'pattern' --include='*.go' .

# ctags
ctags -R --languages=Go,Python,Rust .
# Then jump to definition in editor: :tag SymbolName
```

### Git Investigation

```
git log --oneline -20           # Recent commits
git log --stat                  # Which files changed
git log --follow -- path        # File history with renames
git blame path                  # Who wrote each line
git blame -L 10,20 path         # Blame specific line range
git show <hash>                  # Full diff of a commit
git log -S "functionName"       # Find when a string appeared
git shortlog -sn --all          # Commits by author
```

### Dependency Graphs

```
go mod graph                   # Go module dependency tree
cargo tree                     # Rust crate dependency tree
npm ls                         # Node package dependency tree
pipdeptree                     # Python dependency tree
```

### Entry Points by Language

| Language   | File                    | Function                   |
|------------|-------------------------|----------------------------|
| C/C++      | `main.c`                | `int main(int argc, char** argv)` |
| Go         | `cmd/*/main.go`         | `func main()`              |
| Rust       | `src/main.rs`           | `fn main()`                |
| Python     | `app.py`, `manage.py`   | `if __name__ == "__main__"` |
| JavaScript | `index.js`, `app.js`    | Top-level or `createServer` |
| Java       | `*Application.java`     | `public static void main`  |
| Ruby       | `config.ru`, `bin/*`    | Rack app or CLI entry       |

---

## Architecture Patterns — Quick Guide

### Layered

```
Presentation (handlers, views, controllers)
       │
Business Logic (services, domain)
       │
Data Access (repositories, models)
       │
Database
```
**Read**: Start at presentation, follow a request downward.

### Hexagonal (Ports & Adapters)

```
  Adapters ◀── Ports ◀── Domain ──▶ Ports ──▶ Adapters
```
**Read**: Domain core first (zero external deps), then adapters.

### Event-Driven

```
Producer ──▶ Event Bus ──▶ Consumer A
                          ──▶ Consumer B
```
**Read**: Event definitions first (the contract), then producers, then consumers.

### Microservices

```
Service A ──▶ API Gateway ──▶ Service B
                              ──▶ Service C
```
**Read**: Pick one service. Read its API contract first. Treat it as a monolith internally.

---

## Reading for Purpose

### Bug Fixing

1. Start at the error message or failing test
2. Find where the error is defined (`rg 'ErrSpecific'`)
3. Find where it is returned (`rg 'return.*ErrSpecific'`)
4. Trace backwards: what called this function?
5. Fix the root cause, not the symptom

### Feature Adding

1. Find a similar existing feature
2. Read it end-to-end: route → handler → service → store
3. Copy the pattern, modify for your new feature
4. This is following conventions, not copy-paste programming

### Refactoring

1. Map all callers of the code you want to change
2. Map all dependencies
3. Check tests (they define behavior you must preserve)
4. Check benchmarks (they define constraints you must not violate)
5. Only then begin the refactor

---

## The 20-Minute Rule

Before asking a question about a codebase, spend 20 minutes investigating.

**Bad questions** (vague, unanswerable):

- "How does auth work?"
- "Where is the database?"
- "Why is this slow?"

**Good questions** (precise, answerable):

- "I see `verifyToken` called in `middleware.go:23`. Why does it check `claims.Exp` instead of `claims.Nbf`?"
- "I found `Connect()` in `store.go:15`. Is this the only database connection, or are there read replicas?"
- "The `ListUsers` query in `repository.go:78` fetches all columns including `avatar_blob`. Is there a reason we don't paginate?"

---

## Verify, Don't Guess

| Assumption                           | Verification                                  |
|--------------------------------------|-----------------------------------------------|
| "This function is only called here"  | `rg 'functionName(' -t go`                   |
| "This variable is always non-nil"    | `rg 'variableName.*nil'`                      |
| "This module is thread-safe"         | `rg 'mutex\|sync\.Mutex\|Lock()'`            |
| "This API returns JSON"              | Read the response type, not the URL pattern   |
| "This config comes from env vars"    | Read the config loader, not the variable name  |

When in doubt, add a log/print and run the code. Observations beat assumptions.

---

## Common Diagnostic Commands

```bash
# How big is this codebase?
find . -name '*.go' | xargs wc -l | tail -1

# What are the biggest files?
find . -name '*.go' -exec wc -l {} + | sort -rn | head -20

# What's the most-changed file? (likely the core)
git log --format=format: --name-only | sort | uniq -c | sort -rn | head -20

# What functions are exported?
grep '^func [A-Z]' *.go

# What types are defined?
grep '^type.*struct' *.go

# Find all HTTP endpoints
rg 'r\.(Get|Post|Put|Delete|Handle)\(' -t go

# Find all error types
rg 'type.*Error struct' -t go
```

---

*Reference card for Lesson 21: Reading Large Codebases — Phase 16: Software Engineering & Architecture*