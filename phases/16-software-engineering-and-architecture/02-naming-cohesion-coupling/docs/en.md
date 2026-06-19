# Naming, Cohesion, Coupling

> The three pillars of maintainable code: say what you mean, do one thing well, and depend on little.

**Type:** Learn
**Languages:** TypeScript, Python
**Prerequisites:** Phase 16 lesson 01
**Time:** ~45 minutes

## Learning Objectives

- Write intent-revealing, pronounceable, searchable names and recognize common naming anti-patterns.
- Classify cohesion levels (functional through coincidental) and refactor toward high cohesion.
- Classify coupling types (content through data) and apply decoupling techniques (DI, interfaces, IoC).
- Compute simple cohesion metrics (LCOM, afferent/efferent coupling, instability).
- Use the cohesion/coupling matrix to diagnose module health and plan targeted refactors.

## The Problem

You join a team and open `utils.py` — 2,400 lines, 47 functions, imports everywhere. A single change in the database schema breaks the PDF export. A variable called `d` holds a dictionary of customer orders. A class called `Manager` does everything. You can't tell what the code does without reading every line.

This isn't rare — it's the default state of undisciplined code. Naming, cohesion, and coupling are the three diagnostics that tell you *why* code rots and *how* to fix it. They are the foundation of every pattern, principle, and architecture you'll learn later.

## The Concept

### Naming: Code Is Read 10× More Than Written

Names are the primary documentation. A good name answers three questions:

1. **Why does it exist?** (intent-revealing)
2. **What does it do?** (descriptive)
3. **How is it used?** (consistent with the domain)

**Good names** are intent-revealing, pronounceable, and searchable:

```python
# Good — you know what and why
days_since_last_payment = 14
elapsed_time_in_seconds = 3.6
customer_orders = {"alice": 3, "bob": 7}

# Bad — requires a comment, unpronounceable, unsearchable
d = 14  # days since last payment
ets = 3.6
co = {"a": 3, "b": 7}
```

**Bad naming patterns to avoid:**

| Anti-pattern | Example | Why it hurts |
|---|---|---|
| Single-letter names | `d = {}` | Unsearchable, meaningless outside context |
| Abbreviations | `usr_mgr` | Ambiguous, requires mental translation |
| Type in name | `name_string` | Redundant, breaks if type changes |
| Number series | `data1, data2` | No semantic distinction |
| Comments explaining names | `x = 5  # timeout in seconds` | If you need a comment, rename it |
| Noise words | `the_message`, `info_data` | Adds no information |

**When single letters are acceptable:** loop counters (`i`, `j`), lambda parameters in trivial scopes, coordinate pairs (`x`, `y`). The rule: if the scope is ≤ 3 lines and the meaning is obvious, a single letter is fine.

**Consistency is king.** If you call it `customer_id` in one place, don't call it `cust_id`, `clientId`, or `uid` elsewhere. Pick one name and use it everywhere.

### Cohesion: How Focused Is a Module?

Cohesion measures **how strongly the elements of a module belong together**. Larry Constantine defined levels from highest (best) to lowest (worst):

| Level | Description | Example |
|---|---|---|
| **Functional** | All elements contribute to a single well-defined task | `sqrt(x)` — one job, done completely |
| **Sequential** | Output of one element feeds into the next | `parse → validate → transform` pipeline |
| **Communicational** | All elements operate on the same data | `print_report` and `export_csv` both use `order_data` |
| **Procedural** | Elements follow a specific order of execution, but don't share data | `init_db(); start_server()` |
| **Temporal** | Elements are grouped because they happen at the same time | `on_startup()` — init log, init db, load config |
| **Logical** | Elements are logically related but do different things | `print_report(type)` where type switches behavior |
| **Coincidental** | No meaningful relationship | `utils.py` — a junk drawer |

**High cohesion = single responsibility.** A module with functional cohesion can be described in one sentence: "This module does X." If you need "and" or "also," cohesion is too low.

**Refactoring toward high cohesion:**

- **Extract Method:** Move a block of logic into its own function with a name.
- **Extract Class:** When a class has two sets of responsibilities, split it.
- **Move Method:** When a method uses more data from another class than its own, move it there.

### Coupling: How Dependent Are Modules on Each Other?

Coupling measures **how much one module depends on the internal details of another**. From tightest (worst) to loosest (best):

| Level | Description | Example |
|---|---|---|
| **Content** | One module directly modifies another's internal data | Reaching into another class's private fields |
| **Common** | Modules share global data | Global `config` dict mutated by multiple modules |
| **Control** | One module controls the flow of another via flags | `process(action="create")` with switch inside |
| **Stamp** | Modules share a data structure but only use parts of it | Passing a full `User` object when only `user_id` is needed |
| **Data** | Modules communicate through simple parameters | `calculate_total(items: list[LineItem])` |

**Low coupling = modules communicate through well-defined interfaces,** not shared internals. When module A only knows the public interface of module B, you can rewrite B's internals without touching A.

**Decoupling techniques:**

- **Dependency Injection:** Pass dependencies in — don't create them inside.
- **Interfaces / Protocols:** Depend on abstractions, not concrete types.
- **Events / Pub-Sub:** Modules emit events; consumers subscribe. No direct references.
- **Inversion of Control:** The framework calls you; you don't call the framework.

### The Cohesion / Coupling Matrix

```
                    Low Coupling         High Coupling
                 ┌─────────────────�┬─────────────────┐
  High Cohesion  │     IDEAL       │  Fragile but    │
                 │  Focused + free  │  focused        │
                 ├─────────────────┼─────────────────┤
  Low Cohesion   │  Scattered but   │    WORST        │
                 │  independent     │  Scattered and  │
                 │                  │  tangled        │
                 └─────────────────┴─────────────────┘
```

**Goal: high cohesion + low coupling.** This is the design sweet spot. Each module does one thing well, and modules talk through minimal, well-defined interfaces.

### Metrics: Measuring What Matters

**LCOM (Lack of Cohesion of Methods):** For a class with methods M and instance variables V, count the number of method pairs that don't share an instance variable (m) and the number that do (q). LCOM = m - q (if positive; else 0). Higher LCOM = lower cohesion.

**Afferent Coupling (Ca):** How many other modules depend on this one. High Ca = this module is important (and must be stable).

**Efferent Coupling (Ce):** How many other modules does this one depend on. High Ce = this module is fragile (affected by changes elsewhere).

**Instability (I):** I = Ce / (Ca + Ce). Ranges 0 (maximally stable) to 1 (maximally unstable). Abstract modules should be stable (I ≈ 0); concrete modules should be unstable (I ≈ 1) so they can change without breaking dependents.

## Build It

### Step 1: Minimal Version — Bad vs Good Naming, Simple Cohesion Check

```python
# --- BAD NAMING ---
def proc(d):
    # process data
    r = []
    for i in d:
        if i.get("a") == 1:
            r.append(i)
    return r

# --- GOOD NAMING ---
def filter_active_customers(customers: list[dict]) -> list[dict]:
    active_customers = []
    for customer in customers:
        if customer.get("status") == "active":
            active_customers.append(customer)
    return active_customers
```

### Step 2: Realistic Version — Full Refactoring Pipeline

See `code/main.py` for the complete implementation:

1. **Naming refactoring** demo — before/after with measurable improvements (searchability, readability).
2. **Cohesion refactoring** — extracting a God class into focused modules.
3. **Coupling refactoring** — replacing hard-coded dependencies with dependency injection.
4. **Metric computation** — LCOM, afferent/efferent coupling, instability.

## Use It

### Production Examples

**Clean Code (Robert C. Martin):** The definitive source on naming. Chapter 2 ("Meaningful Names") is 30 pages of before/after examples.

**Spring Framework (Java):** Dependency injection as a first-class concept. The entire framework is built around high cohesion + low coupling — every bean declares its dependencies through interfaces.

**Python's `abc` module:** Protocol-based decoupling. Define an `ABC` with `@abstractmethod`, and modules depend on the protocol, not the concrete class.

**TypeScript Interfaces:** The primary mechanism for decoupling. `interface NotificationService` lets you swap email for SMS without touching business logic.

Compare your LCOM calculation against pylint's `R0904` (too-many-public-methods) and `R0914` (too-many-locals) heuristics. These are approximations of cohesion analysis.

## Read the Source

- **Django REST Framework** (`rest_framework/views.py`): The `APIView` class demonstrates high cohesion — it handles exactly one concern (processing an HTTP request into a response) and delegates serialization, authentication, and permissions through injected dependencies.
- **Python `logging` module** (`logging/__init__.py`): A case study in low coupling — loggers, handlers, formatters, and filters are all independently configurable and communicate through the `LogRecord` data structure (data coupling).

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **A reference card** summarizing the cohesion levels, coupling types, naming rules, and metric formulas.

## Exercises

1. **Easy** — Find three badly-named variables in any open-source project. Rename them with intent-revealing names. What changed in readability?
2. **Medium** — Take a class with LCOM > 0 and refactor it into two classes with LCOM = 0. Write a test that proves behavior is preserved.
3. **Hard** — Design a module with afferent coupling > 5 (many depend on it) and instability < 0.3 (stable). What design decisions keep it stable? How would you version it?

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Cohesion | "This class does too much" | How focused a module's responsibilities are; high = one clear purpose |
| Coupling | "These modules are tangled" | How much one module depends on another's internals; low = minimal dependency |
| LCOM | "Cohesion metric" | Lack of Cohesion of Methods — counts method pairs that don't share instance variables |
| Dependency Injection | "Pass it in, don't create it" | Providing dependencies from outside rather than constructing them internally |
| Instability | "How fragile is this?" | Ce / (Ca + Ce) — ratio of outgoing to total dependencies; high = easy to change |
| Afferent Coupling | "How many depend on this" | Count of incoming dependencies; high = this module must be stable |
| Efferent Coupling | "How many does this depend on" | Count of outgoing dependencies; high = this module is fragile |
| IoC | "Don't call us, we'll call you" | Inversion of Control — the framework calls your code, not the other way around |

## Further Reading

- *Clean Code* by Robert C. Martin — Chapters 2–3 on naming and functions
- *Structured Design* by Yourdon & Constantine — Original cohesion/coupling taxonomy
- *Agile Software Development* by Robert C. Martin — Principles, patterns, and practices (SRP, OCP, DIP)
- Martin Fowler's *Refactoring* — Extract Method, Extract Class, Move Method catalogs
- *Domain-Driven Design* by Eric Evans — The Ubiquitous Language (naming as domain alignment)