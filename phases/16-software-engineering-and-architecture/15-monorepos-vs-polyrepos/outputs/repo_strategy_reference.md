# Repository Strategy Decision Framework

> Use this reference when deciding between monorepo, polyrepo, or hybrid for your project.

## Quick Decision

```
Do you share code between projects?
├── No → Polyrepo
└── Yes
    └── Do you make cross-project changes frequently?
        ├── No → Polyrepo with published packages
        └── Yes
            └── Do you have CI expertise or a platform team?
                ├── No → Monorepo with Turborepo (simplest)
                └── Yes → Monorepo with Nx or Bazel
```

## Scoring Model

Rate your project 1–5 on each dimension. Higher score = more monorepo-friendly.

| Dimension | 1 (Polyrepo) | 3 (Neutral) | 5 (Monorepo) |
|-----------|-------------|-------------|---------------|
| Code sharing frequency | Rarely share code | Some shared libs | Heavy code sharing |
| Team size | 1–3 devs | 4–15 devs | 16–50 devs |
| Cross-project change frequency | Rare | Monthly | Weekly/daily |
| Tech stack overlap | Different per project | Some overlap | Same language/framework |
| Release cadence alignment | Independent deploys | Some coordination | Deploy together |
| Access control strictness | Strict per-project | Some restrictions | Open within team |
| CI/DevOps expertise | Limited | Some | Strong |

**Interpretation:**
- **7–14:** Polyrepo — monorepo tooling complexity isn't justified.
- **15–24:** Evaluate — either could work; start simple (Turborepo monorepo or polyrepo with published packages).
- **25–35:** Monorepo — polyrepo coordination costs will slow you down.

## Strategy Comparison

| Strategy | Structure | Best For | Watch Out For |
|----------|-----------|----------|---------------|
| **Monorepo** | All projects in one repo | Small-medium teams, shared code, frequent cross-project changes | CI complexity, repo size, access control |
| **Polyrepo** | One repo per project | Large orgs with independent teams, open-source libs, regulatory isolation | Dependency hell, version skew, cross-repo refactoring |
| **Hybrid: Monorepo + Submodules** | Monorepo with git submodules for external code | Mostly monorepo with some vendor/partner code | Submodule complexity, detached HEAD issues |
| **Hybrid: Domain Monorepos** | Multiple monorepos by business domain | Large orgs with clear domain boundaries | Cross-domain coordination still needed |
| **Hybrid: Monorepo + Published Packages** | Internal monorepo, publish for external consumers | Companies with both internal and open-source packages | Publish orchestration, version management |

## Tool Selection

| Team Size | Languages | Tool | Why |
|-----------|-----------|------|-----|
| 2–10 | JS/TS only | Turborepo | Simple setup, fast caching, low learning curve |
| 5–50 | JS/TS focused | Nx | Affected commands, dependency constraints, remote cache |
| 20–500 | JS/TS, enterprise | Rush | Change management, governance, strict versioning |
| 100+ | Any (polyglot) | Bazel | Hermetic builds, remote execution, language agnostic |
| 3–30 | JS/TS, package publishing | Lerna | Publish workflow (pair with Nx or Turborepo for builds) |

## CI Optimization Checklist

### For Monorepos

- [ ] Set up affected target detection (Nx `affected`, Bazel `rdeps`, or Turborepo `--filter`)
- [ ] Enable remote caching (Nx Cloud, Turborepo Remote Cache, Bazel Remote Cache)
- [ ] Configure sparse checkouts for large repos
- [ ] Set up CODEOWNERS for per-directory review requirements
- [ ] Add dependency constraints to prevent circular or cross-scope imports
- [ ] Benchmark CI time — target < 10 min for affected tests

### For Polyrepos

- [ ] Use a shared library versioning strategy (semver, changelogen)
- [ ] Automate dependency updates (Renovate, Dependabot)
- [ ] Create a shared CI template for consistency
- [ ] Document package publishing workflow
- [ ] Set up integration tests that verify cross-repo compatibility
- [ ] Track version skew — audit which services run which library versions

## Architecture Templates

### Monorepo (Nx)

```
my-org/
├── apps/
│   ├── api-server/
│   ├── web-frontend/
│   └── admin-dashboard/
├── libs/
│   ├── auth/
│   ├── ui-components/
│   └── utils/
├── nx.json
├── package.json
├── tsconfig.base.json
└── CODEOWNERS
```

### Polyrepo

```
# Per-service repos
github.com/my-org/
├── api-server/        ← own repo, own CI, own deploy
├── web-frontend/      ← own repo, own CI, own deploy
├── auth-lib/          ← own repo, published to npm/registry
└── ui-components/    ← own repo, published to npm/registry
```

### Hybrid (Domain Monorepos)

```
github.com/my-org/
├── payments-monorepo/    ← payments team
│   ├── apps/payment-api/
│   └── libs/transaction-lib/
├── content-monorepo/     ← content team
│   ├── apps/content-api/
│   └── libs/content-lib/
└── platform-libs/        ← shared infra team
    ├── auth-lib/          (published as package)
    └── observability/     (published as package)
```

## Anti-Patterns to Avoid

1. **Monorepo without affected-target CI** — Every PR rebuilds everything. CI takes hours.
2. **Polyrepo without automated dependency updates** — Services drift to old library versions.
3. **Git submodules for internally-owned code** — Use workspace packages instead; submodules add friction.
4. **Monorepo without CODEOWNERS** — Anyone can modify any project without review from the owning team.
5. **Publishing every change from a monorepo** — If you're in a monorepo, consume via workspaces, not package publishes.

## Migration Paths

### Polyrepo → Monorepo

1. Choose a monorepo tool (Nx, Turborepo, Bazel).
2. Create the monorepo structure with root config.
3. Migrate one project at a time, starting with the most-shared library.
4. Replace package dependencies with workspace references (`workspace:*`).
5. Set up affected-target CI before adding the 3rd project.
6. Remove old repos after migration is validated.

### Monorepo → Polyrepo

1. Identify projects to extract (those with independent teams/release cycles).
2. Set up CI/CD in the new repo before migrating code.
3. Publish shared libraries as packages to a registry.
4. Replace workspace references with package version constraints.
5. Migrate code and verify builds.
6. Archive the project directory in the monorepo.

---

*Reference from Phase 16, Lesson 15: Monorepos vs Polyrepos*