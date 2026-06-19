# Notes — Technical Debt — Measure, Pay Down, Negotiate

## Debt Quadrant Diagram

```
                    Deliberate                    Inadvertent
                 (we chose this)               (we didn't notice)
              ┌──────────────────────┬──────────────────────────┐
              │                      │                          │
   Reckless   │  "We don't have      │  "What's design?"        │
              │   time for design."  │                          │
              │                      │  Naive, untrained code   │
              │  QUICK FIX: Track &  │  that no one realized    │
              │  schedule paydown    │  was problematic         │
              │                      │                          │
              ├──────────────────────┼──────────────────────────┤
              │                      │                          │
   Prudent    │  "We must ship now;  │  "Now we know how to     │
              │   we'll deal later"  │   do this right."        │
              │                      │                          │
              │  Strategic shortcut  │  Learned a better way     │
              │  with paydown plan   │  after the fact          │
              │                      │                          │
              └──────────────────────┴──────────────────────────┘
```

## Tech Debt Register Template

```
| ID      | Title                          | Type       | Quadrant              | Interest    | Principal | Zone | Priority |
|---------|--------------------------------|------------|-----------------------|-------------|-----------|------|----------|
| TD-001  | Hardcoded API endpoint         | Deliberate | Prudent+Deliberate    | 2h/sprint   | 4h        | P    | Medium   |
| TD-002  | No input validation on form     | Accidental| Reckless+Inadvertent  | 3h/sprint   | 16h       | P    | Critical |
| TD-003  | God class in order processor    | Accidental| Prudent+Inadvertent   | 6h/sprint   | 40h       | P    | Medium   |
| TD-004  | Skipped error handling in sync  | Deliberate | Reckless+Deliberate   | 4h/sprint   | 8h        | T    | High     |
| TD-005  | Duplicated auth logic (3 places)| Accidental| Prudent+Inadvertent   | 1h/sprint   | 6h        | P    | Low      |

Zone: P = Production code, T = Test code, D = Dev tooling, I = Infrastructure
```

## Measurement Techniques

### SQALE Method

```
                     ┌─────────────────────────────────────────┐
                     │           SQALE Quality Model          │
                     ├─────────────────────────────────────────┤
                     │                                       │
                     │  Characteristic        Sub-           │
                     │  (ISO 25010)           characteristic │
                     │  ─────────────────────────────────── │
                     │  Maintainability       Analyzability  │
                     │                         Changeability  │
                     │                         Stability       │
                     │                         Testability     │
                     │  Reliability            Fault tolerance│
                     │                         Recoverability  │
                     │  Efficiency             Time behavior  │
                     │                         Resource util.  │
                     │  Security               Confidentiality│
                     │  ...                    Integrity       │
                     │                                       │
                     └─────────────────────────────────────────┘

Measurement process:
1. Define remediation effort per violation type (in person-hours)
2. Scan codebase for violations using static analysis
3. Sum remediation effort per characteristic
4. Compute Technical Debt Ratio = (Total Remediation Effort / Total Development Effort) × 100
5. Track ratio over time: should be decreasing or stable, not increasing
```

### Code Smell Scan Checklist

```
Smell                    Detection Method              Threshold
────────────────────────────────────────────────────────────────
God class                LOC > 500 or methods > 20     > 0 per KLOC
Long method              Lines > 30                    > 5 per KLOC
Feature envy             Foreign class usage > own      > 2 per KLOC
Data clump               3+ same params in 2+ methods  > 0 per KLOC
Shotgun surgery           One change touches 3+ files   > 3 per KLOC
Divergent change          One class changed for 3+ reasons > 2 per KLOC
Dead code                 Unreachable / unused          > 1 per KLOC
```

### Dependency Freshness Audit

```
For each direct dependency:
  1. Current version vs latest stable version
     - Major versions behind: risk_score += 3
     - Minor versions behind: risk_score += 1
  2. Known CVEs in current version
     - Critical: risk_score += 5
     - High:     risk_score += 3
     - Medium:   risk_score += 1
  3. Last release date
     - > 2 years: risk_score += 3 (possibly abandoned)
     - > 1 year:  risk_score += 1
  4. Is this a transitive dependency we re-export?
     - Yes: risk_score += 2 (amplifies blast radius)

Sort by risk_score descending. Top = pay down first.
```

### Complexity Metrics Quick Reference

```
Cyclomatic Complexity (CC):
  CC = Decisions + 1
  Decisions: if, elif, for, while, case, catch, &&, ||, ?:
  
  Range        Rating        Action
  ──────────────────────────────────────────
  1-10         Low           No action needed
  11-20        Moderate      Simplify or split
  21-50        High          Break down urgently
  50+          Very high     Rewrite function

Cognitive Complexity:
  Increments for: Nesting, logical operators, recursion,
                  breaks/continues in loops, method references
  Does NOT increment for: else, catch (structural), 
                           closing braces
  Read on SonarSource spec for full rules.

Duplication Rate:
  = (Duplicated Lines / Total Lines) × 100
  Target: < 5%
  Action at > 5%: Find and extract common patterns
```

## Negotiation Scripts

### Script 1: Frame as Risk

```
Stakeholder:  "Why can't we just add the feature?"
You:          "We can, and it'll take about 2 days. But there's a risk
               I need you to see. The module we're modifying has no
               error handling for invalid input [TD-002]. Right now it
               works because our traffic is under 100 req/min. At
               1,000 req/min, which is our Q3 target, this will cause
               a customer-facing outage. I can add the feature in 2
               days, or I can add the feature AND fix the risk in 4
               days. The 2 extra days now prevent a multi-hour outage
               later. Which would you prefer?"
```

### Script 2: Frame as Velocity Loss

```
Stakeholder:  "Why is the team so slow?"
You:          "I looked at our last 3 sprints. We planned 60 points
               and delivered 36. Of the 24 points we missed, 15 were
               consumed by working around known debt in the payment
               module [TD-003]. That module has accumulated 40 hours
               of principal. If we spend one sprint paying it down,
               we recover 15 points per sprint going forward. That's
               a 1-sprint investment for a sustained velocity increase
               of ~40%. Can we allocate Sprint 17 to paydown?"
```

### Script 3: Frame as Cost

```
Stakeholder:  "Can we defer the refactoring?"
You:          "Let me put numbers to it. Each change to the order
               service costs about $2,000 in developer time because
               of TD-001, TD-002, and TD-005. After the refactoring,
               each change would cost about $500. We make roughly 10
               changes to that service per quarter. The refactoring
               costs $20,000 one time. It saves $15,000 per quarter.
               The payback period is 1.3 months. After that, it's
               pure savings of $60,000/year. If we defer, we're
               choosing to spend $60K/year more than necessary."
```

### Script 4: The 20% Rule

```
Stakeholder:  "We can't afford to slow down for debt paydown."
You:          "I'm not proposing we slow down. I'm proposing we
               allocate 20% of each sprint to debt paydown. That's
               1 day per week per developer. The other 4 days stay
               fully committed to features. This is how high-performing
               teams operate — it's an investment in sustained velocity,
               not a tax on current velocity. Without it, our velocity
               will continue to decline by ~5% per quarter due to
               compounding interest."
```

## Paydown Decision Matrix

```
Question                           Yes → Action              No → Next Question
─────────────────────────────────────────────────────────────────────────────────
Is interest > principal?           → Pay it down now          → Next question
Is the code touched weekly?         → Consider paydown          → Next question
Does debt block feature work?       → Pay it down now          → Next question
Does debt create security risk?     → Pay it down immediately  → Next question
Is a full rewrite planned <6 mo?    → Defer paydown            → Next question
Is the module rarely touched?       → Don't pay down now       → Next question
Is the debt in the reckless quadrant?→ Prioritize paydown      → Live with it
```

## Bankruptcy Warning Signs Checklist

```
[ ] Feature estimates have doubled or tripled from baseline
[ ] Bug introduction rate exceeds bug fix rate
[ ] Engineers refuse to work on specific modules
[ ] New hire onboarding takes > 3 months
[ ] More time in planning meetings than writing code
[ ] Every change causes 2+ regressions
[ ] No one can explain how a core module works
[ ] Deployments require manual intervention > 50% of time
[ ] Test suites take > 1 hour to run (or are disabled)
[ ] Rollbacks happen > 20% of deploys

0-2 checked: Healthy, maintain current paydown rhythm
3-5 checked: Warning, increase paydown allocation to 30%
5-7 checked: Critical, pause features for a paydown sprint
8-10 checked: Bankruptcy, initiate strangler fig or triage plan
```