# Technical Debt Reference Card

## The Debt Quadrant

```
                    Deliberate                    Inadvertent
                 (we chose this)               (we didn't notice)
              ┌──────────────────────┬──────────────────────────┐
   Reckless   │  "No time for design"│  "What's design?"        │
              │  Track it, schedule   │  Train, then pay down    │
              │  paydown              │                          │
              ├──────────────────────┼──────────────────────────┤
   Prudent    │  "Ship now, fix      │  "Now we know better"   │
              │   later" with plan    │  Refactor when touched  │
              │                      │                          │
              └──────────────────────┴──────────────────────────┘
```

## Debt Measurement Methods

| Method | What It Measures | Tool Examples | Healthy Target |
|--------|-----------------|---------------|----------------|
| SQALE | Remediation effort (person-hours) | SonarQube, Squore | Debt ratio < 5% |
| Code smells per KLOC | Design violations | SonarQube, CodeClimate | < 5 per KLOC |
| Dependency freshness | Outdated/vulnerable deps | Dependabot, Snyk, npm audit | 0 critical CVEs |
| Test coverage gaps | Untested code paths | Istanbul, JaCoCo, pytest-cov | > 80% line coverage |
| Duplication rate | Copy-paste violations | SonarQube, PMD CPD | < 5% |
| Cyclomatic complexity | Decision density | SonarQube, lizard | Per-function < 10 |
| Cognitive complexity | Reading difficulty | SonarQube | Per-function < 15 |

## Paydown Strategies

| Strategy | When to Use | Risk | Effort |
|----------|------------|------|--------|
| **Refactor** | Localized debt, tests exist, incremental changes | Low | Low-Medium |
| **Rewrite** | Systemic debt, domain shifted, parallel running possible | High | High |
| **Extract** | Debt needs containment, boundaries unclear | Medium | Medium |
| **Encapsulate** | Can't fix now, callers need protection | Low | Low |

## Paydown Decision Flow

```
1. Does the debt create a security risk? → YES: Pay down immediately
2. Does debt block current feature work? → YES: Pay down now
3. Is interest > principal? → YES: Pay down now
4. Is the code touched weekly or more? → YES: Consider paydown
5. Is a rewrite planned within 6 months? → YES: Defer paydown
6. Is the module rarely touched? → YES: Don't pay down now
7. Is the debt in the reckless quadrant? → YES: Prioritize paydown
8. Otherwise → Live with it, revisit next quarter
```

## The Boy Scout Rule

> Always leave the code better than you found it.

Small wins compound:

- Fix a typo in a comment
- Rename an unclear variable
- Extract a repeated 3-line pattern into a function
- Add a missing test for a function you're touching
- Remove dead code you noticed

**Rules:**
- Boy scout changes go in **separate commits** from behavior changes
- Each change should be too small to break anything
- Never mix refactoring with feature changes in one commit

## Negotiation Quick Reference

### Frame as Risk
> "This module will cause a customer-facing outage at 10× traffic. Fixing it takes 2 days. Not fixing it risks a multi-hour outage."

### Frame as Velocity
> "Our velocity dropped from 20 to 12 points/sprint. 40% of time goes to working around debt. One sprint of paydown recovers sustained velocity."

### Frame as Cost
> "Each change costs $2,000. After refactoring, $500. 10 changes/quarter. Payback: 1.3 months. Savings: $60K/year."

### The 20% Rule
> "Allocate 20% of each sprint to debt paydown — that's 1 day/week/developer. The other 4 days stay on features. This sustains velocity; skipping it guarantees velocity decline."

## Bankruptcy Warning Signs

| Sign | Meaning |
|------|---------|
| Estimates doubled/tripled from baseline | Interest dominates capacity |
| Bug intro rate > bug fix rate | Net negative progress |
| Engineers refuse to work on modules | Code is "radioactive" |
| New hire onboarding > 3 months | Codebase is indecipherable |
| More time in planning than coding | Fear of change |
| Every change causes regressions | Fragile design |
| Deployments need manual intervention > 50% | Broken pipeline |

**0-2 signs**: Healthy. Maintain current rhythm.
**3-5 signs**: Warning. Increase paydown to 30%.
**6-7 signs**: Critical. Pause features for a paydown sprint.
**8+ signs**: Bankruptcy. Initiate strangler fig or triage.

## Debt Register Template

| ID | Title | Type | Quadrant | Interest (h/sprint) | Principal (h) | Zone | Priority |
|----|--------|------|----------|---------------------|---------------|------|----------|
| TD-001 | _title_ | Deliberate/Accidental | _quadrant_ | _X_ | _Y_ | P/T/D/I | Critical/High/Medium/Low |

**Priority formula:**
```
Priority Score = (Interest × Touch Frequency) / Principal
```
Higher score = higher priority for paydown.

## Key Metrics to Track Over Time

```
Metric                         Target         Warning        Critical
───────────────────────────────────────────────────────────────────────
Technical Debt Ratio (SQALE)    < 5%           5-10%          > 10%
Code Smells per KLOC            < 3            3-10           > 10
Duplication Rate                < 3%           3-5%           > 5%
Test Coverage                   > 80%          60-80%         < 60%
Cyclomatic Complexity (avg)    < 10           10-20          > 20
Dependencies with CVEs          0              1-2            > 2 critical
Sprint Velocity Trend           Stable/Growing Declining 5%   Declining > 15%
```