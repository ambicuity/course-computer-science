# Code Review Checklist

A practical checklist for reviewing pull requests. Copy this into your repo's PR template or your team's engineering wiki.

---

## Before Review

- [ ] Read the PR description — understand the goal and design intent
- [ ] Verify CI is green — don't review broken code
- [ ] Identify scope — is this a bugfix, feature, or refactor? Adjust depth accordingly

## Correctness

- [ ] Does the code do what the description says?
- [ ] Logic errors or off-by-one mistakes?
- [ ] Edge cases handled: empty input, null, zero, overflow, concurrent access?
- [ ] Error handling: caught, propagated, logged with actionable context?
- [ ] Return values checked? Resources (file handles, connections) released?

## Design

- [ ] API is minimal and hard to misuse?
- [ ] Fits existing architecture and respects boundaries?
- [ ] No unnecessary abstractions or premature generalization?
- [ ] Simpler approach considered — if rejected, is it documented why?

## Security

- [ ] User input validated and sanitized?
- [ ] No injection vectors (SQL, XSS, command, LDAP)?
- [ ] Auth and authz checked at the correct layer?
- [ ] Secrets not logged, not hardcoded, not in VCS?
- [ ] Sensitive data encrypted at rest and in transit?

## Performance

- [ ] No N+1 queries or unbounded loops?
- [ ] Minimal allocations on hot paths?
- [ ] Caching correct — not stale, not excessive?
- [ ] Scalability concerns flagged for traffic growth?

## Tests

- [ ] New behavior has tests?
- [ ] Edge cases covered, not just happy path?
- [ ] Tests verify behavior, not implementation details?
- [ ] No flaky patterns (time-dependent, order-dependent, nondeterministic)?

## Documentation

- [ ] Public APIs documented?
- [ ] Complex invariants explained?
- [ ] PR description sufficient for a future reader?

## Comment Quality

- [ ] At least one specific observation (not just "LGTM")
- [ ] Comments labeled: **[blocking]**, **[nit]**, **[question]**, or **[suggestion]**
- [ ] Tone is constructive — asks questions, explains why, offers alternatives

---

## Comment Labels

| Label | Meaning | Example |
|-------|---------|---------|
| **[blocking]** | Must fix before merge | `[blocking] This doesn't handle null — will NPE on empty response` |
| **[nit]** | Minor, non-blocking style or naming point | `[nit] Consider `daysUntilExpiry` instead of `d` for readability` |
| **[question]** | Genuinely curious, not demanding a change | `[question] Did you consider using a Result type here?` |
| **[suggestion]** | Alternative approach, not required | `[suggestion] Extracting the validation would make this testable in isolation` |

## Size Guidelines

| Change Size | Lines | Time | Depth |
|-------------|-------|------|-------|
| Tiny | 1–20 | 5–15 min | Line-by-line |
| Small | 21–100 | 15–30 min | Line-by-line |
| Medium | 101–400 | 30–60 min | Focused read |
| Large | 401–1000 | 1–2 hours | Architectural focus |
| Oversized | 1000+ | Split or require design doc first | — |

**Target:** Under 400 lines of meaningful change. Under 200 is better.

## Anti-Patterns to Avoid

| Anti-pattern | What it looks like | Fix |
|-------------|--------------------|----|
| Bike-shedding | Debating variable names while missing a SQL injection | Let formatters/linters handle style; humans check correctness, security, design |
| Rubber-stamping | LGTM within minutes on a large PR | Require at least one specific observation per approval |
| Nitpicking | 10 comments on formatting, 0 on logic | Delegate formatting to machines; reserve human review for what machines can't check |
| LGTM-without-reading | Approving without reading the diff | Make "LGTM" mean "I read this carefully and it's correct" — require specific observations |
| Review blockade | One unavailable reviewer blocks all progress | Set 24-hour SLA; allow review reassignment after timeout |

---

*Adapted from Google Engineering Practices, Stripe PR guidelines, and lessons from industry postmortems.*