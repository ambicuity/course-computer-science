# Property-Based Testing - QuickCheck, Hypothesis

> Stop guessing examples; search for invariants and counterexamples.

**Type:** Learn
**Languages:** Haskell, Python
**Prerequisites:** Phase 17 lessons 01-03
**Time:** ~75 minutes

## Learning Objectives

- Explain the difference between example-based and property-based testing.
- Write properties that describe invariants over broad input spaces.
- Use generators and shrinking to isolate minimal counterexamples.
- Recognize weak properties that pass while behavior remains wrong.

## The Problem

Example-based tests often encode known scenarios:

- sort([3,1,2]) == [1,2,3]
- sort([]) == []
- sort([5]) == [5]

These checks are useful, but they can miss defects outside listed examples.
A bug may appear only for duplicated elements, large lists, or specific value
patterns. Developers then add more ad hoc examples, often missing the true shape
of the failing region.

### A real-world scenario

Consider a JSON serializer your team maintains. You write tests for common
objects: strings, numbers, nested arrays, null values. All green. Then a user
reports that serializing `{"key": NaN}` produces invalid JSON. Your tests never
tried `NaN` because you forgot it existed in the float domain.

Or take a date parser. You test `"2024-01-15"`, `"2000-02-29"`, `"1999-12-31"`.
All pass. But `"2024-01-00"` crashes the parser with an index-out-of-bounds
instead of returning an error. You never tested day-zero because no human would
type that. An attacker would.

The pattern repeats across domains:

| Domain | Example test | Missed failure |
|---|---|---|
| Sorting | `[3,1,2]` | Duplicate elements cause infinite loop |
| Parser | `"hello\nworld"` | Null byte in string segfaults |
| Encoder | `{"a": 1}` | Unicode keys corrupt output buffer |
| Math lib | `add(2, 3)` | Integer overflow on `add(INT_MAX, 1)` |
| State machine | valid transitions | Re-entering initial state from final state |

Each row shows the same gap: hand-picked examples cover the happy path but miss
the edges where real bugs live.

### The scaling problem

As code grows, the input space explodes combinatorially. A function that accepts
three integers each in range [0, 100] has one million input combinations. No team
writes one million test cases. They pick ten and hope.

Property-based testing flips the workflow. Instead of hand-picking fixed examples,
you define a general truth and let the engine search the space. When it finds a
failure, it shrinks the input toward the smallest counterexample, making triage
faster and root cause clearer.

## The Concept

### Property structure

A property states:

- for all generated inputs x
- predicate P(x) should hold

This borrows directly from universal quantification in formal logic. The testing
framework acts as an existential searcher: it tries to find an `x` where `P(x)`
is false.

```
  Property: forall x in Domain. P(x)

  Framework searches:
  ┌─────────────────────────────────────────────┐
  │  x1 → P(x1) = true   ✓                     │
  │  x2 → P(x2) = true   ✓                     │
  │  x3 → P(x3) = false  ✗  ← counterexample!  │
  │         ↓                                    │
  │  shrink x3 → x3' → still fails              │
  │  shrink x3' → x3'' → minimal                │
  └─────────────────────────────────────────────┘
```

Typical properties for sorting:

- output length equals input length
- output is non-decreasing
- output is a permutation of input

Together, these provide stronger guarantees than a short list of examples.

### Property categories

Not all properties test the same thing. Here's a taxonomy:

| Category | Pattern | Example |
|---|---|---|
| Round-trip | decode(encode(x)) = x | JSON parse/serialize |
| Idempotence | f(f(x)) = f(x) | sort, dedup, normalize |
| Invariant preservation | f(x) preserves some attribute | sort preserves length |
| Oracle comparison | f(x) = reference_impl(x) | new vs old algorithm |
| metamorphic | f(x) relates to f(transform(x)) | sort(reverse(x)) = sort(x) | 
| Injection | x != y implies f(x) != f(y) | unique ID generation |
| Monotonicity | x <= y implies f(x) <= f(y) | price calculations |

Choosing the right category for your function is half the work. A sort function
tested only for "output is a list" gives almost no confidence. Tested for
sortedness, permutation preservation, and idempotence together, it's robust.

### Generators

Generators control explored input space. Their design determines how useful your
search is.

```
  Generator quality spectrum:

  Low confidence ◄──────────────────────► High confidence
  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
  │ Random   │  │ Bounded  │  │ Biased   │  │ Smart    │
  │ ints     │  │ ints     │  │ edges    │  │ composed │
  │          │  │ [0,100]  │  │ 0,1,MAX  │  │ structs  │
  └──────────┘  └──────────┘  └──────────┘  └──────────┘
```

Key generator strategies:

- **Uniform random**: good baseline, misses edges.
- **Bounded ranges**: constrains search to plausible inputs.
- **Edge-biased**: weights toward 0, 1, -1, MAX, MIN, empty, huge.
- **Compositional**: builds complex structures (lists of pairs of strings).
- **Shrinking-aware**: generators that know how to simplify their output.

Hypothesis calls these "strategies." QuickCheck calls them "Arbitrary instances."
Same idea, different API surface.

### Shrinking

When a property fails, shrinkers try smaller/simpler inputs that still fail.

```
  Initial failure: [847, -23, 0, 999999, -1, 42, 0, -1]
                          ↓ shrink
  Try: [847, -23, 0, 999999]     ← still fails
                          ↓ shrink
  Try: [0, -1]                   ← still fails
                          ↓ shrink
  Try: [0, -1]                   ← minimal, can't shrink further
```

Shrinking dramatically improves debugging speed. Without it, you'd stare at a
list of 400 random integers wondering which ones matter. With it, the framework
hands you `[0, -1]` and the bug becomes obvious.

Shrinking works differently in each framework:

- **Hypothesis**: integrated into every strategy, aggressively tries smaller values.
- **QuickCheck**: requires explicit shrink implementations per type, more control.
- **fast-check** (JS): similar to Hypothesis with built-in shrinking.

### Good vs weak properties

Weak property:

- "result is list" for a sort function.

Strong properties:

- sortedness
- permutation preservation
- idempotence: sort(sort(x)) = sort(x)

Here's how to evaluate property strength:

| Property | Catches | Misses |
|---|---|---|
| `isinstance(result, list)` | Wrong return type | Everything else |
| `len(result) == len(input)` | Dropped elements | Order, duplicates |
| `sorted(result) == result` | Wrong ordering | Lost elements |
| `multiset(result) == multiset(input)` | Lost/duplicated elements | Wrong order |
| All three together | Most sort bugs | Very unlikely |

The lesson: a single property is usually weak. Combine three or four that
cover different failure modes.

### Common pitfalls

**Pitfall 1: Properties that mirror the implementation.**

If your property calls the same buggy function you're testing, it will pass
even when the function is wrong. Always test against an independent definition
of correctness (a simpler implementation, a specification, or a different
algorithm).

**Pitfall 2: Overly constrained generators.**

If your generator only produces sorted lists, you'll never find bugs in your
sort function. Generators should cover the full input domain, including
edge cases.

**Pitfall 3: Flaky properties from floating point.**

Floating-point arithmetic is not associative. `(a + b) + c != a + (b + c)` in
general. Don't write properties that assume exact floating-point equality. Use
epsilon comparisons or test exact properties (like "result is finite").

**Pitfall 4: State leakage across runs.**

If your property modifies global state (files, databases, static variables),
runs interfere with each other. Each property invocation should be independent.

**Pitfall 5: Ignoring distribution coverage.**

A generator that produces `[1, 1, 1, 1, 1]` 90% of the time is technically
random but practically useless. Monitor what your generator actually produces.

### Where PBT excels

- parser/serializer round trips
- algebraic laws (monoid, functor, monad laws)
- data-structure invariants (BST ordering, heap property)
- state machine command sequences (push/pop, connect/disconnect)
- codec correctness (encode then decode returns original)
- concurrent data structure linearizability

### Where PBT is not enough alone

- external side effects requiring real integration checks
- UX correctness (visual regression, accessibility)
- performance SLO verification
- security properties requiring adversarial reasoning
- properties that are hard to state concisely

## Build It

We build property suites for a small sorting example in Python and Haskell.

### Step 1: Start from examples

Write a few example tests to frame expected behavior.

```python
# test_sort_examples.py
from my_sort import my_sort

def test_empty():
    assert my_sort([]) == []

def test_single():
    assert my_sort([42]) == [42]

def test_sorted():
    assert my_sort([1, 2, 3]) == [1, 2, 3]

def test_reverse():
    assert my_sort([3, 2, 1]) == [1, 2, 3]
```

These pass. They tell you nothing about `[3, 1, 2, 3]` or lists with negative
numbers.

### Step 2: Add core properties

```python
# test_sort_properties.py
from hypothesis import given, strategies as st

@given(st.lists(st.integers()))
def test_output_is_sorted(xs):
    result = my_sort(xs)
    assert all(result[i] <= result[i+1] for i in range(len(result) - 1))

@given(st.lists(st.integers()))
def test_preserves_length(xs):
    assert len(my_sort(xs)) == len(xs)

@given(st.lists(st.integers()))
def test_is_permutation(xs):
    assert sorted(my_sort(xs)) == sorted(xs)

@given(st.lists(st.integers()))
def test_idempotent(xs):
    assert my_sort(my_sort(xs)) == my_sort(xs)
```

In Haskell with QuickCheck:

```haskell
-- SortProperties.hs
import Test.QuickCheck
import Data.List (sort)

prop_sorted :: [Int] -> Bool
prop_sorted xs = let result = mySort xs
                 in all (\(a,b) -> a <= b) (zip result (tail result))

prop_length :: [Int] -> Bool
prop_length xs = length (mySort xs) == length xs

prop_idempotent :: [Int] -> Bool
prop_idempotent xs = mySort (mySort xs) == mySort xs

-- Run with: quickCheck prop_sorted
```

### Step 3: Add edge-focused generators

Bias generation toward duplicates, zeros, and already sorted/reverse inputs.

```python
# Custom strategy with edge bias
from hypothesis import strategies as st

edge_integers = st.sampled_from([0, 1, -1, 42, -42])

biased_lists = st.one_of(
    st.lists(st.integers()),           # normal random
    st.lists(edge_integers),           # edge values only
    st.lists(st.integers(), min_size=0, max_size=0),  # empty
    st.lists(st.integers(), min_size=1000),            # large
)

@given(biased_lists)
def test_sorted_biased(xs):
    result = my_sort(xs)
    assert all(result[i] <= result[i+1] for i in range(len(result) - 1))
```

### Step 4: Observe and shrink failures

Introduce a buggy sort implementation and inspect shrunk counterexample.

```python
def buggy_sort(xs):
    """Bug: drops duplicates."""
    return list(set(xs))  # wrong: loses multiplicity

@given(st.lists(st.integers()))
def test_preserves_multiplicity(xs):
    result = buggy_sort(xs)
    assert sorted(result) == sorted(xs)  # fails!
```

Hypothesis output:

```
Falsifying example: test_preserves_multiplicity(
    xs=[0, 0],
)
```

The framework shrunk from a list of hundreds of integers to `[0, 0]`. Two
elements, both zero. The bug (using `set`) is now obvious.

### Step 5: Integrate with CI

Run PBT with bounded examples per commit and larger sweeps nightly.

```yaml
# CI config
- name: Fast property tests
  run: pytest test_sort_properties.py --hypothesis-seed=0 -x

- name: Nightly deep sweep
  if: github.event_name == 'schedule'
  run: |
    pytest test_sort_properties.py \
      --hypothesis-seed=$RANDOM \
      --hypothesis-max-examples=10000
```

Fixing the seed in CI makes failures reproducible. Nightly runs with more
examples catch rare edge cases.

## Use It

Production teams typically combine:

- deterministic unit regression tests for known bugs
- property suites for broad invariant search
- fuzzing for malformed/input robustness

QuickCheck and Hypothesis both support this style:

- **Hypothesis**: strategy-rich Python ecosystem and excellent shrinking. Integrates
  with pytest, Django, and has database-backed example replay.
- **QuickCheck**: concise algebraic properties in Haskell. The original PBT tool,
  influential on everything that followed.
- **fast-check**: JavaScript/TypeScript port with similar ergonomics to Hypothesis.
- **PropTest**: Rust property testing with shrinking, inspired by QuickCheck.
- **JUnit-QuickCheck**: Java property testing via JUnit integration.

### Real-world adoption

| Company | Tool | Use case |
|---|---|---|
| Basho (Riak) | QuickCheck | Distributed KV store protocol testing |
| Volvo | QuickCheck | CAN bus message parsing |
| Dropbox | Hypothesis | Sync engine state machine verification |
| Stripe | Hypothesis | Payment calculation edge cases |
| Mozilla | QuickCheck + fuzzing | Firefox media parser security |

### Property-based testing vs fuzzing

These techniques overlap but serve different goals:

| Aspect | PBT | Fuzzing |
|---|---|---|
| Input generation | Typed, structured | Raw bytes or structured |
| Focus | Logical correctness | Memory safety, crashes |
| Shrinking | Built-in, semantic | Minimization, syntactic |
| Feedback loop | Property violations | Coverage + crashes |
| Integration | Unit test framework | Standalone harness |

Best practice: use both. PBT for correctness properties, fuzzing for robustness.

## Read the Source

- Hypothesis docs and strategy internals for generator design.
- QuickCheck papers and package docs for shrinking and laws.
- Property suites in mature parser/serialization libraries.
- Erlang's PropEr for stateful property testing of concurrent systems.

## Ship It

This lesson ships:

- `code/main.py`: property demonstrations and a purposeful buggy function.
- `code/Main.hs`: QuickCheck properties for sorting invariants.
- `outputs/README.md`: property design checklist for future lessons.

## Exercises

1. **Easy** - Add a round-trip property for parse/serialize of a small DSL.
2. **Medium** - Define generator distributions explicitly and compare failure
   discovery rates across random vs biased strategies.
3. **Hard** - Build a stateful property for a queue API with push/pop commands
   using Hypothesis's `RuleBasedStateMachine`.
4. **Hard** - Add mutation testing to verify properties catch seeded defects.
   Introduce five bugs and check which properties detect each one.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Property | "Random test" | Universal claim over generated inputs |
| Generator | "Input randomizer" | Structured input distribution for search |
| Shrinking | "Smaller failing case" | Systematic reduction to minimal counterexample |
| Invariant | "Rule" | Predicate expected to hold for all valid executions |
| Idempotence | "No-op repeat" | Applying operation twice yields same result as once |
| Counterexample | "Failing input" | Concrete input violating a property |
| Strategy bias | "Edge coverage trick" | Deliberate weighting of high-risk input regions |
| Law testing | "Mathy tests" | Verifying algebraic properties of APIs or data structures |
| Metamorphic testing | "Relation testing" | Testing via relationships between outputs of related inputs |

## Further Reading

- [Hypothesis Documentation](https://hypothesis.readthedocs.io/) - strategies, shrinking, stateful testing.
- [QuickCheck Package](https://hackage.haskell.org/package/QuickCheck) - property testing for Haskell.
- [Claessen and Hughes: QuickCheck](https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quick.pdf) - foundational paper.
- [Property-based Testing in Practice](https://increment.com/testing/in-praise-of-property-based-testing/) - practical engineering perspective.
- [fast-check](https://github.com/dubzzz/fast-check) - property testing for JavaScript/TypeScript.
- [PropTest](https://proptest-rs.github.io/proptest/) - Rust property testing framework.
