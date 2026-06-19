# Architecture Decision Records (ADRs)

> Architecture Decision Records (ADRs) — the part of CS you can't skip.

**Type:** Learn
**Languages:** Markdown
**Prerequisites:** Phase 16 lessons 01–19
**Time:** ~45 minutes

## Learning Objectives

- Understand what ADRs are and why they are essential for sustainable software projects.
- Write well-structured ADRs that capture context, decisions, and consequences.
- Distinguish between decisions that deserve an ADR and those that do not.
- Manage the ADR lifecycle from proposal through supersession.
- Apply ADR practices to real-world architectural choices.

## The Problem

This lesson sits in **Phase 16 — Software Engineering & Architecture**. Without ADRs, you cannot build the phase's capstone (A refactored real-world OSS repo with ADRs). Concretely, *not* knowing this means you get stuck the moment you try to make code other people can read, change, and ship — at scale, over years.

Picture this: you join a team and discover the system uses Redis for caching, RabbitMQ for messaging, and Kafka for event streaming. No one can tell you *why* those choices were made. A new engineer proposes replacing RabbitMQ with SQS because "it's simpler." Two sprints later, you discover RabbitMQ was chosen because the system needs guaranteed ordering across consumer groups — something SQS doesn't support. The replacer didn't know, because the original decision was never recorded.

That gap — between *making* a decision and *preserving* it — is exactly what ADRs fill.

## The Concept

### What Is an Architecture Decision Record?

An Architecture Decision Record (ADR) is a short text document that captures a single architectural decision: what you chose, *why* you chose it, and what happens because you chose it. ADRs are not design documents, not RFCs, and not meeting minutes. They are **decisions recorded in perpetuity**, stored alongside code in version control.

Michael Nygard introduced the ADR format in a 2011 blog post and later in *Documenting Architecture Decisions* (2015). His insight was simple: most architectural knowledge lives in the heads of senior engineers and evaporates when they leave. ADRs make that knowledge durable and discoverable.

An ADR is:
- **Immutable once accepted** — you don't edit an accepted ADR; you write a new one that supersedes it.
- **Numbered sequentially** —ADR-001, ADR-002, etc., like case law citations.
- **Stored in version control** — alongside the code the decision affects.
- **Short** — typically one to two pages. If it's longer, the decision is probably under-specified.

### Why ADRs Matter

**Institutional memory.** Teams forget why they made decisions. Six months later, someone asks "why are we using PostgreSQL?" and no one remembers. ADRs are the answer key.

**Onboarding accelerant.** New engineers can read the ADR log and understand the system's architectural trajectory — not just what the system *is*, but why it *became* that way.

**Decision trail.** When you revisit a choice, you can read the original ADR to understand the constraints that applied at the time. This prevents "we already decided this, but nobody remembers why" loops.

**Preventing revisiting settled decisions.** If ADR-003 says "use REST for public APIs" with clear reasoning, a new engineer who wants GraphQL can read ADR-003, understand the original rationale, and either accept it or write ADR-012 that supersedes ADR-003 with new information.

**Cross-team alignment.** In organizations with multiple teams, ADRs create a shared vocabulary for architectural decisions. Team A can reference ADR-007 when coordinating with Team B.

### The ADR Template

Michael Nygard's format has five sections:

1. **Title** — A short noun phrase naming the decision. Example: "Use PostgreSQL for Persistent Storage."
2. **Status** — One of: proposed, accepted, deprecated, superseded. If superseded, link to the new ADR.
3. **Context** — The forces at play. Technical constraints, business requirements, team capabilities, timeline pressures. This is the most important section because it explains *why* the decision was even necessary.
4. **Decision** — What you decided. Be specific. "We will use PostgreSQL 15 with the Citus extension for horizontal sharding" is better than "We will use a relational database."
5. **Consequences** — What happens now that you've made this choice. Include both positive and negative. This section is where honesty lives.

Here is the minimal template in full:

```markdown
# ADR-NNNN: [Title]

## Status

[Proposed | Accepted | Deprecated | Superseded by ADR-XXXX]

## Context

[Describe the forces at play — technical, business, organizational.]

## Decision

[What we decided and why.]

## Consequences

[What happens now — benefits, drawbacks, risks.]
```

### When to Write an ADR

Write an ADR for **significant architectural choices**:

- **Technology selections** — choosing a database, messaging system, framework, or language.
- **API design decisions** — choosing REST vs. GraphQL vs. gRPC for a public API.
- **Data model choices** — choosing SQL vs. NoSQL, normalization strategy, sharding approach.
- **Infrastructure decisions** — choosing cloud provider, deployment strategy (containers vs. serverless), CDN selection.
- **Security architecture** — choosing auth strategy (OAuth2 vs. SAML), encryption approach.
- **Observability** — choosing logging framework, tracing system, metrics approach.
- **Structural decisions** — monorepo vs. polyrepo, microservices vs. monolith, synchronous vs. asynchronous communication.

The rule of thumb: if the decision would be expensive or painful to reverse, write an ADR.

### When NOT to Write an ADR

Do *not* write an ADR for:

- **Minor refactoring** — renaming a variable, extracting a method, reorganizing a directory structure.
- **Obvious standard practices** — using Git for version control, writing unit tests, using HTTPS.
- **Tactical implementation details** — choosing between `ArrayList` and `LinkedList` for a specific use case.
- **Decisions already documented elsewhere** — if an existing ADR already covers the ground.

Writing too many ADRs dilutes their value. The ADR log should be a curated collection of genuinely important decisions, not a diary of every choice you make.

### ADR Workflow

ADRs follow a workflow analogous to code review:

1. **Propose** — The author writes an ADR with status "Proposed" and submits it via pull request.
2. **Discuss** — Stakeholders review the ADR, ask questions, suggest alternatives, and debate trade-offs. This discussion happens in the pull request comments, just like code review.
3. **Accept or Reject** — The ADR reaches consensus. If accepted, the status changes to "Accepted." If rejected, the status changes to "Rejected" and the ADR remains in the log as a record of a path not taken.
4. **Supersede** — When new information invalidates an accepted ADR, write a new ADR that supersedes the old one. The old ADR's status changes to "Superseded by ADR-XXXX."

Key principle: **rejected ADRs are still valuable.** They document roads not taken and prevent re-proposal of the same idea without new information.

### ADR Lifecycle

An ADR moves through these statuses:

```
Proposed → Accepted → Deprecated
                  ↓
            Superseded by ADR-XXXX
```

- **Proposed** — The ADR is under discussion. It has no authority.
- **Accepted** — The team has agreed. This is the current architectural direction.
- **Deprecated** — The decision is no longer relevant (the system has moved on, the feature was removed, etc.).
- **Superseded** — A newer ADR replaces this one. Always link the superseding ADR.

Important nuances:
- An accepted ADR is not sacred. It can be superseded when circumstances change.
- ADRs should never be deleted. Even deprecated and superseded ADRs remain in the log for historical reference.
- The ADR number is never reused. If ADR-007 is superseded by ADR-015, there is never a new ADR-007.

### How to Write Good ADRs

**Write specific context.** "We need a database" is not context. "We need a database that supports ACID transactions for financial records, must run on AWS us-east-1, and our team has PostgreSQL expertise but zero MongoDB experience" is context. The more specific you are, the more useful the ADR becomes over time.

**Make clear decisions.** "We will evaluate options" is not a decision. "We will use PostgreSQL 15 with the Citus extension for horizontal sharding, deployed as a managed RDS instance" is a decision. Specificity here enables accountability.

**Be honest about consequences.** The positive consequences are easy to write. The negative ones are the ones that matter most. If choosing PostgreSQL means your maximum write throughput is lower than with Cassandra, say so. Future readers need the honest trade-off, not a sales pitch.

**Include alternatives considered.** Briefly mention the alternatives you evaluated and why you rejected them. This prevents someone from proposing the same alternative six months later.

**Reference relevant resources.** Link to benchmarks, blog posts, internal documents, or Slack threads that informed the decision. This gives future readers a trail to follow.

### ADRs vs. RFCs vs. Design Docs

These three artifacts serve different purposes:

| Aspect | ADR | RFC | Design Doc |
|--------|-----|-----|------------|
| Purpose | Record a decision | Propose a change | Describe a design |
| Timing | After or during decision | Before decision | Before implementation |
| Length | 1-2 pages | Varies | 5-20 pages |
| Scope | Single decision | Can cover multiple decisions | Can cover multiple decisions |
| Immutability | Immutable once accepted | Mutable during review | Mutable during review |
| Outcome | Decision record | Approval or rejection | Implementation plan |

ADRs are the *what and why*. RFCs are the *what if*. Design docs are the *how*. They complement each other — an RFC may result in an ADR, and a design doc may reference ADRs as constraints.

### Real Examples from Open-Source Projects

**Kubernetes** uses ADRs extensively. Their ADR directory includes decisions like:
- ADR-001: Use etcd for the distributed datastore
- ADR-002: Use gRPC for API communication
- ADR-003: Use OpenAPI for API specification

Each ADR captures the specific Kubernetes context — scale requirements, consistency needs, community considerations — that led to the decision.

**Spotify** published their ADR practices in engineering blog posts, showing how they use ADRs to manage architectural decisions across hundreds of autonomous squads. Their key insight: ADRs enable autonomous teams to make local decisions while maintaining alignment with organizational architecture.

### Common Mistakes

**Writing ADRs after the fact.** The most common mistake. If you write an ADR six months after the decision was made, the context is already fuzzy and the consequences are already known — you're writing history, not recording a decision. ADRs should be written *during* the decision-making process.

**Vague context.** "We needed something fast" doesn't tell future readers anything. What were the constraints? What were the alternatives? What were the time pressures?

**Missing negative consequences.** An ADR that lists only benefits is a marketing document, not a decision record. Every architectural choice has downsides. Document them.

**Too many ADRs.** If every code change gets an ADR, the log becomes noise. ADRs should capture *architectural* decisions — choices that affect system structure, not implementation details.

**ADR sprawl.** If you find yourself writing ADRs for trivial choices, raise the bar. Not every database table schema change deserves an ADR.

### How Many ADRs Is Too Many?

There is no strict number, but here are guidelines:

- A startup with 5 engineers might have 5-10 ADRs per year.
- A mid-size company with 50 engineers might have 20-30 ADRs per year.
- A large organization with 500+ engineers might have 50-100 ADRs per year across teams.

If you're writing more than one ADR per engineer per year, you're probably over-documenting. If you're writing fewer than one per year per team, you're probably under-documenting.

The right number depends on how many *architectural* decisions your team makes. The key word is *architectural* — structural choices that affect system behavior, performance, or maintainability.

### ADR Tools and Templates

Several tools support the ADR workflow:

**adr-tools** (https://github.com/npryce/adr-tools) — A command-line tool by Nat Pryce that generates ADR files with sequential numbering and manages the ADR log. Commands like `adr new`, `adr list`, and `adr link` make the workflow frictionless.

**log4brains** (https://github.com/thomvaill/log4brains) — A more modern tool that generates a searchable web interface for your ADRs, supports Markdown, and integrates with CI/CD pipelines. It adds a `log4brains` command that creates, publishes, and serves ADRs.

**adr-viewer** — Renders ADRs as a static website, useful for sharing with stakeholders who don't have repository access.

For teams starting out, even a simple `docs/adr/` directory with numbered Markdown files is sufficient. The format matters less than the practice of writing them.

### Linking ADRs to Code and Commit Messages

ADRs are most valuable when they are discoverable from the code they affect:

- **Commit messages** — Reference ADR numbers in commit messages: "Implement connection pooling per ADR-005." This creates a two-way link between decisions and implementations.
- **Code comments** — In key architectural files, add a comment: `// See ADR-005 for the decision to use connection pooling.`
- **PR descriptions** — When a PR implements an ADR, link to it in the description. When a PR motivates a new ADR, write the ADR first, then link it from the PR.
- **README references** — Include an "Architecture Decisions" section in your project README that links to the ADR directory.

The goal is that any engineer, at any point in the codebase, can trace an architectural choice back to its original reasoning.

## Build It

### Step 1: Minimal Version

Create a minimal ADR in `docs/adr/0001-use-postgresql-for-persistent-storage.md`:

```markdown
# ADR-0001: Use PostgreSQL for Persistent Storage

## Status

Accepted

## Context

We need a relational database for the application's persistent storage.
The team has extensive PostgreSQL experience and the application requires
ACID compliance for financial transactions.

## Decision

We will use PostgreSQL 15 as our primary data store.

## Consequences

- Positive: ACID compliance, team expertise, mature ecosystem.
- Negative: Does not horizontally scale writes without Citus.
```

### Step 2: Realistic Version

Expand the minimal version with alternatives considered, more specific context, and honest consequences:

```markdown
# ADR-0001: Use PostgreSQL for Persistent Storage

## Status

Accepted

## Context

We need a primary data store for the application. Key requirements:
- ACID compliance for financial transaction records
- Must run on AWS us-east-1
- Must support complex joins for reporting queries
- Team has 8 years of PostgreSQL experience, 0 MongoDB experience
- Project timeline allows 2 weeks for infrastructure setup
- Budget permits managed database service (RDS)

Alternatives considered:
- MongoDB: No team experience, no ACID guarantees for multi-document transactions
- MySQL: Less feature-rich for our reporting needs, team less familiar
- DynamoDB: No joins, would require denormalization, significantly more complex application logic

## Decision

We will use PostgreSQL 15 deployed as an AWS RDS instance with Multi-AZ enabled.

## Consequences

- Positive: ACID compliance for financial data, team can be productive immediately,
  mature tooling (pg_dump, psql, pg_stat_statements), excellent JOIN performance
  for reporting queries.
- Negative: Write scaling is limited to vertical scaling unless we add Citus later,
  connection pooling requires PgBouncer as a sidecar, RDS storage costs scale with
  data volume, no built-in change data capture (requires Debezium).
- Neutral: We are committing to the PostgreSQL ecosystem for the foreseeable future.
```

## Use It

The production tool for ADR management is **adr-tools** by Nat Pryce. Install it and try these commands:

```bash
# Initialize ADR directory
adr init docs/adr

# Create a new ADR
adr new "Use PostgreSQL for Persistent Storage"

# List all ADRs
adr list

# Link ADRs (supersede, etc.)
adr supersede 1 5
```

For teams wanting a web interface, **log4brains** offers:

```bash
# Initialize log4brains in a project
npx log4brains init

# Create a new ADR interactively
npx log4brains new

# Serve a local web UI for browsing ADRs
npx log4brains preview
```

Both tools enforce sequential numbering, ensure the template is followed, and make ADR management frictionless.

### Comparing Your Hand-Built ADR to the Production Tool

| What you built | What the production tool does |
|----------------|-------------------------------|
| Manual numbering | Auto-increments ADR numbers |
| Hand-written template | Generates template with date, author |
| Manual supersession links | `adr supersede` auto-links old and new ADRs |
| Flat directory listing | Generates an ADR index |

The production tools reduce the friction of writing ADRs, but the core practice — writing clear context, specific decisions, and honest consequences — is the same whether you use a tool or a text editor.

## Read the Source

- **adr-tools**: https://github.com/npryce/adr-tools — Shell scripts that generate ADR files. Read `src/adr-new` to see how it creates numbered files from a template.
- **log4brains**: https://github.com/thomvaill/log4brains — TypeScript-based ADR management. Read `packages/cli/src/commands/new.ts` to see how it creates ADRs interactively.
- **Michael Nygard's original essay**: "Documenting Architecture Decisions" — the foundational text on ADRs.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A ready-to-use ADR template** that teams can adopt immediately — copy it into `docs/adr/` and start writing decisions.

## Exercises

1. **Easy** — Write an ADR for a technology choice you recently made on a project. Use the full template with context, decision, consequences, and alternatives considered.
2. **Medium** — Find an architectural decision in a codebase you work on that was made without an ADR. Write the ADR retroactively, noting which context you had to reconstruct and what was lost.
3. **Hard** — Implement an ADR governance workflow for your team: propose → review → accept/reject → supersede. Include criteria for when an ADR is required and when it's not. Write a meta-ADR (an ADR about adopting ADRs) documenting this workflow.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| ADR | "We should document that" | A numbered, immutable record of a specific architectural decision, its context, and its consequences |
| Status | "Is this decided?" | The lifecycle state of an ADR: proposed, accepted, deprecated, or superseded |
| Context | "Why did we choose this?" | The forces, constraints, and requirements that motivated the decision — the most important section |
| Consequences | "What happens now?" | The positive, negative, and neutral outcomes of the decision — must include trade-offs |
| Superseded | "That's old" | An ADR replaced by a newer one; linked to maintain the decision trail |
| adr-tools | "ADR tooling" | A CLI by Nat Pryce that generates and manages ADR files with sequential numbering |
| RFC | "Let's put that in an RFC" | A Request for Comments — a proposal before a decision, not a record of the decision itself |

## Further Reading

- Michael Nygard, "Documenting Architecture Decisions" (2015) — the original ADR essay
- Nat Pryce, adr-tools (https://github.com/npryce/adr-tools)
- Thom Vaill, log4brains (https://github.com/thomvaill/log4brains)
- Spotify Engineering Blog, "When to Write an ADR" (2020)
- Kubernetes ADR directory: https://github.com/kubernetes/community/tree/master/sig-architecture
- ThoughtWorks Technology Radar entries on "Architecture Decision Records"