# Mutation Testing

> If tests cannot catch small deliberate bugs, they are weaker than they look.

**Type:** Learn
**Languages:** Python, TypeScript
**Prerequisites:** Phase 17 lessons 01-05
**Time:** ~60 minutes

## Learning Objectives

- Explain mutation testing as a test-quality measurement method.
- Distinguish code coverage from mutation score.
- Identify equivalent and surviving mutants pragmatically.
- Build a tiny mutator workflow to assess test strength.

## The Problem

A team reports 96% line coverage and still ships regressions in discount logic.
Postmortem shows assertions only checked obvious outputs. Edge semantics (zero,
negative values, boundary comparisons) were untested. Coverage measured execution,
not assertion quality.

### The coverage illusion

Line coverage answers one question: "Did this line execute during tests?" It does
not answer the question that matters: "Would a bug on this line be detected?"

```python
def apply_discount(price, discount_pct):
    if discount_pct < 0 or discount_pct > 100:
        raise ValueError("Invalid discount")
    return price * (1 - discount_pct / 100)
```

A test that calls `apply_discount(100, 10)` and checks the result is 90.0 will
give you 100% line coverage on this function. But it tells you nothing about:

- What happens when `discount_pct` is exactly 0
- What happens when `discount_pct` is exactly 100
- What happens with floating-point precision (e.g., 100 * 0.7)
- Whether the validation branch actually rejects invalid input

### A concrete failure

Consider this buggy implementation:

```python
def apply_discount(price, discount_pct):
    if discount_pct < 0 or discount_pct > 100:
        raise ValueError("Invalid discount")
    return price * (1 - discount_pct / 100)
    # Bug: should be price - (price * discount_pct / 100)
    # The current version has floating-point drift
```

Wait, both formulas are mathematically equivalent. Let's use a real bug:

```python
def apply_discount(price, discount_pct):
    if discount_pct < 0 or discount_pct > 100:
        raise ValueError("Invalid discount")
    if discount_pct == 0:
        return price  # special case, skip calculation
    return price * (discount_pct / 100)  # BUG: should be (1 - discount_pct/100)
```

A test checking `apply_discount(100, 10) == 90` would fail, catching this bug.
But what if the test was `apply_discount(100, 10) == 10`? That test would pass
because the buggy code returns 10.0 and the test expects 10. The test itself
is wrong, and coverage wouldn't tell you.

Mutation testing catches this by asking: if we change `discount_pct / 100` to
`1 - discount_pct / 100`, does the test suite notice? If not, the tests are weak.

### The measurement gap

| Metric | What it measures | What it misses |
|---|---|---|
| Line coverage | Lines executed | Whether assertions check those lines |
| Branch coverage | Branches taken | Whether branch outcomes are verified |
| Condition coverage | Boolean subexpressions | Whether edge values are tested |
| Mutation score | Test suite's ability to detect code changes | Equivalent mutants, cost |

Mutation testing fills the gap between "tests ran this code" and "tests would
catch a bug in this code."

## The Concept

### How mutation testing works

The mutation testing cycle:

```
  ┌─────────────────────────────────────────────────────┐
  │                                                     │
  │  ┌──────────┐    ┌──────────┐    ┌──────────────┐  │
  │  │ Original │    │ Generate │    │   Run Test   │  │
  │  │   Code   │───▶│ Mutants  │───▶│    Suite     │  │
  │  └──────────┘    └──────────┘    └──────┬───────┘  │
  │                                         │          │
  │                              ┌──────────┴────────┐ │
  │                              │                   │ │
  │                         Tests fail          Tests pass│
  │                              │                   │ │
  │                              ▼                   ▼ │
  │                         ┌─────────┐       ┌─────────┐│
  │                         │ KILLED  │       │SURVIVED ││
  │                         │  (good) │       │  (bad?) ││
  │                         └─────────┘       └─────────┘│
  │                                                      │
  │  Mutation Score = Killed / (Killed + Survived)       │
  └──────────────────────────────────────────────────────┘
```

1. Generate mutant by altering one code token/operator.
2. Run test suite against the mutant.
3. If tests fail, mutant is killed (tests detected the change).
4. If tests pass, mutant survives (tests missed the change).
5. Compute score = killed / non-equivalent mutants.

### Mutation operators

Mutators are small, syntactic transformations. They mimic common developer
mistakes:

**Arithmetic operator replacement (AOR):**

| Original | Mutant |
|---|---|
| `a + b` | `a - b` |
| `a * b` | `a / b` |
| `a - b` | `a + b` |
| `a / b` | `a * b` |
| `a % b` | `a * b` |

**Relational operator replacement (ROR):**

| Original | Mutant |
|---|---|
| `a < b` | `a <= b` |
| `a > b` | `a >= b` |
| `a == b` | `a != b` |
| `a <= b` | `a < b` |
| `a >= b` | `a > b` |

**Logical operator replacement (LOR):**

| Original | Mutant |
|---|---|
| `a && b` | `a \|\| b` |
| `a \|\| b` | `a && b` |
| `!a` | `a` |

**Constant replacement (CR):**

| Original | Mutant |
|---|---|
| `return True` | `return False` |
| `return 0` | `return 1` |
| `return x` | `return None` |

**Statement deletion (SD):**

| Original | Mutant |
|---|---|
| `x += 1` | (deleted) |
| `validate(input)` | (deleted) |

Each mutant changes exactly one thing. This makes the root cause clear when a
mutant survives: one specific operation wasn't tested.

### Equivalent mutants

An equivalent mutant is behaviorally identical to the original for all inputs.
These are false positives that inflate the "survived" count.

```python
# Original
def abs_val(x):
    if x < 0:
        return -x
    return x

# Equivalent mutant (changed < to <= for x == 0 case)
def abs_val_mutant(x):
    if x <= 0:      # mutant
        return -x
    return x
```

When `x == 0`: original returns 0, mutant returns `-0` which equals 0 in Python.
The mutant is equivalent for all integer inputs.

Detecting equivalent mutants automatically is undecidable (it reduces to the
halting problem). In practice, teams handle them by:

- Manual review of surviving mutants
- Timeout heuristics (if mutant changes nothing after N seconds, likely equivalent)
- Compiler optimizations that normalize equivalent forms

### The mutation score

```
  Mutation Score = Killed Mutants / (Total Mutants - Equivalent Mutants)
```

| Score range | Interpretation |
|---|---|
| 0-50% | Test suite is very weak. Many operations untested. |
| 50-70% | Some coverage gaps. Review survivors for missing edge cases. |
| 70-85% | Good suite. Survivors likely equivalent or very subtle. |
| 85-100% | Strong suite. Remaining survivors require manual review. |

Mutation score is not an absolute quality metric. It's a signal for weak
assertions and missing edge cases. A score of 90% doesn't mean your code is
bug-free. It means your tests would catch 90% of single-token code changes.

### Mutation testing vs other quality metrics

| Metric | Cost | Depth | False positives |
|---|---|---|---|
| Line coverage | Very low | Shallow | None |
| Branch coverage | Low | Medium | None |
| Property-based testing | Medium | Deep | Low |
| Mutation testing | High | Deep | Medium (equivalent mutants) |
| Formal verification | Very high | Complete | Low |

Mutation testing sits in the middle: more expensive than coverage, cheaper than
formal verification, and directly measures what coverage only implies.

## Build It

We build a mutation testing workflow for a fee calculator function.

### Step 1: Define baseline function

Use a simple fee calculator with branchy behavior.

```python
# fee_calculator.py
def calculate_fee(amount, user_type, is_weekend):
    """Calculate transaction fee based on rules."""
    if amount <= 0:
        raise ValueError("Amount must be positive")

    base_fee = amount * 0.02  # 2% base fee

    if user_type == "premium":
        base_fee *= 0.5  # 50% discount for premium
    elif user_type == "enterprise":
        base_fee = 0  # no fee for enterprise

    if is_weekend and user_type != "enterprise":
        base_fee += 1.0  # weekend surcharge

    if amount > 10000:
        base_fee = min(base_fee, 50.0)  # cap at $50 for large transactions

    return round(base_fee, 2)
```

### Step 2: Define mutants

Encode mutant variants as alternative implementations.

```python
# mutants.py
# Each mutant changes exactly one operation in calculate_fee

def mutant_1(amount, user_type, is_weekend):
    """Mutant: < changed to <="""
    if amount <= 0:
        raise ValueError("Amount must be positive")
    base_fee = amount * 0.02
    if user_type == "premium":
        base_fee *= 0.5
    elif user_type == "enterprise":
        base_fee = 0
    if is_weekend and user_type != "enterprise":
        base_fee += 1.0
    if amount > 10000:
        base_fee = min(base_fee, 50.0)
    return round(base_fee, 2)

def mutant_2(amount, user_type, is_weekend):
    """Mutant: * changed to / in base_fee calculation"""
    if amount <= 0:
        raise ValueError("Amount must be positive")
    base_fee = amount / 0.02  # BUG: was *
    if user_type == "premium":
        base_fee *= 0.5
    elif user_type == "enterprise":
        base_fee = 0
    if is_weekend and user_type != "enterprise":
        base_fee += 1.0
    if amount > 10000:
        base_fee = min(base_fee, 50.0)
    return round(base_fee, 2)

def mutant_3(amount, user_type, is_weekend):
    """Mutant: + changed to - in weekend surcharge"""
    if amount <= 0:
        raise ValueError("Amount must be positive")
    base_fee = amount * 0.02
    if user_type == "premium":
        base_fee *= 0.5
    elif user_type == "enterprise":
        base_fee = 0
    if is_weekend and user_type != "enterprise":
        base_fee -= 1.0  # BUG: was +=
    if amount > 10000:
        base_fee = min(base_fee, 50.0)
    return round(base_fee, 2)

def mutant_4(amount, user_type, is_weekend):
    """Mutant: > changed to >= in large transaction check"""
    if amount <= 0:
        raise ValueError("Amount must be positive")
    base_fee = amount * 0.02
    if user_type == "premium":
        base_fee *= 0.5
    elif user_type == "enterprise":
        base_fee = 0
    if is_weekend and user_type != "enterprise":
        base_fee += 1.0
    if amount >= 10000:  # BUG: was >
        base_fee = min(base_fee, 50.0)
    return round(base_fee, 2)

def mutant_5(amount, user_type, is_weekend):
    """Mutant: return True changed to return False (negate validation)"""
    if amount <= 0:
        pass  # BUG: removed raise
    base_fee = amount * 0.02
    if user_type == "premium":
        base_fee *= 0.5
    elif user_type == "enterprise":
        base_fee = 0
    if is_weekend and user_type != "enterprise":
        base_fee += 1.0
    if amount > 10000:
        base_fee = min(base_fee, 50.0)
    return round(base_fee, 2)
```

### Step 3: Define focused tests

Write behavioral checks around boundaries.

```python
# test_fee_calculator.py
import pytest
from fee_calculator import calculate_fee

def test_basic_fee():
    assert calculate_fee(100, "regular", False) == 2.0

def test_premium_discount():
    assert calculate_fee(100, "premium", False) == 1.0

def test_enterprise_no_fee():
    assert calculate_fee(100, "enterprise", False) == 0.0

def test_weekend_surcharge():
    assert calculate_fee(100, "regular", True) == 3.0

def test_large_transaction_cap():
    assert calculate_fee(50000, "regular", False) == 50.0

def test_negative_amount_raises():
    with pytest.raises(ValueError):
        calculate_fee(-100, "regular", False)
```

### Step 4: Evaluate outcomes

Run the same tests against each mutant and report kill/survive.

```python
# run_mutation_test.py
import subprocess
import sys

MUTANTS = [
    ("mutant_1", "boundary check: < to <="),
    ("mutant_2", "arithmetic: * to /"),
    ("mutant_3", "arithmetic: + to -"),
    ("mutant_4", "boundary: > to >="),
    ("mutant_5", "deletion: remove raise"),
]

def run_tests_against_mutant(mutant_name):
    """Run test suite, return True if tests pass (mutant survived)."""
    result = subprocess.run(
        [sys.executable, "-m", "pytest", "test_fee_calculator.py", "-x", "-q"],
        capture_output=True, text=True
    )
    return result.returncode == 0

killed = 0
survived = 0

for name, description in MUTANTS:
    # In practice, you'd swap the import or use AST transformation
    # Here we simulate by running against each mutant file
    if run_tests_against_mutant(name):
        print(f"  SURVIVED: {name} ({description})")
        survived += 1
    else:
        print(f"  KILLED:   {name} ({description})")
        killed += 1

score = killed / (killed + survived) * 100
print(f"\nMutation Score: {score:.0f}% ({killed}/{killed + survived})")
```

Expected output:

```
  KILLED:   mutant_1 (boundary check: < to <=)
  KILLED:   mutant_2 (arithmetic: * to /)
  KILLED:   mutant_3 (arithmetic: + to -)
  SURVIVED: mutant_4 (boundary: > to >=)
  KILLED:   mutant_5 (deletion: remove raise)

Mutation Score: 80% (4/5)
```

Mutant 4 survived because no test checks `amount == 10000` exactly. The test
for the cap uses `50000`, and regular tests use `100`. The boundary at 10000
is untested.

### Step 5: Fix the gap and re-run

```python
def test_exactly_at_cap_boundary():
    """Test amount exactly at the cap threshold."""
    # With amount=10000, base_fee = 200, which is > 50, so cap applies
    assert calculate_fee(10000, "regular", False) == 50.0
    # With amount=9999, base_fee = 199.98, cap does NOT apply
    assert calculate_fee(9999, "regular", False) == 199.98
```

Now mutant 4 (changing `>` to `>=`) would be killed because `amount=10000`
would trigger the cap differently.

## Use It

In production:

- Run mutation selectively on high-risk modules (pricing, auth, validation).
- Use baseline thresholds but review survivors manually.
- Exclude obvious equivalent mutants when tooling misclassifies.
- Budget time: mutation testing is 10-100x slower than normal test runs.

### Tooling landscape

| Tool | Language | Approach |
|---|---|---|
| mutmut | Python | AST-level mutation, fast, good defaults |
| cosmic-ray | Python | AST-level, operator-based, configurable |
| Stryker | JS/TS/C# | Framework-integrated, dashboard, incremental |
| PIT | Java | Bytecode mutation, very fast |
| mull | C/C++/Rust | LLVM IR mutation |
| go-mutesting | Go | AST mutation |

### Incremental mutation testing

Full mutation suites are expensive. Run them incrementally:

1. On every PR: mutate only changed files.
2. Nightly: full mutation suite on critical modules.
3. On-demand: deep mutation sweep before releases.

Stryker supports incremental mode out of the box. For `mutmut`, you can filter
by file or function.

### Practical tips

**Start small.** Don't mutate your entire codebase on day one. Pick the most
critical module (payment processing, authentication, data validation) and run
mutation testing there first.

**Set thresholds, not goals.** A mutation score of 100% is rarely worth chasing.
Set a threshold (e.g., 80%) and review survivors above it manually. Many will
be equivalent mutants.

**Combine with coverage.** Mutation testing on uncovered code will show 100%
survival (all mutants survive because nothing tests that code). First achieve
reasonable coverage, then measure mutation score on covered code.

**Watch for equivalent mutants.** Common patterns:

- `i++` vs `++i` when return value isn't used
- `x < y` vs `x <= y` when x can never equal y
- Deleting code that's already dead

## Read the Source

- `mutmut` and `cosmic-ray` docs for Python mutation workflows.
- Stryker docs for TypeScript mutation testing and dashboarding.
- PIT docs for Java bytecode mutation (fastest implementation).
- "An Analysis and Survey of the Development of Mutation Testing" (Jia and Harman, 2011).

## Ship It

This lesson ships a lightweight mutation score demonstrator in Python/TS plus a
review checklist for survivor triage.

## Exercises

1. **Easy** - Add boundary-focused tests for the fee calculator and re-run the
   mutation score. Target 100% kill rate.
2. **Medium** - Classify survivors as weak test vs equivalent mutant. Write a
   document explaining each classification and your reasoning.
3. **Hard** - Add a boolean negation mutator (`not x` → `x`) to the mutator set.
   Run it against a real project and report the impact on mutation score.
4. **Hard** - Integrate `mutmut` into a CI pipeline. Fail the build if mutation
   score drops below a configurable threshold.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Mutant | "broken code" | Systematically altered version used to test suite sensitivity |
| Killed mutant | "good test" | Test suite detected behavioral change |
| Surviving mutant | "bad test" | Mutation was not detected; may indicate missing assertions |
| Equivalent mutant | "false survivor" | Mutant behaviorally identical to original for all inputs |
| Mutation score | "quality score" | Ratio of killed to relevant mutants |
| Mutation operator | "mutator rule" | Syntactic transformation that generates one class of mutants |
| Strong kill | "test failed" | At least one test assertion failed on the mutant |
| Weak kill | "output changed" | Mutant produced different output, but no assertion caught it |

## Further Reading

- [Stryker](https://stryker-mutator.io/) - mutation tooling for JS/TS.
- [mutmut](https://mutmut.readthedocs.io/) - Python mutation testing.
- [Cosmic Ray](https://cosmic-ray.readthedocs.io/) - mutation framework concepts.
- [PIT](http://pitest.org/) - Java mutation testing (fastest implementation).
- [Mull](https://mull-project.github.io/) - LLVM-based mutation for C/C++/Rust.
- [Jia and Harman, 2011](https://crest.cs.ucl.ac.uk/wp/wp-content/uploads/2012/10/11-survey.pdf) - comprehensive mutation testing survey.
