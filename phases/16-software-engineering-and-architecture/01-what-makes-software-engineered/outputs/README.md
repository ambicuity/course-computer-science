# Engineering Assessment Checklist

This artifact is a reusable self-assessment for evaluating whether a codebase meets the bar for engineered software. Copy it into any project's `CONTRIBUTING.md`, `README.md`, or internal wiki.

## How to Use

1. Clone or open the codebase you want to assess.
2. Work through each section. For each item, mark **Yes** (with evidence) or **No** (with the gap).
3. Any **No** without a mitigation plan is an engineering risk.
4. Re-run the assessment after changes to measure improvement.

## Quick Assessment (Four-Property Checklist)

| Property | Question | Yes/No | Evidence |
|----------|----------|--------|----------|
| Reliability | Can the system fail in expected ways and recover? | | |
| Maintainability | Can a new contributor make a correct change in < 1 day? | | |
| Scalability | Does the system meet requirements at 10x current load? | | |
| Testability | Do automated tests catch regressions before users do? | | |

## Full Engineering Scorecard

### 1. Reproducibility

- [ ] Dependencies are version-pinned (lock file, Dockerfile, nix)
- [ ] Environment is defined and reproducible
- [ ] Build is automated (one command to build and test)
- [ ] Deployment is automated (no manual steps)

### 2. Specification

- [ ] Behavior is specified before code is written (even informally)
- [ ] Public interfaces have defined contracts
- [ ] Edge cases and failure modes are documented

### 3. Review

- [ ] All changes are reviewed by at least one other person
- [ ] Reviewers check for correctness, not just style
- [ ] Review feedback is acted on before merge

### 4. Testing

- [ ] Unit tests cover core logic
- [ ] Integration tests cover critical paths
- [ ] Tests are part of CI (not just run locally)
- [ ] Test failures block deploys

### 5. Observability

- [ ] Production health is monitored (metrics, logs, traces)
- [ ] Alerts are defined for known failure modes
- [ ] There is a runbook for common incidents

### 6. Risk Management

- [ ] Major decisions are documented (ADRs)
- [ ] Rollback procedures exist for deployments
- [ ] Dependencies are periodically audited

## Scoring

Count the checked boxes. Map to a rough engineering maturity level:

| Score | Level | Interpretation |
|-------|-------|----------------|
| 0–4 | Ad-hoc | No engineered practices; relies on individual heroics |
| 5–9 | Emerging | Some practices exist; inconsistently applied |
| 10–14 | Practicing | Core practices in place; gaps in automation or coverage |
| 15–18 | Engineered | Systematic, reproducible, and maintainable |

## Reuse in Later Phases

This checklist reappears throughout Phase 16:

- **Lesson 07 (Code Review Practice)** — Use the Review section as a rubric for evaluating review quality.
- **Lesson 17 (Build & CI/CD)** — Use the Reproducibility and Testing sections as a CI/CD readiness check.
- **Lesson 18 (Observability)** — Use the Observability section as a monitoring gap analysis.
- **Lesson 19 (Technical Debt)** — Use the full scorecard to quantify debt: each unchecked box is a debt item.
- **Lesson 20 (ADRs)** — Use the Risk Management section to identify decisions that need recording.
- **Lesson 22 (Capstone)** — Use the full scorecard before and after refactoring to measure improvement.