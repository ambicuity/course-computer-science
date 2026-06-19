# Technical Debt — Measure, Pay Down, Negotiate

> Technical debt is not an excuse for bad code — it is a financial metaphor for decisions with consequences.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 16 lessons 01–18
**Time:** ~60 minutes

## Learning Objectives

- Define technical debt using Ward Cunningham's original metaphor and distinguish it from mere bad code.
- Classify debt as deliberate or accidental, then place it in the debt quadrant (reckless/prudent × deliberate/inadvertent).
- Measure debt using quantitative metrics: SQALE, code smells per KLOC, dependency freshness, test coverage gaps, duplication, and complexity.
- Decide when to pay down debt (interest > principal) and when not to (rarely-touched code).
- Apply the boy scout rule and choose between rewrite, refactor, extract, and encapsulate.
- Negotiate with stakeholders by framing debt in business terms: risk, velocity, cost.
- Maintain a tech debt register and write remediation stories for the backlog.
- Recognize the bankruptcy scenario: when debt makes a system unmaintainable.

## The Problem

You ship a feature fast by taking a shortcut. Next sprint you pay for it — not in guilt, but in time. Every new change takes longer because the shortcut made the code harder to understand, harder to test, and harder to modify. This compounding cost is **technical debt**, and if you ignore it, the system eventually becomes unmaintainable. This lesson teaches you how to measure it, pay it down, and negotiate for the time to do so.

## The Concept

### Ward Cunningham's Metaphor

In 1992, Ward Cunningham introduced the term "technical debt" at the OOPSLA conference. The metaphor is financial:

- **Principal** — the effort required to fix the shortcut now (pay off the debt).
- **Interest** — the extra cost every future change pays because the shortcut exists.

Cunningham was clear: debt is sometimes **strategic**. A startup takes on debt to ship an MVP, just as a business takes a loan to invest in growth. The problem is not debt itself — it is **unmanaged** debt. Unmanaged debt compounds; managed debt is tracked, measured, and paid down on a schedule.

Technical debt is **not** a synonym for "bad code I wrote because I didn't know better." That is a separate problem. Debt implies a **tradeoff that was conscious or can now be made conscious**.

### Types of Technical Debt

#### Deliberate Debt (Strategic Shortcuts)

You know the code is suboptimal, and you choose it anyway to hit a deadline. Examples:

- Hardcoding a configuration value instead of reading from a file.
- Skipping validation because the data source is trusted _for now_.
- Using a synchronous call instead of async because the current scale doesn't need it.

Deliberate debt is **not shameful** if tracked. The team estimates the interest, records the debt, and plans paydown.

#### Accidental Debt (Ignorance, Haste, Learning)

You didn't know the code was suboptimal when you wrote it. Examples:

- Using an O(n²) algorithm because you didn't realize the dataset would grow.
- Duplicating logic because you didn't know a shared utility existed.
- Using a deprecated API because you didn't read the migration guide.

Accidental debt is discovered retroactively. The response is the same: measure it, track it, pay it down.

#### Bit Rot

Even correct code becomes debt when its environment changes. A library you depend on deprecates a method. A security vulnerability is discovered in a dependency. An API you consume changes its contract. This is debt you didn't choose and didn't cause, but you still pay the interest.

### The Debt Quadrant

Martin Fowler refined the metaphor into a 2×2 matrix:

| | Deliberate | Inadvertent |
|---|---|---|
| **Reckless** | "We don't have time for design." | "What's design?" |
| **Prudent** | "We must ship now; we'll deal with this." | "Now we know how to do this right." |

- **Reckless + Deliberate**: The worst kind. You know the shortcut is bad and you do it anyway with no plan to fix it.
- **Reckless + Inadvertent**: You don't even know you're incurring debt. Common with inexperienced teams.
- **Prudent + Deliberate**: Strategic debt. You take it on consciously with a paydown plan.
- **Prudent + Inadvertent**: You learn a better approach after the fact. Now you have debt you didn't plan for, but you recognize it.

The quadrant is a **reflection tool**. When you identify debt, ask: which cell does it fall in? That determines your response.

## Measuring Debt

You can't manage what you don't measure. Technical debt has both subjective and objective indicators.

### SQALE (Software Quality Assessment based on Lifecycle Expectations)

SQALE is an ISO-standardized method (ISO/IEC 30152) for measuring technical debt. It defines quality characteristics (maintainability, reliability, efficiency, etc.) and maps violations to remediation effort in **person-hours**. Tools like SonarQube implement SQALE and produce a "technical debt ratio":

```
Technical Debt Ratio = (Remediation Effort / Total Development Effort) × 100
```

A ratio under 5% is healthy. Over 20% means debt is dominating your engineering capacity.

### Code Smells per KLOC

Count instances of known code smells per 1,000 lines of code:

- God classes (> 500 LOC or > 20 methods)
- Long methods (> 30 lines)
- Feature envy (a method that uses another class more than its own)
- Inappropriate intimacy (two classes that are too tightly coupled)

Tools: SonarQube, CodeClimate, static analysis linters.

### Dependency Freshness

How stale are your dependencies? For each direct dependency:

```
Freshness = (Your Version / Latest Version) × Age Factor
```

Track:
- Dependencies more than 2 major versions behind.
- Dependencies with known CVEs.
- Dependencies with no release in 2+ years (may be abandoned).

Tools: `npm audit`, `pip audit`, Dependabot, Renovate, Snyk.

### Test Coverage Gaps

Test coverage is not a direct measure of debt, but gaps are debt indicators:

- Lines/functions not covered.
- Branches not exercised.
- Integration paths not tested.

Target: 80%+ line coverage as a floor. The **uncovered** 20% is where debt hides.

### Duplication (DRY Violations)

Duplicated code blocks (usually defined as 6+ consecutive identical lines) indicate missing abstractions:

```
Duplication Rate = (Duplicated Lines / Total Lines) × 100
```

Target: under 3–5%. Tools: SonarQube duplicate detection, PMD CPD (Copy-Paste Detector).

### Complexity Metrics

**Cyclomatic Complexity** (McCabe, 1976): Number of independent paths through a function.

```
CC = Decisions + 1
```

Where "decisions" = `if`, `else if`, `for`, `while`, `case`, `catch`, `&&`, `||`.

| CC Range | Risk |
|----------|------|
| 1–10 | Low — simple, testable |
| 11–20 | Moderate — should be simplified |
| 21–50 | High — difficult to test |
| 50+ | Very high — untangle immediately |

**Cognitive Complexity** (SonarSource): A human-centered measure that discounts structural nesting the compiler already requires (`else`, `catch`) and increments for things that make code hard to read (nesting depth, logical operators, recursion).

Both metrics are available in SonarQube and most modern static analysis tools.

## The Cost of Debt: Interest vs Principal

### Interest

Every change to debt-laden code costs more than it should:

- **Time interest**: It takes longer to understand, modify, and test the code.
- **Defect interest**: Changes to tangled code introduce more bugs.
- **Risk interest**: The code is fragile; changes have unintended side effects.

A team that could deliver 20 story points per week might drop to 12 because of interest payments. That's a 40% velocity tax.

### Principal

The one-time cost to fix the debt:

- Refactoring the god class might take 3 days.
- Extracting the shared utility might take 1 day.
- Removing the deprecated API usage might take 2 days.

### When to Pay Down

**Pay down when interest > principal.** If you spend 2 extra hours per week working around a piece of debt, and it would cost 8 hours to fix it, you break even after 4 weeks. If the code is touched weekly, pay it down now.

### When Not to Pay Down

**Don't pay down debt on code that is rarely touched.** If a module is changed once a year, the interest is negligible. Spending a day refactoring it is a net loss. Unless the debt creates a security vulnerability or blocks other work, leave it.

Other reasons to defer paydown:

- The code is scheduled for a complete rewrite within 6 months.
- The business context that created the debt is about to change (e.g., a regulation makes the feature unnecessary).
- The refactoring is risky and the system is in a critical release window.

### The Boy Scout Rule

"Always leave the code better than you found it." This is not about grand refactoring campaigns. It means:

- Fix a typo in a comment.
- Extract a 3-line repeated pattern into a function.
- Add a missing test when you touch a function.
- Rename an unclear variable.

Small, continuous paydown prevents debt from compounding. But be disciplined: boy scout changes should be **in separate commits** or at least clearly separated from behavior changes. Mixing refactoring with feature changes in one commit makes code review harder and rollbacks riskier.

## Paying Down Debt: Strategies

### Refactor

The most common strategy. Restructure the code **without changing its external behavior**. Examples:

- **Extract Method**: Turn a long method into a composition of smaller methods.
- **Extract Class**: Split a god class into focused classes.
- **Replace Conditional with Polymorphism**: Replace a switch statement with a type hierarchy.
- **Introduce Parameter Object**: Replace a long parameter list with a single object.

Rule: **Refactor in the smallest possible steps.** Each step should leave the code working. If a step breaks tests, the step is too big.

### Rewrite

Replace an entire module or service from scratch. This is the nuclear option and is often **riskier than incremental refactoring**:

- You lose the accumulated bug fixes in the old code.
- Scope creep is tempting ("since we're rewriting, let's add...").
- The rewrite takes longer than estimated while the old system still needs maintenance.

Use rewrite only when:

- The existing code is beyond refactoring (e.g., no tests, no clear boundaries).
- The domain model has fundamentally changed.
- You can run the old and new systems in parallel (strangler fig pattern).

### Extract

Move the problematic code into its own module, service, or library. This doesn't fix the debt — it **contains** it. The extracted module has a clear interface and the rest of the system doesn't depend on its internals. You can then refactor or rewrite the extracted module independently.

### Encapsulate

Wrap the debt in a clean interface. External callers never see the mess. Inside the wrapper, the code is still debt, but the surface area of the debt is minimized. Over time, you can refactor the internals without affecting callers.

**When to use which strategy:**

| Strategy | When |
|----------|------|
| Refactor | Debt is localized; tests exist; changes are incremental |
| Rewrite | Debt is systemic; domain has shifted; parallel running is possible |
| Extract | Debt needs containment; module boundaries are unclear |
| Encapsulate | Debt can't be fixed now but callers must be protected |

## Negotiating with Stakeholders

Engineers know debt is a problem. Stakeholders care about shipments. You must translate debt into **business language**.

### Frame Debt as Risk

> "This module has no error handling for case X. Right now it works because our traffic is low, but when we hit 10× volume, this will cause customer-facing outages. Fixing it takes 2 days. Not fixing it risks a multi-hour outage."

### Frame Debt as Velocity Loss

> "Our velocity has dropped from 20 to 12 story points per sprint. Analysis shows 40% of developer time is consumed by working around known debt in the payment module. A 3-sprint paydown investment would recover that velocity."

### Frame Debt as Cost

> "Each change to this service costs $2,000 in developer time. If we refactored it, each change would cost $500. We make ~10 changes per quarter. The refactoring costs $20,000 one time and saves $60,000 per year."

### What Not to Say

- "The code is messy." — Stakeholders hear "engineers are complaining about aesthetics."
- "We need to refactor." — Stakeholders hear "engineers want to redo work they already did."
- "It's technical debt." — Stakeholders hear "engineers created a problem and now want time to fix their own mistake."

Always tie the debt to a **business outcome**: risk of outage, slower feature delivery, customer-facing bugs, compliance failure.

### The Tech Debt Register

Track debt like a finance team tracks financial debt. A tech debt register is a living document (spreadsheet, wiki page, or specialized tool) that lists every known debt item:

```
| ID | Title | Type | Quadrant | Interest ($/week) | Principal ($) | Last Touched | Priority |
|----|-------|------|----------|-------------------|---------------|--------------|----------|
| TD-001 | Hardcoded config in auth | Deliberate | Prudent+Deliberate | 4h/week | 8h | 2025-04-01 | High |
| TD-002 | No input validation on form endpoint | Accidental | Reckless+Inadvertent | 2h/week | 16h | 2025-03-15 | Critical |
| TD-003 | God class in order processor | Accidental | Prudent+Inadvertent | 6h/week | 40h | 2025-02-20 | Medium |
```

Review the register every sprint. Promote items to remediation stories when their interest exceeds the sprint budget.

### Tech Debt Remediation Stories in the Backlog

Treat paydown as first-class work:

- **Write acceptance criteria.** "Given the auth module reads config from environment variables, when the config changes, then the service picks up the new value without restart."
- **Estimate the story.** Just like feature work.
- **Assign business value.** "This reduces average change time from 4h to 1h, saving $3,000/quarter."
- **Allocate 20% of sprint capacity** to debt paydown as a standing policy. This prevents the backlog from being 100% features.

Some teams use a "debt ratio" rule: for every N story points of features, allocate 1 point of debt paydown.

## Real Examples: How the Best Manage Debt

### Stripe

Stripe's "maturity model" for services defines progressive levels: from Prototype (acceptable debt) through Production-Ready (debt tracked and paydown planned) to Mature (debt minimized). Each level has specific requirements: monitoring coverage, test coverage, runbook completeness, SLA definitions. Services cannot advance levels without addressing their debt. This makes debt visible and creates organizational pressure to pay it down.

### GitHub

GitHub famously rewrote their search infrastructure from a single Ruby process to a distributed system (ElasticSearch-based). The original code was debt-laden — a single process handling all search. Rather than refactoring incrementally (which would have been extremely risky given the system's centrality), they built the new system in parallel and cut over. The key lesson: they **measured the cost of the old system** (outages, latency spikes, development slowdown) to justify the rewrite investment to leadership.

### Google

Google's engineering culture includes several debt-management practices:

- **Readability reviews**: Every code change is reviewed not just for correctness, but for readability and maintainability. This prevents new debt from being introduced.
- **Large-scale change (LSC) infrastructure**: Google has tooling to make automated, repository-wide refactoring changes (e.g., renaming a widely-used API). This makes paying down certain types of debt inexpensive.
- **Hygiene rules**: Rules like "every bug fix must include a regression test" and "no checked-in code can have warnings" prevent debt accumulation.
- **Technical debt sprints**: Teams periodically dedicate entire sprints to paydown, often coordinated across the company.

## The Bankruptcy Scenario

Technical debt bankruptcy occurs when the interest payments exceed the team's capacity to make any progress. Symptoms:

- Every feature request is estimated at weeks instead of days.
- Bug fix introductions exceed bug fix closures (net negative progress).
- Engineers refuse to work on certain modules ("the code is radioactive").
- New hires take 3–6 months to become productive because the codebase is indecipherable.
- The team spends more time in meetings about how to change the code than actually changing it.

When bankruptcy happens, the options are:

1. **Strangler fig rewrite**: Incrementally replace the old system with a new one, running both in parallel. This is the safest approach but requires discipline.
2. **Hard rewrite**: Start over from scratch. Highest risk. Most rewrites fail because they underestimate the accumulated complexity of the old system.
3. **Triage and stabilize**: Stop adding features. Stabilize the existing system: fix critical bugs, add tests, reduce complexity. Then resume feature development at a slower pace.
4. **Amputate**: If part of the system is salvageable and part is not, split them. Maintain the good part. Rewrite or replace the bad part.

Bankruptcy is not inevitable. It results from ignoring interest payments for too long. The earlier you measure and pay down debt, the less likely you are to reach this point.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Technical debt | "My code is messy" | A tradeoff between short-term speed and long-term cost, made consciously or discovered retroactively |
| Principal | "The fix" | The one-time effort to eliminate a debt item |
| Interest | "It's slow to work on" | The ongoing extra cost each change pays because the debt exists |
| Debt quadrant | "Was this on purpose?" | A 2×2 matrix: reckless/prudent × deliberate/inadvertent that classifies debt |
| SQALE | "Some debt metric" | ISO-standardized method measuring technical debt in person-hours of remediation |
| Code smell | "That looks wrong" | A surface-level symptom of deeper design problems (god class, long method, feature envy, etc.) |
| Boy scout rule | "Just clean up a bit" | Leave the code better than you found it, with each change |
| Strangler fig | "Rewrite it gradually" | Incrementally replacing a legacy system by building new alongside old |
| Debt register | "Our debt list" | A living document tracking every known debt item with interest, principal, and priority |
| Bankruptcy | "The codebase is dead" | When cumulative interest exceeds the team's capacity to deliver anything |

## Further Reading

- Ward Cunningham, "The WyCash Portfolio Management System" (OOPSLA 1992) — the original coining of the metaphor
- Martin Fowler, "TechnicalDebt" (martinfowler.com/bliki/TechnicalDebt.html) — the debt quadrant
- SQALE Method, "Software Quality Assessment based on Lifecycle Expectations" (sqale.org)
- SonarQube Documentation on Technical Debt (docs.sonarqube.org)
- Michael Feathers, "Working Effectively with Legacy Code" — the canonical guide to safe refactoring
- Erich Gamma et al., "Design Patterns" — for extract and encapsulate strategies
- Adam Tornhill, "Your Code as a Crime Scene" — using code forensics to find debt hotspots