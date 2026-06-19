# Notes — Monorepos vs Polyrepos

## Comparison Tables

### Monorepo vs Polyrepo: At a Glance

| Aspect | Monorepo | Polyrepo |
|--------|----------|----------|
| Structure | All projects in one repo | One repo per project |
| Atomic commits | Yes — across all projects | No — requires coordinated PRs |
| Shared code | Direct workspace imports | Published packages (npm, Maven, etc.) |
| Cross-project refactoring | Single commit | Multiple PRs + version bumps |
| CI complexity | High — requires affected-target detection | Low — each repo has its own pipeline |
| Repository size | Large — full codebase clone | Small — project-specific clone |
| Access control | CODEOWNERS + CI enforcement | Native repo-level permissions |
| Version skew | Impossible — single version of truth | Likely — different consumers on different versions |
| Tooling consistency | Enforced at root level | Each repo can drift |
| Onboarding | Clone once, everything available | Clone multiple repos, setup varies |
| Dependency management | Build graph + workspace protocol | Package registries + semver |
| Best team size | 2–50 developers | 50+ developers with independent teams |

### Monorepo Advantages and Disadvantages

| Advantage | Description |
|-----------|-------------|
| Atomic commits | Update library + all consumers in one commit |
| No publish step | Import shared code via path aliases |
| Cross-project refactoring | Codemods apply repo-wide |
| Single source of truth | One version of each dependency |
| Consistent tooling | Root-level config for all projects |
| Easy onboarding | Single clone gives full context |

| Disadvantage | Description |
|--------------|-------------|
| Repo size | Clones can be GB-scale |
| CI scalability | Requires affected-target detection |
| Access control | Git has no per-directory permissions |
| Ownership | Requires governance for shared config |
| Build complexity | Need specialized build tooling (Bazel, Nx) |
| Blast radius | A bad commit can break everything |

### Polyrepo Advantages and Disadvantages

| Advantage | Description |
|-----------|-------------|
| Isolation | Each project is self-contained |
| Independent CI/CD | Deploy on any cadence |
| Fine-grained access | Per-repo permissions |
| Small clones | Only what you need |
| Clear ownership | Repo boundary = team boundary |
| Tech stack freedom | Each repo can use different tools |

| Disadvantage | Description |
|--------------|-------------|
| Dependency hell | Version conflicts across services |
| Cross-repo refactoring | Coordinated multi-PR changes |
| Version skew | Services running different library versions |
| Code duplication | Copy-paste across repos |
| Tooling fragmentation | Drifting lint/test/build configs |
| Coordination overhead | Publishing + consuming packages is slow |

## Tool Landscape

### Monorepo Build Tools

| Tool | Languages | Caching | Remote Execution | Learning Curve | Best For |
|------|-----------|---------|-------------------|----------------|----------|
| Bazel | Any | Content-addressable | Yes (remote build execution) | Very high | Large orgs, polyglot |
| Nx | JS/TS | Local + remote | Via Nx Cloud | Medium | Mid-size JS teams |
| Turborepo | JS/TS | Local + remote | Via Vercel | Low | Small JS teams, startups |
| Lerna | JS/TS | Via Nx | No | Low | Package publishing |
| Rush | JS/TS | Build cache | Via Azure DevOps | Medium-high | Enterprise JS teams |

### CI Optimization Strategies

| Strategy | How It Works | Tools |
|----------|--------------|-------|
| Affected-target detection | Only build/test projects reachable from changed files | Nx `affected`, Bazel `rdeps` |
| Remote caching | Store build outputs by content hash; skip rebuilds on cache hit | Bazel remote cache, Nx Cloud, Turborepo |
| Incremental builds | Rebuild only changed targets and their dependents | Bazel, Nx, Turborepo |
| Sparse checkouts | Developers clone only needed directories | `git sparse-checkout`, VFS for Git |
| Shallow clones | Download limited commit history | `git clone --depth N` |
| Parallel execution | Run independent build tasks concurrently | Bazel remote execution, Nx orchestrator |

### Package Managers with Monorepo Support

| Manager | Workspaces | Hoisting | Monorepo Tool Integration |
|---------|-----------|----------|---------------------------|
| pnpm | `pnpm-workspace.yaml` | Strict, no phantom deps | Nx, Turborepo |
| yarn (berry) | `workspaces` | Plug'n'play or node_modules | Nx, Lerna |
| npm | `workspaces` | Hoisted | Basic monorepo setups |

## Decision Framework

### Scoring Model

Rate your project 1–5 on each dimension. Higher = more monorepo-friendly.

| Dimension | 1 (Polyrepo) | 3 (Neutral) | 5 (Monorepo) |
|-----------|-------------|-------------|---------------|
| Code sharing | Rarely share code | Some shared libs | Heavy code sharing |
| Team size | 1–3 devs | 4–15 devs | 16–50 devs |
| Cross-project changes | Rare | Monthly | Weekly or daily |
| Tech stack | Different per project | Some overlap | Same language/framework |
| Release cadence | Independent deploys | Some coordination | Deploy everything together |
| Access control needs | Strict per-project | Some restrictions | Open within team |
| CI expertise | Limited | Some DevOps | Strong platform team |

**Interpretation:**
- Score 7–14: Polyrepo — the complexity of monorepo tooling isn't justified.
- Score 15–24: Evaluate — either could work; start with the simpler option (Turborepo monorepo or polyrepo with published packages).
- Score 25–35: Monorepo — the coordination cost of polyrepos will slow you down.

### Flowchart Decision

```
Do you share code between projects?
├── No → Polyrepo
└── Yes
    └── Do you make cross-project changes frequently?
        ├── No → Polyrepo with published packages
        └── Yes
            └── Do you have CI expertise or a platform team?
                ├── No → Start with Turborepo (simple)
                └── Yes → Bazel/Nx for scale
```

### Example Decisions

| Scenario | Recommendation | Why |
|----------|---------------|-----|
| Startup, 3 devs, 2 services, 1 shared lib | Monorepo (Turborepo) | Low overhead, fast iteration |
| Mid-size SaaS, 20 devs, 12 services, shared auth/UI libs | Monorepo (Nx) | Frequent cross-project changes |
| Large enterprise, 100+ devs, independent product teams | Polyrepo | Team autonomy, per-repo governance |
| Open-source library + commercial SaaS | Hybrid | Separate concerns, different release models |
| Payments team + content team, strict compliance | Polyrepo | Regulatory isolation needs |
| Platform team maintaining 8 internal packages | Monorepo (Nx or Rush) | Publishing coordination is the core problem |

## Architecture Patterns

### Monorepo Directory Structure (Nx-style)

```
my-org/
├── apps/
│   ├── api-server/
│   │   ├── src/
│   │   ├── project.json
│   │   └── package.json
│   ├── web-frontend/
│   │   ├── src/
│   │   ├── project.json
│   │   └── package.json
│   └── admin-dashboard/
│       ├── src/
│       ├── project.json
│       └── package.json
├── libs/
│   ├── auth/
│   │   ├── src/
│   │   ├── project.json
│   │   └── package.json
│   └── ui-components/
│       ├── src/
│       ├── project.json
│       └── package.json
├── tools/
│   └── generators/
├── nx.json
├── package.json
├── tsconfig.base.json
└── CODEOWNERS
```

### Polyrepo Structure (per service)

```
# Each is its own repo
api-server/
├── src/
├── package.json        # depends on @my-org/auth@^2.0.0
├── tsconfig.json
└── .github/
    └── workflows/
        └── ci.yml

auth-lib/
├── src/
├── package.json        # name: @my-org/auth
├── tsconfig.json
└── .github/
    └── workflows/
        └── ci.yml
```

### Hybrid Structure

```
# Core monorepo
core-monorepo/
├── libs/
│   ├── auth/           # published as @my-org/auth
│   └── utils/          # published as @my-org/utils
└── apps/
    └── admin-tool/     # internal only

# Independent service repos
web-frontend/
├── src/
└── package.json        # depends on @my-org/auth@^2.0.0

mobile-app/
├── src/
└── package.json        # depends on @my-org/auth@^2.0.0
```

## Key Metrics to Track

When running a monorepo, track these to detect issues early:

| Metric | Warning Sign | Target |
|--------|-------------|--------|
| Full CI time | > 30 min | < 10 min |
| Affected CI time | > 15 min | < 5 min |
| Cache hit rate | < 60% | > 80% |
| Clone time | > 5 min | < 2 min |
| Time to first build | > 10 min | < 5 min |
| Cross-project change PRs | Increasing | Should decrease with tooling |

## Quick Reference: Commands

### Nx

```bash
# Run affected tests only
npx nx affected --target=test

# Build all projects in dependency order
npx nx run-many --target=build --all

# Generate a new library
npx nx g @nx/js:lib my-lib

# View dependency graph
npx nx graph

# Clear cache
npx nx reset
```

### Turborepo

```bash
# Build all packages in order
npx turbo build

# Run tests for changed packages only
npx turbo test --filter=[HEAD^1]

# Parallel execution with caching
npx turbo build test lint

# View execution plan
npx turbo dry build

# Clear cache
rm -rf .turbo node_modules/.cache
```

### Bazel

```bash
# Build a target
bazel build //apps/api-server:api_server

# Run all tests affected by changes
bazel query "rdeps(//..., $(cat changed_files.txt))" | xargs bazel test

# View dependency graph
bazel query "deps(//apps/api-server:api_server)" --graph

# Clean and rebuild
bazel clean && bazel build //...
```