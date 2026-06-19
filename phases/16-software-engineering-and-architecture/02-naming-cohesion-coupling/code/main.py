"""
Naming, Cohesion, Coupling — Python Implementation
Phase 16 — Software Engineering & Architecture

Before/after examples showing:
  1. Bad naming → good naming
  2. Low cohesion → high cohesion refactoring
  3. Tight coupling → loose coupling via dependency injection
  4. Simple cohesion & coupling metrics (LCOM, Ca, Ce, Instability)
"""

from __future__ import annotations
from dataclasses import dataclass, field
from abc import ABC, abstractmethod
from typing import Protocol


# ═══════════════════════════════════════════════════════════
# SECTION 1: NAMING — Before & After
# ═══════════════════════════════════════════════════════════

def naming_before() -> dict:
    """BAD: single-letter vars, abbreviations, unclear intent."""
    d = {"a": 3, "b": 7}
    r = d["a"] + d["b"]
    fl = 0.0
    for k in d:
        fl += d[k]
    tm = r / len(d)
    return {"s": r, "av": tm}


def naming_after() -> dict:
    """GOOD: intent-revealing, pronounceable, searchable."""
    order_line_items = {"apples": 3, "bananas": 7}
    total_item_count = sum(order_line_items.values())
    order_subtotal = 0.0
    for item_name in order_line_items:
        order_subtotal += order_line_items[item_name]
    average_item_quantity = total_item_count / len(order_line_items)
    return {
        "total_item_count": total_item_count,
        "average_item_quantity": average_item_quantity,
    }


# ═══════════════════════════════════════════════════════════
# SECTION 2: COHESION — Low → High Refactoring
# ═══════════════════════════════════════════════════════════

class LowCohesionOrderProcessor:
    """Coincidental cohesion: does too many unrelated things."""

    def __init__(self):
        self.orders = []
        self.smtp_host = "smtp.example.com"
        self.smtp_port = 587
        self.sender_email = "noreply@example.com"

    def add_order(self, customer_name: str, item: str, quantity: int):
        self.orders.append({
            "customer_name": customer_name,
            "item": item,
            "quantity": quantity,
        })

    def calculate_total(self) -> float:
        price_map = {"widget": 9.99, "gadget": 19.99, "doohickey": 4.99}
        total = 0.0
        for order in self.orders:
            total += price_map.get(order["item"], 0) * order["quantity"]
        return total

    def send_confirmation_email(self, recipient: str, subject: str, body: str):
        print(f"Connecting to {self.smtp_host}:{self.smtp_port}")
        print(f"From: {self.sender_email} To: {recipient}")
        print(f"Subject: {subject}")
        print(body)
        print("Email sent.")

    def generate_invoice_number(self) -> str:
        return f"INV-{len(self.orders):04d}"

    def export_orders_to_csv(self) -> str:
        lines = ["customer,item,quantity"]
        for order in self.orders:
            lines.append(
                f"{order['customer_name']},{order['item']},{order['quantity']}"
            )
        return "\n".join(lines)


@dataclass
class Order:
    customer_name: str
    item: str
    quantity: int


class OrderRepository:
    """Functional cohesion: stores and retrieves orders."""

    def __init__(self):
        self._orders: list[Order] = []

    def add(self, order: Order) -> None:
        self._orders.append(order)

    def list_all(self) -> list[Order]:
        return list(self._orders)

    def count(self) -> int:
        return len(self._orders)


class PricingService:
    """Functional cohesion: calculates order prices."""

    PRICE_MAP = {"widget": 9.99, "gadget": 19.99, "doohickey": 4.99}

    def calculate_total(self, orders: list[Order]) -> float:
        total = 0.0
        for order in orders:
            total += self.PRICE_MAP.get(order.item, 0) * order.quantity
        return total


class EmailService:
    """Functional cohesion: sends emails."""

    def __init__(self, smtp_host: str, smtp_port: int, sender: str):
        self.smtp_host = smtp_host
        self.smtp_port = smtp_port
        self.sender = sender

    def send(self, recipient: str, subject: str, body: str) -> None:
        print(f"Connecting to {self.smtp_host}:{self.smtp_port}")
        print(f"From: {self.sender} To: {recipient}")
        print(f"Subject: {subject}")
        print(body)
        print("Email sent.")


class InvoiceService:
    """Functional cohesion: generates invoice identifiers."""

    def __init__(self, order_repo: OrderRepository):
        self.order_repo = order_repo

    def generate_invoice_number(self) -> str:
        return f"INV-{self.order_repo.count():04d}"


class CsvExportService:
    """Functional cohesion: exports orders to CSV."""

    def export_orders(self, orders: list[Order]) -> str:
        lines = ["customer,item,quantity"]
        for order in orders:
            lines.append(f"{order.customer_name},{order.item},{order.quantity}")
        return "\n".join(lines)


# ═══════════════════════════════════════════════════════════
# SECTION 3: COUPLING — Tight → Loose via Dependency Injection
# ═══════════════════════════════════════════════════════════

class TightlyCoupledOrderService:
    """
    Content + common coupling: creates its own dependencies,
    shares mutable global state, knows implementation details.
    """
    _global_config: dict = {"tax_rate": 0.08, "discount": 0.05}

    def process_order(self, customer_name: str, item: str, quantity: int) -> dict:
        repo = OrderRepository()
        repo.add(Order(customer_name, item, quantity))
        pricing = PricingService()
        subtotal = pricing.calculate_total(repo.list_all())
        tax = subtotal * self._global_config["tax_rate"]
        discount = subtotal * self._global_config["discount"]
        total = subtotal + tax - discount
        self._global_config["discount"] = 0.0
        return {
            "subtotal": subtotal,
            "tax": tax,
            "discount": discount,
            "total": total,
        }


class NotificationSender(Protocol):
    def send(self, recipient: str, subject: str, body: str) -> None: ...


class DiscountStrategy(Protocol):
    def calculate(self, subtotal: float) -> float: ...


class NoDiscount:
    def calculate(self, subtotal: float) -> float:
        return 0.0


class PercentageDiscount:
    def __init__(self, percent: float):
        self.percent = percent

    def calculate(self, subtotal: float) -> float:
        return subtotal * (self.percent / 100)


class FlatDiscount:
    def __init__(self, amount: float):
        self.amount = amount

    def calculate(self, subtotal: float) -> float:
        return min(self.amount, subtotal)


class LooselyCoupledOrderService:
    """
    Data coupling (best): depends on injected abstractions,
    no global state, communicates through simple parameters.
    """

    def __init__(
        self,
        order_repo: OrderRepository,
        pricing_service: PricingService,
        discount_strategy: DiscountStrategy,
        notification: NotificationSender,
        tax_rate: float = 0.08,
    ):
        self.order_repo = order_repo
        self.pricing_service = pricing_service
        self.discount_strategy = discount_strategy
        self.notification = notification
        self.tax_rate = tax_rate

    def process_order(self, customer_name: str, item: str, quantity: int) -> dict:
        self.order_repo.add(Order(customer_name, item, quantity))
        all_orders = self.order_repo.list_all()
        subtotal = self.pricing_service.calculate_total(all_orders)
        tax = subtotal * self.tax_rate
        discount = self.discount_strategy.calculate(subtotal)
        total = subtotal + tax - discount
        self.notification.send(
            customer_name,
            "Order Confirmation",
            f"Your order total is ${total:.2f}",
        )
        return {
            "subtotal": subtotal,
            "tax": tax,
            "discount": discount,
            "total": total,
        }


# ═══════════════════════════════════════════════════════════
# SECTION 4: METRICS — LCOM, Afferent/Efferent Coupling, Instability
# ═══════════════════════════════════════════════════════════

@dataclass
class MethodInfo:
    name: str
    accessed_fields: set[str]


@dataclass
class ClassInfo:
    name: str
    instance_fields: set[str]
    methods: list[MethodInfo] = field(default_factory=list)


def calculate_lcom(class_info: ClassInfo) -> int:
    """
    LCOM = max(0, m - q)
    m = number of method pairs that do NOT share instance variables
    q = number of method pairs that DO share instance variables
    Higher LCOM = lower cohesion. LCOM = 0 means perfect cohesion.
    """
    methods = class_info.methods
    if len(methods) < 2:
        return 0
    m = 0
    q = 0
    for i in range(len(methods)):
        for j in range(i + 1, len(methods)):
            shared = methods[i].accessed_fields & methods[j].accessed_fields
            if shared:
                q += 1
            else:
                m += 1
    return max(0, m - q)


@dataclass
class ModuleInfo:
    name: str
    efferent: set[str] = field(default_factory=set)
    afferent: set[str] = field(default_factory=set)


def calculate_instability(module: ModuleInfo) -> float:
    """
    Instability = Ce / (Ca + Ce)
    Ce = efferent coupling (outgoing dependencies)
    Ca = afferent coupling (incoming dependencies)
    0 = maximally stable, 1 = maximally unstable
    """
    ca = len(module.afferent)
    ce = len(module.efferent)
    total = ca + ce
    if total == 0:
        return 0.0
    return ce / total


def demonstrate_metrics() -> None:
    low_cohesion_class = ClassInfo(
        name="LowCohesionOrderProcessor",
        instance_fields={"orders", "smtp_host", "smtp_port", "sender_email"},
        methods=[
            MethodInfo("add_order", {"orders"}),
            MethodInfo("calculate_total", {"orders"}),
            MethodInfo("send_confirmation_email", {"smtp_host", "smtp_port", "sender_email"}),
            MethodInfo("generate_invoice_number", {"orders"}),
            MethodInfo("export_orders_to_csv", {"orders"}),
        ],
    )
    low_cohesion_class_alt = ClassInfo(
        name="GodObjectUserService",
        instance_fields={"users", "db_connection", "smtp_host", "smtp_port", "logger"},
        methods=[
            MethodInfo("create_user", {"users", "db_connection"}),
            MethodInfo("delete_user", {"users", "db_connection"}),
            MethodInfo("send_welcome_email", {"smtp_host", "smtp_port"}),
            MethodInfo("send_password_reset", {"smtp_host", "smtp_port"}),
            MethodInfo("log_action", {"logger"}),
        ],
    )

    high_cohesion_class = ClassInfo(
        name="PricingService",
        instance_fields=set(),
        methods=[
            MethodInfo("calculate_total", set()),
        ],
    )

    print("=== LCOM Metric ===")
    for cls in [low_cohesion_class, low_cohesion_class_alt, high_cohesion_class]:
        lcom = calculate_lcom(cls)
        print(f"  {cls.name}: LCOM = {lcom}")
        if lcom > 0:
            print(f"    → Low cohesion. Consider splitting into focused classes.")
        else:
            print("    → High cohesion. All methods share purpose.")

    print()
    print("=== Instability Metric ===")

    utils_module = ModuleInfo(
        name="utils",
        efferent={"os", "json", "datetime", "logging", "re", "collections"},
        afferent={"order_service", "email_service", "report_service", "auth_service"},
    )

    auth_module = ModuleInfo(
        name="auth_interface",
        efferent=set(),
        afferent={"user_service", "api_gateway", "admin_panel", "oauth_handler"},
    )

    for mod in [utils_module, auth_module]:
        instability = calculate_instability(mod)
        print(f"  {mod.name}:")
        print(f"    Afferent (Ca) = {len(mod.afferent)}")
        print(f"    Efferent (Ce) = {len(mod.efferent)}")
        print(f"    Instability   = {instability:.2f}")
        if instability < 0.3:
            print("    → Stable. Many depend on this. Changes must be careful.")
        elif instability > 0.7:
            print("    → Unstable. Easy to change. Few depend on this.")
        else:
            print("    → Balanced. Moderate incoming/outgoing dependencies.")


# ═══════════════════════════════════════════════════════════
# SECTION 5: END-TO-END DEMO
# ═══════════════════════════════════════════════════════════

def main() -> None:
    print("=" * 70)
    print("LESSON 16.02: Naming, Cohesion, Coupling — Python Demo")
    print("=" * 70)

    # --- Naming demo ---
    print("\n--- Naming: Before → After ---")
    before = naming_before()
    after = naming_after()
    print(f"  Before: {before}")
    print(f"  After:  {after}")
    print(f"  Before keys: unsearchable ({list(before.keys())})")
    print(f"  After keys:  intent-revealing ({list(after.keys())})")

    # --- Cohesion demo ---
    print("\n--- Cohesion: Low → High ---")
    low_cohesion = LowCohesionOrderProcessor()
    low_cohesion.add_order("Alice", "widget", 2)
    low_cohesion.add_order("Bob", "gadget", 1)
    print(f"  Low cohesion total: ${low_cohesion.calculate_total():.2f}")
    print(f"  Invoice #: {low_cohesion.generate_invoice_number()}")
    print(f"  CSV:\n{low_cohesion.export_orders_to_csv()}")

    repo = OrderRepository()
    repo.add(Order("Alice", "widget", 2))
    repo.add(Order("Bob", "gadget", 1))
    pricing = PricingService()
    invoice = InvoiceService(repo)
    csv_export = CsvExportService()
    print(f"\n  High cohesion total: ${pricing.calculate_total(repo.list_all()):.2f}")
    print(f"  Invoice #: {invoice.generate_invoice_number()}")
    print(f"  CSV:\n{csv_export.export_orders(repo.list_all())}")

    # --- Coupling demo ---
    print("\n--- Coupling: Tight → Loose ---")
    tight = TightlyCoupledOrderService()
    tight_result = tight.process_order("Carol", "widget", 3)
    print(f"  Tight coupling result: {tight_result}")
    print(f"  ⚠ Global config mutated: discount set to 0!")

    loose_repo = OrderRepository()
    pricing_svc = PricingService()
    discount = PercentageDiscount(5)
    notif = EmailService("smtp.example.com", 587, "noreply@example.com")
    loose = LooselyCoupledOrderService(loose_repo, pricing_svc, discount, notif)
    loose_result = loose.process_order("Carol", "widget", 3)
    print(f"\n  Loose coupling result: {loose_result}")
    print(f"  ✅ No global state. Dependencies injected via constructor.")

    # --- Flat discount alternative ---
    flat_discount = FlatDiscount(2.00)
    loose_flat = LooselyCoupledOrderService(
        OrderRepository(), PricingService(), flat_discount, notif
    )
    flat_result = loose_flat.process_order("Dave", "gadget", 1)
    print(f"  Flat discount result: {flat_result}")
    print(f"  ✅ Swapped discount strategy without changing OrderService.")

    # --- Metrics demo ---
    print("\n")
    demonstrate_metrics()

    print("\n" + "=" * 70)
    print("Key takeaway: Name with intent, refactor toward cohesion,")
    print("              decouple through injection and abstraction.")
    print("=" * 70)


if __name__ == "__main__":
    main()