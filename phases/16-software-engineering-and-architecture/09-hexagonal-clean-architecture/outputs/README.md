# Architecture Reference Card — Hexagonal, Clean, Layered, and DDD

## The Dependency Rule (All Architectures Share This)

**Dependencies point inward.** The innermost circle (domain/entities) has zero knowledge of
anything outside it. The outermost circle (frameworks/infrastructure) depends on everything
inside.

```
  Frameworks & Adapters  →  Interface Adapters  →  Use Cases  →  Entities
  (outermost, knows all)       (knows inner 2)      (knows inner 1)  (knows nothing)
```

## Hexagonal Architecture (Alistair Cockburn, 2005)

### One-Liner
The application core defines port interfaces; adapters implement them. The core never
imports infrastructure.

### Structure
```
┌─────────────────────────────────────────┐
│              Driving Adapters            │
│   REST controllers, CLI, gRPC, MQ       │
│              ┌───────────────┐           │
│              │ Primary Ports │           │
│              │ (Use Cases)   │           │
│              └───────┬───────┘           │
│                      │                   │
│         ┌────────────▼────────────┐      │
│         │     APPLICATION CORE     │      │
│         │  Entities, Value Objects │      │
│         │  Domain Rules/Invariants │      │
│         └─┬──────────────────┬─────┘      │
│           │                  │           │
│   ┌───────▼───────┐  ┌──────▼────────┐  │
│   │ Secondary Port │  │ Secondary Port │  │
│   │ (Repository)   │  │ (Notifier)     │  │
│   └───────┬───────┘  └──────┬────────┘  │
│           │                  │           │
│   ┌───────▼───────┐  ┌──────▼────────┐  │
│   │ Driven Adapter │  │ Driven Adapter│  │
│   │ (PostgreSQL)   │  │ (SendGrid)    │  │
│   └───────────────┘  └───────────────┘  │
└─────────────────────────────────────────┘
```

### Key Concepts

| Concept | What It Is | Example |
|---------|-----------|---------|
| **Port** | Interface defined by core | `UserRepository`, `RegisterUserUseCase` |
| **Adapter** | Implementation of a port | `PostgresUserRepository`, `RegisterUserController` |
| **Driving Port** | API — what the app CAN do | `RegisterUserUseCase.execute()` |
| **Driven Port** | SPI — what the app NEEDS | `UserRepository.findByEmail()` |
| **Hexagon** | The application core | Entities, use cases, port interfaces |

### When to Use
- Domain-heavy services where the business model is the core value.
- Microservices that need to stay independent of specific databases or frameworks.
- Any service where you need to swap infrastructure (e.g., migrate from MySQL to DynamoDB).

---

## Clean Architecture (Robert Martin, 2012)

### One-Liner
Four concentric layers. Dependencies point inward. The innermost layer has no framework
dependencies.

### Structure
```
┌──────────────────────────────────────────────┐
│ 4. Frameworks & Drivers                       │
│   Web, Database, External APIs, UI             │
│ ┌──────────────────────────────────────────┐ │
│ │ 3. Interface Adapters                     │ │
│ │   Controllers, Gateways, Presenters       │ │
│ │ ┌──────────────────────────────────────┐ │ │
│ │ │ 2. Application Business Rules        │ │ │
│ │ │   Use Cases (Interactors)             │ │ │
│ │ │ ┌────────────────────────────────┐ │ │ │
│ │ │ │ 1. Enterprise Business Rules    │ │ │ │
│ │ │ │   Entities (domain model)       │ │ │ │
│ │ │ └────────────────────────────────┘ │ │ │
│ │ └──────────────────────────────────────┘ │ │
│ └──────────────────────────────────────────┘ │
└──────────────────────────────────────────────┘
```

### Layer Responsibilities

| Layer | Owns | Depends on | Examples |
|-------|------|-----------|----------|
| **Entities** | Business rules, domain model | Nothing | `User`, `Email`, `Order` |
| **Use Cases** | Application-specific rules | Entities + port interfaces | `RegisterUserInteractor` |
| **Interface Adapters** | Data conversion, controllers, gateways | Use cases | `RegisterUserController`, `UserRepoImpl` |
| **Frameworks** | Framework details, I/O | Interface adapters | Express routes, Spring config, PostgreSQL driver |

### When to Use
- Use-case-heavy applications with many distinct workflows.
- Systems needing clear separation between enterprise rules and application rules.
- When different delivery mechanisms (REST, CLI, gRPC) share the same use cases.

---

## Layered Architecture (Traditional)

### One-Liner
Three layers stacked vertically. Business layer depends on Data Access layer. Simple but
fragile under domain complexity.

### Structure
```
┌──────────────────┐
│   Presentation    │  ← HTTP handlers, UI
├──────────────────┤
│   Business Logic  │  ← Service classes, domain rules
├──────────────────┤      ⚠️ DEPENDS ON DATA ACCESS
│   Data Access     │  ← SQL, ORM, repositories
└──────────────────┘
```

### Problems

| Problem | Why It Happens |
|---------|---------------|
| Business logic depends on infrastructure | `BusinessLayer` imports `SqlRepository` directly |
| Leaky abstractions | Data layer leaks pagination, transaction semantics upward |
| No boundary enforcement | Presentation can call Data directly, bypassing Business |
| Testing requires infrastructure | Can't test business rules without a database |
| Hard to swap technologies | Replacing PostgreSQL with MongoDB requires editing business code |

### When to Use (Rarely)
- Simple CRUD APIs with minimal business logic.
- Prototypes and throwaway scripts.
- When the domain is trivial and unlikely to grow complex.

---

## DDD (Domain-Driven Design)

### One-Liner
Model the domain using the language of domain experts. Aggregates, value objects, and
bounded contexts ensure the code mirrors reality.

### Key Concepts

| Concept | What It Is |
|---------|-----------|
| **Ubiquitous Language** | Shared vocabulary between developers and domain experts |
| **Bounded Context** | A boundary within which a model applies consistently |
| **Aggregate** | A cluster of domain objects treated as a unit (consistency boundary) |
| **Value Object** | Immutable, compared by value (e.g., `Email`, `Money`) |
| **Entity** | Mutable, compared by identity (e.g., `User`) |
| **Domain Event** | Something that happened in the domain that other parts care about |

### DDD vs. Hexagonal vs. Clean Architecture

| Concern | Hexagonal | Clean | DDD |
|---------|-----------|-------|-----|
| **Primary focus** | Architectural boundaries | Layer separation | Domain modeling |
| **Key mechanism** | Ports & adapters | Four layers | Ubiquitous language, aggregates |
| **Domain model** | Can be simple | Can be simple | Must be rich and expressive |
| **Testability** | Mock any adapter | Mock gateway interfaces | Test aggregates in isolation |
| **Complexity** | Low–medium | Medium | High |

### When to Use
- Complex domains where the business rules ARE the product (banking, healthcare, logistics).
- When communication with domain experts is ongoing and the model must evolve.
- Combine with Hexagonal: rich DDD domain inside the hexagon, ports for boundaries.

---

## Decision Matrix

| If your system is... | Use | Because |
|----------------------|-----|---------|
| A CRUD API with 5 endpoints | Layered | The domain isn't complex enough to justify ports |
| A microservice with real business rules | Hexagonal | Ports make the core independently testable and deployable |
| An application with many workflows/use cases | Clean Architecture | Use-case interactors give each workflow a clear home |
| A complex domain where the model IS the value | DDD + Hexagonal | Rich domain model inside ports, ubiquitous language outside |
| A team struggling with mock-heavy tests | Hexagonal or FC/IS | Swap adapters (or make core pure) to eliminate mocks |

## Quick Implementation Checklist

1. **Create a `domain/` module** with zero framework dependencies (no Spring, no Express, no ORM).
2. **Define port interfaces in `domain/`** — one driving port per use case, one driven port per external dependency.
3. **Implement use cases** in an `application/` module that depends only on `domain/`.
4. **Implement adapters** in separate packages that depend on `domain/` (for interface definitions) and their respective frameworks.
5. **Wire in `main()`** — compose the object graph: `new UseCase(new InMemoryRepo(), new ConsoleNotifier())`.
6. **Enforce with a build tool** — Gradle/Maven modules or Go module boundaries that prevent `domain/` from importing `adapters/`.
7. **Test the core with fakes** — `InMemoryUserRepository`, `ConsoleNotifier`. No databases. No mock frameworks.
8. **Test adapters with real infrastructure** — Integration tests for PostgreSQL, SendGrid. Only a handful needed.

## Key Terms

- **Port** — Interface defined by the core. Driving ports = API (what the app does). Driven ports = SPI (what the app needs).
- **Adapter** — Implementation of a port. Connects the core to a specific technology.
- **Dependency Rule** — Dependencies point inward. The core never imports from adapters or frameworks.
- **Hexagon** — The application core: entities, use cases, and port interfaces. Independent of all infrastructure.
- **Interactor** — A use-case class that implements a driving port and orchestrates domain logic via driven ports.
- **Bounded Context** (DDD) — A boundary within which a domain model is consistent and ubiquitous language applies.
- **Value Object** — Immutable object compared by value, not identity (e.g., `Email`, `Money`).
- **Aggregate** (DDD) — A consistency boundary: a cluster of objects that must be changed together.