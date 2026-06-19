# Notes — Code Review Practice

## Review Checklist

### Before Review

1. Read the PR description — understand goal and design intent
2. Verify CI is green — don't review broken code
3. Identify scope — bugfix, feature, refactor? Adjust depth accordingly

### Correctness

- Does the code do what the description says?
- Logic errors, off-by-one mistakes
- Edge cases: empty input, null, zero, overflow, concurrent access
- Error handling: caught, propagated, logged with actionable context
- Return values checked, resources released (file handles, connections)

### Design

- API is minimal and hard to misuse
- Fits existing architecture and respects boundaries
- No unnecessary abstractions or premature generalization
- Simpler approach considered and documented if rejected

### Security

- User input validated and sanitized
- No injection vectors (SQL, XSS, command, LDAP)
- Auth and authz at the correct layer
- Secrets not logged, not hardcoded, not in VCS
- Sensitive data encrypted at rest and in transit

### Performance

- No N+1 queries or unbounded loops
- Minimal allocations on hot paths
- Caching correct (not stale, not excessive)
- Scalability concerns flagged for traffic growth

### Tests

- New behavior has tests
- Edge cases covered, not just happy path
- Tests verify behavior, not implementation details
- No flaky patterns (time-dependent, order-dependent, nondeterministic)

### Documentation

- Public APIs documented
- Complex invariants explained
- PR description sufficient for future readers

### Post-Review

- At least one specific observation left (not just "LGTM")
- Comments labeled: [blocking], [nit], [question], [suggestion]
- Would you be comfortable maintaining this code in six months?

---

## Tone Guidelines

### Do

- **Ask questions** — "What happens when the list is empty?" teaches more than "Handle empty list."
- **Explain why** — "This could be a SQL injection risk because user input is concatenated into the query string. Use parameterized queries instead."
- **Offer alternatives** — "What if we extract the validation into a helper? It would make the main path easier to follow."
- **Acknowledge good work** — "The retry logic here is well-structured — exponential backoff with jitter is exactly right."
- **Match depth to seniority** — Juniors need pattern explanations. Seniors need architectural implications.

### Don't

- **Don't be vague** — "This isn't great" tells the author nothing actionable.
- **Don't be prescriptive without explanation** — "Use a Map here" without saying why.
- **Don't make it personal** — "You wrote this wrong" vs. "This function has a bug."
- **Don't gatekeep** — "This isn't how WE do things" creates an in-group/out-group dynamic.
- **Don't pile on** — If three people have already flagged the same issue, don't add a fourth "me too" comment.

### Comment Templates

```
# Blocking — must fix before merge
[blocking] This function doesn't handle the case where `users` is null.
When `fetchUsers()` returns null (which it does on network timeout), this
will throw an NPE. Consider adding a null check or changing the contract
to return an empty list.

# Question — genuinely curious
[question] Did you consider using an existing date library here? I'm
wondering if the custom parsing handles timezone offsets correctly.

# Suggestion — alternative approach, not required
[suggestion] What if we extracted the validation into a separate function?
Something like `validateOrder(order)` that returns a Result type. It would
make the main path cleaner and make the validation testable in isolation.

# Nit — minor, author can accept or dismiss
[nit] This variable name `d` is a bit terse — consider `daysUntilExpiry`
for readability.
```

---

## Size Guidelines

| Change Size | Lines Changed | Expected Review | Review Depth |
|-------------|--------------|-----------------|--------------|
| Tiny | 1–20 | 5–15 min | Line-by-line |
| Small | 21–100 | 15–30 min | Line-by-line |
| Medium | 101–400 | 30–60 min | Focused read |
| Large | 401–1000 | 60–120 min | Architectural focus |
| Oversized | 1000+ | Split or require design doc first |

- Target: Under 400 lines of meaningful change
- If you must go over 400, start with a design discussion, not a surprise PR
- Stacked PRs: PR #1 (data model) → PR #2 (read path) → PR #3 (write path)
- Refactors separate from behavior changes: verify refactor by confirming tests still pass

---

## Common Anti-Patterns

### 1. Bike-shedding

Debating trivial choices while serious issues go unnoticed.

```
Reviewer A: "Should this be `idx` or `index`?"
Reviewer B: "I prefer `i` — it's shorter."
Reviewer C: "The style guide says `index`."
// Meanwhile: a SQL injection on line 42 goes entirely unmentioned.
```

Fix: Move naming/style debates to the style guide. Use formatters and linters. Focus review time on logic, security, and design.

### 2. Rubber-stamping

Approving every PR without reading it.

```
// Typical rubber-stamp pattern:
// - PR submitted at 2:00 PM
// - Approved at 2:05 PM
// - 3,200 lines changed
// - 0 review comments
```

Fix: Require at least one specific observation per approval. Track review metrics. Make it culturally normal to push back on large PRs.

### 3. Nitpicking

Active but ineffective review — commenting on minor issues while missing major ones.

```
Reviewer: "Missing trailing comma on line 17."   (nit)
Reviewer: "Extra blank line on line 42."           (nit)
Reviewer: "Could use `const` instead of `let`."    (nit)
// Unnoticed: the race condition on line 30 that will corrupt data under load.
```

Fix: Let formatters handle formatting. Let linters handle style. Reserve human review for what machines can't check: correctness, design, and security.

### 4. LGTM-without-Reading

The most dangerous pattern. Every major postmortem involves this.

```
Context: A senior engineer submits a "small fix" PR.
Reviewer: *glances at title* "LGTM"
Three days later: production incident caused by the "small fix."
Postmortem: "The reviewer approved within 2 minutes of assignment."
```

Fix: Make it explicit that LGTM means "I read this carefully and it is correct." Require specific observations. Use tools that show which files a reviewer actually looked at.

### 5. Review Blockade

When a single reviewer blocks progress by being perpetually unavailable or excessively demanding.

```
// PR submitted Monday
// Reviewer assigned Tuesday
// First comment Friday: "Can you explain the design?"
// Author responds Monday
// Reviewer responds next Friday: "I still have concerns"
// PR is now 2 weeks old, branch is stale, author has forgotten context
```

Fix: Set SLA for first-round review (24 hours). Allow revocation of stale reviews. Default to two-reviewer requirement so one blocker doesn't halt all progress.

---

## Company Practices Reference

### Google

- One primary reviewer responsible for thoroughness
- Review order: correctness → design → readability
- CLs must be small; reviewers push back on oversized changes
- Reviewers respond within one business day
- Automated checks (lint, format, test) must pass before human review
- No LGTM without reading — explicit cultural norm

### Microsoft

- Two reviewers required for most changes
- Security review as a separate, required step for sensitive code
- PR template includes: what, why, test plan, rollback plan, risk assessment
- Large changes require design review before code review
- Review focuses on implementation correctness, not architecture debates

### Stripe

- Readability review as a first-class concern
- Comments explain *why*, not *what*; if code needs a "what" comment, refactor
- API changes reviewed by API design experts, separate from code review
- PR template includes review checklist (error handling, backwards compat, observability)
- Asynchronous-first — comments are self-contained and clear, designed for distributed review