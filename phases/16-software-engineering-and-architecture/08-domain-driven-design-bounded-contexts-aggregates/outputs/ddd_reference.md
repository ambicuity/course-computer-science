# DDD Reference Card

## Strategic Patterns

| Pattern | Purpose | When to Use |
|---------|---------|-------------|
| **Bounded Context** | Define an explicit boundary where one model and ubiquitous language apply consistently | Always — carve the domain into coherent models |
| **Ubiquitous Language** | Ensure code and conversation use the same terms with the same meanings | Always within each context |
| **Context Map** | Show how bounded contexts relate and translate at boundaries | Always when you have >1 context |
| **Anti-Corruption Layer** | Translate upstream models to protect downstream model integrity | When integrating with models you don't control (legacy, external) |
| **Conformist** | Downstream team adopts upstream model as-is | When upstream is stable and downstream has no leverage |
| **Open-Host Service** | Upstream publishes a well-defined protocol for many consumers | When one context serves many downstream contexts |
| **Published Language** | Use a shared interchange format (JSON schema, EDI) | When contexts need a standard communication format |
| **Shared Kernel** | A small subset of model shared explicitly between contexts | Rarely — only when a subset genuinely benefits both |
| **Customer/Supplier** | Downstream can influence upstream contracts (contract tests) | When downstream teams have negotiating power |
| **Separate Ways** | No integration — teams go independent | When integration cost exceeds benefit |

## Tactical Patterns

| Pattern | Role | Key Rule |
|---------|------|----------|
| **Aggregate** | Transactional consistency boundary | One transaction = one aggregate |
| **Aggregate Root** | Entry point that enforces invariants | All mutations go through the root |
| **Entity** | Object with identity that persists across state changes | Identity matters — same data + different ID = different entity |
| **Value Object** | Immutable object compared by attributes | Prefer over entities; no identity, no lifecycle |
| **Domain Event** | Record of something that happened in the domain | Named in past tense; immutable; enables eventual consistency |
| **Repository** | Collection-like interface for persisting aggregates | Works with aggregate roots only; returns reconstituted domain objects |
| **Factory** | Encapsulates complex creation logic | Delegates invariant enforcement to the aggregate root |

## Aggregate Design Rules

1. **One transaction = one aggregate.** Never modify two aggregates in one transaction. Use domain events for cross-aggregate consistency.
2. **Reference other aggregates by identity, not reference.** Hold `orderId`, not `Order` objects, across boundaries.
3. **Keep aggregates small.** Minimize the number of entities within a single aggregate to reduce contention.
4. **Enforce invariants at the root.** The aggregate root validates all business rules before allowing any mutation.
5. **Load and save entire aggregates.** Repositories work with whole aggregates, not individual entities.

## Entity vs Value Object

| Aspect | Entity | Value Object |
|--------|--------|--------------|
| Identity | Has distinct ID | No identity — equality by value |
| Mutability | Mutable (state changes, identity stays) | Immutable (create new instead of mutate) |
| Lifecycle | Created, modified, archived | No lifecycle — just exists as a value |
| Example | `OrderLine(id=42, ...)` | `Money(amount=9.99, currency="USD")` |
| Guideline | Use when identity matters | Prefer — simpler, safer, no side effects |

## Domain Events

- **Immutable** — represent something that happened
- **Past tense naming** — `OrderPlaced`, not `PlaceOrder`
- **Carry minimum data** — listeners act without querying back
- **Published by aggregate roots** — after successful state change
- **Enable eventual consistency** — between aggregates and across contexts

## Repository Rules

- Works with **aggregate roots only** (never `OrderLineRepository`)
- Returns **fully reconstituted** domain objects (not DTOs)
- Provides **collection-like** interface: `add()`, `getById()`, `remove()`
- Hides all **persistence details** behind the interface

## Anti-Corruption Layer

```
Upstream Context ──> ACL (translates) ──> Downstream Context
   (legacy schema)      (adapter)         (clean domain model)
```

- **Receives** upstream data in its format
- **Translates** into downstream context's model
- **Isolates** downstream from upstream changes
- Only the ACL needs updating when upstream schema changes

## When DDD Is Overkill vs When It Shines

| Overkill | Shines |
|----------|--------|
| Simple CRUD apps | Complex business logic with many rules and edge cases |
| Single team, single model | Multiple teams with overlapping concepts |
| Prototypes and throwaway code | Long-lived systems that must evolve |
| Data storage and retrieval only | Integration boundaries with external/legacy systems |
| Few business invariants | Ubiquitous language alignment is critical to success |