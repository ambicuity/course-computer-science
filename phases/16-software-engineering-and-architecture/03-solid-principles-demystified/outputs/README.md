# SOLID Principles — Quick Reference Card

## S — Single Responsibility
- **One reason to change** per module
- **Smell:** God class, merging conflicts, test setup requires unrelated state
- **Fix:** Extract classes until each has one actor who requests changes

```
Before: Employee does pay + DB + email + reports
After:  PayCalculator | EmployeeRepository | ReportFormatter | Notifier
```

## O — Open/Closed
- **Open for extension, closed for modification**
- **Smell:** Switch/case or if/elif chain that grows with each new variant
- **Fix:** Strategy pattern — add new behavior as a new class, no edits to existing code

```
Before: if type == "regular": ... elif type == "premium": ...
After:  calculate_discount(strategy, total)  # strategy is injected
```

## L — Liskov Substitution
- **Subtypes must be substitutable** for base types without breaking correctness
- **Smell:** Subclass overrides method to throw or weaken post-conditions
- **Fix:** Don't force is-a where invariants differ — use a shared interface instead

```
Before: Square extends Rectangle (breaks width/height independence)
After:  Rectangle and Square both implement Shape interface
```

## I — Interface Segregation
- **Many specific interfaces > one general interface**
- **Smell:** Class implements methods just to throw UnsupportedOperation
- **Fix:** Split fat interface into role-based interfaces; implement only what you support

```
Before: Machine { print, scan, fax } — OldPrinter stubs scan/fax
After:  Printer | Scanner | FaxMachine — implement only what you need
```

## D — Dependency Inversion
- **Depend on abstractions, not concretions**
- **Smell:** `new Concrete()` in a constructor; can't test without real infrastructure
- **Fix:** Inject abstractions (interfaces/ABCs) through the constructor

```
Before: UserService creates MySQLDatabase internally
After:  UserService receives Database abstraction via constructor
```

## The Unifying Idea

All five principles are about **dependency management**:

| Principle | Dependency question |
|-----------|-------------------|
| SRP | Does this module have more than one reason to change? |
| OCP | Must I edit existing code to add new behavior? |
| LSP | Can this subtype stand in for its base without surprises? |
| ISP | Am I forced to depend on methods I never call? |
| DIP | Am I wired to a concrete class I could abstract away? |

**Pin this card next to your monitor. When code feels rigid, check each column.**