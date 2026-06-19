"""
Domain-Driven Design — Bounded Contexts, Aggregates
Phase 16 — Software Engineering & Architecture

Order Management bounded context:
  - Value objects: Money, Address
  - Entities: OrderLine
  - Aggregate root: Order (enforces invariants)
  - Domain events: OrderPlaced, OrderCancelled
  - Repository: OrderRepository (collection-like interface)
"""

from __future__ import annotations

import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from decimal import Decimal
from typing import List, Optional


# ---------------------------------------------------------------------------
# Bounded Context Boundary: Order Management
# Everything below this line belongs to the Order Management context.
# Other contexts (Inventory, Billing) have their own models.
# ---------------------------------------------------------------------------


# --- Domain Events ---------------------------------------------------------

@dataclass(frozen=True)
class DomainEvent:
    occurred_at: datetime


@dataclass(frozen=True)
class OrderPlaced(DomainEvent):
    order_id: str
    customer_id: str
    total_amount: Decimal
    total_currency: str


@dataclass(frozen=True)
class OrderCancelled(DomainEvent):
    order_id: str
    reason: str


# --- Value Objects ---------------------------------------------------------

@dataclass(frozen=True)
class Money:
    """
    A value object representing a monetary amount.
    Immutable — all operations return a new instance.
    Compared by value, not identity.
    """
    amount: Decimal
    currency: str

    def __post_init__(self):
        if self.currency not in ("USD", "EUR", "GBP"):
            raise ValueError(f"Unsupported currency: {self.currency}")
        if self.amount < 0:
            raise ValueError(f"Money amount cannot be negative: {self.amount}")

    def add(self, other: Money) -> Money:
        if self.currency != other.currency:
            raise ValueError(
                f"Cannot add {self.currency} to {other.currency}"
            )
        return Money(amount=self.amount + other.amount, currency=self.currency)

    def multiply(self, factor: Decimal) -> Money:
        return Money(amount=self.amount * factor, currency=self.currency)

    def zero(self) -> Money:
        return Money(amount=Decimal("0"), currency=self.currency)

    @classmethod
    def from_float(cls, amount: float, currency: str = "USD") -> Money:
        return cls(amount=Decimal(str(round(amount, 2))), currency=currency)


@dataclass(frozen=True)
class Address:
    """
    A value object representing a shipping address.
    Immutable — if the address changes, create a new instance.
    Compared by value, not identity.
    """
    street: str
    city: str
    state: str
    postal_code: str
    country: str

    def __post_init__(self):
        if not self.street.strip():
            raise ValueError("Street is required")
        if not self.city.strip():
            raise ValueError("City is required")
        if not self.country.strip():
            raise ValueError("Country is required")

    def formatted(self) -> str:
        return f"{self.street}, {self.city}, {self.state} {self.postal_code}, {self.country}"


# --- Entities ---------------------------------------------------------------

@dataclass
class OrderLine:
    """
    An entity within the Order aggregate.
    Has identity (line_id) that persists across state changes.
    Never referenced directly from outside the aggregate —
    all access goes through the Order aggregate root.
    """
    line_id: str
    product_id: str
    product_name: str
    quantity: int
    unit_price: Money

    def __post_init__(self):
        if self.quantity <= 0:
            raise ValueError(f"Quantity must be positive, got {self.quantity}")

    def line_total(self) -> Money:
        return self.unit_price.multiply(Decimal(self.quantity))

    def change_quantity(self, new_quantity: int) -> None:
        if new_quantity <= 0:
            raise ValueError(f"Quantity must be positive, got {new_quantity}")
        self.quantity = new_quantity


# --- Aggregate Root ---------------------------------------------------------

class Order:
    """
    Aggregate root for the Order Management bounded context.

    Invariants enforced:
      1. An order cannot contain duplicate lines for the same product.
      2. Quantities must be positive.
      3. Unit prices must be non-negative.
      4. A placed order cannot have lines added or removed.
      5. A cancelled order cannot be modified.

    All mutations go through the aggregate root.
    External references use order_id, not Order objects directly.
    """

    STATUS_DRAFT = "draft"
    STATUS_PLACED = "placed"
    STATUS_CANCELLED = "cancelled"

    def __init__(
        self,
        order_id: str,
        customer_id: str,
        shipping_address: Address,
    ):
        if not customer_id.strip():
            raise ValueError("Customer ID is required")
        self._order_id = order_id
        self._customer_id = customer_id
        self._shipping_address = shipping_address
        self._lines: List[OrderLine] = []
        self._status = self.STATUS_DRAFT
        self._events: List[DomainEvent] = []

    @property
    def order_id(self) -> str:
        return self._order_id

    @property
    def customer_id(self) -> str:
        return self._customer_id

    @property
    def shipping_address(self) -> Address:
        return self._shipping_address

    @property
    def status(self) -> str:
        return self._status

    @property
    def events(self) -> List[DomainEvent]:
        return list(self._events)

    def _ensure_draft(self) -> None:
        if self._status != self.STATUS_DRAFT:
            raise RuntimeError(
                f"Cannot modify order in status '{self._status}'"
            )

    def add_line(
        self,
        product_id: str,
        product_name: str,
        quantity: int,
        unit_price: Money,
    ) -> None:
        self._ensure_draft()

        # Invariant: no duplicate products
        if any(line.product_id == product_id for line in self._lines):
            raise ValueError(
                f"Product {product_id} already in order {self._order_id}"
            )

        # Invariant: quantity must be positive
        if quantity <= 0:
            raise ValueError(f"Quantity must be positive, got {quantity}")

        line = OrderLine(
            line_id=str(uuid.uuid4()),
            product_id=product_id,
            product_name=product_name,
            quantity=quantity,
            unit_price=unit_price,
        )
        self._lines.append(line)

    def remove_line(self, product_id: str) -> None:
        self._ensure_draft()
        if not any(line.product_id == product_id for line in self._lines):
            raise ValueError(
                f"Product {product_id} not found in order {self._order_id}"
            )
        self._lines = [l for l in self._lines if l.product_id != product_id]

    def change_line_quantity(self, product_id: str, new_quantity: int) -> None:
        self._ensure_draft()
        for line in self._lines:
            if line.product_id == product_id:
                line.change_quantity(new_quantity)
                return
        raise ValueError(
            f"Product {product_id} not found in order {self._order_id}"
        )

    def total(self) -> Money:
        if not self._lines:
            return Money(amount=Decimal("0"), currency="USD")
        result = self._lines[0].line_total()
        for line in self._lines[1:]:
            result = result.add(line.line_total())
        return result

    def place(self) -> None:
        if self._status != self.STATUS_DRAFT:
            raise RuntimeError(
                f"Can only place a draft order, current status: {self._status}"
            )
        if not self._lines:
            raise ValueError("Cannot place an empty order")
        self._status = self.STATUS_PLACED
        total = self.total()
        self._events.append(
            OrderPlaced(
                occurred_at=datetime.now(timezone.utc),
                order_id=self._order_id,
                customer_id=self._customer_id,
                total_amount=total.amount,
                total_currency=total.currency,
            )
        )

    def cancel(self, reason: str) -> None:
        if self._status != self.STATUS_DRAFT:
            raise RuntimeError(
                f"Can only cancel a draft order, current status: {self._status}"
            )
        self._status = self.STATUS_CANCELLED
        self._events.append(
            OrderCancelled(
                occurred_at=datetime.now(timezone.utc),
                order_id=self._order_id,
                reason=reason,
            )
        )

    def __repr__(self) -> str:
        return (
            f"Order(id={self._order_id}, customer={self._customer_id}, "
            f"lines={len(self._lines)}, status={self._status})"
        )


# --- Repository -------------------------------------------------------------

class OrderRepository:
    """
    Provides a collection-like interface for Order aggregates.
    Works with aggregate roots only — no OrderLineRepository.
    """

    def __init__(self) -> None:
        self._store: dict[str, Order] = {}

    def add(self, order: Order) -> None:
        if order.order_id in self._store:
            raise ValueError(
                f"Order {order.order_id} already exists"
            )
        self._store[order.order_id] = order

    def get_by_id(self, order_id: str) -> Optional[Order]:
        return self._store.get(order_id)

    def remove(self, order_id: str) -> None:
        if order_id not in self._store:
            raise ValueError(f"Order {order_id} not found")
        del self._store[order_id]

    def next_identity(self) -> str:
        return str(uuid.uuid4())


# --- Anti-Corruption Layer --------------------------------------------------
# Translates data from an external (upstream) context into our model.

class LegacyOrderDTO:
    """Represents data coming from an upstream legacy system."""
    raw_order_num: str
    raw_customer_ref: str
    raw_lines: List[dict]

    def __init__(
        self,
        raw_order_num: str,
        raw_customer_ref: str,
        raw_lines: List[dict],
    ):
        self.raw_order_num = raw_order_num
        self.raw_customer_ref = raw_customer_ref
        self.raw_lines = raw_lines


class OrderACL:
    """
    Anti-Corruption Layer: translates legacy data into the
    Order Management bounded context's model, protecting our
    invariants from upstream schema changes.
    """

    @staticmethod
    def translate(dto: LegacyOrderDTO) -> Order:
        order = Order(
            order_id=f"ORD-{dto.raw_order_num}",
            customer_id=dto.raw_customer_ref,
            shipping_address=Address(
                street="Unknown",
                city="Unknown",
                state="N/A",
                postal_code="00000",
                country="US",
            ),
        )
        for raw_line in dto.raw_lines:
            order.add_line(
                product_id=raw_line["sku"],
                product_name=raw_line.get("name", "Unknown Product"),
                quantity=int(raw_line["qty"]),
                unit_price=Money.from_float(float(raw_line["price"])),
            )
        return order


# --- Demo -------------------------------------------------------------------

def main() -> None:
    repo = OrderRepository()

    # Create an order
    order_id = repo.next_identity()
    order = Order(
        order_id=order_id,
        customer_id="CUST-42",
        shipping_address=Address(
            street="123 Elm St",
            city="Springfield",
            state="IL",
            postal_code="62704",
            country="US",
        ),
    )

    # Add lines through the aggregate root
    order.add_line(
        product_id="PROD-A",
        product_name="Widget",
        quantity=3,
        unit_price=Money(Decimal("9.99"), "USD"),
    )
    order.add_line(
        product_id="PROD-B",
        product_name="Gadget",
        quantity=1,
        unit_price=Money(Decimal("49.99"), "USD"),
    )

    print(f"Order total: {order.total()}")

    # Invariant: duplicate product rejected
    try:
        order.add_line(
            product_id="PROD-A",
            product_name="Widget",
            quantity=1,
            unit_price=Money(Decimal("9.99"), "USD"),
        )
    except ValueError as e:
        print(f"Invariant enforced: {e}")

    # Place the order — transitions state and publishes event
    order.place()
    print(f"Order status: {order.status}")
    print(f"Events published: {order.events}")

    # Invariant: cannot modify a placed order
    try:
        order.remove_line("PROD-B")
    except RuntimeError as e:
        print(f"Invariant enforced: {e}")

    # Persist via repository
    repo.add(order)

    # Anti-Corruption Layer: translate legacy data
    legacy_dto = LegacyOrderDTO(
        raw_order_num="LEG-007",
        raw_customer_ref="EXT-CUST-99",
        raw_lines=[
            {"sku": "PROD-C", "name": "Doohickey", "qty": "2", "price": "14.50"},
        ],
    )
    translated_order = OrderACL.translate(legacy_dto)
    print(f"ACL-translated order: {translated_order}")

    # Invariant: cannot cancel a placed order
    try:
        order.cancel("changed mind")
    except RuntimeError as e:
        print(f"Invariant enforced: {e}")

    print(f"Total lines in order: {len(order._lines)}")


if __name__ == "__main__":
    main()