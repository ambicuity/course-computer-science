/**
 * Domain-Driven Design — Bounded Contexts, Aggregates
 * Phase 16 — Software Engineering & Architecture
 *
 * Order Management bounded context (TypeScript):
 *   - Value objects: Money, Address
 *   - Entities: OrderLine
 *   - Aggregate root: Order (enforces invariants)
 *   - Domain events: OrderPlaced, OrderCancelled
 *   - Repository: OrderRepository (collection-like interface)
 *   - Anti-Corruption Layer: OrderACL
 */

// ---------------------------------------------------------------------------
// Bounded Context Boundary: Order Management
// Everything below this line belongs to the Order Management context.
// Other contexts (Inventory, Billing) have their own models.
// ---------------------------------------------------------------------------

// --- Domain Events ---------------------------------------------------------

interface DomainEvent {
  readonly occurredAt: Date;
  readonly eventType: string;
}

interface OrderPlacedEvent extends DomainEvent {
  readonly eventType: "OrderPlaced";
  readonly orderId: string;
  readonly customerId: string;
  readonly totalAmount: number;
  readonly totalCurrency: Currency;
}

interface OrderCancelledEvent extends DomainEvent {
  readonly eventType: "OrderCancelled";
  readonly orderId: string;
  readonly reason: string;
}

type OrderEvent = OrderPlacedEvent | OrderCancelledEvent;

// --- Value Objects ---------------------------------------------------------

type Currency = "USD" | "EUR" | "GBP";

class Money {
  private constructor(
    public readonly amount: number,
    public readonly currency: Currency,
  ) {
    if (amount < 0) {
      throw new Error(`Money amount cannot be negative: ${amount}`);
    }
  }

  static of(amount: number, currency: Currency = "USD"): Money {
    return new Money(Math.round(amount * 100) / 100, currency);
  }

  add(other: Money): Money {
    if (this.currency !== other.currency) {
      throw new Error(`Cannot add ${this.currency} to ${other.currency}`);
    }
    return Money.of(this.amount + other.amount, this.currency);
  }

  multiply(factor: number): Money {
    return Money.of(
      Math.round(this.amount * factor * 100) / 100,
      this.currency,
    );
  }

  static zero(currency: Currency = "USD"): Money {
    return Money.of(0, currency);
  }

  equals(other: Money): boolean {
    return this.amount === other.amount && this.currency === other.currency;
  }

  toString(): string {
    return `${this.currency} ${this.amount.toFixed(2)}`;
  }
}

class Address {
  private constructor(
    public readonly street: string,
    public readonly city: string,
    public readonly state: string,
    public readonly postalCode: string,
    public readonly country: string,
  ) {
    if (!street.trim()) throw new Error("Street is required");
    if (!city.trim()) throw new Error("City is required");
    if (!country.trim()) throw new Error("Country is required");
  }

  static of(
    street: string,
    city: string,
    state: string,
    postalCode: string,
    country: string,
  ): Address {
    return new Address(street, city, state, postalCode, country);
  }

  formatted(): string {
    return `${this.street}, ${this.city}, ${this.state} ${this.postalCode}, ${this.country}`;
  }

  equals(other: Address): boolean {
    return (
      this.street === other.street &&
      this.city === other.city &&
      this.state === other.state &&
      this.postalCode === other.postalCode &&
      this.country === other.country
    );
  }
}

// --- Entities ---------------------------------------------------------------

class OrderLine {
  readonly lineId: string;
  readonly productId: string;
  readonly productName: string;
  quantity: number;
  readonly unitPrice: Money;

  constructor(
    lineId: string,
    productId: string,
    productName: string,
    quantity: number,
    unitPrice: Money,
  ) {
    if (quantity <= 0) {
      throw new Error(`Quantity must be positive, got ${quantity}`);
    }
    this.lineId = lineId;
    this.productId = productId;
    this.productName = productName;
    this.quantity = quantity;
    this.unitPrice = unitPrice;
  }

  lineTotal(): Money {
    return this.unitPrice.multiply(this.quantity);
  }

  changeQuantity(newQuantity: number): void {
    if (newQuantity <= 0) {
      throw new Error(`Quantity must be positive, got ${newQuantity}`);
    }
    this.quantity = newQuantity;
  }
}

// --- Aggregate Root ---------------------------------------------------------

type OrderStatus = "draft" | "placed" | "cancelled";

class Order {
  private readonly orderId: string;
  private readonly customerId: string;
  private shippingAddress: Address;
  private lines: OrderLine[];
  private status: OrderStatus;
  private events: OrderEvent[];

  constructor(orderId: string, customerId: string, shippingAddress: Address) {
    if (!customerId.trim()) {
      throw new Error("Customer ID is required");
    }
    this.orderId = orderId;
    this.customerId = customerId;
    this.shippingAddress = shippingAddress;
    this.lines = [];
    this.status = "draft";
    this.events = [];
  }

  get id(): string {
    return this.orderId;
  }

  get currentStatus(): OrderStatus {
    return this.status;
  }

  get allLines(): ReadonlyArray<OrderLine> {
    return this.lines;
  }

  get domainEvents(): ReadonlyArray<OrderEvent> {
    return [...this.events];
  }

  private ensureDraft(): void {
    if (this.status !== "draft") {
      throw new Error(
        `Cannot modify order in status '${this.status}'`,
      );
    }
  }

  private nextLineId(): string {
    return `LINE-${this.lines.length + 1}-${Date.now()}`;
  }

  addLine(
    productId: string,
    productName: string,
    quantity: number,
    unitPrice: Money,
  ): void {
    this.ensureDraft();

    // Invariant: no duplicate products
    const existing = this.lines.find((l) => l.productId === productId);
    if (existing) {
      throw new Error(
        `Product ${productId} already in order ${this.orderId}`,
      );
    }

    // Invariant: quantity must be positive
    if (quantity <= 0) {
      throw new Error(`Quantity must be positive, got ${quantity}`);
    }

    const line = new OrderLine(
      this.nextLineId(),
      productId,
      productName,
      quantity,
      unitPrice,
    );
    this.lines.push(line);
  }

  removeLine(productId: string): void {
    this.ensureDraft();
    const idx = this.lines.findIndex((l) => l.productId === productId);
    if (idx === -1) {
      throw new Error(
        `Product ${productId} not found in order ${this.orderId}`,
      );
    }
    this.lines.splice(idx, 1);
  }

  changeLineQuantity(productId: string, newQuantity: number): void {
    this.ensureDraft();
    const line = this.lines.find((l) => l.productId === productId);
    if (!line) {
      throw new Error(
        `Product ${productId} not found in order ${this.orderId}`,
      );
    }
    line.changeQuantity(newQuantity);
  }

  total(): Money {
    if (this.lines.length === 0) {
      return Money.zero();
    }
    return this.lines
      .map((l) => l.lineTotal())
      .reduce((acc, cur) => acc.add(cur));
  }

  place(): void {
    if (this.status !== "draft") {
      throw new Error(
        `Can only place a draft order, current status: ${this.status}`,
      );
    }
    if (this.lines.length === 0) {
      throw new Error("Cannot place an empty order");
    }

    this.status = "placed";
    const total = this.total();
    this.events.push({
      eventType: "OrderPlaced",
      occurredAt: new Date(),
      orderId: this.orderId,
      customerId: this.customerId,
      totalAmount: total.amount,
      totalCurrency: total.currency,
    });
  }

  cancel(reason: string): void {
    if (this.status !== "draft") {
      throw new Error(
        `Can only cancel a draft order, current status: ${this.status}`,
      );
    }
    this.status = "cancelled";
    this.events.push({
      eventType: "OrderCancelled",
      occurredAt: new Date(),
      orderId: this.orderId,
      reason,
    });
  }

  toString(): string {
    return `Order(id=${this.orderId}, customer=${this.customerId}, lines=${this.lines.length}, status=${this.status})`;
  }
}

// --- Repository -------------------------------------------------------------

class OrderRepository {
  private store: Map<string, Order> = new Map();

  add(order: Order): void {
    if (this.store.has(order.id)) {
      throw new Error(`Order ${order.id} already exists`);
    }
    this.store.set(order.id, order);
  }

  getById(orderId: string): Order | undefined {
    return this.store.get(orderId);
  }

  remove(orderId: string): void {
    if (!this.store.has(orderId)) {
      throw new Error(`Order ${orderId} not found`);
    }
    this.store.delete(orderId);
  }

  nextIdentity(): string {
    return crypto.randomUUID();
  }
}

// --- Anti-Corruption Layer --------------------------------------------------
// Translates data from an external (upstream) context into our model.

interface LegacyOrderDTO {
  rawOrderNum: string;
  rawCustomerRef: string;
  rawLines: Array<{
    sku: string;
    name?: string;
    qty: string;
    price: string;
  }>;
}

class OrderACL {
  static translate(dto: LegacyOrderDTO): Order {
    const order = new Order(
      `ORD-${dto.rawOrderNum}`,
      dto.rawCustomerRef,
      Address.of("Unknown", "Unknown", "N/A", "00000", "US"),
    );

    for (const rawLine of dto.rawLines) {
      order.addLine(
        rawLine.sku,
        rawLine.name ?? "Unknown Product",
        parseInt(rawLine.qty, 10),
        Money.of(parseFloat(rawLine.price)),
      );
    }

    return order;
  }
}

// --- Demo -------------------------------------------------------------------

function main(): void {
  const repo = new OrderRepository();

  // Create an order
  const orderId = repo.nextIdentity();
  const order = new Order(
    orderId,
    "CUST-42",
    Address.of("123 Elm St", "Springfield", "IL", "62704", "US"),
  );

  // Add lines through the aggregate root
  order.addLine("PROD-A", "Widget", 3, Money.of(9.99));
  order.addLine("PROD-B", "Gadget", 1, Money.of(49.99));

  console.log(`Order total: ${order.total().toString()}`);

  // Invariant: duplicate product rejected
  try {
    order.addLine("PROD-A", "Widget", 1, Money.of(9.99));
  } catch (e) {
    console.log(`Invariant enforced: ${(e as Error).message}`);
  }

  // Place the order — transitions state and publishes event
  order.place();
  console.log(`Order status: ${order.currentStatus}`);
  console.log(
    `Events published: ${JSON.stringify(order.domainEvents, null, 2)}`,
  );

  // Invariant: cannot modify a placed order
  try {
    order.removeLine("PROD-B");
  } catch (e) {
    console.log(`Invariant enforced: ${(e as Error).message}`);
  }

  // Persist via repository
  repo.add(order);

  // Anti-Corruption Layer: translate legacy data
  const legacyDTO: LegacyOrderDTO = {
    rawOrderNum: "LEG-007",
    rawCustomerRef: "EXT-CUST-99",
    rawLines: [
      { sku: "PROD-C", name: "Doohickey", qty: "2", price: "14.50" },
    ],
  };
  const translatedOrder = OrderACL.translate(legacyDTO);
  console.log(`ACL-translated order: ${translatedOrder.toString()}`);

  // Invariant: cannot cancel a placed order
  try {
    order.cancel("changed mind");
  } catch (e) {
    console.log(`Invariant enforced: ${(e as Error).message}`);
  }

  console.log(`Total lines in order: ${order.allLines.length}`);
}

main();