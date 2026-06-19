# Hexagonal / Clean Architecture

> Make your domain independent of frameworks, databases, and delivery mechanisms. Dependencies point inward — always.

**Type:** Learn
**Languages:** TypeScript
**Prerequisites:** Phase 16 lessons 01–08
**Time:** ~60 minutes

## Learning Objectives

- Explain the dependency rule and why dependencies must point inward.
- Identify ports (interfaces) and adapters (implementations) in a hexagonal system.
- Distinguish driving (primary) ports from driven (secondary) ports.
- Contrast hexagonal, clean, layered, and DDD architectural styles.
- Implement a feature using hexagonal architecture with swappable adapters.
- Demonstrate testability by swapping adapters without touching the domain core.

## The Problem

You are building a user registration service. Version one looks like this:

```typescript
class RegistrationService {
  register(email: string, password: string): User {
    if (this.db.query("SELECT * FROM users WHERE email = ?", email)) {
      throw new Error("duplicate");
    }
    const hash = bcrypt.hashSync(password, 10);
    const user = this.db.insert("INSERT INTO users ...", email, hash);
    this.smtp.send("welcome", email, "Welcome!");
    this.redis.set(`user:${user.id}`, JSON.stringify(user), 3600);
    return user;
  }
}
```

Five lines of code, four infrastructure dependencies: MySQL, bcrypt, SMTP, Redis. You
cannot test the email-validation rule without a database. You cannot test the duplicate-
check without an SMTP server. The business rule ("no duplicate emails") is welded to the
delivery mechanism (SQL, HTTP, Redis). Every change to the database schema forces a
change to registration logic. This is the **concretion trap**: business rules chained to
infrastructure choices so tightly that neither can evolve independently.

## The Concept

### Hexagonal Architecture (Alistair Cockburn, 2005)

Alistair Cockburn proposed the **Ports and Adapters** pattern — later called Hexagonal
Architecture because the hexagon represents the application core with ports on every side.
The key insight is simple but radical:

> **The application core has no knowledge of any outside technology.**

The core defines **ports** — interfaces that describe what it *needs* ("give me a way to
store and retrieve users") and what it *provides* ("I can register a user given an email
and password"). **Adapters** are the implementations that connect ports to the outside
world: a PostgreSQL adapter, an in-memory adapter for tests, a console adapter for scripts.

```
                    ┌─────────────────────┐
                    │   Driving Adapter    │
                    │   (REST Controller)  │
                    └──────────┬──────────┘
                               │ calls
                    ┌──────────▼──────────┐
                    │   Primary Port      │
                    │   (RegisterUser     │
                    │    interface)       │
                    └──────────┬──────────┘
                               │ implements
              ┌────────────────▼────────────────┐
              │          APPLICATION CORE        │
              │                                  │
              │  RegisterUserUseCase             │
              │  User (domain entity)             │
              │  Email (value object)            │
              │                                  │
              │  defines needs via ports:         │
              │  ├─ UserRepository (secondary)    │
              │  └─ NotificationService (secondary)│
              └───────┬────────────────┬──────────┘
                      │                │
              ┌───────▼──────┐  ┌──────▼──────────┐
              │ Secondary    │  │ Secondary        │
              │ Port         │  │ Port              │
              │ (UserRepo   │  │ (Notifier         │
              │  interface)  │  │  interface)       │
              └───────┬──────┘  └──────┬───────────┘
                      │                │
         ┌────────────▼──┐    ┌────────▼──────────┐
         │ Driven Adapter│    │ Driven Adapter     │
         │ (PostgreSQL   │    │ (SendGrid Email    │
         │  Repository)  │    │  Notifier)         │
         └───────────────┘    └────────────────────┘
```

The hexagon (core) is the inside. Everything else is outside. The **dependency rule** is
absolute: **dependencies point inward**. The core imports nothing from the outside. The
outside imports from the core. An adapter depends on the port interface (defined in the
core), but the core never depends on the adapter.

### Driving (Primary) Ports vs. Driven (Secondary) Ports

**Driving ports** (also called primary or inbound ports) are interfaces that the core
*exposes* to the outside world. They define what the application *can do*. A REST
controller, a CLI command, or a message consumer calls through a driving port. Example:
`RegisterUserUseCase` is a driving port — it describes a capability the application offers.

**Driven ports** (also called secondary or outbound ports) are interfaces that the core
*needs* from the outside world. They describe what the application *requires*. Example:
`UserRepository` is a driven port — the core says "I need a way to persist and retrieve
users." The adapter (PostgreSQL, in-memory, etc.) implements this interface.

The distinction matters:
- Driving ports = **API of the application** (what callers use).
- Driven ports = **SPI of the application** (what the application needs supplied).

### Clean Architecture (Robert Martin, 2012)

Robert Martin's Clean Architecture refines and generalizes the same dependency rule into
four concentric circles:

```
┌────────────────────────────────────────────────────────┐
│                   Frameworks & Drivers                  │
│  Web, Database, UI, External APIs                       │
│  ┌──────────────────────────────────────────────────┐  │
│  │           Interface Adapters                      │  │
│  │  Controllers, Gateways, Presenters               │  │
│  │  ┌──────────────────────────────────────────┐    │  │
│  │  │        Application Business Rules         │    │  │
│  │  │  Use Cases (interactors)                  │    │  │
│  │  │  ┌──────────────────────────────────┐    │  │  │
│  │  │  │    Enterprise Business Rules      │    │  │  │
│  │  │  │    Entities (domain model)       │    │  │  │
│  │  │  └──────────────────────────────────┘    │  │  │
│  │  └──────────────────────────────────────────┘    │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘

The Dependency Rule: dependencies point inward only.
```

The four layers, from inside out:

1. **Entities** — Enterprise-wide business rules. The `User` entity, the `Email` value
   object, the business invariant "no duplicate emails." These are the most stable
   elements of the system and have zero external dependencies.

2. **Use Cases** — Application-specific business rules. `RegisterUserUseCase` orchestrates
   entities, validates input, and enforces application rules (like "enqueue a welcome
   email"). Use cases depend on entities and on driven-port interfaces, but never on
   infrastructure.

3. **Interface Adapters** — Controllers, presenters, and gateways. A `RegisterUserController`
   translates an HTTP request into a use-case call. A `UserRepositoryImpl` translates the
   `UserRepository` interface calls into SQL queries. These are the adapters.

4. **Frameworks & Drivers** — The outermost ring: Express, Spring Boot, PostgreSQL driver,
   React. These are details the core should not know about. You should be able to swap
   Express for Fastify without touching use cases or entities.

### Why Layered Architecture Fails

The traditional three-tier architecture (Presentation → Business → Data) looks similar:

```
┌──────────────────┐
│   Presentation    │
├──────────────────┤
│   Business Logic  │  ← depends on Data layer!
├──────────────────┤
│   Data Access     │
└──────────────────┘
```

Three problems:

1. **Business logic depends on data access.** The "Business Logic" layer imports repository
   concretions (e.g., `SqlUserRepository`). This violates the dependency rule because your
   domain now knows about SQL, connection strings, and ORM annotations. You cannot test
   business rules without a database.

2. **Leaky abstractions.** The data-access layer leaks pagination, transaction semantics,
   and query patterns upward. Your business rules start saying things like "batch insert
   100 rows" instead of "register 100 users."

3. **No enforcement of the boundary.** Nothing in the layering convention prevents
   Presentation from calling Data directly, bypassing Business Logic entirely. The
   convention is a suggestion, not an architecture.

Hexagonal and Clean Architecture fix all three by enforcing the dependency rule
programmatically: the core defines *interfaces* (ports), and the infrastructure
*implements* those interfaces (adapters). The dependency inversion principle ensures
business logic never imports infrastructure code.

### Comparison to DDD

Domain-Driven Design (DDD) and hexagonal architecture are complementary but address
different concerns:

| Concern | Hexagonal / Clean | DDD |
|---------|-------------------|-----|
| **Focus** | Architectural structure — where code lives and how it communicates | Domain modeling — how to express business concepts in code |
| **Key idea** | Dependency rule: inward only | Ubiquitous language: code mirrors domain experts' mental model |
| **Result** | Testable, technology-independent core | Rich domain model with aggregates, value objects, domain events |
| **Ports** | Explicit port interfaces for all I/O | Repository pattern as one port; domain events as another |
| **When** | Use for architectural organization | Use when the domain is complex enough to justify modeling effort |

You can (and should) use both: DDD gives your hexagon core a rich, expressive domain
model. Hexagonal architecture ensures that model stays independent of Spring, PostgreSQL,
and React.

### Testability: The Architecture That Makes Mocking Trivial

This is the payoff. In a hexagonal system, testing the core requires zero infrastructure:

```typescript
// In test — swap the real adapter for a fake
const fakeRepo: UserRepository = new InMemoryUserRepository();
const fakeNotifier: NotificationService = new ConsoleNotifier();
const useCase = new RegisterUserUseCase(fakeRepo, fakeNotifier);

// Run the use case — pure logic, no database, no SMTP
const result = useCase.execute("alice@example.com", "password123");
assert(result.isOk);
```

No mock frameworks. No database containers. No SMTP servers. The core is testable because
it depends only on interfaces, and the interfaces are implemented by cheap, in-memory
fakes for tests. Integration tests are reserved for the thin adapter layer.

```
┌──────────────────────────────────────────────────┐
│           Domain + Use Case Tests                 │
│                                                   │
│  • InMemoryUserRepository (5 lines)              │
│  • ConsoleNotifier (3 lines)                     │
│  • No frameworks, no containers, no I/O           │
│  • Thousands of tests in milliseconds             │
│                                                   │
├──────────────────────────────────────────────────┤
│           Adapter Tests (few, integration)        │
│                                                   │
│  • PostgreSQLUserRepository with real DB          │
│  • SendGrid notifier with real API                │
│  • REST controller with HTTP server               │
│  • A handful of tests, run in CI only             │
└──────────────────────────────────────────────────┘
```

### Real-World Examples

**Spring applications.** Spring's `@Service` beans are use cases. `@RestController` classes
are driving adapters. `@Repository` interfaces are driven ports (Spring Data JPA
auto-generates the adapter). The Spring module system + dependency injection naturally
supports hexagonal architecture — but many teams put business logic inside `@Service`
classes that depend on `JdbcTemplate`, violating the dependency rule. To do hexagonal
in Spring properly: put the domain in a separate Gradle/Maven module with zero Spring
dependencies, then wire adapters via `@Configuration`.

**Go microservices.** In Go, the natural unit of hexagonal architecture is a package or
module. The `domain` package defines entities and port interfaces (`UserRepository`,
`Notifier`). The `adapters/postgres` package implements `UserRepository` using
`database/sql`. The `adapters/http` package implements the driving adapter (a `net/http`
handler). The `cmd/service` package wires everything together in `main()`. Go's interface
satisfaction (structural typing) makes this especially clean: the postgres adapter
implicitly satisfies the domain port with no `implements` clause.

### When to Use Which

| Architecture | Best For | Key Trait |
|-------------|----------|-----------|
| **Hexagonal** | Domain-heavy services where the domain model is the center of the system | Explicit ports and adapters; primary/secondary port distinction |
| **Clean** | Use-case-heavy applications with many application-specific workflows | Four-layer separation; use-case interactor pattern |
| **Layered** | Simple CRUD applications with minimal business logic | Fast to build; no ceremony; breaks down under domain complexity |
| **DDD** | Complex domains where the business rules are the core value of the system | Rich domain model; aggregates; ubiquitous language |
| **Hexagonal + DDD** | Complex domains that also need architectural enforcement | Rich domain inside the hexagon; ports for all boundaries |

## Build It

### Step 1: Minimal Version — Domain Core with Ports

We start with the purest layer: domain entities and value objects. These have zero
dependencies on anything outside.

```typescript
// Domain: a value object that cannot be constructed in an invalid state
class Email {
  readonly value: string;
  private constructor(value: string) { this.value = value; }
  static create(raw: string): Email {
    if (!raw || !raw.includes("@")) throw new Error("Invalid email");
    return new Email(raw.toLowerCase().trim());
  }
}

// Domain: entity with business rule — no duplicate emails
class User {
  readonly id: string;
  readonly email: Email;
  readonly hashedPassword: string;
  constructor(id: string, email: Email, hashedPassword: string) {
    this.id = id;
    this.email = email;
    this.hashedPassword = hashedPassword;
  }
}
```

No database. No HTTP. No framework. Just business rules. `Email.create()` enforces the
invariant "all emails must be valid" at construction time — it is impossible to create
an invalid `Email` object.

Now define the ports — interfaces that describe what the core needs and provides:

```typescript
// Driving port (primary) — what the application CAN do
interface RegisterUserUseCase {
  execute(email: string, password: string): Result<User>;
}

// Driven port (secondary) — what the application NEEDS
interface UserRepository {
  findByEmail(email: Email): Promise<User | null>;
  save(user: User): Promise<void>;
}

interface NotificationService {
  sendWelcomeEmail(user: User): Promise<void>;
}
```

Notice: the ports are defined in the core. The core owns its own API surface.

### Step 2: Realistic Version — Use Case, Adapters, Wiring

The use case orchestrates domain logic using ports. It depends only on domain objects
and port interfaces — never on infrastructure:

```typescript
class RegisterUserInteractor implements RegisterUserUseCase {
  constructor(
    private repo: UserRepository,
    private notifier: NotificationService,
  ) {}

  async execute(email: string, password: string): Promise<Result<User>> {
    const emailVo = Email.create(email);          // domain rule
    if (!emailVo) return { ok: false, error: "Invalid email" };

    const existing = await this.repo.findByEmail(emailVo);  // port call
    if (existing) return { ok: false, error: "Email already registered" };

    const hashed = await hashPassword(password);  // pure function
    const user = new User(uuid(), emailVo, hashed);
    await this.repo.save(user);                    // port call
    await this.notifier.sendWelcomeEmail(user);   // port call
    return { ok: true, value: user };
  }
}
```

Now the adapters — each implements a port interface and contains all infrastructure
concerns:

```typescript
class PostgresUserRepository implements UserRepository {
  constructor(private db: Database) {}
  async findByEmail(email: Email): Promise<User | null> {
    const row = await this.db.query("SELECT ...", [email.value]);
    return row ? mapToUser(row) : null;
  }
  async save(user: User): Promise<void> {
    await this.db.query("INSERT INTO users ...", [user.id, user.email.value, ...]);
  }
}

class SendGridNotifier implements NotificationService {
  async sendWelcomeEmail(user: User): Promise<void> {
    await sendgrid.send({ to: user.email.value, template: "welcome" });
  }
}
```

For tests, swap in trivial fakes:

```typescript
class InMemoryUserRepository implements UserRepository {
  private users = new Map<string, User>();
  async findByEmail(email: Email): Promise<User | null> { ... }
  async save(user: User): Promise<void> { this.users.set(user.id, user); }
}

class ConsoleNotifier implements NotificationService {
  async sendWelcomeEmail(user: User): Promise<void> {
    console.log(`Welcome, ${user.email.value}!`);
  }
}
```

Wiring in `main()`:

```typescript
const repo = new InMemoryUserRepository();   // swap for PostgresUserRepository in prod
const notifier = new ConsoleNotifier();       // swap for SendGridNotifier in prod
const useCase = new RegisterUserInteractor(repo, notifier);
```

Same use case. Same domain. Zero changes when you swap adapters. This is the power of
the dependency rule.

## Use It

### Spring Boot (Java/Kotlin)

Spring Boot naturally supports hexagonal architecture when you follow the conventions:

- `domain/` package: entities, value objects, port interfaces. No Spring annotations.
  No `@Entity`. No `@Service`. Pure Java/Kotlin with no framework imports.
- `application/` package: use-case interactors. Can use `@Service` but must depend only
  on domain interfaces.
- `adapters/in/` package: `@RestController` classes — driving adapters.
- `adapters/out/persistence/` package: `@Repository` implementations — driven adapters.
- `adapters/out/notification/` package: email/SMS implementations.

The critical discipline: the `domain` Gradle/Maven module must have **zero** Spring
dependencies. Check with a build script that `domain/build.gradle` has no `implementation`
lines referencing Spring.

Reference: Spring's "Testing Spring Boot Applications" guide demonstrates this pattern
by injecting `@TestConfiguration` fakes. The Spring team explicitly recommends structuring
applications so "the core domain logic has no dependency on Spring."

### Go Microservices

Go's structural typing makes hexagonal architecture almost frictionless:

```go
// domain/user.go — no imports from any adapter package
type UserRepository interface {
    FindByEmail(ctx context.Context, email string) (*User, error)
    Save(ctx context.Context, user *User) error
}

// adapters/postgres/user_repo.go
type PostgresUserRepository struct { db *sql.DB }
func (r *PostgresUserRepository) FindByEmail(...) (*User, error) { ... }
func (r *PostgresUserRepository) Save(...) error { ... }
```

No `implements` keyword needed — `PostgresUserRepository` satisfies `UserRepository`
implicitly. The `domain` package never imports `adapters/postgres`. The `main` package
wires them together:

```go
repo := postgres.NewUserRepository(db)
notifier := sendgrid.NewNotifier(apiKey)
useCase := user.NewRegisterUseCase(repo, notifier)
handler := http.NewRegisterHandler(useCase)
```

### What Production Does That Our Example Doesn't

1. **Error handling strategies** — Production adapters retry on transient failures,
   implement circuit breakers, and use backoff. Our example uses simple `throw`.
2. **Transaction boundaries** — Production use cases wrap multi-port operations in
   transactions (unit of work pattern). Our example saves then notifies without
   transactional safety.
3. **Configuration injection** — Production systems inject connection strings, API keys,
   and feature flags through adapter configuration. Our example hard-codes values.
4. **Observability** — Production adapters emit metrics, traces, and structured logs for
   every port call. Our example is silent.

## Read the Source

- **Spring PetClinic** (`spring-projects/spring-petclinic` on GitHub) — a reference Spring
  application that demonstrates repository abstraction and service-layer dependency
  injection. Look at how `OwnerRepository` is an interface with a JDBC implementation.

- **Go kit** (`go-kit/kit` on GitHub) — a microservices toolkit that enforces
  hexagonal architecture by design. Each service has `endpoint/`, `transport/`, and
  `service/` packages that correspond to ports and adapters.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **architecture_reference.md** — A reference card comparing hexagonal, clean, layered,
  and DDD architectural styles with decision criteria.

## Exercises

1. **Easy** — Implement `InMemoryUserRepository` and `ConsoleNotifier` from scratch
   without looking at the lesson code. Verify by running the use case with your adapters.
2. **Medium** — Add a "change password" use case with a new driving port. Implement a
   `PasswordHasher` driven port that the use case calls. Swap between a real bcrypt
   adapter and a fake that returns a fixed hash.
3. **Hard** — Implement a **transactional outbox** adapter: instead of calling the
   notifier directly, the use case saves a `NotificationOutbox` entity via the repository
   port. A background process reads the outbox and sends emails. This makes the
   notification eventually consistent with the database transaction.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Port | "An interface for I/O" | An interface defined by the core that describes a contract — driving ports define what the app *does*, driven ports define what the app *needs* |
| Adapter | "An implementation" | A class/module that implements a port interface and connects it to a specific technology (PostgreSQL, SendGrid, Express) |
| Dependency Rule | "Dependencies go inward" | Code in an inner circle cannot import anything from an outer circle — enforced by module boundaries, not convention |
| Driving Port | "Input port" | A port that the outside world calls to *drive* the application — the application's API |
| Driven Port | "Output port" | A port that the application calls to be *driven* by an external system — the application's SPI |
| Hexagon | "The core" | The application core: entities, use cases, and port interfaces. Independent of all frameworks and infrastructure. |
| Interactor | "Use case" | A class that implements a driving port and orchestrates domain logic using driven ports |

## Further Reading

- Alistair Cockburn, "Hexagonal Architecture" (original 2005 article, updated 2017)
- Robert C. Martin, "Clean Architecture" (2017 book, Prentice Hall)
- Steve Freeman & Nat Pryce, "Growing Object-Oriented Software, Guided by Tests" (2009) — Chapter 11 on "Test-Driven Design" shows how testability drives port/adapter boundaries
- Herberto Graça, "Hexagonal Architecture: What Is It? Why Should You Use It?" (2020 blog series on ports and adapters)
- Kenneth Lange, "Spring into Hexagonal Architecture" (2020) — pragmatic guide to applying hexagonal in Spring Boot