# Why We Test (and what tests don't prove)

> Tests reduce risk; they do not certify truth.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 16
**Time:** ~45 minutes

## Learning Objectives

- Explain what a test can prove and what it cannot prove.
- Distinguish validation, verification, and falsification in day-to-day engineering.
- Design a layered testing strategy that maps tests to concrete risks.
- Identify blind spots left by high coverage and passing CI.

## The Problem

A team ships a payment rewrite. Unit tests are green, integration tests are green,
and staging smoke tests are green. Within hours of production traffic, duplicate
charges appear for retry-heavy clients. The postmortem reveals the bug lived in a
race between idempotency key persistence and asynchronous webhook retries. No test
in the suite modeled the exact interleaving.

The failure was not "we forgot to test." The failure was "we tested the wrong
claims." The team treated tests as a blanket guarantee instead of a scoped risk
instrument. They proved implementation details in isolated conditions, but the real
risk sat in temporal behavior across boundaries.

This lesson establishes the core mental model for Phase 17: tests are evidence,
not proof. You need explicit claims, explicit assumptions, and explicit gaps.
Without that discipline, advanced techniques like fuzzing, model checking, and
proof assistants become cargo cult tools instead of engineering leverage.

## The Concept

### 1. A test is a claim checker

A test always has this shape:

- Given assumptions A
- In context C
- Check claim P

If the test passes, you learned: "under A and C, I could not falsify P."
You did not learn: "P is universally true."

### 2. Falsification vs proof

In software practice, most testing is falsification-driven.

- A failing test gives strong information: a concrete counterexample.
- A passing test gives weaker information: no counterexample in explored space.

That asymmetry is the reason a single failing test can block release, while a
single passing test should never justify release by itself.

### 3. The finite-sample wall

Programs define huge state spaces.

- Inputs: often unbounded or combinatorial.
- Timing: scheduler choices and network delays explode possibilities.
- Environment: kernel, filesystem, locale, clocks, and dependencies vary.

A test suite samples this space. Sampling can be excellent and still incomplete.

### 4. Different tools, different guarantees

| Technique | Typical claim strength | Common blind spot |
|---|---|---|
| Unit tests | Local behavioral claims | Integration assumptions |
| Integration tests | Boundary compatibility | Rare timing interleavings |
| E2E tests | User-path confidence | State-space coverage |
| Property tests | Invariant stress over many cases | Wrong property definition |
| Fuzzing | Robustness under malformed/edge inputs | Semantic correctness |
| Model checking | Exhaustive state exploration of model | Model-code drift |
| Proof assistants | Machine-checked theorem in formal system | Spec mismatch with reality |

### 5. "Passing CI" as a signal, not a verdict

A green pipeline means:

- The checks that ran did not detect a problem.

A green pipeline does not mean:

- No production defects remain.
- The architecture meets SLOs.
- Security assumptions hold under adversarial behavior.

The job of a senior engineer is to translate green checks into a confidence
statement with scope.

### 6. Confidence accounting

Use confidence as a budgeted asset.

- Risk statement: "Duplicate charge under retry storm."
- Evidence: "Property test over retry schedules + integration test with delayed
  webhook + idempotency invariant monitor in staging."
- Residual uncertainty: "Kernel-level clock skew and third-party callback bursts
  above 10x baseline not modeled."

This accounting style forces explicit tradeoffs and prevents false certainty.

### 7. Why this phase starts here

Later lessons teach tooling depth: QuickCheck/Hypothesis, fuzzers, TLA+, Alloy,
SMT, symbolic execution, and proof assistants. All of them fail if you skip claim
clarity. Weak claim design turns powerful tools into expensive noise.

## Build It

We will build a practical checklist artifact: a "test claim map" you can attach
to any feature before implementation.

### Step 1: Define risk-first claims

Start from failures you want to prevent, not from file structure.

```text
Feature: Retry-safe order payment capture

Top risks:
1. Duplicate charge
2. Lost charge event
3. Out-of-order state transition

Claims:
C1: For any idempotency key, at most one successful capture exists.
C2: Every successful capture eventually emits exactly one durable event.
C3: Order state monotonicity: CREATED -> PAID -> FULFILLED (no backward edge).
```

### Step 2: Attach each claim to an evidence type

```text
C1 -> Property-based tests over randomized retry schedules
C1 -> Integration test with real persistence and concurrent workers
C2 -> Fault-injection test on event broker outage
C3 -> State-machine model check on simplified transition model
```

The goal is not one test per claim; the goal is complementary evidence.

### Step 3: Record assumptions and non-goals

```text
Assumptions:
- Database unique constraint on (merchant_id, idempotency_key)
- Event broker provides at-least-once delivery
- Clock skew <= 200ms in primary region

Non-goals (for this release):
- Multi-region active-active conflict resolution
- Payment provider outage longer than 30 minutes
```

Assumptions convert hidden brittleness into reviewable contracts.

### Step 4: Define stop-ship triggers

```text
Release blockers:
- Any counterexample violating C1
- Any unreproduced flaky failure in payment pipeline tests
- Mutation score drop > 5% in core idempotency module
```

Stop-ship criteria prevents schedule pressure from reinterpreting failures.

### Step 5: Add production feedback hooks

Tests are pre-release evidence. You also need runtime evidence.

```text
Production monitors:
- duplicate_charge_detected_total
- payment_event_lag_seconds p95/p99
- illegal_order_transition_total

Alert policy:
- P1 if duplicate_charge_detected_total > 0 in 5-minute window
```

This closes the loop between test claims and operational truth.

### Step 6: Keep the claim map versioned

Put the claim map in the repo with the feature.

- Update it when assumptions change.
- Review it in PR like code.
- Link failed tests and incidents back to claim IDs.

Claim maps decay if treated as one-time docs.

## Use It

### How production teams apply this

High-reliability teams separate these layers:

- Fast deterministic checks per commit.
- Slower, broader checks per merge window.
- Continuous runtime verification in production.

They rarely ask "did tests pass?" in isolation. They ask:

- Which risk statements are covered?
- Which assumptions are unvalidated?
- What residual risk is accepted and by whom?

### Concrete mapping in common stacks

- Backend service: unit + integration + property tests for invariants.
- API contracts: schema compatibility tests across versions.
- Distributed workflow: model checks on protocol-level state machine.
- Security-sensitive paths: fuzzing + sanitizer builds + static analysis.

When incidents occur, mature teams backfill the claim map, not just one extra
regression test. That prevents local fixes from masquerading as systemic learning.

## Read the Source

- Linux kernel `tools/testing/selftests/` — pragmatic, subsystem-owned test
  strategy where claims are tied to kernel behavior, not abstract purity.
- PostgreSQL `src/test/` and `src/test/isolation/` — separation of regression,
  isolation, and concurrency-sensitive tests.
- LLVM `llvm/unittests/` + fuzz targets in subprojects — layered confidence from
  deterministic unit checks and edge-case input generation.
- Hypothesis documentation and examples — property-first testing mindset with
  shrinking counterexamples to minimal failing cases.

## Ship It

This lesson ships a reusable artifact in `outputs/README.md`:

- **Testing Confidence Map Template** with
  - risk register
  - claim-to-evidence matrix
  - assumptions table
  - stop-ship gate checklist
  - production feedback mapping

You will reuse this artifact in every later lesson, especially fuzzing, model
checking, and the phase capstone.

## Exercises

1. **Easy** — Pick a recent bug from your project and rewrite it as a claim
   that was missing from your test strategy.
2. **Medium** — For one feature, build a claim-to-evidence matrix with at least
   three complementary techniques.
3. **Hard** — Take a flaky test and classify whether it indicates: ambiguous
   claim, unstable environment, nondeterministic code path, or wrong oracle.
4. **Hard** — Add two production metrics that validate a test claim at runtime,
   then define alert thresholds and owners.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Verification | "Testing" | Building evidence that implementation satisfies specified claims under scoped assumptions |
| Validation | "User acceptance testing" | Checking the built system solves the real problem and stakeholder intent |
| Falsification | "Bug finding" | Searching for counterexamples that violate explicit claims |
| Oracle | "Expected result" | Mechanism deciding pass/fail; can be brittle, incomplete, or wrong |
| Coverage | "Quality percent" | Structural execution signal (lines/branches) that does not equal semantic guarantee |
| Flaky test | "CI noise" | Non-deterministic signal that destroys trust and hides real regressions |
| Invariant | "Rule" | Property that must hold across all reachable states in defined scope |
| Residual risk | "Edge case" | Explicitly accepted uncertainty after available checks are applied |
| Assumption | "Environment detail" | External condition required for claim validity |
| Confidence map | "Test plan" | Traceable mapping from risks to claims to evidence to monitoring |

## Further Reading

- [Dijkstra: Notes on Structured Programming](https://www.cs.utexas.edu/~EWD/transcriptions/EWD02xx/EWD249.html) — classic argument for reasoning discipline beyond ad-hoc debugging.
- [Poul-Henning Kamp: The Most Expensive One-byte Mistake](https://queue.acm.org/detail.cfm?id=2010365) — reliability lessons from small defects with large blast radius.
- [Google SRE Workbook - Testing for Reliability](https://sre.google/workbook/) — practical reliability-centric testing and operational feedback loops.
- [Hypothesis Documentation](https://hypothesis.readthedocs.io/) — property-based testing methodology and counterexample shrinking.
- [TLA+ Hyperbook](https://lamport.azurewebsites.net/tla/hyperbook.html) — model-based reasoning for concurrent/distributed systems.
- [PostgreSQL Testing Infrastructure](https://www.postgresql.org/docs/current/regress-run.html) — mature multi-layer database testing practices.
