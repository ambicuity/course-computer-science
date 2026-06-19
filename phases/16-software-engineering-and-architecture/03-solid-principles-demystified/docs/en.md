# SOLID Principles — Demystified

> Five principles that make code survive contact with the next developer.

**Type:** Learn
**Languages:** TypeScript, Python
**Prerequisites:** Phase 16 lessons 01–02
**Time:** ~60 minutes

## Learning Objectives

- Name all five SOLID principles and explain each in one sentence.
- Spot violations in real code — and articulate *why* they hurt.
- Apply each principle through before/after refactorings in Python and TypeScript.
- Recognize that SOLID is about **dependency management**, not aesthetic purity.

## The Problem

You join a project where a `UserManager` class creates users, validates emails, sends welcome emails, writes audit logs, and generates PDF reports. When the email provider changes, you must modify the same class that handles user creation. When the PDF format changes, that same class moves again.

Every change touches the same file. Every merge conflicts with every other feature team. Bugs in email formatting break user creation. This is not a scaling problem — it is a **dependency** problem. SOLID gives you five lenses for spotting and fixing it.

## Mental Model

Think of SOLID as five answers to one question: **who depends on what, and how tightly?**

| Principle | Dependency question |
|-----------|-------------------|
| **S** — Single Responsibility | Does this module have more than one reason to change? |
| **O** — Open/Closed | Must I edit existing code to add new behavior? |
| **L** — Liskov Substitution | Can a subclass stand in for its parent without surprises? |
| **I** — Interface Segregation | Am I forced to depend on methods I never call? |
| **D** — Dependency Inversion | Am I wired to a concrete class I could abstract away? |

All five push you toward **loose coupling** — code that changes independently, tests that run in isolation, modules you can reason about without holding the whole system in your head.

---

## 1. Single Responsibility Principle (SRP)

> A class should have **one reason to change**.

### Violation: The God Class

```python
class Employee:
    def __init__(self, name, salary):
        self.name = name
        self.salary = salary

    def calculate_pay(self):
        return self.salary * 1.1   # tax logic mixed in

    def save_to_database(self, db):
        db.execute(f"INSERT INTO employees VALUES ('{self.name}', {self.salary})")

    def generate_report(self):
        return f"Employee: {self.name}, Salary: {self.salary}"

    def send_payslip_email(self, smtp):
        smtp.send(to=self.name + "@co.com", body=self.generate_report())
```

Three different actors cause changes here:
- **Payroll policy** changes `calculate_pay`
- **DBA schema migration** changes `save_to_database`
- **Design team** changes `generate_report` format

All three touch the same file. Any one of them can break the others.

### Fix: One responsibility per class

```python
class Employee:
    def __init__(self, name, salary):
        self.name = name
        self.salary = salary

class PayCalculator:
    def calculate(self, employee):
        return employee.salary * 1.1

class EmployeeRepository:
    def save(self, db, employee):
        db.execute("INSERT INTO employees VALUES (?, ?)", (employee.name, employee.salary))

class ReportFormatter:
    def format(self, employee):
        return f"Employee: {employee.name}, Salary: {employee.salary}"
```

Now payroll changes stay in `PayCalculator`, schema changes in `EmployeeRepository`, formatting in `ReportFormatter`. Merge conflicts vanish. Tests isolate each concern.

### Why it hurts when violated

- Touching one responsibility risks breaking another.
- Merge conflicts increase linearly with the number of actors.
- Tests for one responsibility must set up state for all responsibilities.

---

## 2. Open/Closed Principle (OCP)

> Open for **extension**, closed for **modification**.

### Violation: The growing switch

```python
def calculate_discount(customer_type, total):
    if customer_type == "regular":
        return total * 0.95
    elif customer_type == "premium":
        return total * 0.9
    elif customer_type == "vip":
        return total * 0.8
    # Adding "student" requires editing this function again
```

Every new customer type means editing the same function. That is a modification, not an extension. Every edit risks introducing a regression for existing types.

### Fix: Strategy pattern — extend by adding, not editing

```python
from abc import ABC, abstractmethod

class DiscountStrategy(ABC):
    @abstractmethod
    def apply(self, total: float) -> float: ...

class RegularDiscount(DiscountStrategy):
    def apply(self, total): return total * 0.95

class PremiumDiscount(DiscountStrategy):
    def apply(self, total): return total * 0.9

class VIPDiscount(DiscountStrategy):
    def apply(self, total): return total * 0.8

class StudentDiscount(DiscountStrategy):
    def apply(self, total): return total * 0.85

def calculate_discount(strategy: DiscountStrategy, total: float) -> float:
    return strategy.apply(total)
```

Adding `StudentDiscount` requires **zero changes** to existing code. You add a new class and wire it in. The `calculate_discount` function never opens for editing.

### Why it hurts when violated

- Adding behavior requires editing tested, working code.
- Regression risk grows with every new case added.
- Branch coverage in tests balloons with each `elif`.

---

## 3. Liskov Substitution Principle (LSP)

> Subtypes must be **substitutable** for their base types without breaking the program.

### Violation: Square extends Rectangle

```python
class Rectangle:
    def __init__(self, width, height):
        self._width = width
        self._height = height

    def set_width(self, w):  self._width = w
    def set_height(self, h): self._height = h
    def area(self):          return self._width * self._height

class Square(Rectangle):
    def __init__(self, side):
        super().__init__(side, side)

    def set_width(self, w):
        self._width = w
        self._height = w   # force equal sides

    def set_height(self, h):
        self._width = h
        self._height = h
```

A function expecting `Rectangle` passes a `Square`, calls `set_width(5)` then `set_height(3)`, and gets area 9 instead of 15. The post-condition of `Rectangle` ("width and height are independent") is broken by `Square`.

### Fix: Separate the hierarchy properly

```python
from abc import ABC, abstractmethod

class Shape(ABC):
    @abstractmethod
    def area(self) -> float: ...

class Rectangle(Shape):
    def __init__(self, width, height):
        self.width = width
        self.height = height
    def area(self): return self.width * self.height

class Square(Shape):
    def __init__(self, side):
        self.side = side
    def area(self): return self.side * self.side
```

Both satisfy `Shape.area()`. Neither pretends to support invariants it cannot uphold. No surprise behavior.

### Why it hurts when violated

- Callers cannot reason about behavior through the base type.
- Tests that pass with the base type fail with the subtype.
- Debugging is nightmarish: "it works with Rectangle but not Square — why?"

---

## 4. Interface Segregation Principle (ISP)

> Many **specific** interfaces are better than one **general** interface.

### Violation: The fat interface

```python
class Machine:
    def print(self, document): ...
    def scan(self, document): ...
    def fax(self, document): ...

class OldPrinter(Machine):
    def print(self, document):
        print(f"Printing: {document}")
    def scan(self, document):
        raise UnsupportedOperation("Cannot scan")
    def fax(self, document):
        raise UnsupportedOperation("Cannot fax")
```

`OldPrinter` is forced to implement `scan` and `fax` — methods it will never support. Any client depending on `Machine` also depends on methods it may never call.

### Fix: Role-based interfaces

```python
from abc import ABC, abstractmethod

class Printer(ABC):
    @abstractmethod
    def print(self, document: str) -> None: ...

class Scanner(ABC):
    @abstractmethod
    def scan(self, document: str) -> str: ...

class FaxMachine(ABC):
    @abstractmethod
    def fax(self, document: str) -> None: ...

class OldPrinter(Printer):
    def print(self, document):
        print(f"Printing: {document}")

class MultiFunctionPrinter(Printer, Scanner, FaxMachine):
    def print(self, document):
        print(f"Printing: {document}")
    def scan(self, document):
        return f"Scanned: {document}"
    def fax(self, document):
        print(f"Faxing: {document}")
```

`OldPrinter` implements only `Printer`. No stubs, no `UnsupportedOperation`, no wasted methods.

### Why it hurts when violated

- Clients depend on methods they never use — recompiling/retesting for changes they don't care about.
- Implementors must write no-op or throw-stub methods.
- Fat interfaces mask the real contract: which capabilities does this object *actually* have?

---

## 5. Dependency Inversion Principle (DIP)

> Depend on **abstractions**, not concretions. High-level modules should not depend on low-level modules.

### Violation: Hard-wired concretions

```python
class MySQLDatabase:
    def query(self, sql):
        print(f"MySQL executing: {sql}")
        return [{"id": 1}]

class UserService:
    def __init__(self):
        self.db = MySQLDatabase()  # hard dependency

    def get_user(self, user_id):
        return self.db.query(f"SELECT * FROM users WHERE id = {user_id}")
```

Switching to PostgreSQL means editing `UserService`. Writing a unit test means running a real MySQL instance. The high-level policy (find a user) is welded to the low-level detail (MySQL wire protocol).

### Fix: Inject the abstraction

```python
from abc import ABC, abstractmethod

class Database(ABC):
    @abstractmethod
    def query(self, sql: str) -> list: ...

class MySQLDatabase(Database):
    def query(self, sql):
        print(f"MySQL executing: {sql}")
        return [{"id": 1}]

class PostgresDatabase(Database):
    def query(self, sql):
        print(f"Postgres executing: {sql}")
        return [{"id": 1}]

class FakeDatabase(Database):
    def query(self, sql):
        return [{"id": 99, "name": "Test User"}]

class UserService:
    def __init__(self, db: Database):
        self.db = db

    def get_user(self, user_id):
        return self.db.query(f"SELECT * FROM users WHERE id = {user_id}")
```

`UserService` now depends on `Database` — an abstraction. Swap MySQL for Postgres: zero changes to `UserService`. Swap for `FakeDatabase` in tests: zero changes to `UserService`.

### Why it hurts when violated

- Every `new()` call in a constructor is a hidden dependency that resists testing and swapping.
- Changes to low-level modules force changes to high-level modules — the opposite of sustainable architecture.
- Unit tests require real infrastructure instead of fakes or mocks.

---

## Full Example: Putting It All Together

Imagine an order processing system that violates every principle:

```python
class OrderProcessor:
    def process(self, order):
        # SRP violation: validation, pricing, persistence, notification all here
        if not order.get("items"):
            raise ValueError("Empty order")

        total = 0
        for item in order["items"]:
            # OCP violation: discount logic hardcoded per type
            if item["type"] == "book":
                total += item["price"] * 0.9
            elif item["type"] == "electronics":
                total += item["price"] * 0.95
            elif item["type"] == "clothing":
                total += item["price"]

        # DIP violation: direct concretion
        db = MySQLDatabase()
        db.query(f"INSERT INTO orders VALUES ({total})")

        # ISP violation: notification assumes email
        import smtplib
        smtp = smtplib.SMTP("localhost")
        smtp.sendmail("noreply@co.com", order["email"], f"Order total: {total}")
```

Applying all five principles:

- **SRP**: Split into `OrderValidator`, `PriceCalculator`, `OrderRepository`, `Notifier`
- **OCP**: Make `PriceCalculator` use a `DiscountStrategy` per item type
- **LSP**: Ensure all discount strategies satisfy the same `DiscountStrategy` contract
- **ISP**: `Notifier` implements `MessageSender`, not a fat `NotificationService`
- **DIP**: `OrderRepository` depends on `Database` abstraction, not `MySQLDatabase`

---

## Measurement: Does SOLID Actually Help?

| Metric | Before SOLID | After SOLID |
|--------|-------------|-------------|
| Classes touched per feature | 1 (god class) | 1 (target class) |
| Merge conflicts per sprint | High | Low |
| Test setup complexity | Full infrastructure | In-memory fakes |
| Coupling score (afferent + efferent) | High | Low |
| Lines of code per class | 300+ | 30–80 |

SOLID does not reduce total lines of code. It distributes them so each class has **one reason to change** and **one axis of extension**.

---

## Use It

The SOLID principles appear throughout well-engineered codebases:

- **Python's `abc` module** enforces ISP and DIP at the language level.
- **Django's middleware** is OCP in action — add new behavior by creating a class, not by editing the core handler.
- **SQLAlchemy's dialect system** is OCP + DIP: new database backends are added as dialect classes implementing `Dialect`, zero edits to the core engine.
- **Spring Framework (Java)** is built on DIP — the entire DI container exists to invert dependencies.

### Read the Source

- **SQLAlchemy**: `lib/sqlalchemy/engine/default.py` — see how `DefaultDialect` implements the Dialect interface (OCP + DIP).
- **Django**: `django/core/handlers/base.py` — `BaseHandler.load_middleware` chains middleware classes, each a strategy (OCP).

---

## Ship It

The reusable artifact produced by this lesson lives in `outputs/`. It is:

- **A reference card** listing each principle, its violation smell, and the fix pattern — one page you can pin to your monitor.

## Exercises

1. **Easy** — Take any three-class module you wrote recently. Identify which SOLID principle it violates and refactor it.
2. **Medium** — Build a notification system that supports email, SMS, and push. Use ISP to split the interface and OCP to add a new channel without editing existing code.
3. **Hard** — Refactor `main.py` so that the `OrderProcessor` uses dependency injection for all collaborators. Write a test suite that uses only `FakeDatabase` and `FakeNotifier`.

## Key Terms

| Term | What people say | What it actually means |
|------|-----------------|----------------------|
| Single Responsibility | "One class, one job" | A module has one *reason to change* — one actor whose requests drive its evolution |
| Open/Closed | "Don't touch old code" | You *can* modify code to fix bugs; you shouldn't need to modify it to *add behavior* — extend instead |
| Liskov Substitution | "Inheritance must work" | If `S` is a subtype of `T`, any program that works with `T` must also work with `S` without modification |
| Interface Segregation | "Small interfaces" | Clients should not be forced to depend on methods they do not use — split fat interfaces by role |
| Dependency Inversion | "Use interfaces" | High-level policy must not depend on low-level detail; both must depend on abstractions |

## Further Reading

- Robert C. Martin, *Clean Architecture* (2017) — chapters 7–12 cover SOLID in depth.
- Barbara Liskov, "A Behavioral Notion of Subtyping" (1994) — the original LSP paper.
- Martin Fowler, "ISP" essay on martinfowler.com — practical take on when ISP matters.
- SQLAlchemy source: `lib/sqlalchemy/engine/` — production OCP+DIP.