# Code Review Practice

> Code review is where senior engineers earn their salary — and where junior engineers level up fastest.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 16 lessons 01–06
**Time:** ~45 minutes

## Learning Objectives

- Explain why code review exists beyond "catching bugs" — knowledge sharing, standards enforcement, and culture building.
- Conduct a review that focuses on correctness, edge cases, and design rather than formatting preferences.
- Give feedback that is constructive, specific, and question-driven rather than prescriptive.
- Recognize and avoid common review anti-patterns: bike-shedding, rubber-stamping, nitpicking, and LGTM-without-reading.
- Apply a practical review checklist that you can ship to your team.
- Describe how Google, Microsoft, and Stripe approach code review and what you can borrow from each.

## The Problem

You ship a pull request. A senior engineer glances at it, types "LGTM," and merges it. Two weeks later a null-pointer exception takes down production. The fix is a one-line guard — something anyone actually reading the diff would have spotted.

This happens every day. The gap is not tooling. The gap is **review culture**: what reviewers look for, how they communicate, and whether they actually read the code. Without a deliberate review practice, you get one of two failure modes:

1. **Rubber-stamp reviews** — "LGTM" in five minutes. Bugs ship. Knowledge doesn't transfer. Standards decay.
2. **Nitpick marathons** — three days of debating variable names while a SQL injection goes unnoticed. Velocity dies. Morale follows.

This lesson builds the mental model, the checklist, and the cultural habits that make review actually work.

## Why Code Review Exists

### 1. Catch Bugs

The original purpose and still the most concrete one. A second pair of eyes catches logic errors, off-by-one mistakes, missing error handling, and assumptions that the author was too close to see. Studies at Google found that review catches 60–90% of defects before they reach production, depending on review thoroughness.

### 2. Share Knowledge

Code review is the highest-bandwidth knowledge-transfer mechanism on a team. When you read someone else's diff, you learn what they're building, how they approach problems, and what patterns the team is converging on. When they read yours, they learn your domain. Over months, every engineer on the team builds a mental model of every subsystem — not just their own.

### 3. Enforce Standards

Without review, style guides are aspirational documents that nobody reads. With review, standards are enforced by peers in context — at the moment the code is written, when the cost of change is lowest. This includes naming conventions, error-handling patterns, logging standards, and architectural boundaries.

### 4. Build Culture

Review is a conversation. The tone of that conversation — curious vs. dismissive, specific vs. vague, encouraging vs. gatekeeping — defines engineering culture more than any mission statement. Healthy review culture attracts strong engineers. Toxic review culture drives them away.

## How to Review

### Read the Description First

Before you look at a single line of code, read the PR description. Understand:

- **What** the author is trying to accomplish.
- **Why** they're doing it this way (often includes design rationale or links to issues).
- **How** they want you to review it (e.g., "Focus on the API contract," or "Just sanity-check the migration").

A review without context is a syntax check, not a design review.

### Understand the Context

Look at the surrounding code. A 10-line diff in isolation might look fine, but if it contradicts an existing pattern, duplicates logic three directories away, or breaks an invariant that the rest of the file depends on, the diff alone won't tell you. Pull up the file. Read the function above and below.

### Check the Tests

If the PR has no tests, that's your first signal. If it has tests, run them mentally. Do they actually cover the new behavior, or do they just confirm the happy path? Look for:

- Missing edge-case tests.
- Tests that are tightly coupled to implementation details (testing *how* rather than *what*).
- Flaky test patterns (time-dependent, order-dependent, nondeterministic).

### Look for Issues, Not Style

Your job is to find problems that matter. Focus on:

- **Correctness** — Does the code do what the description says? Are there logic errors?
- **Edge cases** — What happens with empty input? Null? Negative numbers? Concurrent access?
- **Error handling** — Are errors caught? Propagated correctly? Logged with actionable context?
- **Naming** — Do names reveal intent? Can a new team member understand `processItems()` without reading the implementation?
- **API design** — Is the public interface minimal, consistent, and hard to misuse?
- **Security** — Is user input validated? Are there injection vectors? Is auth checked?
- **Performance** — Are there N+1 queries? Unbounded allocations? Hot-path allocations?

## What NOT to Look For

### Formatting — Use Formatters

If your review comment is about indentation, line length, or brace placement, you are wasting time. Install Prettier, Black, gofmt, or whatever is standard for the language. Let the machine format. Let humans review logic.

### Naming Preferences — Use Linters

"I would have called this `idx` instead of `index`" is not a review comment. It's a style opinion. If your team has a naming convention, codify it in a linter rule. If it's genuinely ambiguous (a name that's misleading or conflates two concepts), that's a real issue worth commenting on.

### Personal Preferences

"I prefer early returns" is not actionable unless the team has agreed on that pattern. Review for the team's standards, not your personal taste.

## Review Tone

### Be Constructive

Bad: `This is wrong.`
Good: `What happens when the user list is empty? I think this might throw a NullPointerException.`

The first asserts dominance. The second shares a concern and gives the author a chance to explain or fix.

### Ask Questions

Instead of `Change this to X`, try `Did you consider X? The current approach seems like it might have trouble with Y.`

Questions do three things:
1. They're less confrontational.
2. They reveal the author's reasoning (maybe they already considered X and rejected it for a good reason).
3. They teach — the author learns *why* X might be better, not just that they should do it.

### Suggest Alternatives

When you point out a problem, offer a concrete alternative. "This function is doing too much" is frustrating. "This function is doing too much — what if we extract the validation into a separate helper so the main path stays readable?" is helpful.

### Separate Blocking from Non-Blocking

Not every comment is a hill to die on. Use labels or prefixes:

- **[blocking]** — Must be addressed before merge. For correctness, security, or data-loss bugs.
- **[nit]** — Minor style or naming point. Author can accept or dismiss.
- **[question]** — Genuinely curious, not demanding a change.
- **[suggestion]** — An alternative approach, not a requirement.

## Review Size

### Small PRs (< 400 lines)

Small PRs get better reviews. Period. A 50-line diff gets a careful line-by-line read. A 500-line diff gets a skim. A 2000-line diff gets a rubber stamp.

**Target:** Under 400 lines of meaningful change (excluding generated code, test fixtures, and data files). Under 200 is even better.

If your PR exceeds 400 lines, it probably does more than one thing. Split it.

### Large PRs Need Discussion First

If a change genuinely must be large (a major refactor, a new subsystem), start with a design doc or an RFC rather than a surprise mega-PR. Give reviewers context before giving them code. They'll review better and faster.

### Breaking Up Large Changes

Techniques for splitting:
- **Stacked PRs** — PR #1 adds the data model, PR #2 adds the read path, PR #3 adds the write path.
- **Feature flags** — Merge the code behind a flag, then flip it on in a follow-up.
- **Refactor first** — Separate pure refactors from behavior changes. Reviewers can verify a refactor by confirming tests still pass.

## Review Speed

### Review Within 24 Hours

If you're assigned as a reviewer, the author is blocked on you. A 24-hour turnaround is the minimum acceptable standard. At Google, the explicit expectation is that *first-round* review feedback comes within one business day.

### Why Speed Matters

Every hour a PR waits, the author context-switches. By the time feedback arrives, they've forgotten the details and need to rebuild that mental model. Fast review loops mean:

- The author still has the code in their head when they address feedback.
- The team's branch rate stays high.
- Reviewers don't accumulate a backlog that leads to rubber-stamping.

### It's Okay to Say "I Need More Time"

If a PR is complex and you can't give it a thoughtful review in one sitting, say so. "I've started reading this — will finish by tomorrow morning" is infinitely better than a rushed LGTM.

## LGTM Culture and Its Dangers

### What LGTM Really Means

LGTM stands for "Looks Good To Me." In practice, it often means "I glanced at it and nothing exploded." This is the most dangerous review pattern in the industry.

When LGTM becomes the default, the review process is theater. The PR will be merged regardless. Bugs will ship. And the team loses the knowledge-sharing and standards-enforcement benefits entirely.

### The LGTM-without-Reading Anti-Pattern

This is exactly what it sounds like. A reviewer clicks "Approve" without reading the diff, often because:

- They trust the author ("they're senior, it's probably fine").
- They're too busy to actually review.
- The PR is "just a small change" (famous last words).
- The team has an unspoken culture of fast merges over thorough reviews.

**Every major postmortem at a tech company has involved a rubber-stamped PR.** Every single one.

### Building a No-LGTM-without-Reading Culture

- **Require at least one specific observation** before approving. Even "I like the extraction of the validation helper — clean separation of concerns" confirms the reviewer actually read the code.
- **Normalize blocking feedback.** It shouldn't be socially awkward to say "I need to think about this more" or "This deserves a face-to-face conversation."
- **Use review tools that make it visible** how much of the diff a reviewer actually looked at. GitHub shows which files were viewed. Use that signal.

## The Review as Teaching Moment

For senior engineers, review is one of the most powerful teaching tools available. It's one-on-one, context-rich, and immediately applicable.

### What Good Teaching Reviews Look Like

- **Explain *why***, not just *what*. "This could be a SQL injection risk — always parameterize queries. Here's the pattern we use..." teaches more than "SQL injection risk."
- **Link to documentation.** "Our API design guidelines recommend returning Result types for fallible operations: [link]. This makes error handling explicit at call sites."
- **Acknowledge good work.** "The error handling here is thorough — I like the retry with exponential backoff." Positive reinforcement shapes behavior more than negative feedback.
- **Match depth to seniority.** A junior engineer needs different feedback than a staff engineer. Juniors need patterns explained. Seniors need architectural implications surfaced.

### Code Review as Mentorship

If you're reviewing someone more junior, you are mentoring. Your comments will shape how they write code for years. One thoughtful review comment can change an engineer's approach to error handling, API design, or testing forever. Wield that power carefully.

## Common Review Anti-Patterns

### Bike-shedding

Spending disproportionate time on trivial decisions (naming, formatting, minor preference debates) while overlooking serious issues. Named after C. Northcote Parkinson's observation that a committee will spend minutes on a nuclear power plant and hours on the bike shed.

**Fix:** If you're debating a naming convention for more than two comments, take it offline and add it to the style guide. Move on.

### Rubber-Stamping

Approving every PR without substantive review. Often driven by time pressure, trust in the author, or review fatigue.

**Fix:** Set explicit norms — every approval must include at least one specific observation. Use review analytics to track comment density.

### Nitpicking

Commenting on trivial style issues while missing correctness or design problems. This is the flip side of rubber-stamping — the reviewer is *active* but not *effective*.

**Fix:** Delegate style to formatters and linters. Review for logic, design, and correctness only.

### LGTM-without-Reading

The most dangerous pattern. See "LGTM Culture and Its Dangers" above.

**Fix:** Make it culturally unacceptable. Require specific observations. Track review depth.

## How Top Companies Do Review

### Google

Google's review process is documented in their Engineering Practices guidelines and is among the most studied in the industry.

**Key practices:**

- **One primary reviewer** is responsible for thoroughness. Others can comment, but the primary owns the quality of the review.
- **Review for correctness first, then design, then readability.** The ordering is deliberate — a well-formatted bug is still a bug.
- **Small changes are mandatory.** Google engineers are expected to keep CLs (change lists) small. Reviewers can and do push back on oversized changes.
- **Reviewers must respond within one business day.** Speed is a cultural norm, not just a suggestion.
- **No LGTM without reading.** Explicit cultural norm backed by leadership.
- **Automated checks first.** Lint, format, and test must pass before human review begins. Humans never review what machines can check.

Source: [Google Engineering Practices](https://google.github.io/eng-practices/)

### Microsoft

Microsoft's practices vary by team but share common threads, documented in internal engineering guidelines and public blog posts.

**Key practices:**

- **Two reviewers required** for most changes, with at least one from the affected team.
- **Security review** is a separate, required step for changes touching auth, crypto, or user data.
- **PR descriptions use a standard template** that includes: what changed, why, test plan, rollback plan, and risk assessment.
- **Large changes go through a design review first.** Code review happens after the design is approved, keeping reviewers focused on implementation rather than architecture debates.

### Stripe

Stripe's review culture emphasizes readability and documentation.

**Key practices:**

- **Readability reviews** are a first-class concern. Code must be understandable by someone unfamiliar with the system.
- **Comments explain *why*, not *what*.** If the code needs a comment to explain what it does, refactor until it's self-explanatory. Then comment on why.
- **Every API change** gets reviewed by an API design expert, separate from the code review.
- **Review checklists** are embedded in the PR template, reminding reviewers of common concerns (error handling, backwards compatibility, observability).
- **Asynchronous-first.** Stripe is distributed, so reviews are designed to work well without synchronous conversation. Comments are expected to be self-contained and clear.

## The Review Checklist

This is the practical artifact. It's also shipped in `outputs/review_checklist.md` as a standalone document you can adopt for your team.

### Before You Review

- [ ] Read the PR description. Understand the goal and the design intent.
- [ ] Check that CI is green. Don't review red code.
- [ ] Note the scope. Is this a bugfix, a feature, or a refactor? Different changes warrant different review depth.

### Correctness

- [ ] Does the code do what the description says?
- [ ] Are there logic errors?
- [ ] Are edge cases handled (empty input, null, zero, overflow)?
- [ ] Are there off-by-one errors?
- [ ] Is error handling correct (caught, propagated, logged with context)?

### Design

- [ ] Is the API minimal and hard to misuse?
- [ ] Does the change fit the existing architecture, or does it break a boundary?
- [ ] Are there unnecessary abstractions?
- [ ] Would a simpler approach work?

### Security

- [ ] Is user input validated and sanitized?
- [ ] Are there injection vectors (SQL, XSS, command)?
- [ ] Are auth and authz checked at the right layer?
- [ ] Are secrets handled correctly (not logged, not hardcoded)?

### Performance

- [ ] Are there N+1 queries or unbounded loops?
- [ ] Are allocations on hot paths minimized?
- [ ] Is caching used correctly (not stale, not excessive)?
- [ ] Are there potential scalability issues?

### Tests

- [ ] Are there tests for the new behavior?
- [ ] Do tests cover edge cases, not just the happy path?
- [ ] Are tests testing *behavior*, not implementation details?
- [ ] Will tests be flaky (time-dependent, order-dependent)?

### Documentation

- [ ] Are public APIs documented?
- [ ] Are complex invariants explained?
- [ ] Is the PR description sufficient for a future reader?

### After You Review

- [ ] Did you leave at least one specific observation (not just LGTM)?
- [ ] Are your comments labeled (blocking / nit / question / suggestion)?
- [ ] Would you be comfortable maintaining this code in six months?

## Build It

### Step 1: Minimal — The Personal Checklist

Write a personal review checklist. Start with the categories above, but tailor them to your stack and your team's most common failure modes. Keep it under 20 items. Post it next to your monitor. Use it on every review for two weeks. Then revise.

### Step 2: Realistic — The Team Review Checklist

Take the output from Step 1 and socialize it. Get input from your team on what they always look for. Codify it into a PR template with checkboxes. This is what shipped in `outputs/review_checklist.md`.

## Use It

The production equivalent of this checklist is embedded in the PR templates and engineering guidelines at companies like Google (public eng-practices guide), Stripe (PR template with built-in checklist), and Uber (code review guidelines document).

Compare your hand-built checklist against Google's [eng-practices](https://google.github.io/eng-practices/) review guidelines. Notice that Google's guide is exhaustive — yours should be concise. A checklist that's too long won't be used. The 20–30 item range is the sweet spot.

## Read the Source

- **Google's Engineering Practices** — [google.github.io/eng-practices](https://google.github.io/eng-practices/) — The gold standard for public review guidelines. Read the "How to do a code review" section in full.
- **Mozilla's Code Review Guidelines** — [mozilla version control docs](https://mozilla-version-control-tools.readthedocs.io/en/mozreview/), specifically the review policy section for a large-scale open-source project's approach.
- **The Morning After, Shopify** — Postmortem of a bug that was LGTM'd without thorough review. Required reading for why review speed without depth is dangerous.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A practical code review checklist** (`review_checklist.md`) that teams can adopt as-is or customize. Copy it into your repo's `.github/` directory or your team's engineering wiki.

## Exercises

1. **Easy** — Use the review checklist from `outputs/` on your next three PRs. Track how many issues you catch that you would have missed without it.
2. **Medium** — Review an open-source project's recent PRs. Identify which anti-patterns appear (bike-shedding, rubber-stamping, nitpicking) and write up what you'd change about their review culture.
3. **Hard** — Design a review process for a distributed team across 4+ time zones. Address: async review norms, review rotation, escalation for blocking disagreements, and metrics for review health (comment density, time-to-first-review, rework rate).

## Key Terms

| Term | What people say | What it actually means |
|------|-----------------|------------------------|
| LGTM | "Looks good, merge it" | "I approve this change for merge" — should mean you've read and verified the code, not just glanced at it |
| Bike-shedding | "Arguing about trivia" | Spending disproportionate time on unimportant decisions while overlooking serious issues |
| Rubber-stamping | "Barely reviewing" | Approving PRs without substantive examination; often driven by trust, busyness, or cultural norms |
| Blocking comment | "This must be fixed" | A review comment that must be addressed before the PR can merge; reserved for correctness, security, and design issues |
| Nit | "Minor point" | A non-blocking comment about style, naming, or minor preference — the author can accept or dismiss |
| Stacked PRs | "Chained PRs" | A series of small, dependent PRs where each builds on the previous one, enabling incremental review |
| Review depth | "How thoroughly it was reviewed" | A measure of how much of the diff the reviewer actually read and understood, as opposed to glance-and-approve |

## Further Reading

- **Google Engineering Practices** — [google.github.io/eng-practices](https://google.github.io/eng-practices/) — The canonical public reference for code review best practices.
- **Smart Bear's "Best Practices for Peer Code Review"** — Industry study on the ROI of different review approaches.
- **Michael Lynch, "A Guide to Code Review"** — [mtlynch.io/code-review](https://mtlynch.io/code-review/) — Practical, opinionated guide focused on tone and effectiveness.
- **On Code Review, Stripe** — Engineering blog posts on Stripe's review culture and readability standards.
- **"How Code Review Works at Microsoft"** — Research paper (Bosu et al.) on the characteristics of useful review comments at scale.