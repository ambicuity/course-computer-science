# Monorepos vs Polyrepos

> One repo to rule them all, or one repo per microservice? The answer depends on your scale, your team, and your pain tolerance.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 16 lessons 01–14
**Time:** ~45 minutes

## Learning Objectives

- Define monorepo and polyrepo and explain the structural difference.
- List the advantages and disadvantages of each approach with concrete examples.
- Describe how major companies (Google, Meta, Amazon, Microsoft) solve monorepo CI/CD at scale.
- Explain dependency management strategies in monorepos: explicit deps, build graph caching, affected target detection.
- Compare monorepo tooling (Bazel, Nx, Turborepo, Lerna, Rush) and when each is appropriate.
- Apply a decision framework to choose monorepo, polyrepo, or hybrid for a given project context.

## The Problem

You're building a platform with three services: an API server, a web frontend, and a shared utility library. Where does the code live?

- **Option A:** One repository containing all three projects. A single `git clone` gives you everything. A commit can atomically update the library and both consumers.
- **Option B:** Three separate repositories. Each has its own CI pipeline, its own versioning, its own deploy cycle. The shared library is published as a package.

This is the monorepo vs polyrepo decision, and it shows up the moment your project grows beyond a single deployable unit. Make the wrong call and you'll spend your days fighting dependency hell, broken builds, or access control nightmares.

## What Is a Monorepo?

A **monorepo** (monolithic repository) stores all related projects — services, libraries, frontends, tooling — in a single version-controlled repository. The defining trait is not size but **unified versioning**: every project shares one commit history.

```
my-monorepo/
├── apps/
│   ├── api-server/
│   ├── web-frontend/
│   └── admin-dashboard/
├── libs/
│   ├── auth-lib/
│   ├── ui-components/
│   └── utils/
├── tools/
│   └── deploy-scripts/
├── package.json          # root-level config
├── nx.json               # monorepo tool config
└── tsconfig.base.json    # shared compiler config
```

Key properties of a monorepo:

1. **Single source of truth** — One repo, one commit graph, one set of branches.
2. **Atomic commits** — A change to a shared library and all its consumers lands in a single commit.
3. **Code sharing without publishing** — Libraries are imported directly via path aliases, not through a package registry.
4. **Unified tooling** — Linting, formatting, testing, and building share configuration across every project.

Google, Meta, Microsoft, and Amazon all operate monorepos with tens of thousands of developers and billions of lines of code.

## What Is a Polyrepo?

A **polyrepo** (poly-repository, also called multi-repo) gives each project — each service, each library — its own repository. Each repo has independent versioning, independent CI/CD, and independent access control.

```
github.com/my-org/
├── api-server/        ← its own repo
├── web-frontend/     ← its own repo
├── admin-dashboard/  ← its own repo
├── auth-lib/         ← its own repo
├── ui-components/    ← its own repo
└── utils/            ← its own repo
```

Key properties of a polyrepo:

1. **Full isolation** — Each project is self-contained with its own CI pipeline and release cycle.
2. **Independent versioning** — Each library publishes its own semver; consumers pin versions.
3. **Fine-grained access** — Permissions are per-repo; the frontend team can't push to the API repo.
4. **Small clone size** — Developers download only what they need.

Most open-source ecosystems are polyrepo by default — each library lives in its own GitHub repository.

## Monorepo Advantages

### 1. Atomic Commits Across Projects

The single biggest win. When you fix a bug in `auth-lib`, you can update `api-server` and `web-frontend` in the same commit. No version bump, no package publish, no downstream PRs.

```
commit abc123
Author: you
Date:   today

    Fix token expiry bug in auth-lib and update all callers
    
    libs/auth-lib/src/validate.ts    | 2 +-
    apps/api-server/src/middleware.ts | 4 ++--
    apps/web-frontend/src/hooks.ts    | 3 +--
```

In a polyrepo, this requires: (1) fix auth-lib, (2) publish auth-lib v2.1.1, (3) update api-server's package.json, (4) update web-frontend's package.json, (5) open two PRs, (6) coordinate the deploy.

### 2. Shared Code Without Package Publishing

In a monorepo, shared libraries are local packages imported via workspace paths:

```json
// apps/api-server/package.json
{
  "dependencies": {
    "@my-org/auth-lib": "workspace:*",
    "@my-org/utils": "workspace:*"
  }
}
```

No npm publish step. No `npm install @my-org/auth-lib@latest`. The build system resolves the dependency to a local directory. This eliminates the publish-consume lag entirely.

### 3. Cross-Project Refactoring

Renaming an exported function across 12 packages in a monorepo is one codemod run. In a polyrepo, it's 12 PRs across 12 repos, each blocked on the previous one because you must maintain backward compatibility until all consumers migrate.

Tools like `jscodeshift`, TypeScript's `--declarationMap`, or Nx's `nx generate` can perform project-wide refactors atomically in a monorepo.

### 4. Single Source of Truth

One repo means one place to look. Want to know which version of `auth-lib` the API server uses? Check the workspace config — it's always the current version because they share the same commit history.

In a polyrepo, you might find five different versions of `auth-lib` pinned across services, and no one knows which ones are actually compatible with each other.

### 5. Consistent Tooling

A single root-level config ensures every project uses the same linter, formatter, TypeScript version, and test runner:

```
.eslintrc.base.json    → shared by all projects
.prettierrc            → same formatting everywhere
tsconfig.base.json     → shared compiler options
jest.preset.js         → shared test config
```

This eliminates the "works on my machine" class of bugs where Project A uses Jest 27 and Project B uses Jest 29 with different configs.

## Monorepo Disadvantages

### 1. Repository Size

A monorepo contains everything. Google's monorepo is over 86 million files. Even for a mid-size company, clones can take minutes and consume gigabytes of disk.

Mitigations:
- **Sparse checkouts** (`git sparse-checkout`) let developers clone only the directories they need.
- **Shallow clones** (`git clone --depth 1`) reduce history download.
- **Virtual file systems** (Microsoft's VFS for Git, now largely replaced by scalar) stream file contents on demand.

### 2. CI Scalability

Every push triggers a CI pipeline. In a monorepo with 50 projects, a commit to project A shouldn't rebuild and retest projects B through Z.

Solutions (detailed later in "The Build Problem"):
- **Affected target detection** — only build/test what changed.
- **Build graph caching** — cache remote build artifacts.
- **Incremental builds** — rebuild only changed targets and dependents.

Without these, CI time grows linearly with monorepo size, and developers stop committing.

### 3. Access Control

Git doesn't natively support per-directory permissions. In a monorepo, every developer with write access can theoretically modify any project.

Mitigations:
- **CODEOWNERS files** — define required reviewers per directory.
- **Branch protection rules** — require reviews from specific teams.
- **CI-level enforcement** — reject PRs that modify files outside the author's domain.
- **Monorepo tools** — Nx and Bazel support per-project permission configs.

### 4. Ownership and Governance

When 200 developers share a repo, who decides the linting rules? Who approves the upgrade to TypeScript 5.4? Who owns the shared CI pipeline?

A monorepo requires:
- A platform or infra team that maintains the build system.
- Clear governance for shared configurations.
- A way to opt-out or override for projects with legitimate differences.

## Polyrepo Advantages

### 1. Project Isolation

Each repo is an independent unit. Breaking changes in `auth-lib` don't block deployments of `api-server` — they pin their own version and upgrade on their own schedule.

Isolation also means a broken CI pipeline in one repo doesn't affect another. If the `admin-dashboard` tests are flaky, the `api-server` deploy still proceeds.

### 2. Independent CI/CD

Each repo has its own pipeline with its own deploy cadence. The `web-frontend` team can deploy 10 times a day while the `api-server` team deploys once a week. No coordination overhead.

```
api-server repo     → CI: 8 min, deploy: manual approval
web-frontend repo   → CI: 3 min, deploy: auto on merge
admin-dashboard repo → CI: 5 min, deploy: auto on merge
```

### 3. Fine-Grained Access Control

In a polyrepo, access is per-repo. The payments team owns the `payments-service` repo. The frontend team can't push to it, and they don't need to.

GitHub, GitLab, and Bitbucket all provide repo-level permission models that work directly with polyrepos.

### 4. Small Repository Size

Each clone is small. Developers download only the project they work on. This means faster clones, faster IDE indexing, and faster git operations.

For a polyrepo with 10 services averaging 50k LOC each, a developer clones 50k LOC instead of 500k.

## Polyrepo Disadvantages

### 1. Dependency Hell

In a polyrepo, shared libraries are consumed as versioned packages. This creates coordination problems:

- **Version skew** — Service A uses `auth-lib@2.0.0` while Service B still uses `auth-lib@1.3.0`. They behave differently.
- **Diamond dependencies** — Service A depends on `utils@3.0.0` which depends on `logger@2.0.0`, but Service A also depends on `http-client@1.0.0` which depends on `logger@1.5.0`. Two versions of `logger` coexist.
- **Orphaned versions** — `auth-lib@2.3.0-beta.1` is published but never consumed. Nobody knows if it can be deleted.

```
api-server ─── auth-lib@2.0.0 ─── logger@2.0.0
     └── http-client@1.0.0 ─── logger@1.5.0  ← conflict!
```

### 2. Cross-Repo Refactoring

Renaming a function in `auth-lib` requires:
1. Update and publish `auth-lib@3.0.0` (breaking change).
2. Update `api-server` to use `auth-lib@3.0.0`.
3. Update `web-frontend` to use `auth-lib@3.0.0`.
4. Each update is a separate PR in a separate repo with a separate review.

This can take days or weeks to fully propagate. During the transition, `auth-lib@2.x` and `auth-lib@3.x` must coexist, meaning you maintain backward compatibility for the migration period.

### 3. Version Skew Across Services

At any given time, your production environment runs different versions of shared libraries across services. This means:
- Inconsistent behavior (a bug fix in `auth-lib@2.1.0` is live in `api-server` but not yet in `admin-dashboard`).
- Integration bugs that only appear when specific version combinations interact.
- Difficulty reproducing production issues because you must reconstruct the exact version matrix.

### 4. Code Dupination

Without a shared library published as a package, teams copy-paste utility code. The `formatDate` function exists in 5 repos with 5 slightly different implementations. This is the natural consequence of the coordination cost of maintaining a shared library in a polyrepo.

Even with a shared library, the friction of publishing and consuming packages discourages small, incremental improvements. Teams tend to "bundle up" changes into larger releases rather than making frequent small updates.

### 5. Tooling Fragmentation

Each repo can drift toward different tooling. Project A uses Prettier, Project B uses StandardJS. Project A uses Jest, Project B uses Vitest. Project A uses TypeScript 4.9, Project B uses TypeScript 5.3.

This fragmentation creates inconsistency that slows down developers who work across repos. Every new repo requires learning a slightly different setup.

## The Build Problem: Monorepo CI/CD at Scale

The central challenge of monorepos is CI/CD performance. If every push rebuilds everything, CI becomes a bottleneck that grows with the repo. Here is how the major companies solve it.

### Google — Bazel

Google's monorepo (the largest known, with 86+ million files) runs on **Bazel**, their internally-developed build system (open-sourced in 2015).

Key mechanisms:

- **Hermetic builds** — Every build action declares its inputs and outputs explicitly. Given the same inputs, the output is deterministic. This enables aggressive caching.
- **Remote execution** — Build actions can be distributed across thousands of workers. Your laptop doesn't compile; a cluster does.
- **Incremental builds** — Bazel computes the dependency graph and only rebuilds targets affected by changes. If `libs/utils` didn't change, its cached output is reused even if `apps/api-server` (which depends on it) did change.
- **Content-addressable caching** — Build artifacts are stored by hash of their inputs. If two developers build the same target with the same inputs, the second one gets a cache hit.

```python
# Bazel BUILD file — explicit dependency declaration
py_binary(
    name = "api_server",
    srcs = ["main.py"],
    deps = [
        "//libs/auth_lib:auth_lib",
        "//libs/utils:utils",
    ],
)
```

### Meta — Buck

Meta (Facebook) uses **Buck**, their own build system. Buck2 (the Rust rewrite, open-sourced in 2023) uses the same core ideas as Bazel:

- **Fine-grained dependency tracking** — Buck tracks dependencies at the file level, not just the target level. A change to `utils.py` only invalidates targets that import `utils.py`.
- **Distributed caching** — Build results are cached globally. A change tested by one engineer is available to every other engineer.
- **Concurrent execution** — Build actions that don't depend on each other run in parallel.

Meta also uses **getdeps**, a dependency manager that builds third-party dependencies from source within the monorepo, ensuring they're built with the same toolchain and cached the same way.

### Amazon — Internal Build Systems

Amazon's monorepo strategy is more polyrepo-leaning, but their internal build systems (not fully open-sourced) use similar principles:

- **Build graphs** — Every package declares its dependencies, forming a directed acyclic graph (DAG).
- **Artifact caching** — Built packages are stored in an internal artifact repository. Rebuilding a package checks the cache first.
- **Affected target detection** — Only packages reachable from changed files are built and tested.

Amazon's AWS services are individually versioned and deployed, so even within a monorepo, the deployment model is closer to polyrepo-style independent releases.

### Microsoft — Azure DevOps + Git Virtual File System

Microsoft's Windows dev team famously moved the Windows codebase into a Git monorepo (300+ GB). The challenges they faced led to key innovations:

- **VFS for Git** (now **Scalar**) — A Git virtualization layer that only downloads files when they're accessed, not during clone. A 300 GB repo clones in minutes.
- **GVFS (Git Virtual File System)** — Provides on-demand file hydration, so developers only download the files they actually open.
- **Azure DevOps pipeline caching** — CI pipelines cache build outputs by content hash, similar to Bazel's approach.
- **Repository-specific CI triggers** — Only run CI for projects affected by a change.

Microsoft's TypeScript repo itself is a monorepo managed with **Lerna** (now migrating to Turborepo), demonstrating how even tooling teams use monorepos internally.

## Dependency Management in Monorepos

Dependency management is where monorepos shine and where they add complexity. Three key strategies:

### Explicit Dependency Declarations

Every project must declare what it depends on. No implicit dependencies through relative imports that bypass the build system.

```json
// In an Nx monorepo — explicit workspace dependency
{
  "name": "api-server",
  "dependencies": {
    "@myorg/auth-lib": "workspace:*"
  }
}
```

In Bazel, this is enforced at the BUILD file level. In Nx, the `implicitDependencies` and `tags` system enforces dependency boundaries. Without explicit declarations, you get hidden coupling that makes CI unreliable.

### Build Graph Caching

The build system maintains a directed acyclic graph (DAG) of all targets and their dependencies. When a file changes:

1. The build system identifies which targets are affected.
2. It checks the remote cache for those targets.
3. Cached outputs are downloaded instead of rebuilt.
4. Only targets whose inputs have changed (or whose dependencies changed) are rebuilt.

```
         ┌──────────┐
         │  auth-lib │
         └─────┬────┘
               │
    ┌──────────┼──────────┐
    │          │          │
┌───▼───┐  ┌──▼───┐  ┌──▼──────┐
│ api   │  │ web  │  │ admin    │
│ server│  │ FE   │  │ dashboard│
└───────┘  └──────┘  └─────────┘

Change in auth-lib → rebuild api-server, web-FE, admin-dashboard
Change in web-FE only → rebuild web-FE only
```

### Affected Target Detection

This is the critical optimization. When you commit a change, the CI system must know: which projects need to be tested?

```
# Nx affected command — only test what changed
npx nx affected --target=test --base=main~1 --head=main

# Bazel query — find all targets depending on changed files
bazel query "rdeps(//..., <changed_files>)"
```

Without affected target detection, CI rebuilds and tests everything on every commit. With it, a change to the web frontend doesn't trigger API server tests.

The three common implementations:
1. **Git diff analysis** — Compare the changed files between commits, map files to projects.
2. **Dependency graph traversal** — Starting from changed targets, traverse the dependency graph to find all dependents.
3. **Content hashing** — Hash the inputs of each target; if the hash matches the cache, skip it.

## Code Ownership

In a monorepo, code ownership is critical for governance.

### CODEOWNERS

GitHub's `CODEOWNERS` file defines who must review changes to specific paths:

```
# CODEOWNERS file in repo root
/libs/auth-lib/       @my-org/auth-team
/apps/api-server/     @my-org/backend-team
/apps/web-frontend/   @my-org/frontend-team
/tools/*              @my-org/platform-team
```

When a PR modifies files in `/libs/auth-lib/`, GitHub automatically requests review from `@my-org/auth-team`. This provides access control without repository-level isolation.

### Per-Directory Permissions

Some monorepo tools enforce ownership at the CI level:

- **Nx** — The `nx.json` tags system can enforce that a project tagged `scope:auth` can only depend on other `scope:auth` or `scope:shared` projects.
- **Bazel** — Visibility labels restrict which targets can depend on a given target.
- **Custom CI checks** — Scripts that validate PR scope against team ownership before merging.

### Ownership at Scale

Google uses a `OWNERS` file in each directory (similar to CODEOWNERS but more granular). It lists reviewers and approvers. A change must be approved by an OWNER of every file it touches. This is enforced by automation, not by trust.

## Monorepo Tools

### Bazel

**Best for:** Large organizations, polyglot codebases, maximum build performance.

- Language-agnostic (Java, Python, Go, C++, TypeScript, and more).
- Hermetic, reproducible builds.
- Remote execution and caching.
- Steep learning curve; BUILD files are verbose.
- Industry use: Google, Stripe, Pinterest, Databricks.

```python
# Bazel BUILD file
load("@npm//:defs.bzl", "npm_link_all_packages")

ts_library(
    name = "auth_lib",
    srcs = glob(["src/**/*.ts"]),
    deps = ["//libs/utils"],
)

ts_project(
    name = "api_server",
    srcs = glob(["src/**/*.ts"]),
    deps = [":auth_lib"],
)
```

### Nx

**Best for:** TypeScript/JavaScript monorepos, mid-size teams, incremental adoption.

- Built-in affected command for CI optimization.
- Computation caching (local and remote).
- Task orchestration (run tasks in dependency order).
- Powerful dependency constraints via tags.
- Industry use: Google (for some JS projects), Cisco, VMware, Stripe.

```json
// nx.json — dependency constraints
{
  "affected": { "defaultBase": "main" },
  "targetDefaults": {
    "build": { "dependsOn": ["^build"], "cache": true },
    "test": { "dependsOn": ["^build"], "cache": true }
  }
}
```

### Turborepo

**Best for:** Small-to-mid JavaScript/TypeScript monorepos, teams new to monorepos.

- Simple setup, minimal configuration.
- Pipeline-based task execution (define build order in `turbo.json`).
- Remote caching (via Vercel or self-hosted).
- Good DX for teams already using pnpm workspaces.
- Industry use: Vercel's own monorepo, many startups.

```json
// turbo.json
{
  "pipeline": {
    "build": { "dependsOn": ["^build"], "outputs": ["dist/**"] },
    "test": { "dependsOn": ["build"] },
    "lint": {}
  }
}
```

### Lerna

**Best for:** Publishing multiple npm packages from a monorepo.

- Focuses on package versioning and publishing.
- Often paired with Nx or Turborepo for build management.
- Lerna 6+ uses Nx under the hood for task running.
- Industry use: Babel, Jest, React (historically), TypeScript.

### Rush

**Best for:** Enterprise JavaScript monorepos, strict governance needs.

- Built-in change management (rush change).
- Strict dependency version policies.
- Bulk versioning and publishing.
- Supports multiple package managers (pnpm, npm, yarn).
- Industry use: Microsoft (Office 365, Azure SDK), many enterprise teams.

```
// rush.json — repo config
{
  "projectFolderMinDepth": 2,
  "projectFolderMaxDepth": 2,
  "pnpm": { "version": "8.6.0" },
  "projects": [
    { "packageName": "api-server", "reviewCategory": "production" },
    { "packageName": "web-frontend", "reviewCategory": "production" }
  ]
}
```

### Tool Comparison Summary

When to reach for each:

| Tool       | Team size | Languages     | Learning curve | Best feature                    |
|------------|-----------|---------------|-----------------|---------------------------------|
| Bazel      | 100+      | Any           | Very high       | Hermetic builds, remote exec   |
| Nx         | 5–200     | JS/TS focused | Medium          | Affected commands, constraints  |
| Turborepo  | 2–50      | JS/TS only    | Low             | Simple setup, fast caching     |
| Lerna      | 3–30      | JS/TS only    | Low             | Package publishing              |
| Rush       | 20–500    | JS/TS focused | Medium-high     | Change management, governance   |

## When to Choose Monorepo

Choose a monorepo when:

1. **Small-to-medium team (2–50 developers)** working on related projects. The coordination cost of polyrepos exceeds the CI complexity of a monorepo.
2. **Shared libraries with frequent changes.** If you're updating a shared auth library weekly, the publish-consume cycle of polyrepos becomes painful.
3. **Frequent cross-project refactors.** If your API contracts and frontend types need to stay in sync, a monorepo makes this atomic.
4. **Prototyping and rapid iteration.** Starting a new service that depends on existing libraries is a `mkdir` away — no repo setup, no CI config, no package publishing.
5. **Startup or early-stage product.** One repo means one CI pipeline to configure, one set of lint rules to set up, one deploy process to learn.

## When to Choose Polyrepo

Choose a polyrepo when:

1. **Large organization with independent teams.** If the payments team and the content team never share code, separate repos reduce coordination overhead.
2. **Open-source ecosystem.** Public packages need their own repos for独立的 versioning,贡献, and release cycles.
3. **Regulatory isolation.** HIPAA, PCI-DSS, or SOX compliance may require separate access controls, audit trails, and deploy processes that align better with repo boundaries.
4. **Heterogeneous tech stacks.** If one team uses Python/FastAPI and another uses Rust/Actix, a monorepo's shared tooling provides less value.
5. **Different release cadences.** If Service A deploys hourly and Service B deploys quarterly, independent repos let each team optimize their own CI pipeline.

## The Hybrid Approach

Most organizations end up somewhere between pure monorepo and pure polyrepo.

### Monorepo with Git Submodules

Include external repositories as submodules within a monorepo. This lets you keep vendor dependencies or partner code in their own repos while maintaining a unified development environment.

```
main-monorepo/
├── apps/
├── libs/
├── vendor/
│   ├── partner-sdk/   ← git submodule
│   └── oss-lib/       ← git submodule
└── package.json
```

Tradeoffs:
- Submodules add complexity to clone and update operations.
- They provide a middle ground for "mostly monorepo, some external code."
- Git submodules have known UX issues (detached HEAD state, update friction).

### Multiple Monorepos by Domain

Split repositories by business domain or team boundary:

```
github.com/my-org/
├── payments-monorepo/    ← payments team
│   ├── apps/api/
│   └── libs/
├── content-monorepo/     ← content team
│   ├── apps/web/
│   └── libs/
└── platform-libraries/  ← shared infra
    ├── auth-lib/
    └── observability/
```

Tradeoffs:
- Each monorepo is manageable in size.
- Cross-domain changes require coordination between repos (the polyrepo problem).
- Shared libraries need a publishing mechanism (the polyrepo problem, but less frequent).

### Published Packages from a Monorepo

Some organizations use a monorepo internally but publish packages externally:

```
my-monorepo/
├── libs/
│   ├── auth-lib/       ← internally used via workspace:*
│   └── ui-components/  ← published as npm package @my-org/ui
└── apps/
    └── api-server/     ← private, not published
```

This gives you monorepo advantages internally while still publishing packages for external consumers or other repos.

## Exercises

1. **Easy** — Set up a monorepo with Nx or Turborepo containing two apps and one shared library. Verify that a change to the library triggers rebuilds of both apps but not vice versa.

2. **Medium** — Create a polyrepo setup where three services consume a shared library via npm packages. Introduce a breaking change to the library and experience the coordination cost (update the library, publish a new version, update each consumer).

3. **Hard** — Design a hybrid architecture for a mid-size SaaS company (5 teams, 12 services, 4 shared libraries). Document which projects go in which repos and justify each decision using the tradeoffs discussed in this lesson. Consider CI/CD, access control, and release cadence.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Monorepo | "One big repo" | A single repository containing multiple related projects with unified versioning and tooling |
| Polyrepo | "Microservices repos" | Each project/service has its own repository with independent versioning and CI/CD |
| Affected target | "What changed" | A project that is transitively reachable from modified files in the dependency graph |
| Hermetic build | "Reproducible build" | A build that declares all inputs explicitly and produces deterministic output regardless of the environment |
| Build graph | "Dependency tree" | A directed acyclic graph (DAG) of all build targets and their declared dependencies |
| Diamond dependency | "Version conflict" | When two dependencies of the same project require different versions of a shared library |
| CODEOWNERS | "Review assignments" | A file that maps file paths to teams/individuals who must review changes to those paths |
| Sparse checkout | "Partial clone" | A Git feature that checks out only specific directories, reducing clone time and disk usage |
| Workspace protocol | "Local package link" | A package manager feature (e.g., `workspace:*`) that resolves a dependency to a local project instead of a registry |

## Further Reading

- [Bazel Documentation](https://bazel.build/) — Google's build system for large-scale monorepos
- [Nx Documentation](https://nx.dev/) — Monorepo management for TypeScript/JavaScript
- [Turborepo Documentation](https://turbo.build/repo) — High-performance build system for JavaScript monorepos
- ["Why Google Stores Billions of Lines of Code in a Single Repository"](https://cacm.acm.org/research/why-google-stores-billions-of-lines-of-code-in-a-single-repository/) — Rachel Potvin, IEEE 2016
- ["Monorepo: Please don't"](https://medium.com/@maxbeatty/monorepo-please-dont-7aa5b6c2fad4) and the subsequent rebuttals — a balanced view of monorepo tradeoffs
- [Rush Documentation](https://rushjs.io/) — Scalable monorepo manager for JavaScript
- [Lerna Documentation](https://lerna.js.org/) — Tool for managing JavaScript monorepos with multiple packages