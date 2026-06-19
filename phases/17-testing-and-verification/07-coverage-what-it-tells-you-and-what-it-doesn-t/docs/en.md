# Coverage - What It Tells You and What It Doesn't

> Coverage tells where execution went, not whether your claims are strong.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 17 lessons 01-06
**Time:** ~45 minutes

## Learning Objectives

- Interpret line, branch, and path coverage correctly.
- Explain limits of coverage as a confidence signal.
- Pair coverage with complementary evidence (mutation, properties, contracts).
- Build a practical coverage policy that avoids gaming.

## The Problem

Teams often set a blunt gate: "80% coverage required." Engineers then write tests
that execute lines but assert little, or avoid difficult code behind exclusion
annotations. Dashboard looks green while defects escape.

Consider a payment processor with this function:

```python
def apply_discount(price, discount_pct):
    if discount_pct < 0:
        raise ValueError("Negative discount")
    if discount_pct > 100:
        raise ValueError("Discount exceeds 100%")
    return price * (1 - discount_pct / 100)
```

A test that calls `apply_discount(100, 10)` hits both the happy-path line and
the two `if` lines (they evaluate false and skip). That's 100% line coverage.
But it never checks what happens with `discount_pct = 0`, `discount_pct = 100`,
or floating-point rounding at `discount_pct = 33.33`. The assertions could be
`assert result is not None` and coverage still reads green.

Coverage is useful but narrow. Misused, it rewards volume of executed lines rather
than semantic confidence. The dashboard becomes a vanity metric.

## The Concept

### Coverage Dimensions

```
                        Coverage Hierarchy
                        
    Line          Branch           Path           Mutation
    ────          ──────           ────           ────────
    "Did it       "Did both        "Did every     "Can tests
     run?"         outcomes         combination    catch small
                    run?"           run?"          changes?"
    
    ████░░░░      ██████░░         ████████░      ██████████
    ~60% easy     ~75% effort      ~90% hard      ~95% very hard
    
    Cheapest      Good enough      Combinatorial  Strongest
    signal        for most code     explosion      signal
```

**Line coverage** reports which lines executed. A function with 20 lines where
tests execute 16 has 80% line coverage. This tells you nothing about whether
the 16 lines produced correct outputs.

**Branch coverage** reports which decision outcomes executed. Every `if`, `elif`,
`else`, `for`, `while`, `try/except` has at least two outcomes (true/false,
entered/skipped, exception/normal). Branch coverage says: did you exercise both
sides?

**Path coverage** reports combinations of branches through a function. A function
with three independent `if` statements has 2^3 = 8 paths. Full path coverage is
exponential and impractical for real code. Tools approximate with bounded path
depth.

**Condition/decision coverage** (MC/DC) is used in avionics (DO-178C). It requires
each boolean sub-expression to independently affect the outcome. This catches
cases like `if (a and b)` where only `a=true, b=true` and `a=false, b=false`
are tested (both branches covered, but `b` never independently controls the result).

### What Coverage Cannot Tell You

```
    Coverage answers:              Coverage does NOT answer:
    ─────────────────              ─────────────────────────
    "Was this line executed?"      "Was the output correct?"
    "Was this branch taken?"       "Were invariants preserved?"
    "Was this path explored?"      "Were race conditions found?"
    "Was this function called?"    "Was the assertion meaningful?"
```

A test with `assert True` after every function call achieves 100% coverage
on the lines it touches. Coverage tools report execution, not semantic quality.

### The Gaming Problem

Engineers under coverage pressure learn to game metrics:

- Add `# pragma: no cover` to hard-to-test code.
- Write tests that call functions without meaningful assertions.
- Extract complex logic into untested utility modules.
- Test getters and setters instead of business logic.

The result: high coverage number, low actual confidence.

### Complementary Signals

Coverage works best as one signal among several:

| Signal | What it measures | Cost | Strength |
|---|---|---|---|
| Line coverage | Execution breadth | Low | Finds untested code |
| Branch coverage | Decision completeness | Low | Finds untested outcomes |
| Mutation score | Assertion quality | Medium | Finds weak assertions |
| Property tests | Behavioral invariants | Medium | Finds edge cases |
| Contracts | Runtime assumptions | Low-Medium | Finds boundary violations |
| Fuzzing | Crash resistance | Medium | Finds unexpected inputs |

## Build It

We build a coverage analysis exercise that shows exactly where coverage misleads.

### Step 1: Define a branchy function

```python
def classify_transaction(amount, account_type, is_international):
    """Classify a banking transaction into fee tiers."""
    fee = 0.0
    
    if amount < 0:
        raise ValueError("Negative amount")
    
    if account_type == "premium":
        fee = 0.0
    elif account_type == "standard":
        fee = amount * 0.02
    else:
        fee = amount * 0.05
    
    if is_international:
        fee += amount * 0.01
    
    if amount > 10000:
        fee += 50  # large transfer surcharge
    
    return round(fee, 2)
```

### Step 2: Write tests that achieve 100% line coverage

```python
import pytest

def test_premium_domestic():
    assert classify_transaction(1000, "premium", False) == 0.0

def test_standard_international():
    assert classify_transaction(1000, "standard", True) == 30.0

def test_other_large():
    assert classify_transaction(15000, "business", False) == 800.0

def test_negative_raises():
    with pytest.raises(ValueError):
        classify_transaction(-100, "standard", False)
```

Running `pytest --cov=. --cov-branch` reports 100% line coverage and ~90%
branch coverage. Looks great.

### Step 3: Find what's missing

These tests miss:

- `amount = 0` (zero-fee edge case)
- `amount = 10000` (boundary: just under surcharge)
- `amount = 10001` (boundary: just over surcharge)
- `account_type = "premium"` with `is_international = True`
- Floating-point precision with amounts like `0.01`

A mutation test would catch some of these. Changing `amount > 10000` to
`amount >= 10000` would survive if no test hits the boundary exactly.

### Step 4: Add boundary tests and compare

```python
def test_zero_amount():
    assert classify_transaction(0, "standard", False) == 0.0

def test_boundary_under_surcharge():
    assert classify_transaction(10000, "standard", False) == 200.0

def test_boundary_at_surcharge():
    assert classify_transaction(10001, "standard", False) == 250.02

def test_premium_international():
    assert classify_transaction(5000, "premium", True) == 50.0
```

Same 100% line coverage. But now the assertion quality is higher. Mutation
testing would kill more mutants with these tests.

## Use It

Practical strategy for coverage in production:

1. **Use coverage to find untested areas**, not to prove quality. A coverage
   report showing 40% on a critical module is a clear signal to write more
   tests. A report showing 95% tells you almost nothing.

2. **Set branch coverage floors for critical modules.** Payment processing,
   authentication, and data migration modules should have high branch coverage.
   Utility formatting functions can tolerate lower thresholds.

3. **Pair coverage with mutation testing.** Run `mutmut` or `Stryker` on your
   highest-risk modules. If your mutation score is 60% but coverage is 90%,
   your assertions are weak.

4. **Track uncovered high-risk blocks explicitly in code reviews.** Don't just
   look at the percentage. Look at *which* lines are uncovered and whether
   they're in critical paths.

5. **Use coverage contexts** (available in `coverage.py`) to measure coverage
   per test, per feature branch, or per commit. This reveals whether new code
   ships with tests.

Production references:

- Google's testing blog: "coverage is a necessary but not sufficient condition
  for test quality."
- Facebook's Sapienz: combines coverage with mutation and search-based testing.
- Linux kernel: uses `kcov` for coverage-guided fuzzing, not quality gates.

## Read the Source

- `coverage.py` docs: [coverage.readthedocs.io](https://coverage.readthedocs.io/) — branch measurement and contexts.
- Engineering blogs on "coverage is a lagging indicator" patterns.
- [Practical Test Pyramid](https://martinfowler.com/articles/practical-test-pyramid.html) — balancing signals.

## Ship It

This lesson ships a Python script that:

1. Runs tests with `coverage.py` and reports line/branch counts.
2. Introduces deliberate mutations and checks which survive.
3. Generates a coverage policy template per module risk level.

```bash
python code/main.py
# Output: line coverage, branch coverage, mutation score comparison
```

## Quiz

**Pre-questions (answer before reading Build It):**

**Q1.** A test suite achieves 100% line coverage on a function. What can you
conclude?

- A) The function is bug-free.
- B) Every line was executed at least once during testing.
- C) All edge cases are covered.
- D) The assertions are meaningful.

**Answer: B.** Line coverage only measures execution, not correctness. A test
that calls a function and asserts nothing still achieves 100% line coverage.
You cannot conclude anything about bug-freeness, edge cases, or assertion
quality from line coverage alone.

**Q2.** Branch coverage is strictly stronger than line coverage because:

- A) It requires more tests.
- B) It ensures both true and false outcomes of each decision are exercised.
- C) It verifies output correctness.
- D) It catches race conditions.

**Answer: B.** Branch coverage requires that each decision evaluates to both
true and false at least once. This is strictly stronger than line coverage
because some branches may be on the same line (e.g., ternary expressions) and
some lines may never be reached by a single test run. It does not verify
correctness or catch concurrency bugs.

**Post-questions (answer after Build It):**

**Q3.** You have 95% line coverage and 40% mutation score. What does this
combination suggest?

- A) Your tests are comprehensive.
- B) Your tests execute most code but have weak or missing assertions.
- C) Your code is too complex to test.
- D) You need more integration tests.

**Answer: B.** High line coverage with low mutation score means tests call
most functions but don't catch small code changes. The assertions are either
too loose (`assert result is not None`) or missing entirely. Focus on
strengthening assertions, not adding more test cases.

**Q4.** Why is 100% path coverage impractical for real software?

- A) Tools don't support it.
- B) The number of paths grows exponentially with the number of branches.
- C) It requires formal verification.
- D) It only works for functional code.

**Answer: B.** A function with n independent binary decisions has 2^n paths.
A function with 20 `if` statements has over a million paths. Tools approximate
with bounded path depth or focus on feasible paths, but exhaustive path
coverage is infeasible for production code.

**Q5.** A team sets a 90% coverage gate. Engineers start adding `# pragma: no
cover` annotations. What pattern is this?

- A) Good engineering practice.
- B) Coverage gaming: moving hard-to-test code out of measurement.
- C) Appropriate use of coverage exclusions.
- D) A sign the gate is too low.

**Answer: B.** When engineers add exclusions to meet a coverage threshold
rather than because the code is genuinely untestable, they're gaming the
metric. The coverage number stays high while untested code accumulates
behind exclusions. Review exclusion annotations as carefully as you review
code.

## Exercises

**Easy:** Add tests that cover the `amount = 0` and `amount = 10000` boundary
cases for `classify_transaction`. Verify branch coverage increases.

**Medium:** Introduce a deliberate bug in `classify_transaction` (change `>` to
`>=` in the surcharge check) that preserves 100% line coverage. Write a test
that catches this bug. Explain why coverage didn't help.

**Hard:** Create a risk-weighted coverage policy. Assign each module in a small
project a risk level (critical/high/medium/low). Set different coverage and
mutation thresholds per level. Implement a script that checks the policy and
fails CI if thresholds aren't met.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Line coverage | "test completeness" | Fraction of source lines executed at least once |
| Branch coverage | "better coverage" | Fraction of decision outcomes (true/false) exercised |
| Path coverage | "thorough testing" | Fraction of execution paths through a function explored |
| Mutation score | "test quality" | Fraction of injected code changes detected by tests |
| Coverage gap | "untested code" | Lines or branches not executed by any test in the suite |
| Assertion depth | "good test" | Strength of behavioral claims checked by assertions |
| Coverage gaming | "hitting the metric" | Manipulating exclusions or writing hollow tests to inflate numbers |
| MC/DC | "aviation coverage" | Modified Condition/Decision Coverage, each sub-expression independently affects outcome |

## Further Reading

- [coverage.py](https://coverage.readthedocs.io/) — Python coverage tooling with branch and context support.
- [Practical Test Pyramid](https://martinfowler.com/articles/practical-test-pyramid.html) — Martin Fowler on balancing test signals.
- [How Google Tests Software](https://books.google.com/books/about/How_Google_Tests_Software.html) — Google's approach to coverage as one signal among many.
- [mutation-testing.org](https://mutation-testing.org/) — Community resources on mutation testing tools and practices.
