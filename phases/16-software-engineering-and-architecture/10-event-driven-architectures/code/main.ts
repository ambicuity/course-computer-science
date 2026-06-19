type EventData = Record<string, string | number | boolean>;

interface Event {
  type: string;
  data: EventData;
  id: string;
}

type EventHandler = (event: Event) => void;

class EventBus {
  private handlers: Map<string, EventHandler[]> = new Map();
  private processed: Set<string> = new Set();

  subscribe(eventType: string, handler: EventHandler): void {
    const existing = this.handlers.get(eventType) || [];
    existing.push(handler);
    this.handlers.set(eventType, existing);
  }

  publish(event: Event): void {
    const handlers = this.handlers.get(event.type) || [];
    for (const handler of handlers) {
      handler(event);
    }
  }

  isProcessed(id: string): boolean {
    return this.processed.has(id);
  }

  markProcessed(id: string): void {
    this.processed.add(id);
  }
}

class InventoryService {
  private reserved: Map<string, boolean> = new Map();

  constructor(private bus: EventBus) {
    bus.subscribe("OrderCreated", (e) => this.handleOrderCreated(e));
    bus.subscribe("PaymentFailed", (e) => this.handlePaymentFailed(e));
  }

  private handleOrderCreated(event: Event): void {
    const orderId = event.data.orderId as string;
    const dedupeKey = `reserve-${orderId}`;
    if (this.bus.isProcessed(dedupeKey)) {
      console.log(`  [Inventory] Skipping duplicate reservation for order ${orderId}`);
      return;
    }
    this.reserved.set(orderId, true);
    this.bus.markProcessed(dedupeKey);
    console.log(`  [Inventory] Reserved stock for order ${orderId}`);
    this.bus.publish({
      type: "InventoryReserved",
      data: { orderId },
      id: `inv-reserved-${orderId}`,
    });
  }

  private handlePaymentFailed(event: Event): void {
    const orderId = event.data.orderId as string;
    if (this.reserved.has(orderId)) {
      this.reserved.delete(orderId);
      console.log(`  [Inventory] Released stock for order ${orderId} (compensating)`);
      this.bus.publish({
        type: "InventoryReleased",
        data: { orderId },
        id: `inv-released-${orderId}`,
      });
    } else {
      console.log(`  [Inventory] No reservation to release for order ${orderId}`);
    }
  }
}

class PaymentService {
  private charged: Map<string, boolean> = new Map();

  constructor(private bus: EventBus, private shouldFail: boolean = false) {
    bus.subscribe("InventoryReserved", (e) => this.handleInventoryReserved(e));
  }

  private handleInventoryReserved(event: Event): void {
    const orderId = event.data.orderId as string;
    const dedupeKey = `pay-${orderId}`;
    if (this.bus.isProcessed(dedupeKey)) {
      console.log(`  [Payment] Skipping duplicate charge for order ${orderId}`);
      return;
    }
    this.charged.set(orderId, true);
    this.bus.markProcessed(dedupeKey);

    if (this.shouldFail) {
      console.log(`  [Payment] FAILED to charge order ${orderId}`);
      this.bus.publish({
        type: "PaymentFailed",
        data: { orderId },
        id: `pay-failed-${orderId}`,
      });
      return;
    }

    console.log(`  [Payment] Charged order ${orderId}`);
    this.bus.publish({
      type: "PaymentProcessed",
      data: { orderId },
      id: `pay-processed-${orderId}`,
    });
  }
}

class ShippingService {
  private shipped: Map<string, boolean> = new Map();

  constructor(private bus: EventBus) {
    bus.subscribe("PaymentProcessed", (e) => this.handlePaymentProcessed(e));
  }

  private handlePaymentProcessed(event: Event): void {
    const orderId = event.data.orderId as string;
    const dedupeKey = `ship-${orderId}`;
    if (this.bus.isProcessed(dedupeKey)) {
      console.log(`  [Shipping] Skipping duplicate shipment for order ${orderId}`);
      return;
    }
    this.shipped.set(orderId, true);
    this.bus.markProcessed(dedupeKey);
    console.log(`  [Shipping] Shipped order ${orderId}`);
    this.bus.publish({
      type: "OrderShipped",
      data: { orderId },
      id: `order-shipped-${orderId}`,
    });
  }
}

class OrderService {
  constructor(private bus: EventBus) {}

  createOrder(orderId: string): void {
    console.log(`[Order] Creating order ${orderId}`);
    this.bus.publish({
      type: "OrderCreated",
      data: { orderId },
      id: `order-created-${orderId}`,
    });
  }
}

class AnalyticsService {
  private events: string[] = [];

  constructor(private bus: EventBus) {
    bus.subscribe("OrderCreated", (e) => this.track(e));
    bus.subscribe("OrderShipped", (e) => this.track(e));
    bus.subscribe("PaymentFailed", (e) => this.track(e));
  }

  private track(event: Event): void {
    this.events.push(event.type);
  }

  printLog(): void {
    console.log(`\n[Analytics] Tracked events: ${this.events.join(", ")}`);
  }
}

class NotificationService {
  constructor(private bus: EventBus) {
    bus.subscribe("OrderShipped", (e) => this.sendConfirmation(e));
    bus.subscribe("PaymentFailed", (e) => this.sendFailureNotice(e));
  }

  private sendConfirmation(event: Event): void {
    const orderId = event.data.orderId as string;
    console.log(`  [Notification] Sending shipment confirmation for order ${orderId}`);
  }

  private sendFailureNotice(event: Event): void {
    const orderId = event.data.orderId as string;
    console.log(`  [Notification] Sending payment failure notice for order ${orderId}`);
  }
}

function demoSuccessfulOrder(): void {
  console.log("=== Demo 1: Successful Order Saga ===");
  console.log("Flow: OrderCreated → InventoryReserved → PaymentProcessed → OrderShipped\n");

  const bus = new EventBus();
  new AnalyticsService(bus);
  new NotificationService(bus);
  new InventoryService(bus);
  new PaymentService(bus, false);
  new ShippingService(bus);
  const orderService = new OrderService(bus);

  orderService.createOrder("ORD-001");
  console.log("\n  ✅ Order completed successfully via choreographed saga");
}

function demoFailedPayment(): void {
  console.log("\n\n=== Demo 2: Failed Payment — Saga Compensation ===");
  console.log("Flow: OrderCreated → InventoryReserved → PaymentFailed → InventoryReleased\n");

  const bus = new EventBus();
  new AnalyticsService(bus);
  new NotificationService(bus);
  new InventoryService(bus);
  new PaymentService(bus, true);
  new ShippingService(bus);
  const orderService = new OrderService(bus);

  orderService.createOrder("ORD-002");
  console.log("\n  ❌ Order failed, compensating transactions executed");
}

function demoIdempotency(): void {
  console.log("\n\n=== Demo 3: Idempotent Handlers — Duplicate Event ===");
  console.log("Publishing OrderCreated twice for the same order\n");

  const bus = new EventBus();
  new InventoryService(bus);
  new PaymentService(bus, false);
  new ShippingService(bus);
  new OrderService(bus);

  console.log("First event:");
  bus.publish({
    type: "OrderCreated",
    data: { orderId: "ORD-003" },
    id: "order-created-ORD-003",
  });

  console.log("\nSecond event (duplicate):");
  bus.publish({
    type: "OrderCreated",
    data: { orderId: "ORD-003" },
    id: "order-created-ORD-003",
  });

  console.log("\n  🔄 Second event was skipped by idempotent handlers");
}

function demoDecoupling(): void {
  console.log("\n\n=== Demo 4: Adding a Consumer Without Touching Producers ===");
  console.log("New AuditService subscribes to existing events — no producer changes needed\n");

  const bus = new EventBus();
  const auditLog: string[] = [];
  const auditHandler = (event: Event): void => {
    auditLog.push(event.type);
    console.log(`  [Audit] Logged event: ${event.type}`);
  };

  bus.subscribe("OrderCreated", auditHandler);
  bus.subscribe("OrderShipped", auditHandler);
  bus.subscribe("InventoryReleased", auditHandler);

  new InventoryService(bus);
  new PaymentService(bus, true);
  new ShippingService(bus);

  bus.publish({
    type: "OrderCreated",
    data: { orderId: "ORD-004" },
    id: "order-created-ORD-004",
  });

  console.log(`\n  📋 Audit log captured ${auditLog.length} event(s) without any producer knowing about it`);
}

function demoSagaChoreographyVsOrchestration(): void {
  console.log("\n\n=== Demo 5: Choreography vs Orchestration Comparison ===");
  console.log("Choreography: services react to events autonomously (what we built above)");
  console.log("Orchestration: a central coordinator explicitly calls each step\n");
  console.log("  Choreography flow (event-driven, decentralized):");
  console.log("    OrderService → OrderCreated → InventoryService reacts");
  console.log("    InventoryService → InventoryReserved → PaymentService reacts");
  console.log("    PaymentService → PaymentProcessed → ShippingService reacts");
  console.log("");
  console.log("  Orchestration flow (centralized, explicit):");
  console.log("    Orchestrator calls InventoryService.reserve()");
  console.log("    Orchestrator calls PaymentService.charge()");
  console.log("    Orchestrator calls ShippingService.ship()");
  console.log("    On failure: Orchestrator calls InventoryService.release()");
  console.log("");
  console.log("  Choreography: easier to extend, harder to debug");
  console.log("  Orchestration: easier to debug, harder to scale (orchestrator bottleneck)");
}

function main(): void {
  demoSuccessfulOrder();
  demoFailedPayment();
  demoIdempotency();
  demoDecoupling();
  demoSagaChoreographyVsOrchestration();
}

main();