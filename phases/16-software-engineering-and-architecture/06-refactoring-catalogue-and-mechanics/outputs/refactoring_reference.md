# Refactoring Quick Reference

> A companion card for Phase 16, Lesson 06 — Refactoring Catalogue and Mechanics.

## Refactoring Workflow Checklist

- [ ] **Write tests first** (characterization tests for legacy code)
- [ ] **Run tests** — all green
- [ ] **Apply ONE small refactoring step**
- [ ] **Run tests** — all green
- [ ] **Commit** with message naming the refactoring (e.g., `refactor: extract method calculateLateFee`)
- [ ] Repeat until design is clean
- [ ] **Never mix refactorings and feature changes in one commit**

---

## Code Smell → Refactoring Map

| Smell | Symptom | Primary Refactorings |
|-------|---------|---------------------|
| Long Method | Function does too much, hard to understand | Extract Method, Replace Temp with Query, Decompose Conditional |
| Large Class | Too many fields and methods | Extract Class, Extract Subclass, Extract Interface |
| Feature Envy | Method uses another class's data more than its own | Move Method, Extract Method then Move |
| Switch Statements | Duplicated type checks across codebase | Replace Conditional with Polymorphism, Move Method |
| Duplicated Code | Same logic in multiple places | Extract Method, Pull Up Method, Form Template Method |
| Speculative Generality | Unused abstractions, premature flexibility | Inline Method, Collapse Hierarchy, Remove Parameter |
| Refused Bequest | Subclass doesn't use inherited methods | Push Down Method, Replace Inheritance with Delegation |
| Data Clumps | Same group of parameters appear together | Introduce Parameter Object, Extract Class |
| Primitive Obsession | Using primitives instead of small objects | Replace Data Value with Object, Introduce Parameter Object |
| Temporary Field | Instance variable that's sometimes null | Extract Class, Introduce Null Object |

---

## Key Refactorings — Quick Mechanics

### Extract Method
**When:** Code fragment can be grouped and its purpose is clearer than the surrounding code.
1. Create new method named after *what* the fragment does
2. Copy fragment into new method
3. Replace local variables with parameters
4. Replace original fragment with call
5. Test

### Extract Variable
**When:** Expression is hard to understand or used multiple times.
1. Declare variable named after the *intent* of the expression
2. Replace expression with variable
3. Test

### Rename
**When:** Name no longer describes what it names.
1. Check name isn't used elsewhere for a different purpose
2. Change declaration
3. Change all references
4. Test

### Replace Conditional with Polymorphism
**When:** Conditional chooses behavior based on type.
1. Move method to type object (if not already there)
2. Create subclass for each variant
3. Override method in each subclass with corresponding branch logic
4. Replace conditional with polymorphic call
5. Test after each subclass

### Replace Temp with Query
**When:** Temp variable holds result of an expression needed in multiple methods.
1. Extract expression into a method
2. Replace all references to temp with call to new method
3. Remove temp
4. Test

### Introduce Parameter Object
**When:** Group of parameters travel together across methods.
1. Create data class / NamedTuple for the group
2. Add fields from parameter group
3. Change method signature to accept new object
4. Update callers
5. Test after each signature change

### Decompose Conditional
**When:** Complex conditional is hard to follow.
1. Extract condition → `isSummer(date)`
2. Extract then-branch → `summerCharge(quantity)`
3. Extract else-branch → `winterCharge(quantity)`
4. Test

### Move Method
**When:** Method uses more features of another class than its own (Feature Envy).
1. Copy method to target class
2. Adjust references (change `self` calls)
3. Replace source body with delegation call
4. Test
5. Optionally inline delegation and remove source method

### Inline Method
**When:** Method body is as clear as its name (indirection adds no value).
1. Find all callers
2. Replace each call with method body
3. Remove method definition
4. Test

---

## When to Refactor vs. When to Rewrite

| Refactor When... | Rewrite When... |
|-------------------|-----------------|
| Existing code works and has tests | Architecture is fundamentally wrong |
| Changes are localized | < 10% of code will survive |
| Must ship features while improving | No tests and no one understands it well enough |
| Each step produces working software | Domain model has shifted beyond recognition |
| Team can work incrementally | Cost of understanding exceeds cost of rebuilding |

**The rule:** If you can express what you need as a sequence of small, behavior-preserving steps → **refactor**. If every step requires tearing down a fundamental assumption → **consider rewrite** (but write characterization tests first).

---

## Trigger Patterns

| Trigger | Action |
|---------|--------|
| Rule of Three (3rd duplication) | Stop and Extract Method or Pull Up Method |
| Before adding a feature | Refactor to make the feature easy to add |
| While understanding code | Rename and Extract to capture your understanding |
| During code review | Suggest named refactorings ("Extract Method here") |
| After fixing a bug | Refactor to make the bug category impossible |

---

## Vocabulary

| Term | Meaning |
|------|---------|
| Refactoring | Behavior-preserving transformation applied in small tested steps |
| Code Smell | Symptom indicating a deeper design problem |
| Characterization Test | Test capturing current behavior (bugs included) before refactoring |
| Mechanics | Step-by-step procedure for applying a refactoring safely |
| Rule of Three | Refactor on the 3rd instance of duplication |
| Strangler Fig | Incrementally replacing a system by routing new calls to new code |