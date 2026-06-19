# Software Project Management and Estimation

> You can write perfect code and still fail if you ship it late, over budget, or solving the wrong problem. Project management is the skill that turns engineering into delivery.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 16 Lessons 01-22
**Time:** ~60 minutes

## Learning Objectives

- Apply planning poker and reference class forecasting to produce realistic estimates.
- Use the Cone of Uncertainty to calibrate confidence intervals on schedules.
- Decompose work into tasks with clear dependencies using a work breakdown structure.
- Explain why the Mythical Man-Month's "adding people to a late project makes it later" is true.
- Connect project management to CS: sprint planning, technical debt tradeoffs, stakeholder communication.

## The Problem

Most software projects fail not because of technical problems, but because of estimation and planning failures:

- "It'll be done next week" for the past three months
- Adding engineers to a late project, which slows it down further (Brooks's Law)
- Shipping features nobody asked for while critical bugs wait
- Accumulating technical debt until velocity drops to zero

The 1994 Standish Group CHAOS report found only 16% of software projects succeeded on time and budget. The #1 factor wasn't technical skill — it was requirements management and estimation accuracy.

## The Concept

### The Cone of Uncertainty

Estimates become more accurate as you learn more about the project:

```
Phase                    Accuracy Range
─────────────────────────────────────────
Initial concept          0.25× to 4×     (16:1 range)
After requirements       0.5× to 2×      (4:1 range)
After design             0.67× to 1.5×   (2.25:1 range)
During implementation    0.8× to 1.25×   (1.6:1 range)
Near completion          0.9× to 1.1×    (1.2:1 range)
```

Early estimates are wildly unreliable. This is why "when will it be done?" at the start of a project is unanswerable with precision. The honest answer is a range.

### Estimation Techniques

**Planning Poker:** Team estimates tasks using Fibonacci-scale cards (1, 2, 3, 5, 8, 13, 21). Each person reveals simultaneously. Outliers explain their reasoning. Repeat until convergence.

Why Fibonacci? Human perception is logarithmic, not linear. The difference between 1 and 2 is significant; the difference between 17 and 18 is noise. Fibonacci forces categorical thinking.

**Reference Class Forecasting:** Instead of estimating from the inside (bottom-up), look at similar past projects. "Last time we built a payment integration, it took 6 weeks. This is similar complexity, so estimate 6 weeks."

This corrects for the planning fallacy — the systematic tendency to underestimate effort.

**Three-Point Estimation:** For each task, estimate:
- Optimistic (O): best case, everything goes right
- Most likely (M): realistic estimate
- Pessimistic (P): worst case, Murphy strikes

Expected time = (O + 4M + P) / 6
Standard deviation = (P - O) / 6

This gives you a distribution, not a single number.

### Brooks's Law

"Adding manpower to a late software project makes it later." — Fred Brooks, *The Mythical Man-Month* (1975)

Why?
1. **Communication overhead:** n people have n(n-1)/2 communication channels. Adding people increases quadratic overhead.
2. **Ramp-up time:** New people need weeks to become productive. Existing people spend time teaching instead of building.
3. **Task divisibility:** Some tasks can't be parallelized. Nine women can't make a baby in one month.

The implication: plan for small, stable teams. Prefer 5-person teams over 15-person teams for the same scope.

### Work Breakdown Structure (WBS)

Decompose the project into a tree of deliverables:

```
Project: User Authentication System
├── 1. Database Schema
│   ├── 1.1 Users table
│   ├── 1.2 Sessions table
│   └── 1.3 Migrations
├── 2. API Endpoints
│   ├── 2.1 POST /auth/register
│   ├── 2.2 POST /auth/login
│   ├── 2.3 POST /auth/logout
│   └── 2.4 GET /auth/me
├── 3. Security
│   ├── 3.1 Password hashing (bcrypt)
│   ├── 3.2 JWT token generation
│   └── 3.3 Rate limiting
└── 4. Testing
    ├── 4.1 Unit tests
    ├── 4.2 Integration tests
    └── 4.3 Security audit
```

Each leaf task should be:
- **Independent:** can be worked on without blocking dependencies
- **Estimable:** can be assigned a time estimate
- **Testable:** has clear completion criteria
- **Small:** 1-3 days of work (larger tasks hide uncertainty)

### Technical Debt Tradeoffs

Sometimes you ship fast and accumulate debt. Sometimes you pay down debt and slow down. The key insight: **debt is a business decision, not a technical one.**

```
Velocity over time:

No debt:        ████████████████████████  (steady)
Managed debt:   ██████████████████░░░░░░  (occasional slowdown for repayment)
Unmanaged debt: ████████░░░░░░░░░░░░░░░░  (velocity collapses)
```

Track debt explicitly. Negotiate repayment with stakeholders: "We can ship this feature in 2 weeks with debt, or 3 weeks without. The debt will cost us ~1 week per quarter in maintenance. Which do you prefer?"

### Connection to CS

| CS Application | Project Management Principle |
|----------------|------------------------------|
| Sprint Planning | WBS decomposition, planning poker, velocity tracking |
| Technical Debt | Debt tracking, repayment negotiation, velocity impact |
| Open Source | Contributor onboarding (Brooks's Law), release planning |
| System Design | Estimation accuracy, dependency management |
| Team Leadership | Communication overhead, team size optimization |

## Build It

### Step 1: Three-Point Estimation

```python
import math

def three_point_estimate(optimistic: float, most_likely: float, pessimistic: float) -> dict:
    """PERT three-point estimation."""
    expected = (optimistic + 4 * most_likely + pessimistic) / 6
    std_dev = (pessimistic - optimistic) / 6
    variance = std_dev ** 2
    return {
        'expected': round(expected, 1),
        'std_dev': round(std_dev, 2),
        'variance': round(variance, 2),
        'range_68': (round(expected - std_dev, 1), round(expected + std_dev, 1)),
        'range_95': (round(expected - 2*std_dev, 1), round(expected + 2*std_dev, 1)),
    }

# Task: Build authentication API
result = three_point_estimate(3, 5, 12)  # days
print(f"Expected: {result['expected']} days")
print(f"68% confidence: {result['range_68']} days")
print(f"95% confidence: {result['range_95']} days")
```

### Step 2: Planning Poker Simulator

```python
def planning_poker(estimates: list[int], task: str) -> dict:
    """Simulate a planning poker round."""
    from collections import Counter
    counts = Counter(estimates)
    most_common = counts.most_common(1)[0]

    if most_common[1] == len(estimates):
        return {'task': task, 'estimate': most_common[0], 'converged': True, 'rounds': 1}

    # Check for outliers
    median = sorted(estimates)[len(estimates) // 2]
    outliers = [e for e in estimates if abs(e - median) > median * 0.5]

    return {
        'task': task,
        'estimates': estimates,
        'median': median,
        'outliers': outliers,
        'converged': False,
        'discussion_needed': len(outliers) > 0,
    }

# Team estimates
result = planning_poker([3, 5, 5, 8, 13], "Build user registration")
print(f"Estimates: {result['estimates']}")
print(f"Median: {result['median']}")
print(f"Outliers: {result['outliers']} (need discussion)")
```

### Step 3: Project Timeline Calculator

```python
def project_timeline(tasks: list[dict]) -> dict:
    """Calculate project timeline with dependencies."""
    # tasks: [{name, estimate, dependencies: [task_names]}]
    completed = {}
    timeline = []

    remaining = tasks.copy()
    while remaining:
        # Find tasks with all dependencies met
        ready = [t for t in remaining if all(d in completed for d in t.get('dependencies', []))]
        if not ready:
            raise ValueError("Circular dependency detected!")

        # Schedule ready tasks
        for task in ready:
            start = max(completed.get(d, 0) for d in task.get('dependencies', [])) if task.get('dependencies') else 0
            end = start + task['estimate']
            completed[task['name']] = end
            timeline.append({'name': task['name'], 'start': start, 'end': end})
            remaining.remove(task)

    total = max(completed.values())
    return {'timeline': timeline, 'total_days': total}

tasks = [
    {'name': 'Schema', 'estimate': 2, 'dependencies': []},
    {'name': 'API', 'estimate': 5, 'dependencies': ['Schema']},
    {'name': 'Auth', 'estimate': 3, 'dependencies': ['Schema']},
    {'name': 'Tests', 'estimate': 2, 'dependencies': ['API', 'Auth']},
]

result = project_timeline(tasks)
print(f"\nProject Timeline ({result['total_days']} days):")
for t in result['timeline']:
    print(f"  Day {t['start']:2.0f}-{t['end']:2.0f}: {t['name']}")
```

## Use It

- **Jira, Linear, GitHub Projects** — WBS decomposition and sprint planning
- **Planning Poker apps** — distributed team estimation
- **Velocity tracking** — measure story points completed per sprint, predict future capacity
- **Risk registers** — track known risks and mitigation plans

## Read the Source

- [The Mythical Man-Month](https://www.amazon.com/Mythical-Man-Month-Software-Engineering-Anniversary/dp/0201835959) — Fred Brooks, the classic
- [Software Estimation: Demystifying the Black Art](https://www.amazon.com/Software-Estimation-Demystifying-Steve-McConnell/dp/0735605351) — Steve McConnell
- [NoEstimates](https://neilkillick.com/2013/01/31/noestimates-part-1-do-we-need-estimates/) — alternative perspective

## Ship It

- `code/main.py`: three-point estimation, planning poker, project timeline calculator
- `outputs/README.md`: estimation cheat sheet

## Exercises

1. **Easy:** Estimate a feature you recently built using three-point estimation. Compare your expected time with actual time.
2. **Medium:** Decompose a project into a WBS with 10+ leaf tasks. Identify the critical path (longest dependency chain).
3. **Hard:** Simulate Brooks's Law: model a project with 5 people and show how adding 3 more at 50% completion affects total time (accounting for communication overhead and ramp-up).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Cone of Uncertainty | "Estimates get better over time" | Accuracy range narrows as project progresses; early estimates have 16:1 range |
| Brooks's Law | "Adding people makes it later" | Communication overhead and ramp-up time exceed productivity gains from new people |
| Planning Poker | "Team estimation" | Consensus-based estimation using Fibonacci cards; outliers discuss reasoning |
| Velocity | "How fast we go" | Story points completed per sprint; used to predict future capacity |
| Technical Debt | "Quick and dirty" | Deliberate shortcuts that increase future maintenance cost; a business decision, not a technical one |

## Further Reading

- [The Mythical Man-Month](https://www.amazon.com/Mythical-Man-Month-Software-Engineering-Anniversary/dp/0201835959) — Brooks's Law and more
- [Software Estimation: Demystifying the Black Art](https://www.amazon.com/Software-Estimation-Demystifying-Steve-McConnell/dp/0735605351) — estimation techniques
- [Cone of Uncertainty](https://www.construx.com/books/the-cone-of-uncertainty/) — original research by Steve McConnell
