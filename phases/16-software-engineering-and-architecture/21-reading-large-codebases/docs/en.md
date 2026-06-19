# Reading Large Codebases

> You spend far more time reading code than writing it. Learn to read well.

**Type:** Learn
**Languages:** Markdown, Shell
**Prerequisites:** Phase 16 lessons 01–20
**Time:** ~60 minutes

## Learning Objectives

- Understand why reading code is a distinct skill from writing code — and why it matters more.
- Build a repeatable strategy for approaching any unfamiliar codebase.
- Use navigation tools (grep, ripgrep, ctags, LSP, code search) to explore efficiently.
- Read commit history, tests, and build files to understand intent and structure.
- Sketch module dependencies, data flow, and call graphs to build a mental map.
- Adapt your reading strategy based on purpose: bug fixing, feature adding, or refactoring.

## The Problem

This lesson sits in **Phase 16 — Software Engineering & Architecture**. Without the ability to read large codebases, you cannot contribute to any realistic project. You will stare at directories containing thousands of files, open one at random, get lost, and close it. Every junior developer has done this.

The capstone for this phase asks you to refactor a real-world OSS repo with Architecture Decision Records. You cannot refactor what you cannot read. You cannot write ADRs for decisions you cannot identify. The skill gap is simple: nobody taught you _how to read code_. They only taught you how to write it.

This lesson fixes that.

## The Concept: Why Reading Code Is a Skill

### The Ratio

Studies of professional developers consistently show a **10:1 read-to-write ratio**. For every line you write, you read roughly ten. During code review, during debugging, during onboarding onto a new team — you are always reading first.

Yet nearly all formal education focuses on writing. The implicit assumption is that reading is "just the inverse" of writing. It is not. Writing code is creative: you express intent. Reading code is investigative: you reconstruct intent from evidence. The cognitive task is fundamentally different.

### The Analogy

Think of reading a codebase like exploring a city you have never visited. You do not start by walking down every street. You look at a map first. You find landmarks. You orient yourself. Then you explore neighborhoods one at a time, building a mental model that gets more detailed over time.

A large codebase is a city. The README is the visitor's guide. The entry point is the airport. The module structure is the neighborhood map. Tests are the tour guide. Commit history is the city archives.

## Build It: A Strategy for Approaching an Unfamiliar Codebase

### Phase 1 — Orient Yourself (First 20 Minutes)

#### Start with the README

The README tells you three things:

1. **What the project does** — the problem domain.
2. **How to build and run it** — the technical surface.
3. **How to contribute** — the social contract.

If the README is missing or useless, that itself is information. The project likely relies on tribal knowledge.

#### Find the Entry Point

Every program has a place where execution begins. Find it:

| Language    | Entry Point                    |
|-------------|-------------------------------|
| C/C++       | `main()` in `main.c` or similar |
| Go          | `func main()` in `cmd/` or root |
| Rust        | `fn main()` in `src/main.rs`    |
| Python      | `if __name__ == "__main__"`      |
| JavaScript  | `index.js`, `app.js`, or `src/index.ts` |
| Java        | `public static void main`        |
| Ruby        | `config.ru` or `bin/` directory  |

The entry point reveals the first function call, which reveals the first module, which reveals the architecture.

#### Understand the Build System

Build files are not just for building. They are a map of project structure:

| File                | Language       | What it reveals                              |
|---------------------|----------------|----------------------------------------------|
| `Makefile`          | C/C++/mixed    | Targets, dependencies, phony tasks           |
| `Cargo.toml`        | Rust           | Workspace members, features, dependencies    |
| `package.json`      | JavaScript     | Scripts, dependencies, workspaces           |
| `go.mod`            | Go             | Module path, Go version, dependencies        |
| `pyproject.toml`    | Python         | Build system, project metadata               |
| `BUILD` / `BUILD.bazel` | Bazel     | Build targets, visibility, dependencies     |

Read the build file before you read the source. It tells you what exists, what depends on what, and what the project considers important enough to define as a target.

#### Understand the Dependencies

Dependency files tell you what the project relies on — and what the authors chose not to reinvent:

| File                 | What to look for                              |
|----------------------|-------------------------------------------------|
| `go.mod`             | What libraries the project imports             |
| `Cargo.lock`         | Exact versions — shows stability expectations   |
| `package-lock.json`  | Transitive dependency tree                     |
| `requirements.txt`   | Direct dependencies (often incomplete)          |
| `go.sum`             | Integrity checksums for dependencies            |

Dependencies also reveal architectural choices. A project that imports `tokio` is async. One that imports `actix-web` is a web service. One that imports `rayon` does data-parallel computation. The dependency list is a fingerprint.

### Phase 2 — Trace the Main Flow (Next 40 Minutes)

#### Trace from Entry Point to First Output

Open the entry point. Read the first function. Follow the chain of calls until you reach the first meaningful output — a log line, a response, a file written. This is your first "spine" through the codebase.

```bash
# Find the entry point in a Go project
grep -rn 'func main(' --include='*.go' .

# Trace what main() calls
grep -rn 'func main(' --include='*.go' . | head -1
# Then open that file and follow the calls
```

You are not trying to understand everything. You are building a single thread of comprehension from input to output.

#### Understand the Module Structure

Most well-organized projects group code by feature or by layer. Look at the top-level directory structure:

```
project/
├── cmd/           # Entry points (Go convention)
├── internal/      # Private packages (Go convention)
├── pkg/           # Public packages (Go convention)
├── api/            # API definitions (protocol buffers, OpenAPI)
├── web/            # Frontend assets
├── configs/        # Configuration files
├── docs/           # Documentation
├── scripts/        # Build and deploy scripts
├── tests/          # Integration tests
└── tools/          # Development tooling
```

Not every project follows this layout. But every project _has_ a layout, and understanding it is the fastest way to know where to look for something.

#### Understand the Architecture

After you have traced the main flow, step back and ask: what architecture pattern does this codebase follow?

Common patterns:

| Pattern        | Telltale signs                                          |
|----------------|--------------------------------------------------------|
| Layered        | `models/`, `views/`, `controllers/` directories        |
| Hexagonal      | `ports/`, `adapters/`, `domain/` directories            |
| Event-driven   | `events/`, `handlers/`, `queue/` directories             |
| Microservices  | Multiple `Dockerfile`s, `docker-compose.yml`, HTTP/gRPC |
| Plugin-based   | `plugins/` or `extensions/` directories                  |

You do not need to name the pattern correctly. You need to understand the _flow_: where does data enter, where is it transformed, where does it exit?

### Phase 3 — Read Tests First

#### Tests Are Documentation

Tests show intent more clearly than implementation. A test says: "Given this input, the output should be this." The implementation says: "Here is how I make that happen." Always read the "what" before the "how."

```bash
# Find test files in a project
find . -name '*_test.go' -o -name 'test_*.py' -o -name '*.test.ts' -o -name '*.spec.ts'

# Count tests to understand coverage scope
grep -rn 'func Test' --include='*_test.go' . | wc -l
```

A well-tested codebase is a well-documented codebase. Tests reveal:

- **Expected behavior** — what the code is supposed to do.
- **Edge cases** — what inputs the authors considered.
- **Integration boundaries** — where one module talks to another.
- **Deprecated behavior** — tests that are skipped or commented out.

#### Read Tests Before Implementation

When you open a module, read its tests first. If the tests are good, you will understand the module's purpose without reading a single line of implementation. If the tests are bad — missing, flaky, or unclear — that is also information: the module is likely poorly understood by its own authors.

### Phase 4 — Navigation Tools

#### grep and ripgrep

```bash
# Basic pattern search
grep -rn 'TODO' --include='*.go' .

# ripgrep — faster, respects .gitignore
rg 'TODO' -t go

# Find all error types
rg 'type.*Error struct' -t go

# Find all HTTP handlers
rg 'func.*Handler.*http\.ResponseWriter' -t go

# Find all exported functions
rg '^func [A-Z]' -t go
```

`ripgrep` (`rg`) is preferred over `grep` for large codebases because it:
- Respects `.gitignore` automatically.
- Searches only tracked files (skips `vendor/`, `node_modules/`).
- Is 5-20x faster than `grep` on large repositories.

#### ctags — Code Navigation

```bash
# Generate tags file for a project
ctags -R --languages=Go,Python,Rust .

# Jump to definition of a symbol (in Vim)
# :tag SymbolName

# List all tags
cat tags | head -20
```

`ctags` builds an index of every symbol definition in the codebase. Your editor can then jump to any definition instantly — no grep needed.

#### LSP — Language Server Protocol

Modern editors (VS Code, Neovim, Emacs) use LSP servers to provide:
- **Go to definition** — jump to where a symbol is defined.
- **Find all references** — find every place a symbol is used.
- **Hover documentation** — see docs without leaving your cursor.
- **Rename** — refactor across the entire codebase safely.

Set up LSP before reading any large codebase. It is the single highest-impact tool for comprehension.

| Language    | LSP Server         |
|-------------|-------------------|
| Go          | `gopls`           |
| Rust        | `rust-analyzer`   |
| Python      | `pylsp` / `pyright` |
| TypeScript  | `tsserver`        |
| C/C++       | `clangd`          |

#### Code Search — GitHub and Sourcegraph

For open-source projects, use web-based code search:

- **GitHub code search** — works for any public repo. Fast for keyword search.
- **Sourcegraph** — purpose-built for code search across many repos. Supports regex, symbol search, and code intelligence.

```bash
# Search across GitHub
gh search code "func main" --language=go --repo=gorilla/mux

# Sourcegraph (browser-based)
# https://sourcegraph.com/search?q=repo:gorilla/mux+func+main
```

### Phase 5 — Read Commit History

#### git log — Understand the Timeline

```bash
# See the last 20 commits with author and date
git log --oneline -20

# See commits that touched a specific file
git log --follow -- path/to/file.go

# See commits by a specific author
git log --author="username"

# See a diff summary for each commit
git log --stat

# See the full diff for the last commit
git show HEAD
```

The commit log tells you:
- **What changed** — the diff.
- **When it changed** — timestamps reveal release cadence.
- **Who changed it** — the author, who you can ask questions.
- **Why it changed** — the commit message (if well-written).

#### git blame — Understand Why a Line Exists

```bash
# Who wrote each line of a file?
git blame path/to/file.go

# When was this line last modified?
git blame -f path/to/file.go

# Blame a specific range of lines
git blame -L 10,20 path/to/file.go
```

`git blame` answers: "Why is this line here?" Follow up by reading the commit that introduced it:
```bash
git show <commit-hash>
```

Corollary: write good commit messages. The person reading `git blame` six months from now is you.

### Phase 6 — The Architecture Onion: Read Interfaces First

#### Interfaces Before Implementations

When you encounter a module, read its public interface (API, type signatures, exported functions) before its implementation. The interface tells you _what_ the module does. The implementation tells you _how_ it does it. Always understand "what" before "how."

```go
// Read this first — the interface
type Cache interface {
    Get(key string) (string, error)
    Set(key string, value string, ttl time.Duration) error
    Delete(key string) error
}

// Then read this — the implementation
type redisCache struct {
    client *redis.Client
}

func (r *redisCache) Get(key string) (string, error) {
    return r.client.Get(context.Background(), key).Result()
}
```

This applies at every scale: read the package-level docs before the file-level code. Read the file-level exports before the internal functions. Read the function signature before the function body.

#### The Onion Model

1. **Outer layer** — README, CONTRIBUTING.md, top-level directory structure.
2. **Interface layer** — Public APIs, type signatures, protocol definitions.
3. **Module layer** — Directory structure, module boundaries, dependency graph.
4. **Implementation layer** — Internal functions, algorithms, data structures.
5. **Detail layer** — Line-by-line logic, edge case handling, performance hacks.

Read from the outside in. You do not need to reach the detail layer for most of the codebase most of the time.

### Phase 7 — Drawing the Map

#### Sketch Module Dependencies

Before you understand every module, sketch how they depend on each other:

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│   HTTP   │────▶│  Service │────▶│   Store   │
│  Handler │     │   Layer  │     │   Layer   │
└──────────┘     └──────────┘     └──────────┘
     │                │                  │
     ▼                ▼                  ▼
┌──────────┐     ┌──────────┐     ┌──────────┐
│   Auth   │     │   Cache  │     │ Database │
│  Module  │     │  Module  │     │  Driver  │
└──────────┘     └──────────┘     └──────────┘
```

This can be as simple as penciled boxes on paper. The act of sketching forces you to name the modules and decide what depends on what.

#### Trace Data Flow

For the main path through the system, trace how data flows:

```
HTTP Request
  → Router (match path to handler)
  → Auth Middleware (validate token)
  → Handler (parse request)
  → Service (apply business logic)
  → Repository (query database)
  → Database
  ← Response (serialize result)
```

#### Build a Call Graph

```bash
# Find all function calls in a Go file
grep -o '\w\+(.*' file.go | sort | uniq

# Find all callers of a specific function
rg 'functionName\(' -t go

# Visualize dependencies (Go)
go mod graph | head -20
```

### Phase 8 — Reading for Different Purposes

#### Bug Fixing: Trace Backwards from Failure

When you are fixing a bug, your reading strategy is surgical:

1. Start at the error message or failing test.
2. Trace backwards: what called the function that failed?
3. What data was passed to it? Was it valid?
4. Where was that data constructed?
5. Fix the root cause, not the symptom.

```bash
# Find where an error is defined
rg 'ErrSpecificError' -t go

# Find where it is returned
rg 'return.*ErrSpecificError' -t go

# Find the tests that exercise this error path
rg 'ErrSpecificError' --include='*_test.go'
```

#### Feature Adding: Find Similar Features

When you are adding a feature, your reading strategy is imitative:

1. Find a feature that is similar to what you want to add.
2. Read how it is implemented end-to-end: route → handler → service → store.
3. Copy the pattern, modifying for your new feature.
4. This is not "copy-paste programming" — it is _following established conventions_.

```bash
# Find an existing HTTP endpoint
rg 'r.HandleFunc.*Handle' -t go

# Read its handler, service, and repository
# Then add your new endpoint following the same pattern
```

#### Refactoring: Understand Ownership

When you are refactoring, your reading strategy is comprehensive:

1. Map all callers of the code you want to change.
2. Map all dependencies of the code you want to change.
3. Check tests: they define the behavior you must preserve.
4. Check performance benchmarks: they define constraints you must not violate.
5. Only then begin the refactor.

```bash
# Find all callers of a function
rg 'myFunction\(' -t go

# Find all implementations of an interface
rg 'type.*struct.*{[^}]*}' -A 5 -t go | rg 'myInterface'

# Find all tests for a package
find . -path '*/my_package/*_test.go'
```

### The 20-Minute Rule

**Spend 20 minutes reading before asking a question.**

This rule is not about pride. It is about efficiency. When you ask a question too early, you do not yet know enough to understand the answer. After 20 minutes of focused reading, you will either:

1. Have found the answer yourself (most common outcome).
2. Have a precise, answerable question that a senior developer can resolve in one sentence.

The question "How does authentication work?" will get you a 30-minute explanation that you will forget. The question "Why does `verifyToken` in `auth.go:47` return `ErrExpired` when `claims.Exp` is in the past?" will get you a precise, useful answer.

### Common Codebase Patterns

#### Layered Architecture

The most common pattern. Code is organized in horizontal layers:

```
Presentation Layer (handlers, views, controllers)
     │
Business Logic Layer (services, domain logic)
     │
Data Access Layer (repositories, DAOs, models)
     │
Database / External Services
```

Reading strategy: Start at the presentation layer. Follow a request down through each layer. Each layer should add behavior, not just pass data through.

#### Hexagonal Architecture (Ports and Adapters)

Core domain logic sits in the center. Everything else is an adapter:

```
         ┌──────────┐
         │ HTTP     │──▶ Adapter
         │ Handler  │
         └──────────┘
              │
         ┌──────────┐
         │ Domain   │ ◀── Core (no external dependencies)
         │ Logic    │
         └──────────┘
              │
         ┌──────────┐
         │ Database │──▶ Adapter
         │ Repo     │
         └──────────┘
```

Reading strategy: Read the domain core first. It should contain zero knowledge about HTTP, databases, or any external system. Then read the adapters — each one is a thin translation layer.

#### Event-Driven Architecture

Producers emit events. Consumers react to them. Decoupling is high; tracing is hard.

```
Producer ──▶ Event Bus ──▶ Consumer A
                       ──▶ Consumer B
                       ──▶ Consumer C
```

Reading strategy: Find the event definitions first. They are the contract. Then trace: who produces each event? Who consumes it? What happens when a consumer fails?

#### Microservices Architecture

Multiple independently deployable services communicate over the network.

Reading strategy: You cannot read the entire system at once. Pick one service. Read its API contract (OpenAPI spec, protobuf definitions). Then read its internal structure as a monolith. Cross-service boundaries only when necessary.

### The Danger of Assumptions

#### Verify, Don't Guess

Every assumption you make about a codebase is a risk. Common assumptions that turn out to be wrong:

- "This function must be called from here" — verify with `rg 'functionName\('`.
- "This variable is always non-nil" — verify with `rg 'variableName.*nil'`.
- "This module is thread-safe" — verify with `rg 'mutex\|sync\.Mutex\|Lock\(\)'`.
- "This API returns JSON" — verify by reading the response type, not the URL pattern.
- "This config is loaded from the environment" — verify by reading the config loader.

#### Verify with Running Code

When in doubt, add a print statement or a log line and run the code. Observations trump assumptions:

```bash
# Run with debug logging
DEBUG=1 go run ./cmd/server

# Run tests with verbose output
go test -v ./...

# Run with a specific test
go test -run TestSpecificFunction -v
```

### How to Ask Good Questions About a Codebase

After your 20 minutes of reading, if you still need help, frame your question precisely:

| Bad Question                       | Good Question                                           |
|------------------------------------|--------------------------------------------------------|
| "How does auth work?"              | "I see `verifyToken` called in `middleware.go:23`. Why does it check `claims.Exp` instead of `claims.Nbf`?" |
| "Where is the database?"           | "I found `Connect()` in `store.go:15`. Is this the only database connection, or are there read replicas elsewhere?" |
| "Why is this slow?"                | "The `ListUsers` query in `repository.go:78` fetches all columns including the `avatar_blob` field. Is there a reason we don't paginate?" |
| "Can you explain this module?"     | "The `processor` package has three exported types: `Queue`, `Worker`, and `Router`. I understand `Queue` and `Worker`, but `Router` does not seem to be called from anywhere except tests. Is it deprecated?" |

A good question shows you have done your homework, identifies a specific gap, and gives the respondent enough context to answer precisely.

## Use It

### Production Examples

- **The Go standard library** — Read `src/net/http/server.go`. The entry point `ListenAndServe` leads to `Server.ListenAndServe`, which calls `Serve`, which calls `serveHTTP`. This is the main flow. From there, the `Handler` interface is the extensibility point.

- **The Linux kernel** — Start with `init/main.c`. The `start_kernel` function is the entry point. Trace it through subsystem initialization to understand boot order.

- **PostgreSQL** — Start with `src/backend/tcop/postgres.c`. The `PostgresMain` function is the main loop. Read how it receives a query, parses, plans, and executes.

All three projects follow the principle of reading interfaces first. In Go, read the `Handler` interface. In Linux, read the `file_operations` struct. In PostgreSQL, read the `ExecutorRun` function signature. The implementations are detail layers.

## Read the Source

- Go standard library: `src/net/http/server.go` — read the `Handler` interface and `ServeMux` to understand how HTTP routing works at the standard library level.
- Linux kernel: `init/main.c` — read `start_kernel` to see subsystem initialization order.
- PostgreSQL: `src/backend/tcop/postgres.c` — read `PostgresMain` to understand the query processing loop.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`reading_codebases_guide.md`** — A reference card with strategies, tools, and patterns for reading any unfamiliar codebase.

## Exercises

1. **Easy** — Pick an open-source project. Find its entry point, trace the main flow to the first output, and write a one-paragraph summary. Do not look at any documentation first.

2. **Medium** — Pick a module from a codebase you work with. Read its tests first, then its public interface, then its implementation. Write a module dependency sketch showing what it depends on and what depends on it.

3. **Hard** — Pick a bug in an open-source project's issue tracker. Trace backwards from the reported error to the root cause using only `git log`, `git blame`, and code search. Write up your trace as a narrative: "I started at X, which led me to Y, which revealed Z, which is the root cause because..."

## Key Terms

| Term                | What people say         | What it actually means                                    |
|---------------------|------------------------|-----------------------------------------------------------|
| Entry point         | "Where does it start?" | The first function executed; the root of the call tree.    |
| Call graph          | "Who calls what?"      | A directed graph of function calls showing dependencies.   |
| Module dependency   | "What depends on what" | Which modules import or reference which other modules.     |
| Interface leak      | "It knows too much"    | When implementation details bleed into the public API.     |
| Architecture onion | "Read outside-in"      | Reading interfaces before implementations, layer by layer.|
| 20-minute rule      | "Read before asking"   | Spend 20 minutes investigating before asking for help.     |

## Further Reading

- *Reading Code* by Girish Suryanarayana — systematic approaches to code comprehension.
- *The Programmer's Brain* by Felienne Hermans — cognitive science of reading code.
- *Software Design X-Rays* by Adam Tornhill — using version history to understand architecture.
- *A Philosophy of Software Design* by John Ousterhout — why depth-first reading beats breadth-first.
- `ripgrep` guide: https://github.com/BurntSushi/ripgrep/blob/master/GUIDE.md
- Sourcegraph documentation: https://docs.sourcegraph.com