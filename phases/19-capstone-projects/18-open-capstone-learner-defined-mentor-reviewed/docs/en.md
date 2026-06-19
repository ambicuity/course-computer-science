# Open Capstone — Learner-Defined, Mentor-Reviewed

> Ship one ambitious artifact that demonstrates multi-domain competency.

**Type:** Build
**Languages:** Markdown
**Prerequisites:** Phase 19 lessons 01-17
**Time:** ~720 minutes

## Learning Objectives

- Scope an ambitious project from problem statement to delivery.
- Write a technical design document with architecture and interfaces.
- Execute iteratively with measurable milestones.
- Communicate design decisions, tradeoffs, and outcomes.

## The Problem

Open-ended projects fail in two opposite ways: too narrow (a trivial extension of a homework assignment) and too broad (rebuild Kubernetes from scratch). The first doesn't demonstrate competency; the second never ships.

The fix: a structured framework that constrains scope while leaving the technical choice open. The framework forces you to articulate what you're building, why it matters, how you'll know it works, and what you're explicitly not building. This mirrors how real engineering projects start: with a proposal, not with code.

## The Concept

The capstone delivery framework has five phases:

```
1. Proposal        What, why, for whom, what not
        │
        ▼
2. Technical Design Architecture, interfaces, data model, failure modes
        │
        ▼
3. Implementation  Iterative milestones with thin vertical slices
        │
        ▼
4. Review/Harden   Mentor feedback, bug triage, reliability fixes
        │
        ▼
5. Final Delivery  Source, runnable instructions, demo, outcomes report
```

Each phase produces a concrete artifact. The proposal is a document. The technical design includes diagrams. The implementation produces code. The review produces a checklist of fixes. The final delivery is a package someone else can run.

## Build It

### Step 1: Project Proposal Template

Write a proposal covering these sections:

**Problem statement**: What system are you building? What problem does it solve? Who cares?

**Users and constraints**: Who will use this? What are the performance, correctness, or usability constraints? What hardware/language constraints exist?

**Non-goals**: What are you explicitly not building? This is as important as the goals. Non-goals prevent scope creep and clarify the boundary of your work.

**Milestones and timeline**: Break the project into 3-5 milestones, each producing a testable artifact. Each milestone should take 1-3 days. If a milestone takes longer than a week, split it.

**Risk register**: What could go wrong? For each risk, describe the impact (what breaks) and the mitigation (how you'll handle it). Common risks: underestimated complexity, missing dependencies, performance below target.

Example proposal:

```
# Capstone Proposal: Distributed Rate Limiter

## Problem Statement
Build a distributed rate limiter that enforces request limits
across multiple service instances. Single-instance rate limiters
are trivial; distributed rate limiting requires consensus on the
current count.

## Users and Constraints
- Target: backend API services with 3-10 instances
- Constraint: must enforce limits within 1% accuracy
- Constraint: p99 latency added < 1ms
- Language: Go

## Non-Goals
- Not building a full API gateway
- Not handling network partitions (will fail closed)
- Not implementing sliding window (fixed window only)

## Milestones
1. Single-instance token bucket with Redis backend (day 1)
2. Multi-instance with Redis Lua script for atomicity (day 2)
3. Add configurable rules per endpoint (day 3)
4. Load test and measure accuracy/latency (day 4)
5. Write documentation and demo script (day 5)

## Risks
- Redis latency spikes → mitigation: local cache with TTL
- Clock skew across instances → mitigation: use Redis server time
```

### Step 2: Technical Design Document

Produce these artifacts:

**System diagram**: Draw the components and their interactions. Use ASCII, Mermaid, or any tool you prefer. Show data flow, not just boxes.

```
Client → Load Balancer → Service Instance → Rate Limiter → Redis
                                    ↓
                              Local Cache (TTL)
```

**Component interfaces**: For each component, define the API. What does it take as input? What does it return? What errors can it produce?

```go
type RateLimiter interface {
    // Allow returns true if the request is within the rate limit.
    // key: the identifier to rate limit on (e.g., IP, user ID)
    // limit: max requests per window
    // window: time window duration
    Allow(key string, limit int, window time.Duration) (bool, error)
}
```

**Data model**: What data structures does the system use? How is state stored, serialized, and queried?

**Failure modes**: What happens when Redis is down? When the network is slow? When the clock is wrong? For each failure mode, define the behavior (fail open, fail closed, retry, degrade).

**Testing strategy**: How will you verify correctness? Unit tests for the algorithm, integration tests for the Redis interaction, load tests for performance. Define the acceptance criteria for each milestone.

### Step 3: Iterative Implementation

For each milestone:

1. **Build a thin vertical slice**: the smallest piece that demonstrates the milestone works end-to-end.
2. **Add tests and instrumentation**: write at least one test that verifies the slice works. Add logging or metrics that show what's happening.
3. **Record key decisions**: write an ADR (Architecture Decision Record) for any non-obvious choice. Why Redis instead of memcached? Why fixed window instead of sliding?

### Step 4: Review and Hardening

- **Mentor review checkpoints**: after each milestone, have someone review the code and the design. Are the interfaces clean? Are the error cases handled? Is the naming clear?
- **Bug triage**: list all known bugs. Categorize them as P0 (blocks delivery), P1 (significant but workaround exists), P2 (cosmetic or minor). Fix all P0s, fix P1s if time permits, document P2s.
- **Release candidate**: the final build. Test the full workflow from start to finish. Write a rollback plan: if the system fails in production, what's the fallback?

### Step 5: Final Delivery

Deliver these artifacts:

**Source**: clean, well-organized code with a README that explains the project structure.

**Runnable instructions**: someone else should be able to clone your repo and run the system in under 5 minutes. Include all dependencies, setup steps, and example usage.

**Demo script**: a step-by-step walkthrough of the system's key features. Include expected output for each step.

**Outcomes report**: compare your results against the initial goals. Did you meet the performance constraints? Did you implement all milestones? What would you do differently?

### Step 6: Evaluation Rubric

Your capstone is evaluated on five dimensions:

| Dimension | What we're looking for |
|---|---|
| **Correctness** | Does it work? Are edge cases handled? Are there tests? |
| **Engineering quality** | Is the code readable, well-structured, maintainable? |
| **Validation depth** | How thorough are the tests, benchmarks, and analysis? |
| **Communication** | Are the docs clear? Are design decisions explained? Are tradeoffs acknowledged? |
| **Professional execution** | Was scope controlled? Were milestones met? Is the delivery complete? |

Each dimension is scored 1-5. A passing capstone scores 3+ on all dimensions. A strong capstone scores 4+ on most.

## Use It

This framework mirrors how real engineering projects are scoped and delivered:

- **Google design docs**: every significant project at Google starts with a design doc that covers the same sections: problem, design, alternatives, milestones, risks.
- **Amazon PR/FAQ**: Amazon starts with a press release and FAQ before building. This forces clarity about the customer and the value proposition.
- **RFC processes**: open-source projects (Rust, Python, Kubernetes) use RFCs to propose and discuss changes before implementation.

The key production lesson: **writing the proposal is the hardest part**. If you can't articulate what you're building and why in one page, you don't understand it well enough to build it. The proposal is the design; the code is the implementation.

## Read the Source

- [Google design doc template](https://www.industrialempathy.com/posts/design-docs-at-google/) — How Google structures technical design documents.
- [Amazon's PR/FAQ process](https://www.amazon.science/working-at-amazon/how-to-write-a-prfaq) — Starting with the press release forces clarity.
- [Architecture Decision Records](https://adr.github.io/) — Lightweight documentation for design decisions.

## Ship It

Artifact set in `outputs/`:
- `proposal.md`: your project proposal with problem, goals, non-goals, milestones, and risks.
- `design.md`: technical design with system diagram, interfaces, data model, and failure modes.
- `milestones.md`: milestone tracker with acceptance criteria and status.
- `final-report.md`: outcomes report comparing results against initial goals.

## Exercises

1. **Easy** — Draft a one-page proposal for your chosen capstone. Include: problem statement (2 sentences), users and constraints (3 bullets), non-goals (3 bullets), milestones (3-5 items with day estimates), and risks (2-3 items with mitigations).
2. **Medium** — Define the first two milestones with measurable acceptance criteria. For each milestone, write: what you'll build, how you'll test it, and what "done" looks like. Make the criteria specific enough that someone else could verify you met them.
3. **Hard** — Write a final-demo script before implementation begins. Describe exactly what you'll show: which commands you'll run, what output you'll expect, and what you'll say at each step. This forces you to think about the user experience before you build.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Proposal | "plan" | A document that articulates the what, why, who, and what-not of a project. It constrains scope and enables meaningful review before implementation begins. |
| Milestone | "checkpoint" | A point in the project timeline that produces a testable artifact. Each milestone should be independently verifiable: you can demo it and someone else can run it. |
| ADR | "design decision" | Architecture Decision Record: a short document recording a significant technical choice, the alternatives considered, and the rationale. ADRs prevent "why did we do it this way?" questions months later. |
| Non-goal | "scope boundary" | An explicit statement of what the project will not do. Non-goals are as important as goals: they prevent scope creep and clarify the project's boundary. |
| Risk register | "what could go wrong" | A list of risks with their impact (what breaks) and mitigation (how you'll handle it). The purpose isn't to prevent risks but to have a plan when they materialize. |

## Further Reading

- [Designing Data-Intensive Applications](https://dataintensive.net/) — Kleppmann. The "Designing Data-Intensive Applications" approach to system design.
- [The Staff Engineer's Path](https://www.oreilly.com/library/view/the-staff-engineers/9781098118723/) — Reilly. Covers technical leadership, design documents, and project scoping.
- [ADR GitHub organization](https://adr.github.io/) — Templates and examples for Architecture Decision Records.
