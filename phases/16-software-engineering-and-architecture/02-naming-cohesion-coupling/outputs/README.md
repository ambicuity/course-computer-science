# Naming, Cohesion, Coupling — Reference Card

## Naming Rules

| Rule | Good | Bad |
|---|---|---|
| Reveal intent | `days_since_last_payment` | `d` |
| Pronounceable | `customer_orders` | `custOrds` |
| Searchable | `MAX_RETRIES` | `7` (magic number) |
| Consistent | `customer_id` everywhere | `cust_id`, `clientId`, `uid` |
| No type in name | `name` | `name_string` |
| No abbreviations | `manager` | `mgr` |
| No noise words | `customer` | `customer_info` |
| No comment needed | `elapsed_time_in_seconds = 3.6` | `ets = 3.6  # elapsed time in seconds` |

**Acceptable single letters:** loop counters (`i`, `j`), trivial lambdas (`x => x.id`), coordinates (`x`, `y`).

## Cohesion Levels (High → Low)

| Level | Description | Example |
|---|---|---|
| **Functional** | One clear task, all elements contribute | `sqrt(x)`, `PricingService` |
| **Sequential** | Output of one step feeds the next | `parse → validate → transform` |
| **Communicational** | All elements operate on same data | `print_report` + `export_csv` on `order_data` |
| **Procedural** | Ordered execution, no shared data | `init_db(); start_server()` |
| **Temporal** | Grouped by timing | `on_startup()` |
| **Logical** | Related by category, different behavior | `handle_report("pdf")` with switch |
| **Coincidental** | No meaningful relationship | `utils.py` — junk drawer |

**Refactor toward high cohesion:** Extract Method, Extract Class, Move Method.

## Coupling Levels (Tight → Loose)

| Level | Description | Example |
|---|---|---|
| **Content** | Direct access to internals | Modifying another class's private fields |
| **Common** | Shared global data | Global `config` dict mutated everywhere |
| **Control** | Flags controlling flow | `process(action="create")` with switch |
| **Stamp** | Shared data structure, partial use | Passing full `User` when only `user_id` needed |
| **Data** | Simple parameters | `calculate_total(items: list[LineItem])` |

**Decoupling techniques:**
- **Dependency Injection** — Pass dependencies in; don't create them inside.
- **Interfaces / Protocols** — Depend on abstractions, not concrete types.
- **Events / Pub-Sub** — Emit events; consumers subscribe. No direct references.
- **Inversion of Control** — The framework calls you.

## Cohesion / Coupling Matrix

```
                    Low Coupling         High Coupling
                 ┌─────────────────�┬─────────────────┐
  High Cohesion  │     ★ IDEAL ★   │  Fragile but    │
                 │  Focused + free  │  focused        │
                 ├─────────────────┼─────────────────┤
  Low Cohesion   │  Scattered but   │    ✗ WORST ✗    │
                 │  independent     │  Scattered &     │
                 │                  │  tangled         │
                 └─────────────────┴─────────────────┘
```

**Goal: High Cohesion + Low Coupling**

## Metrics

### LCOM (Lack of Cohesion of Methods)

```
LCOM = max(0, m - q)

m = pairs that do NOT share instance variables
q = pairs that DO share instance variables
```

- LCOM = 0 → High cohesion
- LCOM > 0 → Consider splitting the class

### Instability

```
I = Ce / (Ca + Ce)

Ca = afferent coupling (incoming dependencies)
Ce = efferent coupling (outgoing dependencies)
```

- I ≈ 0 → Stable (many depend on it; change carefully)
- I ≈ 1 → Unstable (few depend on it; easy to change)

**Rule:** Abstract modules should be stable (I ≈ 0). Concrete modules should be unstable (I ≈ 1).

### Quick Diagnostic

| Metric | Healthy | Warning | Critical |
|---|---|---|---|
| LCOM | 0 | 1–5 | > 5 |
| Instability (abstractions) | < 0.3 | 0.3–0.7 | > 0.7 |
| Instability (concrete) | > 0.7 | 0.3–0.7 | < 0.3 |