# Domain-Driven Design — Bounded Contexts, Aggregates

> How to model complex software so the code speaks the language of the business — and stays correct as it grows.

**Type:** Learn
**Languages:** TypeScript, Python
**Prerequisites:** Phase 16 lessons 01–07
**Time:** ~90 minutes

## Learning Objectives

- Explain why Domain-Driven Design is a *philosophy of modeling*, not just a pattern catalog.
- Define and apply **ubiquitous language** so code and conversation share the same vocabulary.
- Draw **bounded context** boundaries and explain context maps, anti-corruption layers, and inter-context relationships.
- Design **aggregates** as transactional consistency boundaries with roots, invariants, and entity/value-object distinctions.
- Identify when DDD is overkill and when it is essential.

## The Problem

You join a company where "Order" means three different things depending on who you ask. The billing team's `Order` carries tax metadata. The fulfillment team's `Order` tracks shipping zones. The catalog team uses `Order` to mean a sort column. Every team models the same word differently, yet all three models live in one database, one codebase, one deployment. A change to shipping logic breaks billing calculations. A billing refactor corrupts fulfillment state. Nobody can reason about the system because the *language* is ambiguous and the *boundaries* are missing.

This is the problem Domain-Driven Design solves: **how to carve a complex domain into coherent pieces where each piece has a single, internally consistent model and a shared vocabulary.**

## What DDD Actually Is

DDD is not a collection of design patterns. It is a **philosophy of modeling** built on one premise: *the heart of software is the domain, and the complexity of software comes from the complexity of the domain itself.* Technical architecture matters only insofar as it serves the domain model.

Eric Evans introduced DDD in 2003 to counter two widespread failures:

1. **Anemic models** — Objects that are bags of getters and setters with no behavior; all logic lives in service classes, and the model says nothing about the business.
2. **Big-ball-of-mud integration** — Systems where every subsystem reaches into every other subsystem's data, so any change can break anything anywhere.

DDD addresses these with two toolkits: **strategic** (how to partition the problem) and **tactical** (how to model inside a partition). Strategy comes first. If you get the boundaries wrong, perfect tactical patterns won't save you.

## Ubiquitous Language

The **ubiquitous language** is a shared, rigorous vocabulary used by developers, domain experts, and stakeholders — expressed identically in code, tests, diagrams, and conversation.

### Why It Matters

When a developer says "line item" but the business says "order line," misalignment creeps in. The code drifts from the domain. Requirements get lost in translation. The ubiquitous language eliminates that drift by insisting:

- The *same term* means the *same thing* everywhere.
- If two teams use the same term differently, they are in **different bounded contexts** and must disambiguate.
- Code identifiers (class names, method names, variables) *are* the language — not translations of it.

### How to Build One

1. **Talk to domain experts.** Record the nouns and verbs they use to describe processes. A shipping clerk says "dispatch consignment," not "update order status to SHIPPED."
2. **Codify in code.** Name your aggregates, entities, and methods after those exact words: `DispatchConsignmentCommand`, `Consignment` aggregate.
3. **Refuse synonyms.** If "customer" and "client" both appear, pick one and delete the other from the codebase.
4. **Evolve.** As understanding deepens, rename relentlessly. If a concept's name no longer fits, rename the class, the tests, the docs.

## Bounded Contexts

A **bounded context** is an explicit boundary within which a particular model applies consistently. Outside that boundary, the same real-world concept may be modeled differently — and that is *correct*, not a mistake.

### The Core Idea

Imagine a retail company. The concept of a "Product" exists in multiple contexts:

| Context | Product means | Key attributes | Operations |
|---------|---------------|----------------|------------|
| Catalog | Something to browse | Name, photos, description, category | Search, filter, display |
| Inventory | Something to stock-keep | SKU, quantity on hand, reorder threshold | Receive, allocate, adjust |
| Pricing | Something to price | Base price, discounts, currency rules | Calculate, apply promotion |

If you try to build one `Product` class with all attributes, you get a god object. Every change touches every team. The solution: let each context own its own `Product` model, bounded by an explicit perimeter.

### Defining a Bounded Context

A bounded context is defined by:

1. **A name** — "Order Processing," "Billing," "Inventory."
2. **A ubiquitous language** — terms have exactly one meaning within this context.
3. **A model** — the set of types, relationships, and rules that are internally consistent.
4. **A boundary** — the code, database schemas, APIs, and team ownership that delimit the context.

In practice, a bounded context often maps to:

- A microservice or a module within a monolith
- A separate database schema or a distinct subset of tables
- A team with clear ownership

### Context Maps

When bounded contexts need to interact, you draw a **context map** — a diagram showing the relationships between contexts. The relationships define how models translate at the boundaries.

Key relationship patterns:

| Pattern | Meaning | Example |
|---------|---------|---------|
| **Upstream/Downstream** | One context depends on another | Billing (downstream) depends on Catalog (upstream) for product names |
| **Conformist** | Downstream team adopts the upstream model as-is | A small reporting tool directly uses the Order context's data schema |
| **Anti-Corruption Layer (ACL)** | Downstream team translates upstream models to protect its own model | A legacy billing system's "Charge" data passes through an ACL before entering the modern Pricing context |
| **Open-Host Service** | Upstream publishes a well-defined protocol for many downstream consumers | The Catalog context exposes a REST API and event streams for any context needing product info |
| **Published Language** | A shared interchange format (often a standard) | Two contexts exchange order data using a predefined JSON schema or EDI format |
| **Shared Kernel** | A small subset of model shared explicitly between contexts | Both Order and Billing contexts share a `Money` value object with currency and amount |
| **Customer/Supplier** | Downstream team has influence over upstream's API | The Fulfillment context negotiates contract tests with the Order context |
| **Separate Ways** | No integration; teams go their own way | The internal tooling context doesn't interact with the customer-facing context at all |

### Anti-Corruption Layer in Depth

An ACL is one of the most powerful strategic patterns. It is a **translation boundary** that prevents an upstream model from polluting a downstream context.

```
Upstream Context          ACL (Translation)          Downstream Context
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│ LegacyOrder       │───>│ LegacyOrderAdapter│───>│ Order (clean)     │
│ - order_num       │    │ toOrder()         │    │ - id              │
│ - cust_ref        │    │                   │    │ - customer        │
│ - line_items_str  │    │                   │    │ - lines: List     │
└──────────────────┘    └──────────────────┘    └──────────────────┘
```

The ACL:
- **Receives** data in the upstream format.
- **Translates** it into the downstream context's model.
- **Isolates** downstream code from upstream changes. If the upstream schema changes, only the ACL needs updating.

## Aggregates

An **aggregate** is a cluster of domain objects treated as a single unit for data changes. Every aggregate has:

1. **An aggregate root** — the entry point and only object that external references can point to.
2. **A boundary** — defines which objects belong to the aggregate and must be kept consistent together.
3. **Invariants** — business rules that must always be true, enforced at the root.

### Why Aggregates Exist

Without aggregates, every object can reference every other object. A `Customer` references an `OrderLine` directly. Someone changes the line's quantity without telling the order. Consistency is violated because there is no single point of control.

Aggregates solve this by declaring: **all changes to objects within an aggregate must go through the aggregate root.** The root enforces invariants. External objects may hold a reference to the root's identity but never to internal entities.

### Aggregate Rules

1. **One transaction = one aggregate.** Never modify two aggregates in the same transaction. If you need cross-aggregate consistency, use domain events and eventual consistency.
2. **Reference other aggregates by identity, not by reference.** Hold `order_id`, not `Order` object references, across aggregate boundaries.
3. **Keep aggregates small.** An aggregate whose transactional boundary spans dozens of entities will serialize access and create contention. Aim for the smallest consistent boundary.

### Aggregate Roots and Invariants

An **aggregate root** is the gatekeeper. Consider an `Order`:

```python
class Order:
    def add_line(self, product_id, quantity, unit_price):
        # Invariant: an order cannot have duplicate lines for the same product
        if any(line.product_id == product_id for line in self._lines):
            raise DuplicateProductError(product_id)
        # Invariant: quantity must be positive
        if quantity <= 0:
            raise InvalidQuantityError(quantity)
        # Invariant: unit price must be non-negative
        if unit_price.amount < 0:
            raise InvalidPriceError(unit_price)
        line = OrderLine(self._next_line_id(), product_id, quantity, unit_price)
        self._lines.append(line)
        self._recalculate_total()
```

All invariant checks happen *before* the mutation. The root never lets the aggregate enter an invalid state.

### Entity vs. Value Object

Within an aggregate, objects are either **entities** or **value objects**:

| Aspect | Entity | Value Object |
|--------|--------|--------------|
| Identity | Has a distinct identity (ID) that persists across state changes | No identity; equality is based on attribute values |
| Mutability | Mutable — state can change, but identity stays the same | Immutable — create a new instance instead of changing |
| Lifecycle | Has a lifecycle — created, modified, archived | No lifecycle — just exists as a value |
| Example | `OrderLine(id=42, product="Widget", qty=3)` | `Money(amount=9.99, currency="USD")` |
| Comparison | Two entities with same attributes but different IDs are *different* | Two value objects with same attributes are *equal* |

**Prefer value objects over entities.** An entity introduces identity complexity. If you can model something as a value object — immutable, compared by value — do it. `Address` should be a value object. `Customer` should be an entity.

## Domain Events

A **domain event** is something that happened in the domain that other contexts or aggregates may care about. Events are the mechanism for **eventual consistency across aggregates and across bounded contexts**.

```python
class OrderPlaced:
    order_id: str
    customer_id: str
    total: Money
    occurred_at: datetime
```

When an order is placed within the Order aggregate, the root publishes an `OrderPlaced` event. The Inventory context listens and allocates stock. The Billing context listens and generates an invoice. No cross-aggregate transaction required — the event carries the payload each context needs.

Key properties of domain events:

- **Immutable:** They represent something that happened; you cannot change history.
- **Named in the past tense:** `OrderPlaced`, `PaymentReceived`, `InventoryDepleted` — not `PlaceOrder`.
- **Carry the minimum data** listeners need to act without querying back.
- **Published by aggregate roots** after a successful state change, not before.

## Repositories

A **repository** provides the illusion of an in-memory collection of aggregates. It hides persistence details behind a collection-like interface.

```python
class OrderRepository:
    def get_by_id(self, order_id: str) -> Order: ...
    def add(self, order: Order) -> None: ...
    def next_identity(self) -> str: ...
```

The repository:

- Works with **aggregate roots only.** You never have a `OrderLineRepository` — you load the `Order` aggregate and navigate to its lines.
- Returns fully reconstituted aggregates with all invariants intact.
- Handles the object-relational mapping or document mapping behind the scenes.
- Is **not** a DAO. A DAO returns data; a repository returns domain objects with behavior.

## The Strategic Patterns Summary

| Pattern | When to Use |
|---------|-------------|
| **Bounded Context** | Always. Carve the domain into explicit, internally consistent models. |
| **Ubiquitous Language** | Always. Within each context, ensure one term = one meaning. |
| **Context Map** | Always when you have >1 context. Shows how contexts relate. |
| **Anti-Corruption Layer** | When integrating with a model you don't control (legacy, external service). |
| **Conformist** | When upstream model is stable and downstream can adopt it as-is. |
| **Open-Host Service** | When one upstream context serves many downstream consumers. |
| **Published Language** | When contexts need a shared interchange format. |
| **Shared Kernel** | Rarely. When a small subset of model genuinely benefits both contexts. |
| **Customer/Supplier** | When downstream teams can influence upstream APIs. |
| **Separate Ways** | When integration isn't worth the cost. |

## The Tactical Patterns Summary

| Pattern | Role |
|---------|------|
| **Aggregate** | Transactional consistency boundary — the fundamental unit of change. |
| **Aggregate Root** | The entry point of an aggregate; enforces invariants. |
| **Entity** | An object with identity that persists across state changes. |
| **Value Object** | An immutable object compared by attribute value. |
| **Domain Event** | A record of something that happened; enables eventual consistency. |
| **Repository** | Collection-like interface for persisting and retrieving aggregates. |
| **Factory** | Encapsulates complex object creation; delegates to the aggregate root for invariant enforcement. |

## When DDD Is Overkill

DDD adds design overhead. Don't use it when:

- **CRUD applications:** A simple admin panel or data-entry form doesn't need aggregates, domain events, or contexts. A `User` table with a REST endpoint is fine.
- **Simple domains with few business rules:** If the logic is "store and retrieve data," DDD's modeling discipline is unnecessary ceremony.
- **Prototypes and throwaway code:** Modeling deeply for something you'll demo once and discard is waste.
- **Single-team, single-model systems:** If there's genuinely one model and no ambiguity, bounded contexts add no value.

## When DDD Shines

Reach for DDD when:

- **Complex business logic** that experts explain with rules, exceptions, and edge cases. If you can't describe the domain in a single sentence, DDD helps you partition it.
- **Multiple teams** working on overlapping concepts. Bounded contexts give each team ownership.
- **Long-lived systems** that must evolve. A clean model can be extended; a big ball of mud cannot.
- **Integration boundaries** with external systems or legacy code. Anti-corruption layers protect your model.
- **Ubiquitous language alignment** between business and code matters for the project's success.

## Build It

We'll implement an Order Management bounded context with:
- **Value objects:** `Money` and `Address`
- **Entities:** `OrderLine` (has identity within the aggregate)
- **Aggregate root:** `Order` (enforces invariants)
- **Domain events:** `OrderPlaced`
- **Repository:** `OrderRepository` (collection-like interface)
- **Bounded context boundary:** A module-level namespace showing what belongs to this context

### Step 1: Minimal Version

See `code/main.py` for a minimal Python implementation that enforces aggregate invariants and shows the bounded context boundary.

### Step 2: Realistic Version

See `code/main.ts` for a TypeScript implementation with proper types, domain events, and aggregate enforcement that mirrors production patterns.

## Use It

In real systems, the bounded-context pattern maps directly to:

- **Microservices** — Each service owns one bounded context. Teams deploy independently.
- **Modules in a monolith** — Package boundaries (Python packages, TypeScript namespaces) enforce context perimeters.
- **Event-driven architectures** — Domain events flow between contexts via message brokers (Kafka, RabbitMQ, EventBridge).

Production frameworks that codify DDD:

- **NestJS** (TypeScript) — Modules, handlers, and event emitters map naturally to contexts, repositories, and domain events.
- **Domain-driven-design Python libraries** — `dddy`, `domains`, and `event_sourcing` packages provide base classes for aggregates, repositories, and event stores.

## Read the Source

- **Eclipse Store** (formerly MicroStream): `https://github.com/eclipse-store/storage` — Look at how the root object defines the aggregate boundary for persistence.
- **Axon Framework** (Java): `https://github.com/AxonFramework/AxonFramework` — The `AggregateTestFixture` shows how aggregate roots enforce invariants and publish events in a production DDD framework.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`ddd_reference.md`** — A one-page reference card for DDD strategic and tactical patterns, aggregate design rules, and context-mapping patterns.

## Exercises

1. **Easy** — Reimplement the `Order` aggregate from memory without looking at the lesson code. Enforce at least two invariants.
2. **Medium** — Design a second bounded context (e.g., `Inventory`) with its own `Product` entity and an anti-corruption layer that translates `OrderPlaced` events into inventory allocations. Show the context map.
3. **Hard** — Implement event sourcing for the `Order` aggregate: instead of storing current state, store the sequence of domain events (`OrderCreated`, `LineAdded`, `OrderCancelled`) and reconstruct state by replaying them. Ensure invariants are checked at replay time.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Bounded Context | "A module" or "a service" | An explicit boundary within which one model and one ubiquitous language apply consistently |
| Aggregate | "A group of objects" | A transactional consistency boundary — all changes go through the root, all invariants are enforced, one aggregate per transaction |
| Aggregate Root | "The main entity" | The entry point of an aggregate — the only object external references may point to; enforces invariants |
| Ubiquitous Language | "Business terms" | A single, rigorous vocabulary shared by domain experts and code, within a bounded context |
| Value Object | "A struct" | An immutable object with no identity, compared by attribute values |
| Entity | "A model object" | An object with a distinct identity that persists across state changes |
| Domain Event | "A notification" | An immutable record of a domain occurrence, named in the past tense, used for eventual consistency |
| Anti-Corruption Layer | "An adapter" | A translation boundary that protects a downstream context's model from an upstream model it does not control |
| Repository | "A DAO" or "a data access class" | A collection-like interface for aggregates that hides persistence and returns fully reconstituted domain objects |
| Context Map | "An architecture diagram" | A diagram showing the relationships and translation patterns between bounded contexts |

## Further Reading

- **Domain-Driven Design** by Eric Evans (2003) — The original book. Chapters 2–4 cover the core model; Chapter 14 covers maintaining model integrity (contexts, ACLs).
- **Implementing Domain-Driven Design** by Vaughn Vernon (2013) — The practical companion. Part I covers strategic design; Part II covers tactical patterns with Java examples.
- **Domain-Driven Design Distilled** by Vaughn Vernon (2016) — A concise summary focused on the essential patterns.
- **Clean Architecture** by Robert C. Martin (2017) — Chapter 22 discusses how bounded contexts map to clean architecture boundaries.
- **Martin Fowler's "BoundedContext" article** — `https://martinfowler.com/bliki/BoundedContext.html` — A short, precise definition.