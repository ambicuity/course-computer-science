# Notes — What Makes Software 'Engineered'

## Core Distinction

Programming solves a problem. Engineering ensures the problem stays solved — by other people, on other machines, under conditions the original author didn't anticipate, for years after the author moved on.

The difference is not seniority or title. It is practices: specs, tests, reviews, reproducible builds, documented decisions.

## The Software Crisis (1968)

NATO conference at Garmisch-Partenkirchen. Projects were consistently over budget, over schedule, under specification. Diagnosis: software lacked the rigor of established engineering disciplines. Response: adopt practices that make software development predictable, measurable, and repeatable.

## Four Properties of Engineered Software

| Property | Definition | Anti-pattern |
|-----------|-----------|-------------|
| **Reliability** | System behaves as specified, even under adversity; failures are expected and handled | "It usually works" |
| **Maintainability** | A competent newcomer can make a correct change in < 1 day | "Only Alex understands that module" |
| **Scalability** | System meets requirements as load, data, and team size increase | "It works for 10 users" |
| **Testability** | Behavior can be automatically verified through well-defined interfaces | "You'd have to spin up the whole stack to test that" |

Formulas to remember:

- Maintainability ≈ 1 / (time for a newcomer to make a correct change)
- Testability ≈ P(a random change breaks a test) for correct tests
- Reliability ≈ P(system meets spec) over time and under stress
- Scalability ≈ lim (performance / resources) as resources → ∞

## "It Works on My Machine"

This phrase means the result is contingent on an environment only you control. Engineering demands **reproducibility**: same code + same inputs + same environment = same output, everywhere.

Three pillars of reproducibility:

1. **Determinism** — No dependence on uninitialized memory, wall-clock ordering, or implicit runtime state
2. **Environment management** — All dependencies (packages, OS, runtime, env vars) are declared and version-pinned
3. **Deployment parity** — Test and production environments match in all ways that affect behavior

## The Exponential Cost of Change

| Phase | Relative Cost to Fix |
|-------|---------------------|
| Requirements | 1× |
| Design | 5× |
| Implementation | 10× |
| Testing | 20× |
| Production | 100×+ |

Why? Compounding. A requirements error discovered in implementation means rewriting code. The same error in production means rewriting code, re-testing, re-deploying, possibly recovering data.

Implication: invest in practices that shift error discovery left (specs, reviews, automated tests).

## Engineering Is Both Technical and Social

- **Specifications** ≈ agreements between people, not just documents
- **Code reviews** ≈ shared ownership, knowledge transfer, assumption challenge
- **Naming conventions / style guides** ≈ shared expectations, reduced cognitive load
- **Commit message formats** ≈ searchable history, not vanity

The sociologist Everett Rogers: innovations spread through social networks, not just on technical merit. The best code is useless if no one can understand, change, or trust it.

## Process: Shared Expectations, Not Bureaucracy

Good process is invisible — it makes the right thing the easy thing:

- `CONTRIBUTING.md` → new contributors know exactly how to set up and submit changes
- CI pipeline → tests, lints, type checks run automatically on every push
- PR template → "What? Why? How tested?" — reviewers don't guess
- Deployment script → tag, build, push — no 2 AM SSH sessions

Bad process is visible — 47-step checklists, status meetings that should be emails, approvals that add latency without reducing risk.

## Engineering as Risk Management

Every decision is a bet. Engineering makes risk visible, measurable, and manageable.

```
Risk management loop:
  1. Identify   → What can go wrong?
  2. Analyze    → How likely? How severe?
  3. Mitigate   → What can we do about it?
  4. Monitor    → Are our mitigations working?
  5. Repeat.
```

Mapping practices to risk:

| Practice | Risk it manages |
|----------|----------------|
| Code review | Defects ship to production |
| Automated testing | Regressions go undetected |
| ADRs | Decision rationale is lost |
| Rollback plan | Deployed changes cannot be safely reversed |
| Dependency auditing | Known vulnerabilities in supply chain |
| Observability | Production incidents go unnoticed |

## Craft vs Engineering

```
Craft:      "I made this, and I can fix it."
Engineering: "Someone else can understand, change, and deploy this."
```

Craft values individual mastery. Engineering values system resilience. The criterion: if you left tomorrow, could the team continue? If yes → engineered. If "only if they reach me" → crafted.

Both are necessary. The claim of engineering is sustainability — the system outlives any individual contributor.

## Phase 16 Roadmap (Scale → Concern)

| Lessons | Scale | What you'll learn |
|---------|-------|-------------------|
| 01 (this one) | Discipline | What makes software engineered |
| 02 | Function/module | Naming, cohesion, coupling |
| 03 | Class/interface | SOLID principles |
| 04 | Design | GoF patterns that still matter |
| 05 | Architecture | Functional core / imperative shell |
| 06 | Code change | Refactoring catalogue and mechanics |
| 07 | Social | Code review practice |
| 08 | Bounded context | Domain-driven design |
| 09 | Layer | Hexagonal / clean architecture |
| 10–12 | System | Event-driven, CQRS, microservices |
| 13–14 | Interface | API design, versioning |
| 15–16 | Repository | Monorepos, dependency management |
| 17 | Pipeline | Build & CI/CD |
| 18 | Operations | Observability |
| 19 | Economics | Technical debt |
| 20 | Decision | ADRs |
| 21 | Skill | Reading large codebases |
| 22 | Integration | Capstone: refactor real OSS repo |

## Quick Self-Assessment (Four-Property Checklist)

For any codebase, ask:

1. **Reliability**: Can the system fail in expected ways and recover? What's the evidence?
2. **Maintainability**: Can a new contributor make a correct change in < 1 day? What's the evidence?
3. **Scalability**: Does the system meet requirements at 10× current load? What's the evidence?
4. **Testability**: Do automated tests catch regressions before users do? What's the evidence?

If you can't answer with evidence, the property isn't engineered — it's accidental.