# Phase Capstone Artifact — Refactored OSS Repo + ADR Bundle

## What This Capstone Produces

This capstone produces three interconnected artifacts that demonstrate mastery of Phase 16 concepts:

### 1. Before & After Code (`code/main.ts`)

A complete TypeScript project simulating a real refactoring capstone:

- **BEFORE section** — A smelly `OrderService` god class with tight coupling, no tests, primitive obsession, and mixed concerns
- **AFTER section** — Refactored hexagonal architecture with domain layer, ports, adapters, and application services
- **TESTS section** — Unit tests for domain objects, application services, and integration between layers
- **COMPARISON** — A `demonstrate()` function running both versions side-by-side

### 2. ADR Bundle (`code/notes.md`)

Three Architecture Decision Records, each cross-referencing specific Phase 16 lessons:

| ADR | Title | Primary Lessons | Secondary Lessons |
|-----|-------|-----------------|-------------------|
| ADR-001 | Adopt Hexagonal Architecture | L09 (Hexagonal), L03 (SOLID — SRP, DIP) | L02 (Coupling), L08 (DDD) |
| ADR-002 | Separate Commands from Queries (CQRS Lite) | L11 (CQRS), L03 (SOLID — ISP) | L09 (Ports), L10 (Events) |
| ADR-003 | Adopt Event-Driven Order Processing | L10 (Events), L08 (DDD — Domain Events) | L04 (Observer), L11 (CQRS) |

### 3. Quiz (`quiz.json`)

Six questions covering refactoring strategy, ADR writing, and cross-lesson connections:

- Pre-stage questions test readiness (characterization tests, project selection)
- Post-stage questions test understanding (ADR timing, OCP in practice, debt analysis, refactoring ordering)

## How Each Phase Lesson Connects to This Capstone

| Lesson | Concept | Where It Appears in the Capstone |
|--------|---------|--------------------------------|
| L01 | What Makes SW Engineered | The entire capstone is engineering: systematic, documented, tested |
| L02 | Naming, Cohesion, Coupling | Value objects replace primitives; `OrderService` split by responsibility |
| L03 | SOLID Principles | SRP (god class → services), OCP (new adapters, no core changes), DIP (ports over concrete) |
| L04 | GoF Patterns | Strategy (discount calculation), Observer (event subscribers), Adapter (infrastructure) |
| L05 | Modern Patterns | Functional core (pure domain) with imperative shell (application services) |
| L06 | Refactoring Mechanics | Seven-step refactoring sequence, each small and tested |
| L07 | Code Review | Each ADR-referenced commit should be reviewed as a PR |
| L08 | DDD | `Order` entity, `Money` value object, `OrderId` identity, `OrderCreatedEvent` |
| L09 | Hexagonal Architecture | domain/ → ports/ → adapters/ dependency rule |
| L10 | Event-Driven | `OrderCreatedEvent`, `InProcessEventBus`, decoupled subscribers |
| L11 | CQRS | `OrderCommandService` vs `OrderQueryService`, separate models |
| L12 | Microservices | Not applied (in scope note: this monolith doesn't need them yet) |
| L13 | API Design | Port interfaces define the domain's API contract |
| L14 | Versioning | Commit messages reference ADRs; event schemas are a versioned contract |
| L15 | Monorepos | Structured as a monorepo-style directory: domain, ports, adapters in one tree |
| L16 | Dependency Management | Domain has zero external dependencies; adapters manage their own |
| L17 | CI/CD | Tests run without infrastructure (in-memory adapters) — CI-friendly |
| L18 | Observability | Event bus enables audit logging and metric subscriber (future) |
| L19 | Technical Debt | Debt inventory template maps each smell to a lesson and ADR |
| L20 | ADRs | Three ADRs document the architectural decisions |
| L21 | Reading Codebases | Three-pass reading method applied to the "before" code |

## Running the Code

```bash
npx ts-node code/main.ts
```

This runs all domain and application tests, then demonstrates the before/after comparison, and prints a summary of architecture improvements tied to specific lessons.