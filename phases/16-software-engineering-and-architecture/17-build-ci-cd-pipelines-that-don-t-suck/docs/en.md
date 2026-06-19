# Build & CI/CD — Pipelines That Don't Suck

> Your code doesn't matter if it never reaches users. CI/CD is the assembly line that makes shipping reliable, repeatable, and boring — exactly how it should be.

**Type:** Learn
**Languages:** YAML, Shell
**Prerequisites:** Phase 16 lessons 01–16
**Time:** ~60 minutes

## Learning Objectives

- Distinguish continuous integration, continuous delivery, and continuous deployment — and know when each applies.
- Design a build pipeline: lint → compile → test → package → deploy, understanding each stage's role and failure modes.
- Apply CI principles: fast (< 10 min), deterministic, isolated, parallel, fail-fast.
- Configure pipeline-as-code using GitHub Actions YAML.
- Identify and avoid pipeline anti-patterns: manual gates, snowflake environments, build-only CI.
- Compare deployment strategies: blue-green, canary, rolling, feature flags.
- Monitor pipeline health: build success rate, test coverage trends, deployment frequency, MTTR.

## The Problem

You push a commit on Friday at 5 PM. Monday morning, a teammate pulls main and the app doesn't build on their machine. "It works on my machine" isn't a deployment strategy — it's a bug. Without a CI/CD pipeline:

- **Integration happens late.** Developers work in isolation for days or weeks, then discover conflicts in a painful merge.
- **Builds are non-reproducible.** One developer's laptop has a library installed that the build server doesn't.
- **Deployments are manual.** Someone SSHs into a server, copies files, and prays.
- **Rollback is word-of-mouth.** "Which version was running before?" becomes an archaeological dig.

The fix: an automated pipeline that proves, on every commit, that your code builds, passes tests, and can be deployed — and then deploys it.

## The Concept

### What Is CI/CD?

**Continuous Integration (CI)** ensures every commit is automatically verified: the code compiles, linting passes, and tests succeed. The goal is to catch problems minutes after they're introduced, not weeks later.

**Continuous Delivery (CD)** extends CI: every commit that passes the pipeline is *ready* to deploy. A human decides *when* to push the button, but the pipeline guarantees that pressing it will work.

**Continuous Deployment (also CD)** goes one step further: every commit that passes the pipeline is *automatically* deployed to production. No human gate. If the pipeline is green, it ships.

```
┌──────────────┐    ┌──────────────────┐    ┌────────────────────┐
│  Continuous  │    │  Continuous      │    │  Continuous        │
│  Integration │    │  Delivery        │    │  Deployment        │
├──────────────┤    ├──────────────────┤    ├────────────────────┤
│ Build + Test │    │ Build + Test +   │    │ Build + Test +     │
│ on every     │    │ Package + Stage  │    │ Package + Stage +  │
│ commit       │    │ on every commit  │    │ Deploy on every    │
│              │    │ (manual promote) │    │ commit (automatic) │
└──────────────┘    └──────────────────┘    └────────────────────┘
```

### The Build Pipeline

A pipeline is a sequence of stages. Each stage is a gate: if it fails, the pipeline stops.

```
  ┌──────┐   ┌─────────┐   ┌──────┐   ┌─────────┐   ┌────────┐
  │ Lint │──▶│ Compile │──▶│ Test │──▶│ Package │──▶│ Deploy │
  └──────┘   └─────────┘   └──────┘   └─────────┘   └────────┘
      │           │             │           │             │
      ▼           ▼             ▼           ▼             ▼
   style      build errors   failures    broken artefacts  downtime
   issues                    regressions
```

| Stage        | What It Catches             | Typical Tools                  | Time Budget |
|--------------|-----------------------------|--------------------------------|-------------|
| **Lint**     | Style violations, unused imports, type errors | ESLint, Ruff, rubocop, shellcheck | < 1 min |
| **Compile**  | Syntax errors, missing modules, type mismatches | gcc, cargo, tsc, go build      | 1–3 min     |
| **Test**     | Logic bugs, regressions, integration failures  | pytest, jest, go test, cargo test | 3–5 min |
| **Package**  | Missing assets, broken Docker builds, bad artefacts | docker build, jar, wheel, tar   | 1–2 min     |
| **Deploy**   | Config errors, missing secrets, infrastructure drift | kubectl, terraform, AWS ECS    | 2–5 min     |

### CI Principles: The Five Pillars

1. **Fast (< 10 minutes).** If the pipeline takes 30 minutes, developers push less often and merge larger changes. Speed comes from parallelism and caching, not from skipping steps.

2. **Deterministic.** Same commit, same result — every time. No "it passed yesterday." Flakiness (non-deterministic failures) destroys trust. Treat flaky tests like production bugs: fix them immediately or quarantine them.

3. **Isolated.** Each pipeline run gets a fresh environment. No shared state between runs. Docker containers or ephemeral VMs, not a persistent build server that accumulates state.

4. **Parallel.** Run independent jobs simultaneously. Lint, unit tests, and integration tests can all run in parallel rather than sequentially. Matrix testing (multiple OS/runtime versions) is parallelism across configurations.

5. **Fail fast.** Put the cheapest, fastest checks first. If linting fails, don't waste 5 minutes running tests. Ordering stages from fast-to-slow minimizes wasted compute and developer wait time.

```
  Fail-Fast Ordering (fastest first):

  Lint (30s) ──▶ Compile (90s) ──▶ Unit Tests (2min) ──▶ Integration (5min) ──▶ Deploy (3min)
       │               │                  │                      │
       ▼               ▼                  ▼                      ▼
   stop here      stop here          stop here              stop here
```

### CD vs CI: The Delivery Gap

CI answers: "Does it build?" CD answers: "Is it releasable?"

A team with CI but no CD has green builds that nobody deploys because:
- Deployments require manual SSH access and runbooks.
- The staging environment drifts from production.
- Nobody trusts the build artefact because it was tested on a different configuration.

CD closes this gap by making the deploy step a one-button (continuous delivery) or zero-button (continuous deployment) operation.

### Pipeline as Code

The pipeline itself is version-controlled alongside the code it builds. This means:

- **Auditable.** Every change to the pipeline is in git history.
- **Reproducible.** A fresh clone includes the pipeline definition.
- **Self-documenting.** The YAML is the source of truth for how the project is built.

Common formats:
- **GitHub Actions** — `.github/workflows/*.yaml`
- **GitLab CI** — `.gitlab-ci.yml`
- **CircleCI** — `.circleci/config.yml`
- **Jenkins** — `Jenkinsfile` (Groovy)

We use GitHub Actions in this lesson because it's the most widely-used CI system for open-source projects and requires zero setup for public repos.

### Common Failures and Mitigations

| Failure              | Symptom                                   | Mitigation                                    |
|----------------------|-------------------------------------------|-----------------------------------------------|
| **Flaky tests**      | Pass/fail on same commit, random order    | Quarantine flaky tests; use `retry` with threshold; fix root cause |
| **Environment drift**| Works locally, fails in CI (or vice versa) | Docker-based builds; pin dependency versions; use lockfiles |
| **Dependency caching**| Builds slow because deps re-download      | Cache `node_modules`, `.cache`, `~/.cargo` between runs |
| **Secrets management**| API keys in git, `.env` committed        | Use GitHub Secrets; never check in credentials; inject at runtime |
| **Broken main**      | Merge succeeds but main is red           | Require passing CI before merge; branch protection rules |

### Deployment Strategies

How you push code to production is as important as the pipeline itself:

**Blue-Green Deployment**
- Two identical environments: blue (live) and green (staging).
- Deploy new version to green, run smoke tests, switch router from blue to green.
- Rollback: switch router back to blue.
- Pros: zero-downtime, instant rollback.
- Cons: requires double infrastructure.

**Canary Deployment**
- Route a small percentage (1–5%) of traffic to the new version.
- Monitor error rates and latency. If healthy, gradually increase to 100%.
- Rollback: route all traffic back to old version.
- Pros: limits blast radius, real user validation.
- Cons: requires traffic splitting and monitoring.

**Rolling Deployment**
- Replace old instances with new ones one at a time.
- No extra infrastructure, but there's a window where old and new versions coexist.
- Rollback: deploy previous version.
- Pros: simple, minimal extra infrastructure.
- Cons: old and new versions serve traffic simultaneously during rollout.

**Feature Flags**
- Deploy code to production with new behaviour behind a toggle.
- Enable for internal users → beta users → all users.
- Rollback: flip the flag off (no redeployment needed).
- Pros: decouples deployment from release; A/B testing.
- Cons: flag debt accumulates; requires discipline to remove flags.

### Rollback Strategies

| Strategy           | Speed       | Risk                                     |
|--------------------|-------------|------------------------------------------|
| Redeploy previous  | Minutes     | Requires building and deploying old artefact |
| Blue-green switch  | Seconds     | Requires double infrastructure           |
| Feature flag flip  | Instant     | Only works for flag-gated features       |
| Git revert + push  | Minutes     | Adds revert commit to history; pipeline runs again |

### Monitoring the Pipeline

CI/CD is infrastructure. You monitor it like any other system:

| Metric                  | What It Tells You                                   | Healthy Signal              |
|-------------------------|-----------------------------------------------------|----------------------------|
| **Build success rate**  | Percentage of builds that pass                      | > 95%                      |
| **Test coverage trends**| Are you adding tests or just code?                  | Coverage stays flat or up  |
| **Deployment frequency**| How often you ship to production                    | Multiple times per day      |
| **MTTR**                | Mean time to recover from a failed deployment        | < 30 minutes                |
| **Pipeline duration**   | How long the full pipeline takes                    | < 10 minutes                |
| **Change failure rate** | What % of deployments cause incidents                 | < 15%                       |

### Trunk-Based Development vs Feature Branches

**Trunk-based development**: All developers commit to `main` (or short-lived branches < 1 day). Feature flags hide incomplete work. CI must be fast because you integrate constantly.

**Feature branches**: Developers work in long-lived branches, merged via pull requests. CI runs on each branch. The risk is merge drift — the longer the branch lives, the more it diverges from main.

| Aspect             | Trunk-Based                      | Feature Branches                  |
|--------------------|----------------------------------|-----------------------------------|
| Integration frequency | Every commit to main          | At merge (days/weeks later)       |
| Branch lifetime    | < 1 day                           | Days to weeks                     |
| CI speed matters   | Extremely (blocks all commits)    | Important but less blocking       |
| Requires feature flags | Yes, for incomplete features  | Optional                          |
| Merge conflicts    | Rare (constant integration)       | Common (deferred integration)     |

### Pipeline Anti-Patterns

**Manual Gates**
A "deploy" job that requires a human to click a button *inside the pipeline*. This isn't continuous delivery; it's a manual process with extra steps. If you need approval, automate the approval policy (e.g., "one reviewer must approve," enforced by branch protection).

**Snowflake Environments**
A build server that's been running since 2019, with packages installed by hand, environment variables set in a GUI, and nobody knows the full configuration. The fix: Docker images defined in code, ephemeral runners, and infrastructure-as-code.

**Build-Only CI**
A pipeline that builds and tests but never deploys. This is CI without CD — it tells you the code compiles, but not whether it runs in production. Even staging deployments are better than none.

**Testing Only the Happy Path**
A pipeline that runs unit tests but skips integration tests, security scans, or performance benchmarks. The code works in isolation but fails when talking to a database, an API, or under load.

**Ignoring Flakes**
A test that fails 1 out of 10 runs is not a "timing issue." It's a real bug — the race condition exists in production too. Quarantine it, track it, and fix it.

## Build It

We'll build a GitHub Actions CI/CD pipeline from scratch — first minimal, then realistic.

### Step 1: Minimal Version

A single workflow that lints and tests on push:

```yaml
# .github/workflows/ci.yaml
name: ci
on: [push, pull_request]
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm ci
      - run: npm run lint

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm ci
      - run: npm test
```

This works, but it has problems: no caching, no matrix, no deployment, no parallelism beyond the two jobs.

### Step 2: Realistic Version

See `code/ci.yaml` for the full pipeline with caching, matrix testing, deployment stages, and all the best practices covered in this lesson. Key improvements over the minimal version:

- **Dependency caching** — `npm ci` hits cache instead of downloading every run.
- **Matrix testing** — runs on Node 18, 20, and 22 simultaneously.
- **Separate jobs per stage** — lint, test, build, deploy each have clear boundaries.
- **Environment protection** — production deployment requires manual approval.
- **Artifact uploads** — build output is saved and passed to deploy job.
- **Concurrency control** — cancel in-progress runs on the same branch.

The companion `code/run.sh` script simulates this pipeline locally so you can debug failures without waiting for GitHub Actions.

## Use It

In real projects, the pipeline is defined in `.github/workflows/` and runs automatically. Here's what production CI systems add beyond what we built:

- **GitHub Actions** supports reusable workflows (`workflow_call`), composite actions, and organization-level secrets.
- **GitLab CI** has environments, review apps, and built-in container registry.
- **Jenkins** has a massive plugin ecosystem but requires self-hosting.
- **CircleCI** has Docker layer caching and resource class management.

The key insight: **they all implement the same stages** (lint, test, build, deploy). The differences are in orchestration, not in concept.

### What Production Pipelines Add

| Feature                 | Why It Matters                                       |
|-------------------------|------------------------------------------------------|
| Secret rotation         | Auto-rotate API keys on a schedule                   |
| OIDC authentication     | No long-lived credentials; short-lived tokens        |
| Security scanning       | SAST, dependency audit, container image scanning      |
| Notification            | Slack/Discord alerts on failure                       |
| Merge queues            | Serialized merges to prevent broken main              |
| Reusable workflows      | Share pipeline logic across repos                     |

## Read the Source

- **GitHub Actions** — Look at the `.github/workflows/` directory in any major open-source project (e.g., [Next.js](https://github.com/vercel/next.js/tree/canary/.github/workflows), [Rust](https://github.com/rust-lang/rust/tree/master/.github/workflows)) for real-world pipeline definitions.
- **GitLab CI** — The [GitLab](https://gitlab.com/gitlab-org/gitlab/-/tree/master/.gitlab-ci.yml) project dogfoods its own CI system. Study how they define stages, use rules, and manage environments.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **ci_reference.md** — A reference card with pipeline stage patterns, CI/CD comparison, deployment strategies, and anti-pattern checklist.

## Exercises

1. **Easy** — Write a GitHub Actions workflow that runs lint and test on pull_request events only. Add caching for your language's dependency manager.

2. **Medium** — Add a canary deployment stage to the pipeline: deploy to 5% of traffic, wait 5 minutes, check error rates, then either promote to 100% or rollback. Use the `workflow_dispatch` event for manual promotion.

3. **Hard** — Implement a merge queue simulation: create a script that accepts multiple PRs, rebase each on main, run the pipeline serially, and only merge if the pipeline passes. Handle the case where a prior merge invalidates later PRs.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| CI | "We have CI" | Every commit is automatically built and tested — but maybe never deployed |
| CD (Delivery) | "We do CD" | Every passing commit is *ready* to deploy; a human presses the button |
| CD (Deployment) | "We auto-deploy" | Every passing commit is automatically deployed to production |
| Pipeline | "The pipeline is green" | A defined sequence of automated stages (lint → test → build → deploy) |
| Flaky test | "It's a timing issue" | A non-deterministic test that passes or fails on the same code — a real bug |
| Blue-green | "We swap environments" | Two identical production environments; switch traffic between them for zero-downtime deploys |
| Canary | "We roll out slowly" | Route a small % of traffic to new version; increase if healthy |
| Feature flag | "Just feature-flag it" | A toggle that decouples deployment from release — deploy hidden, enable later |
| MTTR | "How fast do we fix things?" | Mean Time To Recovery — average time from failure to restored service |
| Trunk-based | "We all commit to main" | Short-lived branches only (< 1 day); continuous integration to main |
| Snowflake | "It only works on that one server" | An environment that can't be reproduced from code — a configuration anti-pattern |

## Further Reading

- **Martin Fowler, "Continuous Integration"** (2006) — The foundational essay that defined CI practices.
- **Jez Humble & David Farley, "Continuous Delivery"** (2010) — The book that coined CD and laid out the deployment pipeline pattern.
- **GitHub Actions Documentation** — https://docs.github.com/en/actions
- **DORA Metrics** — https://dora.dev/ — The four key metrics (deployment frequency, lead time, change failure rate, MTTR) for measuring DevOps performance.
- **Trunk-Based Development** — https://trunkbaseddevelopment.com/ — Paul Hammant's reference site.