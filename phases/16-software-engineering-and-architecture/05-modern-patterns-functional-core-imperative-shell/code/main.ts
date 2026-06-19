// Modern Patterns — Functional Core / Imperative Shell
// Phase 16 — Software Engineering & Architecture
//
// Demonstrates separating pure business logic from side effects
// in an order processing system using TypeScript.

// ============================================================================
// FUNCTIONAL CORE — Pure functions, no side effects, no dependencies
// ============================================================================

interface OrderItem {
  productId: string;
  name: string;
  quantity: number;
  unitPrice: number;
}

interface Order {
  customerId: string;
  items: OrderItem[];
}

interface ValidatedOrder {
  order: Order;
}

interface PricedOrder {
  order: Order;
  subtotal: number;
  discountAmount: number;
  total: number;
  discountReason: string;
}

enum CustomerTier {
  Gold = "gold",
  Silver = "silver",
  Bronze = "bronze",
}

interface Discount {
  amount: number;
  reason: string;
}

type ValidationError =
  | { kind: "empty_order" }
  | { kind: "missing_customer_id" }
  | { kind: "negative_quantity"; productId: string };

type OrderDecision =
  | { kind: "accept"; pricedOrder: PricedOrder }
  | { kind: "reject"; error: ValidationError };

function validateOrder(order: Order): ValidatedOrder {
  if (!order.customerId || order.customerId.trim().length === 0) {
    throw { kind: "missing_customer_id" } as ValidationError;
  }
  if (order.items.length === 0) {
    throw { kind: "empty_order" } as ValidationError;
  }
  for (const item of order.items) {
    if (item.quantity <= 0) {
      throw { kind: "negative_quantity", productId: item.productId } as ValidationError;
    }
  }
  return { order };
}

function calculateDiscount(order: ValidatedOrder, tier: CustomerTier): Discount {
  const subtotal = order.order.items.reduce(
    (sum, item) => sum + item.quantity * item.unitPrice,
    0
  );
  const rates: Record<CustomerTier, { pct: number; label: string }> = {
    [CustomerTier.Gold]: { pct: 0.15, label: "gold_tier" },
    [CustomerTier.Silver]: { pct: 0.10, label: "silver_tier" },
    [CustomerTier.Bronze]: { pct: 0.05, label: "bronze_tier" },
  };
  const { pct, label } = rates[tier];
  return { amount: subtotal * pct, reason: label };
}

function applyDiscount(order: ValidatedOrder, discount: Discount): PricedOrder {
  const subtotal = order.order.items.reduce(
    (sum, item) => sum + item.quantity * item.unitPrice,
    0
  );
  const total = Math.max(0, subtotal - discount.amount);
  return {
    order: order.order,
    subtotal,
    discountAmount: discount.amount,
    total,
    discountReason: discount.reason,
  };
}

function processOrderCore(order: Order, tier: CustomerTier): OrderDecision {
  try {
    const validated = validateOrder(order);
    const discount = calculateDiscount(validated, tier);
    const pricedOrder = applyDiscount(validated, discount);
    return { kind: "accept", pricedOrder };
  } catch (e) {
    return { kind: "reject", error: e as ValidationError };
  }
}

// ============================================================================
// IMPERATIVE SHELL — I/O, database, HTTP, logging
// ============================================================================

interface ShellConfig {
  dbUrl: string;
  logLevel: string;
}

async function shellFetchCustomerTier(
  customerId: string,
  config: ShellConfig
): Promise<CustomerTier> {
  console.log(
    `[SHELL][db] Querying tier for customer '${customerId}' at ${config.dbUrl}`
  );
  // Simulate async database call
  await new Promise((resolve) => setTimeout(resolve, 10));
  if (customerId.startsWith("G")) return CustomerTier.Gold;
  if (customerId.startsWith("S")) return CustomerTier.Silver;
  return CustomerTier.Bronze;
}

async function shellSaveOrder(pricedOrder: PricedOrder, config: ShellConfig): Promise<void> {
  console.log(
    `[SHELL][db] Saving order for customer '${pricedOrder.order.customerId}' — total: $${pricedOrder.total.toFixed(2)} at ${config.dbUrl}`
  );
  await new Promise((resolve) => setTimeout(resolve, 10));
}

async function shellSendNotification(pricedOrder: PricedOrder): Promise<void> {
  console.log(
    `[SHELL][http] POST /notify — customer ${pricedOrder.order.customerId} charged $${pricedOrder.total.toFixed(2)} ` +
      `(discount: $${pricedOrder.discountAmount.toFixed(2)} — ${pricedOrder.discountReason})`
  );
  await new Promise((resolve) => setTimeout(resolve, 10));
}

function shellLogRejection(error: ValidationError): void {
  console.log(`[SHELL][log] Order rejected: ${JSON.stringify(error)}`);
}

async function shellRun(order: Order, config: ShellConfig): Promise<void> {
  console.log(
    `[SHELL] --- Processing order for customer '${order.customerId}' ---`
  );

  const tier = await shellFetchCustomerTier(order.customerId, config);

  const decision = processOrderCore(order, tier);

  switch (decision.kind) {
    case "accept": {
      await shellSaveOrder(decision.pricedOrder, config);
      await shellSendNotification(decision.pricedOrder);
      console.log(
        `[SHELL] ✓ Order accepted — subtotal: $${decision.pricedOrder.subtotal.toFixed(2)}, ` +
          `discount: $${decision.pricedOrder.discountAmount.toFixed(2)}, ` +
          `total: $${decision.pricedOrder.total.toFixed(2)}`
      );
      break;
    }
    case "reject": {
      shellLogRejection(decision.error);
      console.log("[SHELL] ✗ Order rejected");
      break;
    }
  }
}

// ============================================================================
// TESTS — Pure core tests require NO infrastructure, NO mocks
// ============================================================================

function assert(condition: boolean, message: string): void {
  if (!condition) throw new Error(`Assertion failed: ${message}`);
}

function assertEqual<T>(actual: T, expected: T, label: string): void {
  assert(JSON.stringify(actual) === JSON.stringify(expected), `${label}: ${JSON.stringify(actual)} !== ${JSON.stringify(expected)}`);
}

function runCoreTests(): void {
  const sampleOrder: Order = {
    customerId: "CUST-001",
    items: [
      { productId: "SKU-A", name: "Widget", quantity: 3, unitPrice: 10.0 },
      { productId: "SKU-B", name: "Gadget", quantity: 1, unitPrice: 25.0 },
    ],
  };

  // validate — valid order
  const validated = validateOrder(sampleOrder);
  assertEqual(validated.order, sampleOrder, "validate valid order");

  // validate — empty customer ID
  let caught = false;
  try {
    validateOrder({ ...sampleOrder, customerId: "" });
  } catch (e) {
    caught = true;
    assert((e as ValidationError).kind === "missing_customer_id", "empty customer id");
  }
  assert(caught, "should throw for empty customer id");

  // validate — empty items
  caught = false;
  try {
    validateOrder({ customerId: "CUST-001", items: [] });
  } catch (e) {
    caught = true;
    assert((e as ValidationError).kind === "empty_order", "empty order");
  }
  assert(caught, "should throw for empty order");

  // calculate discount — gold tier (15%)
  const goldDiscount = calculateDiscount(validated, CustomerTier.Gold);
  assert(Math.abs(goldDiscount.amount - 8.25) < 0.001, `gold discount: ${goldDiscount.amount}`);
  assertEqual(goldDiscount.reason, "gold_tier", "gold reason");

  // calculate discount — silver tier (10%)
  const silverDiscount = calculateDiscount(validated, CustomerTier.Silver);
  assert(Math.abs(silverDiscount.amount - 5.5) < 0.001, `silver discount: ${silverDiscount.amount}`);

  // apply discount — total never negative
  const megaDiscount: Discount = { amount: 99999, reason: "mega_sale" };
  const pricedWithMega = applyDiscount(validated, megaDiscount);
  assert(pricedWithMega.total === 0, "total should be clamped to 0");

  // process order — accept path
  const acceptDecision = processOrderCore(sampleOrder, CustomerTier.Gold);
  assert(acceptDecision.kind === "accept", "should accept valid order");
  if (acceptDecision.kind === "accept") {
    assert(Math.abs(acceptDecision.pricedOrder.total - 46.75) < 0.01, "total should be 46.75");
  }

  // process order — reject path
  const rejectDecision = processOrderCore(
    { customerId: "CUST-001", items: [] },
    CustomerTier.Gold
  );
  assert(rejectDecision.kind === "reject", "should reject empty order");

  console.log("[CORE TESTS] All core tests passed ✓");
}

// ============================================================================
// MAIN — Shell orchestrates the whole flow
// ============================================================================

async function main(): Promise<void> {
  // Run pure core tests first — no infrastructure needed
  runCoreTests();
  console.log();

  const config: ShellConfig = {
    dbUrl: "postgres://localhost/orders",
    logLevel: "info",
  };

  // Happy path
  const order: Order = {
    customerId: "GOLD-CUST-42",
    items: [
      { productId: "SKU-WIDGET", name: "Premium Widget", quantity: 5, unitPrice: 20.0 },
      { productId: "SKU-GADGET", name: "Super Gadget", quantity: 2, unitPrice: 50.0 },
    ],
  };

  await shellRun(order, config);

  console.log();

  // Sad path — invalid order
  const badOrder: Order = {
    customerId: "",
    items: [],
  };
  await shellRun(badOrder, config);
}

main().catch(console.error);