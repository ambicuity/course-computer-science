# CI/CD Reference Card — `ci_reference.md`

## Pipeline Stage Patterns

```
┌──────────┐   ┌──────────┐   ┌──────────────┐   ┌──────────┐   ┌────────┐
│   Lint   │──▶│  Compile │──▶│    Test      │──▶│ Package  │──▶│ Deploy │
│  (< 1m)  │   │  (1-3m)  │   │  (3-5m)     │   │  (1-2m)  │   │ (2-5m) │
└──────────┘   └──────────┘   └──────────────┘   └──────────┘   └────────┘
```

### Stage Templates

| Stage     | Purpose                  | Exit Condition              | Typical Tools                          |
|-----------|--------------------------|-----------------------------|----------------------------------------|
| Lint      | Catch style/syntax early | Zero warnings              | ESLint, Ruff, shellcheck, prettier     |
| Compile   | Ensure code builds       | Successful compilation      | tsc, cargo, go build, gcc              |
| Test      | Verify correctness       | All tests pass             | pytest, jest, go test, cargo test      |
| Package   | Create deployable unit   | Artefact produced           | docker build, npm pack, jar            |
| Deploy    | Ship to environment      | Health checks pass          | kubectl, terraform, AWS ECS            |

## CI vs CD Comparison

| Aspect             | Continuous Integration    | Continuous Delivery        | Continuous Deployment      |
|--------------------|--------------------------|----------------------------|----------------------------|
| **What it ensures**| Code builds + tests pass | Code is releasable         | Code is released           |
| **Human decision** | None (automated)         | When to promote to prod    | None (fully automated)    |
| **Pipeline stops at** | Package / staging     | Staging (manual promote)  | Production                 |
| **Required maturity** | Moderate              | High                       | Very high                  |
| **Rollback**       | Re-run pipeline          | Switch to previous artefact| Auto-rollback on failure   |
| **Risk level**     | Low                      | Medium                     | Higher (but mitigated)    |

### When to Use Each

- **CI only**: New teams, low deployment automation, learning phase.
- **Continuous Delivery**: Teams that need control over release timing (compliance, marketing coordination).
- **Continuous Deployment**: Mature teams with feature flags, monitoring, and fast rollback.

## The Five CI Principles

| Principle     | Guideline                | Why It Matters                                           | How to Achieve                         |
|---------------|--------------------------|----------------------------------------------------------|----------------------------------------|
| **Fast**      | < 10 minutes total       | Slow pipelines discourage frequent commits                | Parallelism, caching, incremental     |
| **Deterministic**| Same commit = same result| Flaky builds destroy developer trust                     | Pin deps, isolate envs, quarantine flakes |
| **Isolated**  | Fresh env per run        | Prevents "works on my machine" and state leakage         | Docker, ephemeral runners              |
| **Parallel**  | Independent jobs concur. | Reduces total wall-clock time                            | Matrix testing, job dependencies       |
| **Fail fast** | Cheapest checks first    | Minimizes wasted compute and developer frustration      | Lint → Compile → Unit → Integration   |

## Deployment Strategies

### Blue-Green

```
                  ┌─────────────┐
 Users ──────────▶│   Router    │
                  └──────┬──────┘
                         │
               ┌─────────┴─────────┐
               ▼                   ▼
        ┌──────────┐        ┌──────────┐
        │  BLUE    │        │  GREEN   │
        │  (live)  │        │ (staging)│
        └──────────┘        └──────────┘
                               │
          After validation ────┘
          switch router: blue ←→ green
```

- **Best for**: Zero-downtime deployments, instant rollback
- **Trade-off**: Requires double infrastructure
- **Rollback**: Switch router back (seconds)

### Canary

```
                  ┌─────────────┐
 Users ──────────▶│  Traffic    │
                  │   Splitter  │
                  └──────┬──────┘
                         │
           ┌─────────────┼─────────────┐
           ▼ (95%)                    ▼ (5%)
    ┌──────────────┐          ┌──────────────┐
    │   Stable     │          │   Canary     │
    │  (v1.0)      │          │  (v1.1)      │
    └──────────────┘          └──────────────┘
                                     │
                    Monitor error rates, latency
                    If OK → increase canary to 100%
                    If not → route all back to stable
```

- **Best for**: Real-user validation, limiting blast radius
- **Trade-off**: Requires traffic splitting and monitoring
- **Rollback**: Route all traffic back to stable (seconds)

### Rolling

```
  Time ──────────────────────────────────────────▶

  Server 1:  [v1] ────▶ [v2] ────▶ [v2] ────▶ [v2]
  Server 2:  [v1] ────▶ [v1] ────▶ [v2] ────▶ [v2]
  Server 3:  [v1] ────▶ [v1] ────▶ [v1] ────▶ [v2]

  (one server upgraded at a time; v1 and v2 coexist briefly)
```

- **Best for**: Simple setup, minimal extra infrastructure
- **Trade-off**: v1 and v2 coexist during rollout
- **Rollback**: Deploy previous version (minutes)

### Feature Flags

```
  ┌─────────────────┐     ┌─────────────────────────┐
  │  Deploy v1.1     │     │  Flag: new_checkout_flow │
  │  (code shipped)  │────▶│  OFF → only v1 code runs │
  │                  │     │  ON  → v1.1 code runs    │
  └─────────────────┘     └─────────────────────────┘
                                    │
              Internal users ──▶  ON ──▶ monitor ──▶ 5% users ──▶ 100%
              Bug found? ──▶  OFF (instant, no redeployment)
```

- **Best for**: Decoupling deployment from release, A/B testing
- **Trade-off**: Flag debt accumulates; requires discipline to remove
- **Rollback**: Flip flag off (instant, no redeployment)

## Rollback Strategies Quick Reference

| Strategy             | Speed     | Prerequisite                         | Best When                          |
|----------------------|-----------|--------------------------------------|------------------------------------|
| Blue-green switch    | Seconds   | Two identical environments            | Zero-downtime is critical          |
| Feature flag flip    | Instant   | Feature is behind a toggle            | New feature has issues             |
| Redeploy previous    | Minutes   | Previous artefact is available        | General-purpose fallback           |
| Git revert + push    | Minutes   | Pipeline re-runs on push              | No artefacts stored; small team    |

## Pipeline Anti-Patterns Checklist

Use this checklist to audit your CI/CD pipeline:

- [ ] **No manual gates** — If a human must click a button inside the pipeline, it's not continuous. Use branch protection rules for approvals, not manual job triggers.
- [ ] **No snowflake environments** — Build servers must be reproducible from code. Use Docker images or ephemeral runners.
- [ ] **Not build-only CI** — The pipeline must deploy somewhere (at least staging). Unshipped code is undiscovered risk.
- [ ] **No ignored flakes** — Tests that fail intermittently are real bugs. Quarantine them immediately.
- [ ] **No untracked secrets** — Never commit credentials. Use GitHub Secrets, Vault, or OIDC.
- [ ] **No missing caching** — Dependency downloads should hit cache. Uncached builds waste time and money.
- [ ] **No missing concurrency control** — Cancel in-progress runs on the same branch to save resources.
- [ ] **No happy-path-only testing** — Include integration, security, and performance tests.

## Pipeline Health Metrics (DORA)

| Metric                  | Elite         | Good          | Needs Work    |
|-------------------------|---------------|---------------|----------------|
| Deployment frequency    | On demand     | Weekly        | Monthly        |
| Lead time for changes   | < 1 hour      | < 1 week      | > 1 month      |
| Change failure rate     | < 5%          | < 15%         | > 15%          |
| MTTR                    | < 1 hour      | < 1 day       | > 1 week       |
| Build success rate      | > 98%         | > 95%         | < 90%          |
| Pipeline duration       | < 5 min      | < 10 min     | > 15 min       |

## GitHub Actions Quick Reference

```yaml
# Minimal structure
name: my-pipeline
on: [push]
jobs:
  my-job:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: echo "Hello"

# Key patterns
concurrency:                    # Cancel in-progress runs
  group: ${{ github.workflow }}-${{ github.ref }}

strategy:                       # Matrix testing
  matrix:
    node: ['18', '20', '22']

cache: 'npm'                    # Dependency caching (setup-node)

environment:                    # Protection rules
  name: production

needs: [build]                  # Job dependencies

if: failure()                    # Conditional execution

artifacts:                       # Pass data between jobs
  uses: actions/upload-artifact@v4
  uses: actions/download-artifact@v4
```