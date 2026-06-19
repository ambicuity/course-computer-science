# Modern Patterns — Functional Core / Imperative Shell

> Separate your decisions from your effects. Pure logic in the core; side effects at the boundary.

**Type:** Learn
**Languages:** TypeScript, Rust
**Prerequisites:** Phase 16 lessons 01–04
**Time:** ~60 minutes

## Learning Objectives

- Explain why mixing business logic with I/O destroys testability.
- Apply the Functional Core / Imperative Shell pattern to separate decisions from effects.
- Write pure functions for business logic that are trivially unit-testable.
- Build a thin imperative shell that handles database, HTTP, and logging concerns.
- Contrast FC/IS with hexagonal architecture and clean architecture.

## The Problem

You have an order-processing service. It reads from the database, calculates a discount,
validates the order, writes to the database, and sends an HTTP notification — all in one
function. When you try to unit test the discount logic, you need a real database connection.
When you try to test validation, you need an HTTP server. The tests are slow, flaky, and
nobody runs them.

This is the **testability wall**: any code that mixes decisions (pure logic) with effects
(I/O, mutation, network calls) becomes impossible to test in isolation. The testability
wall appears the moment your `calculate_discount` function also opens a database connection.
You can't reason about business logic without dragging in infrastructure.

Object-oriented programming makes this worse, not better. In classic OOP, objects bundle
state and behavior. The behavior (methods) often reaches out to databases, file systems,
or network services through implicit dependencies. You need dependency injection, mocks,
and frameworks **just to test a pricing rule**.

## The Concept

### Decisions vs. Effects

Every program does two fundamentally different things:

1. **Decisions** — pure computations: "given this order, what's the discount?" These are
   deterministic functions of their inputs. Same inputs, same outputs. Always.
2. **Effects** — side effects: "write this to the database," "send this email," "log this
   event." These interact with the outside world and are never deterministic.

The Functional Core / Imperative Shell pattern (FC/IS) says: **put all decisions in pure
functions (the core), and all effects in a thin wrapper (the shell).**

```
┌─────────────────────────────────────────────┐
│                 Shell (I/O)                  │
│  ┌─────────┐  ┌──────────┐  ┌───────────┐  │
│  │ Database │  │ HTTP      │  │ Logging   │  │
│  └────┬─────┘  └─────┬────┘  └─────┬─────┘  │
│       │              │              │         │
│  ┌────▼──────────────▼──────────────▼─────┐  │
│  │          Orchestration Layer           │  │
│  │   (calls core, feeds results to I/O)  │  │
│  └────────────────┬──────────────────────┘  │
│                   │ pure data in/out         │
│  ┌────────────────▼──────────────────────┐  │
│  │           Functional Core              │  │
│  │  validate  │ calculate  │ decide      │  │
│  │  order      │ discount   │ approval    │  │
│  └────────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

### Gary Bernhardt's Talk

Gary Bernhardt crystallized this pattern in his 2012 talk "Boundaries" (and later in
his "Functional Core, Imperative Shell" screencast series). His key insight:

> "Make all the decisions in pure functions. Then, in your imperative shell, execute
> those decisions against the outside world."

Bernhardt observed that Rails applications (and most OOP codebases) interleave decisions
and effects. A controller reads params, validates, queries the database, computes a
result, writes the result, and renders a template — all in one method. Testing this
requires spinning up the entire framework.

The fix is not "more dependency injection" or "better mocks." The fix is structural:
pull the decisions out into functions that take data in and return data out, with no
side effects whatsoever.

### Why OOP Alone Fails for Testability

OOP's core tenet — encapsulation of state + behavior — creates a bias toward objects
that own their data and the operations on it. When those operations need infrastructure
(a repository, a message queue), the object either:

- **Couples directly** to the infrastructure (untestable without the real service).
- **Accepts an interface** (dependency injection), which requires mocks in tests.
  Mocks are a code smell: they test that your code calls what you told it to call,
  not that it produces correct results.
- **Returns a "command" object** — which is essentially the functional core pattern,
  just with more ceremony.

The third option is the only one that scales. But most OOP practitioners reach for
the first two, because the third feels "not object-oriented."

### How FC/IS Differs from Hexagonal / Clean Architecture

All three patterns push I/O to the boundary, but they differ in **what they
protect and how**:

| Aspect | Functional Core / Imperative Shell | Hexagonal Architecture | Clean Architecture |
|--------|------------------------------------|------------------------|--------------------|
| Core principle | Pure functions, no side effects | Ports and adapters | Dependency rule (inward) |
| Boundary mechanism | Data in → data out (functions) | Interfaces (ports) | Interfaces (use cases) |
| Dependency direction | Shell depends on core, never reverse | Adapters depend on ports | Outer depends on inner |
| Enforced constraint | Referential transparency | Port abstraction | Dependency inversion |
| Testing story | Unit test core with plain asserts | Mock adapters at ports | Mock gateways at use case boundaries |
| Purity | Core is **pure** — no I/O, no mocks needed | Core is **clean** — no framework, but may have side effects | Core is **independent** — no framework, but may have side effects |

The critical difference: hexagonal and clean architecture allow side effects inside the
core as long as they go through an abstract interface. FC/IS forbids side effects in
the core entirely. The core is **referentially transparent** — you can replace any
function call with its return value and nothing changes.

This is a stronger guarantee. In hexagonal architecture, your domain service might
call `userRepository.findById()`. That's a side effect. You need a mock. In FC/IS,
the shell calls `userRepository.findById()`, passes the result into a pure function,
and the pure function returns a decision. **No mocks needed. Ever.**

### Testing the Functional Core

Testing a pure function is trivial:

```python
assert calculate_discount(Order(total=100, tier="gold")) == Discount(amount=15, reason="gold_tier")
```

No test database. No mocks. No fixtures. No framework. Just inputs and expected outputs.

You can:
- Run thousands of property-based tests in milliseconds.
- Generate random inputs and check invariants (e.g., "discount never exceeds total").
- Replay production inputs through the core to verify behavior without risk.

### Testing the Shell

The shell is thin by design — it contains only wiring code: "read this, call that pure
function, write the result there." You don't unit-test the shell. Instead, you:

- **Integration-test** the full pipeline with a real (or containerized) database and
  HTTP server.
- ** smoke-test** that the shell correctly passes data between its dependencies and
  the core.

Because the shell contains no business logic, integration tests for it are few and
slow — but that's fine. The bulk of your tests (hundreds, thousands) run against the
pure core in milliseconds.

### The Decision / Effect Protocol

The communication protocol between core and shell is plain data:

1. The shell gathers input (read database, parse HTTP request).
2. The shell calls a pure function with that data: `core::process(order) → Decision`.
3. The shell interprets the `Decision`: "the core says `ChargeCustomer(100)`, so I'll
   call `paymentGateway.charge(100)`."

The `Decision` type is an enum or sum type. Each variant is an instruction the shell
must carry out. The core never says "charge the customer" — it says "the customer
should be charged $100." The shell decides **whether and how** to execute that.

## Build It

### Step 1: Minimal Version (Rust)

We'll build an order processing system. Start with the core — pure functions only.

```rust
// Pure core: no I/O, no database, no network
fn validate_order(order: &Order) -> Result<ValidatedOrder, ValidationError> {
    if order.items.is_empty() {
        return Err(ValidationError::EmptyOrder);
    }
    if order.customer_id.is_empty() {
        return Err(ValidationError::MissingCustomerId);
    }
    Ok(ValidatedOrder { order: order.clone() })
}

fn calculate_discount(order: &ValidatedOrder, tier: CustomerTier) -> Discount {
    let base = order.order.total();
    match tier {
        CustomerTier::Gold => Discount::new(base * 0.15, "gold_tier"),
        CustomerTier::Silver => Discount::new(base * 0.10, "silver_tier"),
        CustomerTier::Bronze => Discount::new(base * 0.05, "bronze_tier"),
    }
}
```

No database. No `unwrap`. No side effects. Just data in, data out.

### Step 2: Realistic Version (Rust)

Now add the shell — a thin layer that reads from the database, calls the core, and
persists results. See `code/main.rs` for the full implementation.

### Step 3: TypeScript Version

See `code/main.ts` for the same pattern in TypeScript, demonstrating that FC/IS
is language-agnostic.

## Use It

### Real-World Examples

**Rust ecosystem**: The pattern is idiomatic in Rust. The standard library's
`std::io::BufRead` trait separates the decision of what to read from the effect
of reading. Serde separates the decision of how to serialize from the effect of
writing bytes. Community crates like `sqlx` encourage "query, then process" rather
than "process inside the query."

**TypeScript / Node.js**: The `fp-ts` library encodes this pattern explicitly with
`TaskEither`. Domain logic returns `Either<Error, Decision>`, and the shell
interprets the `Task` (async effect). The popular `effect` framework (Effect-TS)
is built on this exact principle.

**Erlang/OTP**: Erlang's "let it crash" philosophy is FC/IS at the process level.
The business logic lives in pure functions; the OTP supervisors (shell) handle
restarts and side effects.

### Production Codebase Pointer

Look at how Amazon's Lambda runtime separates the handler (your pure function) from
the runtime loop (the shell). The runtime reads the event, calls your handler, and
writes the response. Your handler is the functional core.

## Read the Source

- **Rust `std::io`** — the Read/Write trait hierarchy is itself an FC/IS boundary:
  pure transformation (core) vs. I/O operations (shell).
- **Erlang/OTP `gen_server`** — the callback module is a functional core; the
  `gen_server` process is the imperative shell.
- **Effect-TS** (`github.com/Effect-TS/effect`) — a TypeScript framework that encodes
  FC/IS as a first-class pattern.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`fcis_reference.md`** — A one-page reference card for the Functional Core /
  Imperative Shell pattern, including the data-flow diagram, the decision type
  protocol, and a testing checklist.

## Exercises

1. **Easy** — Rewrite the Rust `calculate_discount` function as a pure function
   without looking at the lesson code. Test it in isolation with `assert_eq!`.
2. **Medium** — Add an `apply_discount` pure function to the core that takes a
   `ValidatedOrder` and a `Discount`, and returns a `PricedOrder`. Ensure the
   total can never go below zero (invariant). Then wire it through the shell.
3. **Hard** — Refactor an existing OOP service in your codebase to use FC/IS.
   Start by identifying one pure function hiding inside a class method. Extract it.
   Replace its tests (which used mocks) with plain assertions. Measure the
   difference in test runtime and readability.

## Key Terms

| Term | What people say | What it actually means |
|------|-----------------|------------------------|
| Functional Core | "Business logic" | Pure functions that take data in, return decisions out. No side effects. No dependencies. |
| Imperative Shell | "I/O layer" or "infrastructure" | A thin wrapper that reads inputs, calls the core, and executes the core's decisions as side effects. |
| Decision Type | "Command object" or "effect type" | A sum type representing what the core wants the shell to do. The core returns decisions; the shell executes them. |
| Referential Transparency | "Pure function" | You can replace any function call with its return value without changing the program's behavior. |
| Testability Wall | "Hard to test" | The boundary where mixed decisions and effects make unit tests require infrastructure. FC/IS eliminates this wall. |

## Further Reading

- Gary Bernhardt, "Functional Core, Imperative Shell" (screencast series, 2013)
- Gary Bernhardt, "Boundaries" (Ruby Conf 2012 talk)
- Mark Seemann, "Pure Functions" blog series (ploeh.dk)
- Scott Wlaschin, "Functional Programming Patterns" (F# for Fun and Profit)
- Amdahl's Law applied to test suites: your test suite is only as fast as your slowest integration test. FC/IS ensures most tests are unit tests.
- Effect-TS documentation: `effect.website` — a production TypeScript framework built on FC/IS principles