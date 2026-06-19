# Contracts and Design by Contract

> State your obligations explicitly so violations fail early and locally.

**Type:** Learn
**Languages:** Python, Rust
**Prerequisites:** Phase 17 lessons 01-07
**Time:** ~60 minutes

## Learning Objectives

- Define preconditions, postconditions, and invariants precisely.
- Apply runtime contract checks to reduce ambiguous failures.
- Balance contract strictness with ergonomics and performance.
- Connect contracts with testing and formal verification.

## The Problem

Without contracts, assumptions hide in comments and developer memory. Bugs then
surface deep in call stacks as vague exceptions. A payment service calls
`transfer(from_acct, to_acct, amount)` and assumes `amount > 0`. But nothing
in the function signature enforces that. Six months later, a new endpoint passes
a user-supplied negative amount. The balance goes up instead of down. The bug
is found weeks later during reconciliation.

This pattern repeats across every codebase:

- API requires non-empty account ID, but the check lives in a comment.
- Transfer amount must be positive, but the validation is in a different module.
- Ledger total must remain conserved, but nothing enforces it after batch ops.

Contract style makes assumptions explicit and checkable at boundaries. When
checks are explicit, failures occur at the source of violation with clearer
ownership and debugging speed. Instead of "balance is wrong somewhere," you get
"precondition violated in transfer(): amount must be positive, got -50."

## The Concept

### Contract Layers

```
    Caller                    Callee
    ──────                    ──────
    
    ┌─────────────────────────────────┐
    │  Precondition                   │
    │  "What caller must provide"     │
    │  - amount > 0                   │
    │  - account_id is valid          │
    │  - currencies match             │
    └────────────┬────────────────────┘
                 │
                 ▼
    ┌─────────────────────────────────┐
    │  Function executes              │
    │  Can assume preconditions hold  │
    └────────────┬────────────────────┘
                 │
                 ▼
    ┌─────────────────────────────────┐
    │  Postcondition                  │
    │  "What callee guarantees"       │
    │  - result.balance >= 0          │
    │  - total funds conserved        │
    │  - transaction recorded         │
    └─────────────────────────────────┘
    
    ┌─────────────────────────────────┐
    │  Invariant                      │
    │  "What always holds"            │
    │  - account.balance >= 0         │
    │  - ledger.entries is append-only│
    └─────────────────────────────────┘
```

**Preconditions** are the caller's obligations. If a precondition fails, the
caller is at fault. Example: "you must pass a positive amount to `transfer()`."

**Postconditions** are the callee's guarantees. If a postcondition fails, the
callee has a bug. Example: "after `transfer()` returns, the sum of all balances
is unchanged."

**Invariants** are properties that hold across the entire lifetime of an object
or system. Example: "no account balance is ever negative." Invariants are
checked after every public method call.

### Contracts vs Tests

Tests sample behaviors. Contracts continuously enforce assumptions at runtime.
They complement each other:

| Aspect | Tests | Contracts |
|---|---|---|
| When checked | During test runs | Every call (at boundaries) |
| What they catch | Known scenarios | Violated assumptions |
| Who's responsible | Test writer | Caller/callee boundary |
| Failure message | "expected X, got Y" | "precondition violated: amount > 0" |
| Cost | Zero in production | Runtime overhead (configurable) |

### Eiffel: The Original

Eiffel, designed by Bertrand Meyer in 1985, built Design by Contract into the
language itself. Every routine has `require` (preconditions), `ensure`
(postconditions), and classes have `invariant` blocks. Violations trigger
exceptions with clear messages about who broke the contract.

```eiffel
transfer (amount: INTEGER; target: ACCOUNT)
    require
        amount > 0
        target /= Void
        currency_matches: currency = target.currency
    do
        -- implementation
    ensure
        balance = old balance - amount
        target.balance = old target.balance + amount
    end
```

The `old` keyword refers to the pre-state value. This makes postconditions
precise: you can express exactly how state changes.

### Rust: Contracts via the Type System

Rust doesn't have built-in contract syntax, but its type system encodes many
contracts at compile time:

- `Result<T, E>` encodes "this function can fail" as a postcondition.
- `&mut` references encode "exclusive access" as a precondition.
- `Option<T>` encodes "value might be absent" instead of null checks.
- `debug_assert!` provides runtime contract checking in debug builds.

```rust
fn transfer(from: &mut Account, to: &mut Account, amount: u64) -> Result<(), TransferError> {
    // Preconditions encoded as runtime checks
    if amount == 0 {
        return Err(TransferError::ZeroAmount);
    }
    if from.balance < amount {
        return Err(TransferError::InsufficientFunds);
    }
    
    from.balance -= amount;
    to.balance += amount;
    
    // Postcondition: invariant check
    debug_assert!(from.balance <= from.balance + amount + to.balance,
        "Conservation violated after transfer");
    
    Ok(())
}
```

### Overuse Warning

Contracts have costs:

- **Performance:** Checking every precondition on every call in a hot loop
  hurts. Keep heavy checks at trust boundaries (API entry points, data deserialization).
- **Noise:** Too many assertions make code hard to read. Focus on the
  assumptions that would cause the worst bugs if violated.
- **Redundancy:** If the type system already enforces a constraint (e.g.,
  non-null via `Option`), don't add a runtime check too.

## Build It

### Step 1: Implement `transfer` with precondition checks

```python
class InsufficientFunds(Exception):
    pass

class Account:
    def __init__(self, account_id: str, balance: float, currency: str = "USD"):
        # Preconditions
        assert account_id, "account_id must be non-empty"
        assert balance >= 0, f"initial balance must be non-negative, got {balance}"
        
        self.account_id = account_id
        self.balance = balance
        self.currency = currency
    
    def _check_invariant(self):
        """Class invariant: balance must be non-negative."""
        assert self.balance >= 0, f"Invariant violated: {self.account_id} balance = {self.balance}"

def transfer(from_acct: Account, to_acct: Account, amount: float) -> None:
    """Transfer funds between accounts.
    
    Preconditions:
        - amount > 0
        - from_acct.currency == to_acct.currency
        - from_acct.balance >= amount
    Postconditions:
        - from_acct.balance decreased by amount
        - to_acct.balance increased by amount
        - total funds conserved
    """
    # Preconditions
    assert amount > 0, f"Precondition: amount must be positive, got {amount}"
    assert from_acct.currency == to_acct.currency, \
        f"Precondition: currency mismatch {from_acct.currency} vs {to_acct.currency}"
    assert from_acct.balance >= amount, \
        f"Precondition: insufficient funds, have {from_acct.balance} need {amount}"
    
    # Capture pre-state for postcondition
    old_from_balance = from_acct.balance
    old_to_balance = to_acct.balance
    
    # Execute
    from_acct.balance -= amount
    to_acct.balance += amount
    
    # Postconditions
    assert from_acct.balance == old_from_balance - amount, "Postcondition: from balance incorrect"
    assert to_acct.balance == old_to_balance + amount, "Postcondition: to balance incorrect"
    assert from_acct.balance + to_acct.balance == old_from_balance + old_to_balance, \
        "Postcondition: funds not conserved"
    
    # Invariants
    from_acct._check_invariant()
    to_acct._check_invariant()
```

### Step 2: Test contract violations

```python
import pytest

def test_positive_transfer():
    a = Account("A1", 1000)
    b = Account("B2", 500)
    transfer(a, b, 100)
    assert a.balance == 900
    assert b.balance == 600

def test_negative_amount_violates_precondition():
    a = Account("A1", 1000)
    b = Account("B2", 500)
    with pytest.raises(AssertionError, match="amount must be positive"):
        transfer(a, b, -100)

def test_insufficient_funds_violates_precondition():
    a = Account("A1", 50)
    b = Account("B2", 500)
    with pytest.raises(AssertionError, match="insufficient funds"):
        transfer(a, b, 100)

def test_currency_mismatch_violates_precondition():
    a = Account("A1", 1000, "USD")
    b = Account("B2", 500, "EUR")
    with pytest.raises(AssertionError, match="currency mismatch"):
        transfer(a, b, 100)
```

### Step 3: Rust version with Result-based errors

```rust
#[derive(Debug)]
enum TransferError {
    ZeroAmount,
    InsufficientFunds,
    CurrencyMismatch,
}

struct Account {
    id: String,
    balance: f64,
    currency: String,
}

impl Account {
    fn new(id: &str, balance: f64, currency: &str) -> Self {
        assert!(!id.is_empty(), "account_id must be non-empty");
        assert!(balance >= 0.0, "initial balance must be non-negative");
        Account {
            id: id.to_string(),
            balance,
            currency: currency.to_string(),
        }
    }
    
    fn check_invariant(&self) {
        debug_assert!(self.balance >= 0.0,
            "Invariant violated: {} balance = {}", self.id, self.balance);
    }
}

fn transfer(from: &mut Account, to: &mut Account, amount: f64) -> Result<(), TransferError> {
    // Preconditions
    if amount <= 0.0 {
        return Err(TransferError::ZeroAmount);
    }
    if from.currency != to.currency {
        return Err(TransferError::CurrencyMismatch);
    }
    if from.balance < amount {
        return Err(TransferError::InsufficientFunds);
    }
    
    let old_total = from.balance + to.balance;
    
    from.balance -= amount;
    to.balance += amount;
    
    // Postcondition (debug only)
    debug_assert!((from.balance + to.balance - old_total).abs() < 1e-10,
        "Conservation violated");
    
    from.check_invariant();
    to.check_invariant();
    
    Ok(())
}
```

## Use It

In production:

- **Keep contracts in domain/API boundaries.** Don't assert inside every helper
  function. Assert at the entry points where external data enters your system.
- **Use feature flags to tune expensive assertions in release builds.** Rust's
  `debug_assert!` is compiled out in release mode. Python's `-O` flag skips
  `assert` statements. For production, consider explicit `if` checks that log
  rather than crash.
- **Convert recurring contract failures into explicit tests.** If a
  precondition fails in production, add it to your test suite so it's caught
  earlier next time.

Production references:

- Eiffel's DbC model is used in aerospace and finance for safety-critical code.
- Rust's type system encodes many contracts at compile time, reducing runtime
  overhead.
- Microsoft's Code Contracts library (deprecated but influential) for .NET.

## Read the Source

- [Eiffel and DbC references](https://www.eiffel.org/doc/eiffel/ET-_Design_by_Contract_%28tm%29%2C_Assertions_and_Exceptions) — the original contract model.
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html) — explicit failure contracts via `Result`.
- [Bertrand Meyer's OOSC](https://se.ethz.ch/~meyer/publications/object_oriented/) — the book that introduced DbC.

## Ship It

This lesson ships:

- `code/main.py`: Python transfer example with pre/post/invariant checks.
- `code/main.rs`: Rust transfer example with `Result`-based contracts.
- `outputs/README.md`: boundary-contract checklist for API design.

## Quiz

**Pre-questions:**

**Q1.** A function has a precondition `amount > 0`. A caller passes `-50`. Who
is at fault?

- A) The function, for not handling negative amounts.
- B) The caller, for violating the precondition.
- C) Both, for not communicating clearly.
- D) Neither, this is a design issue.

**Answer: B.** Preconditions define the caller's obligations. If the contract
says `amount > 0` and the caller passes `-50`, the caller broke the contract.
The function is not required to handle inputs that violate its precondition.
This is the core principle of DbC: responsibilities are explicit.

**Q2.** How do contracts differ from tests?

- A) Contracts are written by developers, tests by QA.
- B) Contracts enforce assumptions at every call; tests sample specific scenarios.
- C) Contracts replace the need for tests.
- D) Contracts only work in statically typed languages.

**Answer: B.** Tests check specific inputs and expected outputs during test
runs. Contracts check assumptions (preconditions, postconditions, invariants)
at every call during runtime. They complement each other: tests verify known
scenarios, contracts catch unexpected violations in production.

**Post-questions:**

**Q3.** You add `assert balance >= 0` as an invariant to an Account class. In
production, you run Python with `-O` (optimize flag). What happens?

- A) The assertion still runs and crashes on violations.
- B) The assertion is silently removed; violations go undetected at runtime.
- C) The assertion logs a warning instead of crashing.
- D) Python raises a syntax error.

**Answer: B.** Python's `-O` flag strips all `assert` statements. If your
contracts rely on `assert`, they vanish in optimized builds. For production
contracts, use explicit `if` checks that log or raise dedicated exceptions,
rather than bare `assert`.

**Q4.** When should you NOT add a contract check?

- A) At API boundaries where external data enters.
- B) Inside a hot loop called millions of times per second.
- C) Before modifying shared state.
- D) When the type system already enforces the constraint.

**Answer: B.** Contracts have runtime cost. In hot loops, the overhead of
checking preconditions on every iteration can dominate execution time. Move
the check to the loop boundary (check once before the loop) or use
compile-time enforcement (types) instead.

**Q5.** A postcondition says `result.balance >= 0`. After `transfer()` returns,
`result.balance` is `-10`. What does this mean?

- A) The caller passed bad data.
- B) The function has a bug.
- C) The precondition was violated.
- D) The invariant is too strict.

**Answer: B.** Postconditions are the callee's guarantees. If the function
returns and its postcondition is violated, the function implementation is
buggy. The caller upheld its preconditions; the function failed to deliver
its promised outcome.

## Exercises

**Easy:** Add a currency compatibility precondition to `transfer()`. Write
tests that verify the precondition fires when currencies differ.

**Medium:** Add an invariant audit over batch operations. Write a
`batch_transfer()` function that processes a list of transfers. After all
transfers complete, verify that the total funds across all accounts are
conserved. If not, roll back all changes.

**Hard:** Convert one contract failure class into a compile-time type
constraint. For example, instead of checking `amount > 0` at runtime, create
a `PositiveAmount` type that can only be constructed with a positive value.
Show how this eliminates the runtime check entirely.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Precondition | "input check" | Caller obligation before function execution |
| Postcondition | "output check" | Callee guarantee after successful execution |
| Invariant | "always true rule" | Property that must hold across object/system lifetime |
| Defensive programming | "lots of if statements" | Guarding assumptions with actionable boundary checks |
| Design by Contract | "DbC" | Meyer's methodology of explicit caller/callee obligations |
| Class invariant | "object consistency" | Property that holds after every public method call |
| Contract violation | "assertion failure" | When a precondition, postcondition, or invariant is not satisfied |

## Further Reading

- [Design by Contract (Eiffel)](https://www.eiffel.org/doc/eiffel/ET-_Design_by_Contract_%28tm%29%2C_Assertions_and_Exceptions) — the original DbC model.
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html) — explicit failure contracts via `Result`.
- [Bertrand Meyer, Object-Oriented Software Construction](https://se.ethz.ch/~meyer/publications/object_oriented/) — the foundational text on DbC.
- [SPARK Ada](https://www.adacore.com/about-spark) — industrial-strength contracts for safety-critical Ada code.
