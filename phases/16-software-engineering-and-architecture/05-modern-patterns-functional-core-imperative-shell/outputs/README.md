# Functional Core / Imperative Shell — Reference Card

## The Pattern in One Sentence

**All decisions in pure functions; all effects in a thin shell.**

## Data Flow

```
  Input ──→ Shell ──→ Core ──→ Shell ──→ Output
              │                    │          │
           (read)              (pure)     (write)
           (I/O)             (decide)    (I/O)
```

1. **Shell** reads input (HTTP request, database row, file).
2. **Shell** passes plain data into the **Core**.
3. **Core** returns a **Decision** (pure data, no side effects).
4. **Shell** interprets the Decision and executes effects.

## The Decision Type

```typescript
type Decision =
  | { kind: "accept"; pricedOrder: PricedOrder }
  | { kind: "reject"; error: ValidationError };
```

```rust
enum OrderDecision {
    Accept(PricedOrder),
    Reject(ValidationError),
}
```

The core never says _"save to the database."_ It says _"the order should be accepted
with this pricing."_ The shell decides whether and how to persist that decision.

## Core Rules

| Rule | Meaning |
|------|---------|
| **No I/O** | No database, HTTP, file system, console, or `async`. |
| **No global state** | All data comes through function arguments. |
| **Deterministic** | Same inputs → same outputs, always. |
| **Referentially transparent** | Any call can be replaced by its return value. |

## Shell Rules

| Rule | Meaning |
|------|---------|
| **Thin** | Contains only wiring — read, call core, write. |
| **No business logic** | All decisions are delegated to the core. |
| **Interprets decisions** | Maps Decision variants to concrete effects. |
| **Integration-tested** | Tested end-to-end with real infrastructure, not mocked. |

## Testing Strategy

```
┌──────────────────────────────────────────┐
│           Core Tests (many, fast)         │
│                                           │
│  assert(calculate_discount(order, tier)   │
│         === expected_discount)            │
│                                           │
│  • No mocks, no database, no HTTP         │
│  • Property-based testing friendly        │
│  • Run thousands in milliseconds          │
├──────────────────────────────────────────┤
│          Shell Tests (few, slow)           │
│                                           │
│  Start real DB → POST /orders → assert    │
│  response and DB state                    │
│                                           │
│  • Only a handful needed                  │
│  • Test wiring, not logic                 │
└──────────────────────────────────────────┘
```

## FC/IS vs. Other Patterns

| | FC/IS | Hexagonal | Clean Architecture |
|---|---|---|---|
| **Core constraint** | Pure — no side effects | Abstract — no framework | Independent — no framework |
| **I/O boundary** | Data in, Decision out | Ports (interfaces) | Use case boundaries |
| **Test mocks needed** | **Zero** for core | For port adapters | For gateway interfaces |
| **Key insight** | Decisions ≠ Effects | Domain ≠ Infrastructure | Inner ≠ Outer |

## When to Use

- Any service with business logic (pricing, validation, rules).
- Any codebase where mocks feel like you're testing the mocks, not the logic.
- Any team that wants fast, deterministic unit tests.

## When NOT to Use

- Pure I/O plumbing (CRUD APIs with no logic) — just use the shell.
- Scripts with no testable logic.
- Prototypes where speed of writing > speed of testing.

## Quick Refactor Checklist

1. Find a function that mixes logic + I/O.
2. Identify the pure decision inside it.
3. Extract the decision into a pure function (data in → Decision out).
4. Leave the I/O in the original function (now the shell).
5. Write pure-function tests for the core.
6. Write one integration test for the shell wiring.

## Key Terms

- **Functional Core** — Pure functions. No side effects. Trivially testable.
- **Imperative Shell** — Thin I/O wrapper. Reads inputs, calls core, writes outputs.
- **Decision Type** — Sum type returned by core. Describes what should happen.
- **Referential Transparency** — Same inputs → same outputs. No surprises.
- **Testability Wall** — The boundary where mixed decisions/effects make tests require infrastructure. FC/IS eliminates it.