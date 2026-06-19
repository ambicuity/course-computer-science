# Unit, Integration, E2E - Pyramid vs Trophy

> Test shape is a cost-and-feedback strategy, not a religion.

**Type:** Learn
**Languages:** TypeScript, Python
**Prerequisites:** Phase 17 lesson 01
**Time:** ~60 minutes

## Learning Objectives

- Compare the testing pyramid and testing trophy as risk-allocation strategies.
- Choose a balanced mix of unit, integration, and end-to-end tests for a feature.
- Estimate feedback latency and maintenance cost of a test portfolio.
- Build a simple planner that recommends test distribution from risk inputs.

## The Problem

A team migrates from a monolith to service-oriented components. During migration,
they add many browser-driven E2E tests because these feel closest to user behavior.
The suite grows to 900 scenarios. Median feedback rises from 6 minutes to 48
minutes. Flake rate exceeds 7%. Developers stop trusting red pipelines and start
rerunning jobs by habit.

Another team overcorrects: they write mostly unit tests and few integration checks.
CI is fast, but production incidents keep showing schema mismatches, serialization
errors, and transaction-boundary defects.

Both teams made the same core mistake: they selected test levels by intuition,
not by explicit risk and cost. This lesson gives you a practical decision model
for portfolio shape.

## The Concept

### Test levels in one sentence each

- Unit tests: isolate small behavior with fast feedback.
- Integration tests: validate boundaries among real components.
- E2E tests: validate critical user journeys through full stack.

### Why shape matters

You optimize three competing objectives:

- Defect detection power
- Feedback speed
- Maintenance burden

No single level dominates all three.

### Pyramid model

Classic pyramid recommendation:

- Many unit tests
- Fewer integration tests
- Very few E2E tests

Strengths:

- Fast local feedback
- Lower execution cost
- Easier root-cause localization

Failure modes:

- Boundary regressions missed when integration depth is too low
- False confidence if unit seams over-mock reality

### Trophy model

Testing trophy (popularized in frontend ecosystems):

- Emphasize integration tests heavily
- Keep unit tests targeted
- Keep E2E thin but business-critical

Strengths:

- Better realism than isolated tests
- Strong confidence for framework-heavy apps

Failure modes:

- Integration suite bloat can approach E2E-level latency
- Tooling instability can increase flake risk

### Neither is universal

The best shape depends on system constraints:

- Domain criticality (fintech vs internal dashboard)
- Boundary density (few modules vs many services)
- Determinism (pure logic vs distributed side effects)
- Team size and CI budget

### Risk-to-level mapping table

| Risk type | Best primary level | Why |
|---|---|---|
| Algorithmic logic bug | Unit | Tight control, fast exhaustive variants |
| Serialization/schema drift | Integration | Requires real adapter boundaries |
| Permissions/UI route regression | E2E | User-visible path with auth/session context |
| Retry/idempotency race | Integration + property | Needs real persistence and schedule variation |
| Browser rendering mismatch | E2E + visual checks | Environment-specific behavior |

### Cost profile intuition

Approximate cost growth by level:

- Unit: lowest runtime and setup cost
- Integration: medium runtime, medium environment setup
- E2E: highest runtime, highest orchestration and flake mitigation

This is why portfolios usually keep E2E intentionally narrow.

### Flake amplification effect

If each test has small nondeterminism probability, large suites magnify failure
probability. A broad E2E suite can spend more time in triage than in defect
prevention.

### Portfolio design heuristic

1. Start from top risks, not from test-framework defaults.
2. Assign a primary level per risk.
3. Add secondary checks only for residual risk.
4. Cap E2E to highest-value journeys.
5. Track signal quality: failure usefulness and flake rate.

## Build It

We will build a small strategy planner in Python and TypeScript.

### Step 1: Define feature risk inputs

```text
feature: checkout
risk_logic: 0.7
risk_boundary: 0.9
risk_journey: 0.8
change_frequency: 0.6
runtime_budget_minutes: 12
```

### Step 2: Score candidate distributions

Planner evaluates candidate allocations like:

- 70/20/10 (unit/integration/e2e)
- 55/35/10
- 45/40/15

It computes a utility score:

- +coverage from risk alignment
- -execution cost penalty
- -flake exposure penalty

### Step 3: Generate recommendation

The tool emits:

- Recommended percentage split
- Suggested counts for current module set
- Expected CI time estimate
- Watch metrics (flake, escaped defects)

### Step 4: Validate with scenario variants

Run the planner across contexts:

- High-boundary microservices
- High-logic library
- UX-heavy frontend

Compare how recommendation shifts.

## Use It

In production, mature teams do not hardcode one ideology. They periodically tune
portfolio shape with actual telemetry:

- defect escape classification by root cause
- median and p95 CI duration
- flake rate by level
- cost per useful failure

When integration tests catch most escaped defects, teams increase integration
coverage around boundaries. When E2E flake dominates, they narrow E2E to key
flows and push checks down to deterministic layers.

## Read the Source

- Cypress Real World App test strategy docs and repo discussions: pragmatic mix
  of integration and E2E for web applications.
- Playwright documentation on test isolation and retries: concrete guidance for
  reducing nondeterminism at E2E level.
- Google Testing Blog archives: long-term lessons on balancing unit and
  integration signals at scale.

## Ship It

This lesson ships:

- `code/main.py`: CLI planner for portfolio shape.
- `code/main.ts`: TypeScript version for web/backend teams.
- `outputs/README.md`: reusable decision checklist for choosing test mix.

## Exercises

1. **Easy** - Classify five recent bugs from your project by best primary test
   level.
2. **Medium** - Use planner inputs for one service and compare recommendation
   against current suite mix.
3. **Hard** - Add mutation-score and contract-test metrics to the scoring model.
4. **Hard** - Build a quarterly report template that justifies test-portfolio
   changes with defect and latency data.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Testing pyramid | "Mostly unit tests" | Bias toward low-cost checks with narrower top-level E2E |
| Testing trophy | "Mostly integration" | Portfolio emphasizing realistic interactions with controlled E2E surface |
| Escaped defect | "Prod bug" | Defect that passed all pre-release checks |
| Flake rate | "CI instability" | Fraction of failures not caused by product regressions |
| Signal quality | "Good tests" | Probability a failing test indicates actionable product issue |
| Portfolio shape | "Test ratio" | Deliberate allocation of checks across levels based on risk and cost |
| Feedback latency | "CI time" | Time from change to trustworthy pass/fail signal |
| Boundary test | "Integration test" | Check covering interaction contract between components |

## Further Reading

- [Martin Fowler - Test Pyramid](https://martinfowler.com/articles/practical-test-pyramid.html) - historical and practical framing for layered testing.
- [Kent C. Dodds - Write tests. Not too many. Mostly integration.](https://kentcdodds.com/blog/write-tests) - testing trophy perspective and tradeoffs.
- [Google Testing Blog](https://testing.googleblog.com/) - engineering-scale test design and maintenance lessons.
- [Playwright Best Practices](https://playwright.dev/docs/best-practices) - concrete E2E reliability techniques.
- [Cypress Best Practices](https://docs.cypress.io/guides/references/best-practices) - avoiding fragile high-level test design.
