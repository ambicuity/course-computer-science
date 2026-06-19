# Refactoring Catalogue and Mechanics

> Refactoring: improving the design of existing code without changing its behavior.

**Type:** Learn
**Languages:** TypeScript, Python
**Prerequisites:** Phase 16 lessons 01–05
**Time:** ~75 minutes

## Learning Objectives

- Define refactoring as a behavior-preserving transformation and explain why the "no behavior change" constraint matters.
- Navigate Martin Fowler's refactoring catalogue and know when to apply key refactorings.
- Apply the refactoring workflow: test → small step → test → commit.
- Identify code smells and choose the appropriate refactoring to address them.
- Distinguish confidently between when to refactor and when to rewrite.

## The Problem

You join a project with a 2,000-line function that calculates invoices. It has nested `if` blocks twelve levels deep, variables named `x` and `temp2`, and a `switch` statement duplicated in four places. The tests pass, so the code works — but adding a new discount tier takes three days instead of thirty minutes.

This is the refactoring problem: the code is correct but hostile to change. Without disciplined refactoring techniques, teams either live with decay (velocity drops to zero) or rewrite from scratch (throws away working logic, introduces new bugs). Both options waste time and money.

This lesson gives you a systematic alternative: a catalogue of proven transformations, a workflow that keeps you safe, and the judgment to know when refactoring is the right call.

## The Concept

### What Refactoring Is

Refactoring is a **behavior-preserving transformation**: you change the internal structure of code so that it becomes easier to understand and modify, while ensuring that all existing tests continue to pass with identical results.

The critical word is *behavior-preserving*. A refactoring is not:

- Adding a feature — that changes behavior.
- Fixing a bug — that changes behavior.
- Rewriting from scratch — that discards structure.

A refactoring is a **small, verified, reversible step** that improves design. You chain these steps together to get from messy code to clean code, testing after every step.

### Martin Fowler's Catalogue

Martin Fowler's *Refactoring* (1999, 2nd ed. 2018) organizes ~70 refactorings into categories:

| Category | Purpose | Key Examples |
|----------|---------|--------------|
| Composing Methods | Organize code into clean methods | Extract Method, Inline Method, Replace Temp with Query |
| Organizing Data | Simplify data structures | Replace Magic Number with Symbolic Constant, Change Reference to Value |
| Simplifying Conditional Logic | Tame branching | Decompose Conditional, Replace Conditional with Polymorphism |
| Moving Features | Place code where it belongs | Move Method, Move Field, Extract Class |
| Generalization | Reduce duplication | Extract Superclass, Pull Up Method, Push Down Method |

Each entry in the catalogue specifies:

1. **Name** — a standard vocabulary word (e.g., "Extract Method")
2. **Motivation** — when to apply it
3. **Mechanics** — the step-by-step procedure
4. **Examples** — before and after code

Using a shared vocabulary makes code review discussions precise. Instead of "you should clean this up," you say "apply Extract Method on lines 42–60 and name it `calculateLateFee`."

### When to Refactor

Fowler identifies three triggers:

**Rule of Three** — The third time you do something similar (copy-paste then modify), refactor. The first time you just do it. The second time you wince but tolerate duplication. The third time, you extract the common logic.

**Preparation** — Refactor before adding a feature. If the existing structure makes the new feature awkward, reshape the code first, then add the feature. This sounds paradoxical but works: you spend 30 minutes refactoring to save 2 hours of fighting bad structure.

**Comprehension** — Refactor while reading code you don't understand. When you finally grasp what a function does, refactor it so the next person doesn't have to reverse-engineer it. A renamed variable or extracted method captures understanding permanently.

### Key Refactorings in Detail

#### Extract Method

**When:** You have a code fragment that can be grouped together and whose purpose is clearer than the surrounding code.

**Mechanics:**
1. Create a new method named after what the fragment *does* (not *how* it does it).
2. Copy the fragment into the new method.
3. Replace local variables that are only used in the fragment with parameters.
4. Replace the original fragment with a call to the new method.
5. Test.

```
Before:
def print_owing(amount):
    print_banner()
    # calculate outstanding
    outstanding = 0
    for order in orders:
        outstanding += order.amount
    # print details
    print(f"name: {name}")
    print(f"amount: {outstanding}")

After:
def print_owing(amount):
    print_banner()
    outstanding = get_outstanding()
    print_details(outstanding)

def get_outstanding():
    outstanding = 0
    for order in orders:
        outstanding += order.amount
    return outstanding

def print_details(outstanding):
    print(f"name: {name}")
    print(f"amount: {outstanding}")
```

#### Extract Variable

**When:** You have an expression that's hard to understand or is used multiple times.

**Mechanics:**
1. Declare a new variable named after the *intent* of the expression.
2. Replace the expression with the variable.
3. Test.

```
Before:
if platform.upper().startswith("MAC") and browser.upper() == "IE" and was_initialized() and resize > 0:
    # do something

After:
is_mac_ie = platform.upper().startswith("MAC") and browser.upper() == "IE"
was_resized = was_initialized() and resize > 0
if is_mac_ie and was_resized:
    # do something
```

#### Rename

**When:** A name no longer accurately describes what it names — the most common and most underrated refactoring.

**Mechanics:**
1. Check the name isn't already used elsewhere for a different purpose.
2. Find all references to the old name.
3. Change the declaration.
4. Change all references.
5. Test.

Good names reduce the need for comments. `days_since(last_payment)` beats `calc(x) > 30`.

#### Replace Conditional with Polymorphism

**When:** You have a conditional that chooses different behavior depending on the type (or variant) of an object.

**Mechanics:**
1. If the conditional is in a method that's not on the type object, use Move Method first.
2. Create a subclass for each variant in the conditional.
3. Override the method in each subclass with the corresponding branch's logic.
4. Replace the conditional with a polymorphic call.
5. Test after each subclass.

```
Before:
def get_rating(voyage):
    result = 2
    if voyage.zone == "china" and voyage.length > 10:
        result += 1
    if voyage.zone == "east-indies" and voyage.length > 5:
        result += 1
    return result

After:
class VoyageRating:
    def get_rating(self, voyage):
        return 2 + self.zone_risk(voyage)

class ChinaRating(VoyageRating):
    def zone_risk(self, voyage):
        return 1 if voyage.length > 10 else 0

class EastIndiesRating(VoyageRating):
    def zone_risk(self, voyage):
        return 1 if voyage.length > 5 else 0
```

This is one of the most powerful refactorings because it leverages the type system: the compiler ensures you handle every case, and adding a new case means adding a new subclass rather than hunting down every `if`/`switch`.

#### Replace Temp with Query

**When:** You use a temporary variable to hold the result of an expression, and you need to use that expression in multiple methods.

**Mechanics:**
1. Extract the expression into a method.
2. Replace all references to the temp with calls to the new method.
3. Remove the temp declaration.
4. Test.

```
Before:
base_price = quantity * item_price
if base_price > 1000:
    return base_price * 0.95
return base_price

After:
if base_price() > 1000:
    return base_price() * 0.95
return base_price()

def base_price(self):
    return self.quantity * self.item_price
```

This enables further refactorings: `base_price()` can now be used by any method without passing parameters.

#### Introduce Parameter Object

**When:** A group of parameters naturally belong together and are passed as a cluster across multiple methods.

**Mechanics:**
1. Create a new class (or data class / NamedTuple) for the parameter group.
2. Add the fields from the parameter group.
3. Change the method signature to accept the new object.
4. Update all callers to construct the object.
5. Test after each method signature change.

```
Before:
def print_invoice(name, amount, date, tax_rate, discount):
    ...

def record_payment(name, amount, date, tax_rate):
    ...

After:
@dataclass
class InvoiceInfo:
    name: str
    amount: float
    date: datetime
    tax_rate: float
    discount: float = 0

def print_invoice(info: InvoiceInfo): ...
def record_payment(info: InvoiceInfo): ...  # uses only some fields
```

This reduces parameter count, makes call sites cleaner, and centralizes related data.

#### Decompose Conditional

**When:** You have a complex conditional (`if`/`elsif`/`else`) where the condition, then-branch, and else-branch are hard to follow.

**Mechanics:**
1. Extract the condition into a well-named method (e.g., `is_summer()`).
2. Extract the then-branch into a method (e.g., `summer_charge()`).
3. Extract the else-branch into a method (e.g., `winter_charge()`).
4. Test.

```
Before:
if date.before(SUMMER_START) or date.after(SUMMER_END):
    charge = quantity * winter_rate + winter_service_fee
else:
    charge = quantity * summer_rate

After:
if not is_summer(date):
    charge = winter_charge(quantity)
else:
    charge = summer_charge(quantity)
```

#### Move Method

**When:** A method uses (or is used by) more features of another class than its own class.

**Mechanics:**
1. Copy the method to the target class.
2. Adjust references: change `self` calls to use the source object if needed.
3. Replace the body of the source method with a delegation call.
4. Test.
5. Optionally remove the source method and inline the delegation.

This is the core refactoring for curing **feature envy** — a method that's more interested in another class's data than its own.

#### Inline Method

**When:** A method body is as clear as its name, or the indirection adds no value.

**Mechanics:**
1. Find all callers of the method.
2. Replace each call with the method body.
3. Remove the method definition.
4. Test.

This is the inverse of Extract Method. Not all method calls add clarity — sometimes they just add indirection. Use Inline Method when the method is a trivial wrapper that obscures rather than reveals intent.

### The Refactoring Workflow

The safe refactoring workflow is a strict loop:

```
┌─────────────────────────────────────────────┐
│  1. Write tests (if not already present)    │
│  2. Run tests — all green                    │
│  3. Apply ONE small refactoring step         │
│  4. Run tests — all green                    │
│  5. If more steps needed, go to 3            │
│  6. Commit                                    │
└─────────────────────────────────────────────┘
```

**Why this strictness?**

- **Test first:** If you refactor without tests, you have no safety net. A typo becomes a production bug. Write characterization tests for legacy code before touching it.
- **Small steps:** Each step should be so small that if tests break, you know exactly which change caused it. "Extract method" is one step. "Extract method and rename a variable" is two steps.
- **Commit after each refactoring:** If something goes wrong later, you can revert to a known-good state. Git is your undo stack.
- **Never refactor and add features simultaneously:** A commit should be either a refactoring or a feature, never both. Mixed commits make it impossible to bisect or roll back safely.

### Code Smell Catalogue

Refactorings are applied in response to **code smells** — symptoms in the code that indicate a design problem. Fowler identifies 22 smells; the most common are:

#### Long Method

**Symptom:** A method that's hard to understand because it does too much.

**Fix:** Extract Method, Replace Temp with Query, Introduce Parameter Object, Decompose Conditional.

The key insight: the longer a method, the harder it is to understand. Methods should do one thing and have a name that says what that thing is. If you can't name it, it's doing too much.

#### Large Class

**Symptom:** A class that's trying to do too many things — it has too many instance variables and too many methods.

**Fix:** Extract Class, Extract Subclass, Extract Interface.

A class should have a single responsibility. When it accumulates groups of variables that are only used together, that's a sign it's time to split.

#### Feature Envy

**Symptom:** A method that seems more interested in a class other than the one it's in — it uses data from another class more than its own.

**Fix:** Move Method, Extract Method (then Move the extracted method).

The classic example is a method on `Order` that repeatedly accesses `Customer` data. The method probably belongs on `Customer`.

#### Switch Statements

**Symptom:** Complex `switch` or `if`/`elif` chains that duplicate across the codebase.

**Fix:** Replace Conditional with Polymorphism, Extract Method (for the case bodies), Move Method (to get the conditional where the polymorphism should live).

The problem with switch statements isn't that they exist — it's that they *duplicate*. The same type check appears in five places, and adding a new type means updating all five.

#### Speculative Generality

**Symptom:** Code that was written "just in case" — unused abstract classes, unnecessary delegation, parameters that are never varied.

**Fix:** Inline Method, Collapse Hierarchy, Remove Unused Parameter, Inline Class.

This smell is the opposite of the others: instead of too little design, there's too much. YAGNI (You Aren't Gonna Need It) says: don't build flexibility until you actually need it. Speculative generality makes code harder to understand because the reader can't tell what's actually used.

#### Duplicated Code

**Symptom:** The same expression or logic appears in two or more places.

**Fix:** Extract Method (for identical code), Form Template Method (for similar structure with variations), Pull Up Method (into a superclass), Extract Class (if the duplication is across unrelated classes).

Duplicated code is the #1 smell. It means that when you change one copy, you must remember to change the others — and you won't.

### Refactoring vs. Rewriting

When facing technical debt, teams face a choice: refactor incrementally or rewrite from scratch?

**Refactor when:**
- Existing code works and has tests.
- The changes are localized.
- You need to ship features while improving the code.
- The team can work incrementally (each step produces working software).

**Rewrite when:**
- The architecture is fundamentally wrong (e.g., a monolith that should be microservices).
- The codebase has no tests and no one understands it well enough to write characterization tests.
- The domain model has shifted so far that the existing abstractions are actively misleading.
- Less than 10% of the current code will survive the changes you need.

**The hardest lesson:** most rewrites fail. Joel Spolsky's advice is to almost never rewrite. The reason is that existing code captures years of accumulated bug fixes, edge cases, and domain knowledge that you'll rediscover only through production failures. Incremental refactoring preserves this knowledge; a rewrite discards it.

**A practical rule of thumb:** If you can express what you need as a sequence of small, behavior-preserving refactorings, do that. If every refactoring step would require tearing down a fundamental assumption, you may need to rewrite — but write characterization tests for the old code first.

## Build It

### Step 1: Minimal Version — Smelly Code

We start with deliberately smelly code that calculates rental fees for different vehicle types. It works, but it's hard to change. See `code/main.py` and `code/main.ts` for the full kata.

### Step 2: Realistic Version — Apply Refactorings

We apply refactorings one at a time, testing after each. Each version in the code files is a complete, working program:

1. **Extract Method** — split the long `calculate_fee` function into named sub-functions.
2. **Replace Conditional with Polymorphism** — replace the vehicle type `if`/`elif` chain with subclasses.
3. **Introduce Parameter Object** — group the rental parameters into a data class.
4. **Move Method** — move fee calculation logic to where the data lives.
5. **Replace Temp with Query** — turn local calculations into query methods.

After all refactorings, the code is shorter, clearer, and — crucially — adding a new vehicle type means creating a new subclass, not modifying a `switch` statement.

## Use It

### In Production Codebases

- **IntelliJ IDEA / JetBrains IDEs:** Have built-in refactoring support for Extract Method, Rename, Move, and dozens more — they handle the mechanics automatically.
- **Python (Rope, Bottleneck):** The `rope` library provides programmatic refactoring. `pylint` and `flake8` detect many code smells.
- **TypeScript (ts-refactor):** Language Service provides safe Rename, Extract, and Move refactorings.
- **Automated refactoring tools** enforce the "small step, test, commit" workflow by making each step fast and mechanical.

### What Production Does That Our Kata Doesn't

Real refactoring in production codebases faces challenges our kata omits:

- **Legacy code without tests:** Michael Feathers' *Working Effectively with Legacy Code* provides techniques for getting tests on untested code (seam, sprout method, override).
- **Large-scale refactorings:** Some refactorings (e.g., changing a public API) affect hundreds of callers across many repositories. These require deprecation periods and feature flags.
- **Database refactorings:** Schema changes must be backward-compatible across deployment cycles (see Pramod Sadalage and Martin Fowler's *Refactoring Databases*).
- **Refactoring automation:** Tools like `ast-grep` and `semgrep` can automate repetitive refactorings across large codebases.

## Read the Source

- **Fowler's catalogue:** [refactoring.com/catalog](https://refactoring.com/catalog/) — the full searchable catalogue online.
- **IntelliJ IDEA source:** `java/src/org/jetbrains/java/decompiler/modules/decompiler/` — see how production IDEs implement automated refactoring detection.
- **Python `rope`:** [github.com/python-rope/rope](https://github.com/python-rope/rope) — refactoring library for Python.

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. For this lesson it is:

- **`refactoring_reference.md`** — a quick-reference card with the full refactoring catalogue, when to apply each, and the workflow checklist.

## Exercises

1. **Easy** — Starting from the smelly code in `code/main.py`, apply only "Extract Method" and "Rename Variable" to make it more readable. Don't change the structure — just improve naming and decomposition.

2. **Medium** — Add a `TRUCK` vehicle type to the smelly version (before refactoring), then add it to the refactored polymorphic version. Count how many places you modify in each case. Which is easier?

3. **Hard** — Find a real function in a project you work on that has 50+ lines and at least one code smell. Write characterization tests for it, then apply 3+ refactorings from the catalogue. Show the before/after diff in a code review.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Refactoring | "Fixing the code" | A behavior-preserving transformation applied in small, tested steps |
| Code Smell | "Bad code" | A symptom in the code that indicates a deeper design problem, not the problem itself |
| Extract Method | "Break this up" | Take a code fragment, put it in its own method named after what it does |
| Replace Conditional with Polymorphism | "Use subclasses" | Move type-conditional logic into overridden methods on type-specific subclasses |
| Feature Envy | "Wrong class" | A method that accesses another class's data more than its own |
| Rule of Three | "Don't duplicate" | Refactor when you see the third instance of duplicated logic |
| Speculative Generality | "Over-engineered" | Code written for hypothetical future needs that adds complexity now |
| Characterization Test | "Legacy test" | A test that captures current behavior (bugs and all) before you refactor |

## Further Reading

- Martin Fowler, *Refactoring: Improving the Design of Existing Code* (2nd ed., 2018) — the definitive reference.
- Martin Fowler, [refactoring.com](https://refactoring.com) — online catalogue with before/after examples.
- Michael Feathers, *Working Effectively with Legacy Code* (2004) — techniques for adding tests to untested code so you can refactor safely.
- Joshua Kerievsky, *Refactoring to Patterns* (2004) — connects Fowler's refactorings to the Gang of Four design patterns.
- William C. Wake, *Refactoring Workbook* (2004) — exercises for recognizing and fixing code smells.