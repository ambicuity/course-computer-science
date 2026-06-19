"""
SOLID Principles — Demystified
Phase 16 — Software Engineering & Architecture

Runnable demos for all five SOLID principles.
Each principle shows the violation, explains why it hurts, then shows the fix.
Run: python main.py
"""

from abc import ABC, abstractmethod


# ─────────────────────────────────────────────────────────────────────────────
# S — Single Responsibility Principle
# ─────────────────────────────────────────────────────────────────────────────

print("=" * 60)
print("PRINCIPLE 1: Single Responsibility (SRP)")
print("=" * 60)

print("\n--- VIOLATION: God class does everything ---\n")


class EmployeeViolation:
    def __init__(self, name, salary):
        self.name = name
        self.salary = salary

    def calculate_pay(self):
        return self.salary * 1.1

    def save_to_database(self, db_conn):
        db_conn.execute(
            f"INSERT INTO employees VALUES ('{self.name}', {self.salary})"
        )

    def generate_report(self):
        return f"Employee: {self.name}, Salary: {self.salary}"

    def send_payslip_email(self, smtp_client):
        smtp_client.send(to=self.name + "@co.com", body=self.generate_report())


print("EmployeeViolation has 4 reasons to change:")
print("  - Payroll policy  -> calculate_pay")
print("  - DBA schema      -> save_to_database")
print("  - Design team     -> generate_report")
print("  - IT / email team -> send_payslip_email")

print("\n--- FIX: One responsibility per class ---\n")


class Employee:
    def __init__(self, name: str, salary: float):
        self.name = name
        self.salary = salary


class PayCalculator:
    def calculate(self, employee: Employee) -> float:
        return employee.salary * 1.1


class EmployeeRepository:
    def save(self, db_conn, employee: Employee):
        db_conn.execute(
            "INSERT INTO employees VALUES (?, ?)",
            (employee.name, employee.salary),
        )


class ReportFormatter:
    def format(self, employee: Employee) -> str:
        return f"Employee: {employee.name}, Salary: {employee.salary}"


emp = Employee("Ada", 100_000)
calc = PayCalculator()
repo = EmployeeRepository()
fmt = ReportFormatter()

print(f"  Pay:        {calc.calculate(emp)}")
print(f"  Report:     {fmt.format(emp)}")
print(f"  Repository: saves (would call db_conn.execute)")
print("  Each class has ONE reason to change.")


# ─────────────────────────────────────────────────────────────────────────────
# O — Open/Closed Principle
# ─────────────────────────────────────────────────────────────────────────────

print("\n" + "=" * 60)
print("PRINCIPLE 2: Open/Closed (OCP)")
print("=" * 60)

print("\n--- VIOLATION: Growing switch/case ---\n")


def calculate_discount_violation(customer_type: str, total: float) -> float:
    if customer_type == "regular":
        return total * 0.95
    elif customer_type == "premium":
        return total * 0.9
    elif customer_type == "vip":
        return total * 0.8
    return total


print("  calculate_discount_violation('regular', 100) =", calculate_discount_violation("regular", 100))
print("  Adding 'student' requires editing this function — OCP violation!")

print("\n--- FIX: Strategy pattern — extend by adding classes ---\n")


class DiscountStrategy(ABC):
    @abstractmethod
    def apply(self, total: float) -> float: ...


class RegularDiscount(DiscountStrategy):
    def apply(self, total: float) -> float:
        return total * 0.95


class PremiumDiscount(DiscountStrategy):
    def apply(self, total: float) -> float:
        return total * 0.90


class VIPDiscount(DiscountStrategy):
    def apply(self, total: float) -> float:
        return total * 0.80


class StudentDiscount(DiscountStrategy):
    def apply(self, total: float) -> float:
        return total * 0.85


def calculate_discount(strategy: DiscountStrategy, total: float) -> float:
    return strategy.apply(total)


strategies = {
    "regular": RegularDiscount(),
    "premium": PremiumDiscount(),
    "vip": VIPDiscount(),
    "student": StudentDiscount(),
}

for name, strategy in strategies.items():
    result = calculate_discount(strategy, 100)
    print(f"  {name:10s} -> {result:.2f}")

print("  Adding a new discount = adding a new class. No edits to existing code.")


# ─────────────────────────────────────────────────────────────────────────────
# L — Liskov Substitution Principle
# ─────────────────────────────────────────────────────────────────────────────

print("\n" + "=" * 60)
print("PRINCIPLE 3: Liskov Substitution (LSP)")
print("=" * 60)

print("\n--- VIOLATION: Square extends Rectangle ---\n")


class RectangleViolation:
    def __init__(self, width, height):
        self._width = width
        self._height = height

    def set_width(self, w):
        self._width = w

    def set_height(self, h):
        self._height = h

    def area(self):
        return self._width * self._height


class SquareViolation(RectangleViolation):
    def __init__(self, side):
        super().__init__(side, side)

    def set_width(self, w):
        self._width = w
        self._height = w

    def set_height(self, h):
        self._width = h
        self._height = h


def print_area_violation(shape: RectangleViolation):
    shape.set_width(5)
    shape.set_height(3)
    print(f"  Expected area: 15, Got: {shape.area()}")


r = RectangleViolation(2, 3)
s = SquareViolation(2)
print("  Using Rectangle:")
print_area_violation(r)
print("  Using Square (LSP violation!):")
print_area_violation(s)
print("  Square breaks Rectangle's contract — width/height are not independent.")

print("\n--- FIX: Shared base without false promises ---\n")


class Shape(ABC):
    @abstractmethod
    def area(self) -> float: ...


class Rectangle(Shape):
    def __init__(self, width: float, height: float):
        self.width = width
        self.height = height

    def area(self) -> float:
        return self.width * self.height


class Square(Shape):
    def __init__(self, side: float):
        self.side = side

    def area(self) -> float:
        return self.side * self.side


def print_area(shape: Shape):
    print(f"  {shape.__class__.__name__} area = {shape.area()}")


print_area(Rectangle(5, 3))
print_area(Square(4))
print("  Both satisfy Shape.area(). No false promises about shared setters.")


# ─────────────────────────────────────────────────────────────────────────────
# I — Interface Segregation Principle
# ─────────────────────────────────────────────────────────────────────────────

print("\n" + "=" * 60)
print("PRINCIPLE 4: Interface Segregation (ISP)")
print("=" * 60)

print("\n--- VIOLATION: Fat interface forces stubs ---\n")


class MachineViolation(ABC):
    @abstractmethod
    def print_doc(self, document: str) -> None: ...

    @abstractmethod
    def scan_doc(self, document: str) -> str: ...

    @abstractmethod
    def fax_doc(self, document: str) -> None: ...


class OldPrinterViolation(MachineViolation):
    def print_doc(self, document: str) -> None:
        print(f"  Printing: {document}")

    def scan_doc(self, document: str) -> str:
        raise UnsupportedOperation("Cannot scan")

    def fax_doc(self, document: str) -> None:
        raise UnsupportedOperation("Cannot fax")


old = OldPrinterViolation()
old.print_doc("report.pdf")
print("  OldPrinterViolation.scan_doc() raises UnsupportedOperation — ISP violation!")

print("\n--- FIX: Role-based interfaces ---\n")


class Printer(ABC):
    @abstractmethod
    def print_doc(self, document: str) -> None: ...


class Scanner(ABC):
    @abstractmethod
    def scan_doc(self, document: str) -> str: ...


class FaxMachine(ABC):
    @abstractmethod
    def fax_doc(self, document: str) -> None: ...


class SimplePrinter(Printer):
    def print_doc(self, document: str) -> None:
        print(f"  Printing: {document}")


class MultiFunctionDevice(Printer, Scanner, FaxMachine):
    def print_doc(self, document: str) -> None:
        print(f"  Printing: {document}")

    def scan_doc(self, document: str) -> str:
        return f"Scanned: {document}"

    def fax_doc(self, document: str) -> None:
        print(f"  Faxing: {document}")


simple = SimplePrinter()
mfd = MultiFunctionDevice()
simple.print_doc("letter.pdf")
mfd.print_doc("contract.pdf")
print(f"  {mfd.scan_doc('invoice.pdf')}")
mfd.fax_doc("memo.pdf")
print("  SimplePrinter implements only Printer. No stubs, no NotImplementedException.")


# ─────────────────────────────────────────────────────────────────────────────
# D — Dependency Inversion Principle
# ─────────────────────────────────────────────────────────────────────────────

print("\n" + "=" * 60)
print("PRINCIPLE 5: Dependency Inversion (DIP)")
print("=" * 60)

print("\n--- VIOLATION: UserService hard-wires MySQLDatabase ---\n")


class MySQLDatabaseViolation:
    def query(self, sql: str) -> list:
        print(f"  MySQL executing: {sql}")
        return [{"id": 1, "name": "Ada"}]


class UserServiceViolation:
    def __init__(self):
        self.db = MySQLDatabaseViolation()

    def get_user(self, user_id: int) -> dict:
        return self.db.query(f"SELECT * FROM users WHERE id = {user_id}")[0]


svc_v = UserServiceViolation()
u = svc_v.get_user(1)
print(f"  User: {u}")
print("  UserServiceViolation depends on MySQL — can't test without MySQL, can't swap DB.")

print("\n--- FIX: Depend on abstraction ---\n")


class Database(ABC):
    @abstractmethod
    def query(self, sql: str) -> list: ...


class MySQLDatabase(Database):
    def query(self, sql: str) -> list:
        print(f"  MySQL executing: {sql}")
        return [{"id": 1, "name": "Ada"}]


class PostgresDatabase(Database):
    def query(self, sql: str) -> list:
        print(f"  Postgres executing: {sql}")
        return [{"id": 1, "name": "Ada"}]


class FakeDatabase(Database):
    def query(self, sql: str) -> list:
        return [{"id": 99, "name": "Test User"}]


class UserService:
    def __init__(self, db: Database):
        self.db = db

    def get_user(self, user_id: int) -> dict:
        rows = self.db.query(f"SELECT * FROM users WHERE id = {user_id}")
        return rows[0] if rows else {}


print("  With MySQLDatabase:")
svc_mysql = UserService(MySQLDatabase())
print(f"    {svc_mysql.get_user(1)}")

print("  With PostgresDatabase:")
svc_pg = UserService(PostgresDatabase())
print(f"    {svc_pg.get_user(1)}")

print("  With FakeDatabase (for tests):")
svc_fake = UserService(FakeDatabase())
print(f"    {svc_fake.get_user(1)}")

print("\n  UserService depends on Database abstraction — swap freely.")


# ─────────────────────────────────────────────────────────────────────────────
# Summary
# ─────────────────────────────────────────────────────────────────────────────

print("\n" + "=" * 60)
print("SUMMARY")
print("=" * 60)
print("""
  S — Single Responsibility:  One reason to change per module.
  O — Open/Closed:            Extend by adding, not by editing.
  L — Liskov Substitution:    Subtypes keep the base type's promises.
  I — Interface Segregation:  Many small interfaces > one fat interface.
  D — Dependency Inversion:   Depend on abstractions, not concretions.

  All five are about managing dependencies — the root of sustainable code.
""")


if __name__ == "__main__":
    pass