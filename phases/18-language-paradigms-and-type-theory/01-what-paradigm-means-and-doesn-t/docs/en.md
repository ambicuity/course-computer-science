# What 'Paradigm' Means (and Doesn't)

> Paradigms are lenses for tradeoffs, not identity badges.

**Type:** Learn
**Languages:** Markdown, Python, Haskell, Rust
**Prerequisites:** Phase 17
**Time:** ~45 minutes

## Learning Objectives

- Define what a programming paradigm is with precision.
- Distinguish paradigm from language and framework.
- Evaluate paradigm fit using problem constraints.
- Avoid false binaries like "OO vs FP".

## The Problem

A team starts a new project. The lead says "we're doing functional programming." Six months later, half the codebase mutates shared state through `IORef` wrappers, the other half is pure, and nobody agrees on where the boundary should be. The real failure wasn't choosing FP. It was treating a paradigm like an identity rather than a design tool.

This happens constantly. Engineers say "we're an OO shop" or "we write functional code" as if those labels dictate every design decision. They don't. A web server handles requests imperatively, transforms data functionally, and queries a database declaratively, all in the same process. The paradigm label doesn't describe what the code does. It describes which lens the developer used for which piece.

Worse, paradigm arguments mask the actual engineering questions. "Should we use inheritance or composition?" is a real question. "Is OO or FP better?" is not. The first question has context-dependent answers. The second is a category error, like asking whether hammers or saws are better tools.

## The Concept

A paradigm is a recurring design philosophy with preferred abstractions. It's a style of decomposing problems, not a language feature. You can write imperative Haskell (lots of `IO` and mutable refs) and functional C (passing function pointers and avoiding mutation). The paradigm lives in the design, not the syntax.

### The four major families

| Paradigm | Core idea | Preferred abstraction | Example languages |
|----------|-----------|----------------------|-------------------|
| Imperative | Explicit state transitions | Statements that modify memory | C, Go, Assembly |
| Object-oriented | Encapsulated state + behavior | Objects with methods and interfaces | Java, Smalltalk, C++ |
| Functional | Expressions, immutability, composition | Pure functions and algebraic types | Haskell, OCaml, Elm |
| Logic/declarative | Constraints and rules | Relations and search | Prolog, SQL, Datalog |

### Paradigm vs language

```
Language: Rust
├── Imperative:   for loops, mutable locals, explicit control flow
├── Functional:   iterators, pattern matching, Option/Result monads
├── OO-like:      trait objects, dynamic dispatch
└── Systems:      manual memory control, unsafe blocks

Language: Python
├── Imperative:   default style, mutable everything
├── Functional:   list comprehensions, functools, itertools
├── OO:           classes, inheritance, dunder methods
└── Declarative:  SQLAlchemy queries, Django ORM
```

Most production languages are multi-paradigm. Choosing one style per subsystem is a pragmatic engineering decision, not an ideological commitment.

### When each paradigm fits

| Problem characteristic | Better fit | Why |
|----------------------|-----------|-----|
| Tight loop over mutable state | Imperative | Direct mutation avoids allocation overhead |
| Complex domain with invariants | OO | Encapsulation hides state, enforces contracts |
| Data transformation pipelines | Functional | Composition, referential transparency, testability |
| Rule-heavy queries | Logic/declarative | Express relations, let engine find answers |
| Concurrent event processing | Actor/reactive | Message-passing avoids shared mutable state |

### The real decision matrix

When choosing a paradigm style for a module, evaluate these four dimensions:

1. **State mutation intensity.** How much of the logic is "update this, then that"? High mutation favors imperative or OO.
2. **Concurrency complexity.** Shared mutable state across threads is the hardest bug source. Functional and actor models reduce this.
3. **Domain algebraic structure.** If your domain has clear algebraic laws (e.g., commutative operations, identity elements), functional style exploits them.
4. **Team familiarity and tooling.** A paradigm you can debug at 3am beats a "better" paradigm you can't.

## Build It

### Step 1: Same problem, four styles

Write a function that takes a list of integers and returns the sum of squares of even numbers.

**Imperative (Python):**
```python
def sum_even_squares(nums):
    total = 0
    for n in nums:
        if n % 2 == 0:
            total += n * n
    return total

print(sum_even_squares([1, 2, 3, 4, 5]))  # 20
```

**Functional (Haskell):**
```haskell
sumEvenSquares :: [Int] -> Int
sumEvenSquares = sum . map (^2) . filter even

main :: IO ()
main = print $ sumEvenSquares [1, 2, 3, 4, 5]  -- 20
```

**OO-style (Python):**
```python
class EvenSquareSummer:
    def __init__(self, nums):
        self._nums = nums

    def compute(self):
        return sum(n * n for n in self._nums if n % 2 == 0)

print(EvenSquareSummer([1, 2, 3, 4, 5]).compute())  # 20
```

**Rust (multi-paradigm):**
```rust
fn sum_even_squares(nums: &[i32]) -> i32 {
    nums.iter()
        .filter(|&&n| n % 2 == 0)
        .map(|&n| n * n)
        .sum()
}

fn main() {
    println!("{}", sum_even_squares(&[1, 2, 3, 4, 5]));  // 20
}
```

### Step 2: Build a paradigm selection rubric

Create `code/notes.md` with a decision rubric for your project. For each module, answer:

```
Module: [name]
Primary paradigm: [choice]
Reasoning:
  - State mutation intensity: [low/medium/high]
  - Concurrency complexity: [none/single-threaded/multi-threaded]
  - Domain structure: [procedural/algebraic/relational]
  - Team familiarity: [comfortable/learning/unfamiliar]
```

### Step 3: Apply to a real codebase

Pick three modules in a project you know. Classify each by dominant paradigm. Identify one module where the paradigm fit is poor and sketch how you'd improve it.

## Use It

In production systems:

- **Data pipelines** (Airflow, Spark, dbt) benefit from functional composition: each stage is a pure transformation, pipelines compose.
- **UI frameworks** (React, SwiftUI) use a functional core (state → view) with an imperative shell (event handlers, DOM updates).
- **Game engines** use imperative loops over mutable state because allocation and indirection cost frames.
- **Databases** use declarative queries (SQL) because the optimizer can rearrange joins better than a human.
- **Telecom switches** (Erlang/OTP) use actor models because each call is independent and failure isolation matters.

GHC's codebase is mostly functional Haskell, but its garbage collector is imperative C. Rust's compiler is functional in its type-checking passes but imperative in its code generation. Production systems mix paradigms by design.

## Read the Source

- *Structure and Interpretation of Computer Programs* (Abelson & Sussman) teaches paradigm thinking through Scheme.
- *Programming Paradigms for Dummies* (Peter Van Roy) surveys the landscape rigorously.
- GHC's source tree: functional core (typechecker), imperative runtime (STG machine, GC).
- Linux kernel: pure imperative C, but with functional patterns (iterators, callbacks).

## Ship It

- `code/notes.md`: paradigm selection rubric for your project.
- `outputs/README.md`: quick checklist for architecture reviews.

## Quiz

**Q1 (Pre).** A team says "we're a functional shop." Which statement best evaluates this claim?

- A) They use a language with `map` and `filter`.
- B) They decompose problems into pure transformations with explicit effect boundaries.
- C) They avoid classes and inheritance.
- D) They don't use mutable variables.

**Answer: B.** Using `map`/`filter` is a surface feature, not a paradigm commitment. Avoiding classes is incidental (many imperative languages have no classes). Avoiding mutation entirely is impractical in production. The real hallmark of FP is decomposing into pure functions with controlled effects.

**Q2 (Pre).** Which problem characteristic most favors an imperative style?

- A) Complex domain invariants.
- B) High mutation intensity in tight loops.
- C) Rule-heavy queries.
- D) Concurrent event processing.

**Answer: B.** Tight loops over mutable state are where imperative shines: direct memory mutation avoids allocation overhead. Complex invariants favor OO (encapsulation), rule-heavy queries favor logic programming, and concurrency favors actor/functional models.

**Q3 (Post).** A database query optimizer rearranges join order for efficiency. Which paradigm does this best describe?

- A) Imperative, because it modifies the execution plan.
- B) OO, because the optimizer encapsulates plan state.
- C) Functional, because it transforms the AST.
- D) Declarative, because the query states what, not how.

**Answer: D.** SQL is declarative: you specify the desired result, and the engine figures out the execution strategy. The optimizer's internal implementation may be imperative or functional, but the paradigm of the query language itself is declarative.

**Q4 (Post).** A Rust iterator chain `nums.iter().filter(...).map(...).sum()` is best described as which paradigm style?

- A) Imperative, because Rust is a systems language.
- B) OO, because method calls use dot syntax.
- C) Functional, because it composes pure transformations.
- D) Declarative, because it describes a pipeline.

**Answer: C.** Iterator chains in Rust are functional: each stage is a pure transformation (filter, map), they compose without side effects, and `sum` is a fold. The dot syntax is syntactic sugar, not an OO pattern. The pipeline is imperative under the hood, but the paradigm style is functional.

**Q5 (Post).** Why do production systems typically mix paradigms?

- A) Because programmers are inconsistent.
- B) Because different problem domains have different optimal modeling styles.
- C) Because no single language supports multiple paradigms.
- D) Because paradigm mixing reduces code size.

**Answer: B.** Different subsystems have different characteristics. A GC's inner loop needs imperative mutation for performance. A type checker benefits from functional purity for correctness. A query engine needs declarative expressions for optimization. Mixing paradigms is deliberate engineering, not inconsistency.

## Exercises

1. **Easy.** Classify three modules in your current stack by dominant paradigm. Write a one-sentence justification for each.
2. **Medium.** Rewrite one function in two different paradigm styles (e.g., imperative Python and functional Haskell). Compare clarity, testability, and performance.
3. **Hard.** Identify one paradigm anti-pattern in existing code (e.g., deep inheritance hierarchies where composition fits better, or imperative loops where a pipeline would be clearer). Draft a refactoring plan with rationale.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Paradigm | "language type" | A reusable style of decomposition and reasoning about problems |
| Multi-paradigm | "inconsistent" | A language or system supporting several abstraction styles deliberately |
| Paradigm fit | "preference" | Match between problem constraints and the modeling style's strengths |
| Anti-pattern | "bad code" | Repeated misuse of a paradigm in a context where it fights the problem |
| Declarative | "just say what" | Describe the desired result, not the step-by-step procedure |

## Further Reading

- [Structure and Interpretation of Computer Programs](https://mitpress.mit.edu/9780262510872/structure-and-interpretation-of-computer-programs/)
- [Programming Paradigms for Dummies](https://www.cs.rice.edu/~javaplt/311/Notes/10/00.html)
- [Peter Van Roy's "Programming Paradigms" lecture](https://www.info.ucl.ac.be/~pvr/paradigms.html)
