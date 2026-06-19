# Testing Confidence Map Notes

## Scope

Use this note to frame testing as risk reduction, not as a ceremonial checkbox.
Every test must map to a claim. Every claim must map to a risk.

## Minimal Process

1. List top 3 failure modes.
2. Convert each mode to one falsifiable claim.
3. Choose at least two evidence sources for each claim.
4. Record assumptions.
5. Define release blockers.
6. Link production metrics back to claims.

## Claim Quality Checklist

- Is the claim observable?
- Is it measurable within a bounded time?
- Is it independent from implementation detail?
- Can a counterexample be generated?
- Does the claim reference external assumptions?

## Anti-patterns

- "100% coverage means safe release."
- "One E2E test per endpoint is enough."
- "Flaky but usually green" test suites.
- Assertions against unstable clocks or random seeds.
- Hidden assumptions about retries, ordering, or clocks.

## Example Claim Matrix

| Claim ID | Risk | Evidence 1 | Evidence 2 | Runtime signal |
|---|---|---|---|---|
| C1 | Duplicate writes | Property tests | Integration race test | duplicate_write_total |
| C2 | Lost event | Fault-injection test | Replay consistency test | event_gap_total |
| C3 | Illegal state transition | Model check | API contract tests | invalid_transition_total |

## Release Guidance

A release decision should include:

- Verified claims and evidence age.
- Unverified claims and rationale.
- Residual risk and explicit owner sign-off.

## Incident Backfill Loop

After incidents:

1. Add missing claim or assumption.
2. Add deterministic reproducer when possible.
3. Add monitoring tied to the same claim.
4. Re-evaluate if existing tests gave false confidence.
