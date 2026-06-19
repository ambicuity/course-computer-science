// =============================================================================
// PHASE 16 CAPSTONE — Refactor a Real OSS Repo + ADR Bundle
// Lesson 22: Before & After — Smelly Code → Hexagonal Architecture
// =============================================================================
//
// This file demonstrates the complete capstone workflow:
// 1. BEFORE: Smelly OrderService (god class, tight coupling, no tests)
// 2. AFTER:  Refactored hexagonal architecture with CQRS + events
// 3. TESTS:  Unit tests for domain, application, and adapter layers
// 4. COMPARISON: Side-by-side demonstration
//
// References:
//   L02 — Naming, Cohesion, Coupling (primitive obsession → value objects)
//   L03 — SOLID (god class → SRP, DIP via ports)
//   L06 — Refactoring Mechanics (small steps, test between each)
//   L08 — DDD (entities, value objects, aggregates, domain events)
//   L09 — Hexagonal Architecture (ports and adapters)
//   L10 — Event-Driven (domain events, pub/sub)
//   L11 — CQRS (command/query split)
// =============================================================================

// =============================================================================
// SECTION 1: BEFORE — The Smelly Code
// =============================================================================
// L02 violations: primitive obsession (string for OrderId, number for Money)
// L03 violations: god class (SRP), no dependency inversion (DIP)
// L06 violations: no safe refactoring path (no tests)
// L09 violations: no ports, no adapters, everything mixed
// L10 violations: synchronous call chain, no events
// =============================================================================

type SmellyOrderStatus = "pending" | "confirmed" | "cancelled";

interface SmellyOrderItem {
  productId: string;
  quantity: number;
  unitPrice: number;
}

class SmellyDatabaseConnection {
  private orders: Map<string, any> = new Map();

  save(id: string, data: any): void {
    this.orders.set(id, data);
  }

  findById(id: string): any {
    return this.orders.get(id);
  }

  findAll(): any[] {
    return Array.from(this.orders.values());
  }
}

class SmellySmtpClient {
  send(to: string, subject: string, body: string): void {
    console.log(`[SMTP] To: ${to}, Subject: ${subject}`);
  }
}

class SmellyInventoryApi {
  private stock: Map<string, number> = new Map([
    ["p1", 100],
    ["p2", 50],
    ["p3", 25],
  ]);

  checkAndReserve(productId: string, qty: number): boolean {
    const available = this.stock.get(productId) ?? 0;
    if (available >= qty) {
      this.stock.set(productId, available - qty);
      return true;
    }
    return false;
  }
}

// GOD CLASS: 200+ lines mixing business logic, persistence,
// notification, and inventory. Violates SRP (L03), has tight
// coupling (L02), no dependency inversion (L03 DIP).
class SmellyOrderService {
  private db: SmellyDatabaseConnection;
  private smtp: SmellySmtpClient;
  private inventory: SmellyInventoryApi;
  private nextId: number = 1;

  constructor(
    db: SmellyDatabaseConnection,
    smtp: SmellySmtpClient,
    inventory: SmellyInventoryApi
  ) {
    this.db = db;
    this.smtp = smtp;
    this.inventory = inventory;
  }

  // processOrder mixes: validation, pricing, inventory, persistence, notification
  processOrder(
    customerId: string,
    items: SmellyOrderItem[],
    discountCode?: string
  ): { id: string; total: number; status: string } {
    if (!customerId || customerId.length === 0) {
      throw new Error("Invalid customer ID");
    }
    if (!items || items.length === 0) {
      throw new Error("Order must have items");
    }

    // Pricing logic embedded in processOrder (SRP violation)
    let subtotal = 0;
    for (const item of items) {
      subtotal += item.unitPrice * item.quantity;
    }

    // Discount logic mixed in (L02: should be separate concern)
    let discount = 0;
    if (discountCode === "SAVE10") {
      discount = subtotal * 0.1;
    } else if (discountCode === "SAVE20") {
      discount = subtotal * 0.2;
    }

    const total = subtotal - discount;

    // Inventory check — synchronous, blocks order if inventory fails
    for (const item of items) {
      const reserved = this.inventory.checkAndReserve(item.productId, item.quantity);
      if (!reserved) {
        throw new Error(`Insufficient inventory for product ${item.productId}`);
      }
    }

    // Persistence — direct DB call, no repository abstraction
    const id = `ORD-${this.nextId++}`;
    const order = {
      id,
      customerId,
      items,
      subtotal,
      discount,
      total,
      status: "confirmed" as SmellyOrderStatus,
      createdAt: new Date(),
    };
    this.db.save(id, order);

    // Notification — direct SMTP call, no abstraction
    this.smtp.send(
      customerId,
      `Order ${id} Confirmed`,
      `Your order total is $${total.toFixed(2)}`
    );

    return { id, total, status: "confirmed" };
  }

  getOrder(id: string): any {
    const order = this.db.findById(id);
    if (!order) {
      throw new Error(`Order ${id} not found`);
    }
    return order;
  }

  listOrders(): any[] {
    return this.db.findAll();
  }

  cancelOrder(id: string, reason: string): void {
    const order = this.db.findById(id);
    if (!order) {
      throw new Error(`Order ${id} not found`);
    }
    if (order.status === "cancelled") {
      throw new Error(`Order ${id} already cancelled`);
    }
    order.status = "cancelled";
    order.cancelledAt = new Date();
    order.cancelReason = reason;
    this.db.save(id, order);

    this.smtp.send(
      order.customerId,
      `Order ${id} Cancelled`,
      `Order cancelled: ${reason}`
    );
  }
}

// =============================================================================
// SECTION 2: AFTER — Hexagonal Architecture with CQRS + Events
// =============================================================================

// --- Domain Layer (L02: Value Objects, L08: DDD) ---

class OrderId {
  private constructor(private readonly value: string) {}

  static generate(): OrderId {
    return new OrderId(`ORD-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`);
  }

  static fromString(value: string): OrderId {
    if (!value || value.length === 0) {
      throw new Error("OrderId cannot be empty");
    }
    return new OrderId(value);
  }

  equals(other: OrderId): boolean {
    return this.value === other.value;
  }

  toString(): string {
    return this.value;
  }
}

class Money {
  private constructor(private readonly cents: number) {
    if (!Number.isInteger(cents)) {
      throw new Error("Money must be in whole cents");
    }
  }

  static from(amount: number): Money {
    return new Money(Math.round(amount * 100));
  }

  static zero(): Money {
    return new Money(0);
  }

  get amount(): number {
    return this.cents / 100;
  }

  add(other: Money): Money {
    return new Money(this.cents + other.cents);
  }

  subtract(other: Money): Money {
    return new Money(this.cents - other.cents);
  }

  multiply(factor: number): Money {
    return new Money(Math.round(this.cents * factor));
  }

  isNegative(): boolean {
    return this.cents < 0;
  }

  equals(other: Money): boolean {
    return this.cents === other.cents;
  }

  toString(): string {
    return `$${this.amount.toFixed(2)}`;
  }
}

// L08: OrderLineItem is part of the Order aggregate
class OrderLineItem {
  private constructor(
    public readonly productId: string,
    public readonly quantity: number,
    public readonly unitPrice: Money
  ) {
    if (quantity <= 0) {
      throw new Error("Quantity must be positive");
    }
  }

  static create(productId: string, quantity: number, unitPrice: Money): OrderLineItem {
    return new OrderLineItem(productId, quantity, unitPrice);
  }

  get lineTotal(): Money {
    return this.unitPrice.multiply(this.quantity);
  }
}

// L08: Order is an Aggregate Root with domain behavior and events
enum OrderStatus {
  Pending = "pending",
  Confirmed = "confirmed",
  Cancelled = "cancelled",
}

class Order {
  private readonly lineItems: OrderLineItem[] = [];
  private readonly domainEvents: DomainEvent[] = [];
  private _status: OrderStatus = OrderStatus.Pending;
  private _discount: Money = Money.zero();

  private constructor(
    public readonly id: OrderId,
    public readonly customerId: string,
    items: OrderLineItem[],
    public readonly createdAt: Date = new Date()
  ) {}

  static create(customerId: string, rawItems: Array<{ productId: string; quantity: number; unitPrice: Money }>): Order {
    if (!customerId || customerId.length === 0) {
      throw new Error("CustomerId is required");
    }
    if (!rawItems || rawItems.length === 0) {
      throw new Error("Order must have at least one item");
    }

    const id = OrderId.generate();
    const lineItems = rawItems.map((item) =>
      OrderLineItem.create(item.productId, item.quantity, item.unitPrice)
    );
    const order = new Order(id, customerId, lineItems);
    order._status = OrderStatus.Confirmed;

    // L10: Domain event emitted on creation
    order.domainEvents.push(new OrderCreatedEvent(id, customerId, order.subtotal));

    return order;
  }

  get status(): OrderStatus {
    return this._status;
  }

  get items(): ReadonlyArray<OrderLineItem> {
    return this.lineItems;
  }

  get subtotal(): Money {
    return this.lineItems.reduce(
      (sum, item) => sum.add(item.lineTotal),
      Money.zero()
    );
  }

  get discount(): Money {
    return this._discount;
  }

  get total(): Money {
    return this.subtotal.subtract(this._discount);
  }

  getDomainEvents(): DomainEvent[] {
    return [...this.domainEvents];
  }

  clearDomainEvents(): void {
    this.domainEvents.length = 0;
  }

  applyDiscount(discount: Money): void {
    if (discount.isNegative()) {
      throw new Error("Discount cannot be negative");
    }
    if (discount.amount > this.subtotal.amount) {
      throw new Error("Discount cannot exceed subtotal");
    }
    this._discount = discount;
  }

  cancel(reason: string): void {
    if (this._status === OrderStatus.Cancelled) {
      throw new Error("Order already cancelled");
    }
    this._status = OrderStatus.Cancelled;
    this.domainEvents.push(new OrderCancelledEvent(this.id, reason));
  }
}

// --- Domain Events (L10: Event-Driven, L08: Domain Events) ---

abstract class DomainEvent {
  constructor(public readonly occurredAt: Date = new Date()) {}
}

class OrderCreatedEvent extends DomainEvent {
  constructor(
    public readonly orderId: OrderId,
    public readonly customerId: string,
    public readonly total: Money
  ) {
    super();
  }
}

class OrderCancelledEvent extends DomainEvent {
  constructor(
    public readonly orderId: OrderId,
    public readonly reason: string
  ) {
    super();
  }
}

// --- Ports Layer (L09: Hexagonal Architecture interfaces) ---

interface OrderRepository {
  save(order: Order): Promise<void>;
  findById(id: OrderId): Promise<Order | null>;
  findAll(): Promise<Order[]>;
}

interface NotificationPort {
  sendOrderConfirmation(orderId: OrderId, customerId: string, total: Money): Promise<void>;
  sendOrderCancellation(orderId: OrderId, customerId: string, reason: string): Promise<void>;
}

interface InventoryPort {
  reserve(productId: string, quantity: number): Promise<boolean>;
  release(productId: string, quantity: number): Promise<void>;
}

interface EventPublisher {
  publish(event: DomainEvent): void;
  subscribe(eventType: string, handler: EventHandler): void;
}

type EventHandler = (event: DomainEvent) => void;

// --- Adapters Layer (L09: Concrete implementations) ---

class InMemoryOrderRepository implements OrderRepository {
  private orders: Map<string, Order> = new Map();

  async save(order: Order): Promise<void> {
    this.orders.set(order.id.toString(), order);
  }

  async findById(id: OrderId): Promise<Order | null> {
    return this.orders.get(id.toString()) ?? null;
  }

  async findAll(): Promise<Order[]> {
    return Array.from(this.orders.values());
  }
}

class ConsoleNotificationAdapter implements NotificationPort {
  async sendOrderConfirmation(orderId: OrderId, customerId: string, total: Money): Promise<void> {
    console.log(`[NOTIFICATION] Order ${orderId.toString()} confirmed for ${customerId}. Total: ${total.toString()}`);
  }

  async sendOrderCancellation(orderId: OrderId, customerId: string, reason: string): Promise<void> {
    console.log(`[NOTIFICATION] Order ${orderId.toString()} cancelled for ${customerId}. Reason: ${reason}`);
  }
}

class InMemoryInventoryAdapter implements InventoryPort {
  private stock: Map<string, number> = new Map([
    ["p1", 100],
    ["p2", 50],
    ["p3", 25],
  ]);

  async reserve(productId: string, quantity: number): Promise<boolean> {
    const available = this.stock.get(productId) ?? 0;
    if (available >= quantity) {
      this.stock.set(productId, available - quantity);
      return true;
    }
    return false;
  }

  async release(productId: string, quantity: number): Promise<void> {
    const current = this.stock.get(productId) ?? 0;
    this.stock.set(productId, current + quantity);
  }
}

class InProcessEventBus implements EventPublisher {
  private handlers: Map<string, EventHandler[]> = new Map();
  private publishedEvents: DomainEvent[] = [];

  subscribe(eventType: string, handler: EventHandler): void {
    const existing = this.handlers.get(eventType) ?? [];
    existing.push(handler);
    this.handlers.set(eventType, existing);
  }

  publish(event: DomainEvent): void {
    this.publishedEvents.push(event);
    const eventName = event.constructor.name;
    const handlers = this.handlers.get(eventName) ?? [];
    for (const handler of handlers) {
      try {
        handler(event);
      } catch (err) {
        console.error(`[EVENTBUS] Handler error for ${eventName}:`, err);
      }
    }
  }

  getEvents(): DomainEvent[] {
    return [...this.publishedEvents];
  }

  clearEvents(): void {
    this.publishedEvents = [];
  }
}

// --- Application Layer (L11: CQRS split) ---

// L11: Command side — mutates state, publishes events
class OrderCommandService {
  constructor(
    private orderRepo: OrderRepository,
    private eventBus: EventPublisher,
    private inventoryPort: InventoryPort,
    private notificationPort: NotificationPort
  ) {}

  async createOrder(
    customerId: string,
    items: Array<{ productId: string; quantity: number; unitPrice: Money }>,
    discountCode?: string
  ): Promise<OrderId> {
    const order = Order.create(customerId, items);

    if (discountCode) {
      const discount = this.calculateDiscount(order.subtotal, discountCode);
      order.applyDiscount(discount);
    }

    for (const item of order.items) {
      const reserved = await this.inventoryPort.reserve(item.productId, item.quantity);
      if (!reserved) {
        for (const pItem of order.items) {
          if (pItem.productId === item.productId) break;
          await this.inventoryPort.release(pItem.productId, pItem.quantity);
        }
        throw new Error(`Insufficient inventory for product ${item.productId}`);
      }
    }

    await this.orderRepo.save(order);

    const events = order.getDomainEvents();
    for (const event of events) {
      this.eventBus.publish(event);
    }
    order.clearDomainEvents();

    return order.id;
  }

  async cancelOrder(orderId: OrderId, reason: string): Promise<void> {
    const order = await this.orderRepo.findById(orderId);
    if (!order) {
      throw new Error(`Order ${orderId.toString()} not found`);
    }

    order.cancel(reason);
    await this.orderRepo.save(order);

    for (const event of order.getDomainEvents()) {
      this.eventBus.publish(event);
    }
    order.clearDomainEvents();
  }

  private calculateDiscount(subtotal: Money, code: string): Money {
    switch (code) {
      case "SAVE10":
        return subtotal.multiply(0.1);
      case "SAVE20":
        return subtotal.multiply(0.2);
      default:
        return Money.zero();
    }
  }
}

// L11: Query side — read-only, returns DTOs
class OrderDto {
  constructor(
    public readonly id: string,
    public readonly customerId: string,
    public readonly total: number,
    public readonly status: string,
    public readonly itemCount: number
  ) {}
}

class OrderQueryService {
  constructor(private orderRepo: OrderRepository) {}

  async getOrder(orderId: OrderId): Promise<OrderDto | null> {
    const order = await this.orderRepo.findById(orderId);
    if (!order) return null;
    return new OrderDto(
      order.id.toString(),
      order.customerId,
      order.total.amount,
      order.status,
      order.items.length
    );
  }

  async listOrders(): Promise<OrderDto[]> {
    const orders = await this.orderRepo.findAll();
    return orders.map(
      (o) =>
        new OrderDto(
          o.id.toString(),
          o.customerId,
          o.total.amount,
          o.status,
          o.items.length
        )
    );
  }
}

// --- Event Subscribers (L10: Event-Driven side effects) ---

class InventoryEventSubscriber {
  constructor(private inventory: InventoryPort) {}

  async handleOrderCreated(event: OrderCreatedEvent): Promise<void> {
    // Inventory was already reserved in command handler;
    // this subscriber could handle additional scenarios
  }
}

class NotificationEventSubscriber {
  constructor(private notification: NotificationPort) {}

  async handleOrderCreated(event: OrderCreatedEvent): Promise<void> {
    await this.notification.sendOrderConfirmation(
      event.orderId,
      event.customerId,
      event.total
    );
  }

  async handleOrderCancelled(event: OrderCancelledEvent): Promise<void> {
    await this.notification.sendOrderCancellation(
      event.orderId,
      event.customerId,
      event.reason
    );
  }
}

// =============================================================================
// SECTION 3: TESTS — Domain, Application, and Adapter layer tests
// =============================================================================

class TestRunner {
  private passed = 0;
  private failed = 0;
  private errors: string[] = [];

  assert(condition: boolean, message: string): void {
    if (condition) {
      this.passed++;
    } else {
      this.failed++;
      this.errors.push(`FAIL: ${message}`);
    }
  }

  assertEqual(actual: any, expected: any, message: string): void {
    if (actual === expected) {
      this.passed++;
    } else {
      this.failed++;
      this.errors.push(`FAIL: ${message} — expected ${expected}, got ${actual}`);
    }
  }

  assertThrows(fn: () => void, expectedMessage: string, message: string): void {
    try {
      fn();
      this.failed++;
      this.errors.push(`FAIL: ${message} — expected throw`);
    } catch (e: any) {
      if (e.message.includes(expectedMessage)) {
        this.passed++;
      } else {
        this.failed++;
        this.errors.push(`FAIL: ${message} — expected "${expectedMessage}", got "${e.message}"`);
      }
    }
  }

  group(name: string, fn: () => void): void {
    console.log(`\n  📦 ${name}`);
    fn();
  }

  summary(): void {
    console.log(`\n${"=".repeat(60)}`);
    console.log(`  Test Results: ${this.passed} passed, ${this.failed} failed`);
    if (this.errors.length > 0) {
      console.log(`\n  Errors:`);
      this.errors.forEach((e) => console.log(`    ${e}`));
    }
    console.log(`${"=".repeat(60)}\n`);
  }
}

function runDomainTests(runner: TestRunner): void {
  runner.group("Domain: Money", () => {
    const m1 = Money.from(10);
    const m2 = Money.from(5);
    runner.assertEqual(m1.add(m2).amount, 15, "Money.add works");
    runner.assertEqual(m1.subtract(m2).amount, 5, "Money.subtract works");
    runner.assertEqual(m1.multiply(0.1).amount, 1, "Money.multiply works");
    runner.assertEqual(Money.zero().amount, 0, "Money.zero is zero");
    runner.assert(!m1.isNegative(), "Positive money not negative");
    runner.assert(Money.from(-5).isNegative(), "Negative money is negative");
    runner.assertThrows(
      () => Money.fromString(""),
      "empty",
      "OrderId rejects empty string"
    );
  });

  runner.group("Domain: OrderId", () => {
    const id1 = OrderId.generate();
    const id2 = OrderId.generate();
    runner.assert(!id1.equals(id2), "Different OrderIds not equal");
    const id3 = OrderId.fromString("ORD-123");
    const id4 = OrderId.fromString("ORD-123");
    runner.assert(id3.equals(id4), "Same OrderId values equal");
    runner.assertEqual(id3.toString(), "ORD-123", "OrderId toString works");
  });

  runner.group("Domain: Order", () => {
    const order = Order.create("customer-1", [
      { productId: "p1", quantity: 2, unitPrice: Money.from(10) },
      { productId: "p2", quantity: 1, unitPrice: Money.from(25) },
    ]);

    runner.assertEqual(order.status, OrderStatus.Confirmed, "New order is confirmed");
    runner.assertEqual(order.items.length, 2, "Order has 2 items");
    runner.assertEqual(order.subtotal.amount, 45, "Subtotal is 45");
    runner.assertEqual(order.total.amount, 45, "Total equals subtotal without discount");

    const events = order.getDomainEvents();
    runner.assertEqual(events.length, 1, "Order emits one domain event");
    runner.assert(events[0] instanceof OrderCreatedEvent, "Event is OrderCreatedEvent");

    order.applyDiscount(Money.from(10));
    runner.assertEqual(order.discount.amount, 10, "Discount applied");
    runner.assertEqual(order.total.amount, 35, "Total reflects discount");

    order.cancel("customer request");
    runner.assertEqual(order.status, OrderStatus.Cancelled, "Order is cancelled");
    runner.assertThrows(
      () => order.cancel("again"),
      "already cancelled",
      "Cannot cancel twice"
    );

    runner.assertThrows(
      () => Order.create("", []),
      "CustomerId is required",
      "Empty customerId rejected"
    );

    runner.assertThrows(
      () => order.applyDiscount(Money.from(-5)),
      "negative",
      "Negative discount rejected"
    );
  });
}

async function runApplicationTests(runner: TestRunner): Promise<void> {
  runner.group("Application: OrderCommandService", async () => {
    const repo = new InMemoryOrderRepository();
    const eventBus = new InProcessEventBus();
    const inventory = new InMemoryInventoryAdapter();
    const notification = new ConsoleNotificationAdapter();

    const notificationSub = new NotificationEventSubscriber(notification);
    eventBus.subscribe("OrderCreatedEvent", (e) => notificationSub.handleOrderCreated(e as OrderCreatedEvent));
    eventBus.subscribe("OrderCancelledEvent", (e) => notificationSub.handleOrderCancelled(e as OrderCancelledEvent));

    const commandService = new OrderCommandService(repo, eventBus, inventory, notification);

    const orderId = await commandService.createOrder("customer-1", [
      { productId: "p1", quantity: 2, unitPrice: Money.from(10) },
      { productId: "p2", quantity: 1, unitPrice: Money.from(25) },
    ]);

    runner.assert(orderId.toString().startsWith("ORD-"), "OrderId has correct prefix");

    const saved = await repo.findById(orderId);
    runner.assert(saved !== null, "Order persisted");
    runner.assertEqual(saved!.status, OrderStatus.Confirmed, "Saved order is confirmed");

    runner.assertEqual(eventBus.getEvents().length, 1, "One event published");
    runner.assert(eventBus.getEvents()[0] instanceof OrderCreatedEvent, "Published OrderCreatedEvent");

    await commandService.cancelOrder(orderId, "changed mind");
    const cancelled = await repo.findById(orderId);
    runner.assertEqual(cancelled!.status, OrderStatus.Cancelled, "Order cancelled");
    runner.assertEqual(eventBus.getEvents().length, 2, "Cancellation event published");
  });

  runner.group("Application: OrderCommandService with discount", async () => {
    const repo = new InMemoryOrderRepository();
    const eventBus = new InProcessEventBus();
    const inventory = new InMemoryInventoryAdapter();
    const notification = new ConsoleNotificationAdapter();
    const commandService = new OrderCommandService(repo, eventBus, inventory, notification);

    const orderId = await commandService.createOrder(
      "customer-2",
      [{ productId: "p1", quantity: 1, unitPrice: Money.from(100) }],
      "SAVE10"
    );

    const order = await repo.findById(orderId);
    runner.assertEqual(order!.total.amount, 90, "10% discount applied");

    const orderId2 = await commandService.createOrder(
      "customer-2",
      [{ productId: "p2", quantity: 1, unitPrice: Money.from(100) }],
      "SAVE20"
    );
    const order2 = await repo.findById(orderId2);
    runner.assertEqual(order2!.total.amount, 80, "20% discount applied");
  });

  runner.group("Application: OrderQueryService", async () => {
    const repo = new InMemoryOrderRepository();
    const eventBus = new InProcessEventBus();
    const inventory = new InMemoryInventoryAdapter();
    const notification = new ConsoleNotificationAdapter();
    const commandService = new OrderCommandService(repo, eventBus, inventory, notification);
    const queryService = new OrderQueryService(repo);

    const id1 = await commandService.createOrder("c1", [
      { productId: "p3", quantity: 1, unitPrice: Money.from(50) },
    ]);
    const id2 = await commandService.createOrder("c2", [
      { productId: "p1", quantity: 1, unitPrice: Money.from(10) },
    ]);

    const dto = await queryService.getOrder(id1);
    runner.assert(dto !== null, "Order found by ID");
    runner.assertEqual(dto!.customerId, "c1", "DTO has correct customerId");
    runner.assertEqual(dto!.total, 50, "DTO has correct total");
    runner.assertEqual(dto!.status, "confirmed", "DTO status is confirmed");

    const allDtos = await queryService.listOrders();
    runner.assertEqual(allDtos.length, 2, "List returns 2 orders");
  });
}

// =============================================================================
// SECTION 4: COMPARISON — Before vs After side-by-side
// =============================================================================

function demonstrateComparison(): void {
  console.log("\n" + "=".repeat(60));
  console.log("  CAPSTONE DEMONSTRATION: Before vs After");
  console.log("=".repeat(60));

  // --- BEFORE ---
  console.log("\n--- BEFORE: Smelly OrderService (God Class) ---\n");

  const smellyDb = new SmellyDatabaseConnection();
  const smellySmtp = new SmellySmtpClient();
  const smellyInventory = new SmellyInventoryApi();
  const smellyService = new SmellyOrderService(smellyDb, smellySmtp, smellyInventory);

  try {
    const result = smellyService.processOrder("alice", [
      { productId: "p1", quantity: 2, unitPrice: 10 },
      { productId: "p2", quantity: 1, unitPrice: 25 },
    ], "SAVE10");
    console.log(`  Smelly result: id=${result.id}, total=${result.total}, status=${result.status}`);
  } catch (e: any) {
    console.log(`  Smelly error: ${e.message}`);
  }

  // --- AFTER ---
  console.log("\n--- AFTER: Hexagonal Architecture with CQRS + Events ---\n");

  (async () => {
    const repo = new InMemoryOrderRepository();
    const eventBus = new InProcessEventBus();
    const inventory = new InMemoryInventoryAdapter();
    const notification = new ConsoleNotificationAdapter();

    const notificationSub = new NotificationEventSubscriber(notification);
    eventBus.subscribe("OrderCreatedEvent", (e) => notificationSub.handleOrderCreated(e as OrderCreatedEvent));

    const commandService = new OrderCommandService(repo, eventBus, inventory, notification);
    const queryService = new OrderQueryService(repo);

    const orderId = await commandService.createOrder("alice", [
      { productId: "p1", quantity: 2, unitPrice: Money.from(10) },
      { productId: "p2", quantity: 1, unitPrice: Money.from(25) },
    ], "SAVE10");

    console.log(`  Clean orderId: ${orderId.toString()}`);

    const orderDto = await queryService.getOrder(orderId);
    if (orderDto) {
      console.log(`  Clean result: id=${orderDto.id}, total=${orderDto.total}, status=${orderDto.status}`);
    }

    console.log(`  Events published: ${eventBus.getEvents().length}`);
  })();
}

async function main(): Promise<void> {
  console.log("Phase 16, Lesson 22 — Phase Capstone: Refactor OSS Repo + ADR Bundle\n");

  const runner = new TestRunner();

  runDomainTests(runner);
  await runApplicationTests(runner);

  runner.summary();
  demonstrateComparison();

  console.log("\nArchitecture Improvements Demonstrated:");
  console.log("  L02: Primitive obsession → Value objects (Money, OrderId)");
  console.log("  L03: God class → SRP (separate services), DIP (ports/interfaces)");
  console.log("  L06: Refactoring via small, tested steps (see ADR refs in commits)");
  console.log("  L08: DDD entities (Order), VOs (Money), Aggregates, Domain Events");
  console.log("  L09: Hexagonal Architecture — domain → ports → adapters");
  console.log("  L10: Event-Driven — OrderCreatedEvent with subscribers");
  console.log("  L11: CQRS — OrderCommandService vs OrderQueryService");
  console.log("  L20: ADRs document every decision (see code/notes.md)");
  console.log("  L21: Readable structure → domain/ ports/ adapters/ application/");
}

main().catch(console.error);