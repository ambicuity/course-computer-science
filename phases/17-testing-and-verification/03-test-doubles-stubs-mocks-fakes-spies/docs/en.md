# Test Doubles - Stubs, Mocks, Fakes, Spies

> Use doubles to control uncertainty, not to impersonate reality blindly.

**Type:** Learn
**Languages:** TypeScript, Python
**Prerequisites:** Phase 17 lessons 01-02
**Time:** ~60 minutes

## Learning Objectives

- Distinguish stubs, fakes, mocks, and spies by behavior and purpose.
- Select the minimum-power double needed for a specific test claim.
- Avoid over-mocking designs that encode implementation details instead of behavior.
- Build a portable decision rubric for choosing doubles in reviews.

## The Problem

A billing service test suite passes for weeks, then a harmless refactor changes
internal method calls while keeping API behavior identical. More than 70 tests
fail. The failures do not indicate regressions; they indicate overspecified mocks.
The team has tests asserting "which method was called in what order" for logic that
should have been asserted through final observable behavior.

In another subsystem, teams avoid doubles entirely and hit live dependencies in
"unit" tests: real clock, real network, and real database. The result is slow,
flaky feedback and painful triage.

Both problems arise from weak double selection. Duplicating reality everywhere is
expensive. Simulating everything with strict mocks is brittle. You need a clear
rule: pick the least-coupled double that still validates your claim.

## The Concept

### The taxonomy

- **Stub**: returns canned data to satisfy a dependency.
- **Fake**: working but simplified implementation (often in-memory).
- **Spy**: records interactions for later assertions.
- **Mock**: preprogrammed expectations on interactions, often with strict
  verification.

### Claim-first mapping

| Claim type | Recommended double |
|---|---|
| "Output is computed correctly for known dependency values" | Stub |
| "Behavior works with realistic storage semantics" | Fake |
| "A side effect happened once" | Spy |
| "Collaborator contract requires exact interaction protocol" | Mock (narrow use) |

### Why over-mocking hurts

Over-mocking couples tests to call structure, not outcomes.

- Refactors become expensive even if behavior is unchanged.
- Tests discourage internal simplification.
- Suites become fragile and noisy.

### Why no doubles also hurts

Without doubles, tests absorb nondeterminism.

- External services fail or throttle.
- Clock and random state produce flake.
- Feedback loops slow down and reduce developer usage.

### Design pressure signal

If a unit test needs many mocks, your object may have too many responsibilities.
Double pain is often architecture feedback.

### Interaction vs state assertions

Prefer state assertions for domain outcomes.
Use interaction assertions only when interaction itself is the requirement.

### Example boundary

If requirement says "email provider must receive exactly one send request with
idempotency key," interaction checks are valid.
If requirement says "invoice is marked sent," state checks are usually enough.

### Determinism and observability

Good doubles improve both:

- deterministic control over dependency responses
- better visibility into side effects and error paths

## Build It

We build a small notification workflow and test it with four double styles.

### Step 1: Domain behavior

Service logic:

- read user preference
- render message
- send notification
- record audit

### Step 2: Add a stub for preferences

Stub returns fixed user channel preferences so tests can isolate routing logic.

### Step 3: Add a fake repository

Use an in-memory audit repository to validate persistence semantics without real DB.

### Step 4: Add a spy for sender

Spy captures send attempts, allowing assertions like "exactly one notification sent."

### Step 5: Add one narrow mock scenario

For protocol-level contract, verify sender receives a required correlation ID.

## Use It

In production codebases:

- Python teams commonly use `unittest.mock` with autospec + occasional fakes.
- TypeScript teams often combine Jest/Vitest spies with in-memory fakes.
- High-reliability services keep strict mocks rare and local.

A practical workflow:

1. Start with stub/fake for most tests.
2. Add spy where side-effect count or payload matters.
3. Add strict mock only for genuine interaction contract.

## Read the Source

- Python `unittest.mock` docs - patching, autospec, call assertions.
- Mockito docs - strict stubs and interaction verification tradeoffs.
- Testing Library and modern frontend guidance - behavior-first tests,
  implementation-detail avoidance.

## Ship It

This lesson ships:

- `code/main.py`: Python doubles demonstration with self-checks.
- `code/main.ts`: TypeScript doubles demonstration.
- `outputs/README.md`: double-selection rubric usable in code review.

## Exercises

1. **Easy** - Reclassify doubles in one existing test file.
2. **Medium** - Replace brittle mocks with fakes in a module and compare test
   failure rate during refactors.
3. **Hard** - Design a fake that simulates eventual consistency lag and validate
   retry behavior.
4. **Hard** - Add mutation testing to see whether interaction assertions detect
   real behavioral defects.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Stub | "Fake object" | Minimal response provider with no behavioral realism |
| Fake | "Test DB" | Lightweight implementation preserving important semantics |
| Spy | "Mock-lite" | Passive interaction recorder used for post-hoc assertions |
| Mock | "Any patched dependency" | Expectation-driven double verifying interaction protocol |
| Overspecification | "Thorough test" | Assertion of non-essential internals that break valid refactors |
| Test seam | "Injection point" | Place where dependency can be replaced for deterministic tests |
| Behavior test | "Black-box test" | Asserts externally visible outcomes |
| Interaction test | "Call-count test" | Asserts collaborator communication shape |

## Further Reading

- [Python unittest.mock](https://docs.python.org/3/library/unittest.mock.html) - official mock/spy/stub capabilities.
- [Martin Fowler - Test Double](https://martinfowler.com/bliki/TestDouble.html) - taxonomy and rationale.
- [Jest Mock Functions](https://jestjs.io/docs/mock-functions) - practical interaction and spy patterns in TS/JS.
- [Mockito Documentation](https://site.mockito.org/) - strict verification and pitfalls.
- [Testing Library Guiding Principles](https://testing-library.com/docs/guiding-principles) - behavior-oriented testing mindset.
