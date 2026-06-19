/**
 * Naming, Cohesion, Coupling — TypeScript Implementation
 * Phase 16 — Software Engineering & Architecture
 *
 * Before/after examples showing:
 *   1. Bad naming → good naming
 *   2. Low cohesion → high cohesion refactoring
 *   3. Tight coupling → loose coupling via dependency injection
 *   4. Simple cohesion & coupling metrics (LCOM, Ca, Ce, Instability)
 */

// ═══════════════════════════════════════════════════════════
// SECTION 1: NAMING — Before & After
// ═══════════════════════════════════════════════════════════

function namingBefore(): Record<string, number> {
  const d: Record<string, number> = { a: 3, b: 7 };
  const r = d["a"] + d["b"];
  let fl = 0;
  for (const k in d) {
    fl += d[k];
  }
  const tm = r / Object.keys(d).length;
  return { s: r, av: tm };
}

function namingAfter(): Record<string, number> {
  const orderLineItems: Record<string, number> = { apples: 3, bananas: 7 };
  const totalItemCount = orderLineItems["apples"] + orderLineItems["bananas"];
  let orderSubtotal = 0;
  for (const itemName in orderLineItems) {
    orderSubtotal += orderLineItems[itemName];
  }
  const averageItemQuantity = totalItemCount / Object.keys(orderLineItems).length;
  return {
    total_item_count: totalItemCount,
    average_item_quantity: averageItemQuantity,
  };
}

// ═══════════════════════════════════════════════════════════
// SECTION 2: COHESION — Low → High Refactoring
// ═══════════════════════════════════════════════════════════

interface OrderData {
  customerName: string;
  item: string;
  quantity: number;
}

class LowCohesionOrderProcessor {
  private orders: OrderData[] = [];
  private smtpHost = "smtp.example.com";
  private smtpPort = 587;
  private senderEmail = "noreply@example.com";

  addOrder(customerName: string, item: string, quantity: number): void {
    this.orders.push({ customerName, item, quantity });
  }

  calculateTotal(): number {
    const priceMap: Record<string, number> = {
      widget: 9.99,
      gadget: 19.99,
      doohickey: 4.99,
    };
    let total = 0;
    for (const order of this.orders) {
      total += (priceMap[order.item] ?? 0) * order.quantity;
    }
    return total;
  }

  sendConfirmationEmail(recipient: string, subject: string, body: string): void {
    console.log(`Connecting to ${this.smtpHost}:${this.smtpPort}`);
    console.log(`From: ${this.senderEmail} To: ${recipient}`);
    console.log(`Subject: ${subject}`);
    console.log(body);
    console.log("Email sent.");
  }

  generateInvoiceNumber(): string {
    return `INV-${this.orders.length.toString().padStart(4, "0")}`;
  }

  exportOrdersToCsv(): string {
    const lines = ["customer,item,quantity"];
    for (const order of this.orders) {
      lines.push(`${order.customerName},${order.item},${order.quantity}`);
    }
    return lines.join("\n");
  }
}

class Order {
  constructor(
    public customerName: string,
    public item: string,
    public quantity: number,
  ) {}
}

class OrderRepository {
  private orders: Order[] = [];

  add(order: Order): void {
    this.orders.push(order);
  }

  listAll(): Order[] {
    return [...this.orders];
  }

  count(): number {
    return this.orders.length;
  }
}

class PricingService {
  private priceMap: Record<string, number> = {
    widget: 9.99,
    gadget: 19.99,
    doohickey: 4.99,
  };

  calculateTotal(orders: Order[]): number {
    let total = 0;
    for (const order of orders) {
      total += (this.priceMap[order.item] ?? 0) * order.quantity;
    }
    return total;
  }
}

interface NotificationSender {
  send(recipient: string, subject: string, body: string): void;
}

class EmailService implements NotificationSender {
  constructor(
    private smtpHost: string,
    private smtpPort: number,
    private sender: string,
  ) {}

  send(recipient: string, subject: string, body: string): void {
    console.log(`Connecting to ${this.smtpHost}:${this.smtpPort}`);
    console.log(`From: ${this.sender} To: ${recipient}`);
    console.log(`Subject: ${subject}`);
    console.log(body);
    console.log("Email sent.");
  }
}

class InvoiceService {
  constructor(private orderRepo: OrderRepository) {}

  generateInvoiceNumber(): string {
    return `INV-${this.orderRepo.count().toString().padStart(4, "0")}`;
  }
}

class CsvExportService {
  exportOrders(orders: Order[]): string {
    const lines = ["customer,item,quantity"];
    for (const order of orders) {
      lines.push(`${order.customerName},${order.item},${order.quantity}`);
    }
    return lines.join("\n");
  }
}

// ═══════════════════════════════════════════════════════════
// SECTION 3: COUPLING — Tight → Loose via Dependency Injection
// ═══════════════════════════════════════════════════════════

const globalConfig: { taxRate: number; discount: number } = {
  taxRate: 0.08,
  discount: 0.05,
};

class TightlyCoupledOrderService {
  processOrder(customerName: string, item: string, quantity: number): Record<string, number> {
    const repo = new OrderRepository();
    repo.add(new Order(customerName, item, quantity));
    const pricing = new PricingService();
    const subtotal = pricing.calculateTotal(repo.listAll());
    const tax = subtotal * globalConfig.taxRate;
    const discount = subtotal * globalConfig.discount;
    const total = subtotal + tax - discount;
    globalConfig.discount = 0;
    return { subtotal, tax, discount, total };
  }
}

interface DiscountStrategy {
  calculate(subtotal: number): number;
}

class NoDiscount implements DiscountStrategy {
  calculate(_subtotal: number): number {
    return 0;
  }
}

class PercentageDiscount implements DiscountStrategy {
  constructor(private percent: number) {}

  calculate(subtotal: number): number {
    return subtotal * (this.percent / 100);
  }
}

class FlatDiscount implements DiscountStrategy {
  constructor(private amount: number) {}

  calculate(subtotal: number): number {
    return Math.min(this.amount, subtotal);
  }
}

class LooselyCoupledOrderService {
  constructor(
    private orderRepo: OrderRepository,
    private pricingService: PricingService,
    private discountStrategy: DiscountStrategy,
    private notification: NotificationSender,
    private taxRate: number = 0.08,
  ) {}

  processOrder(customerName: string, item: string, quantity: number): Record<string, number> {
    this.orderRepo.add(new Order(customerName, item, quantity));
    const allOrders = this.orderRepo.listAll();
    const subtotal = this.pricingService.calculateTotal(allOrders);
    const tax = subtotal * this.taxRate;
    const discount = this.discountStrategy.calculate(subtotal);
    const total = subtotal + tax - discount;
    this.notification.send(
      customerName,
      "Order Confirmation",
      `Your order total is $${total.toFixed(2)}`,
    );
    return { subtotal, tax, discount, total };
  }
}

// ═══════════════════════════════════════════════════════════
// SECTION 4: METRICS — LCOM, Afferent/Efferent Coupling, Instability
// ═══════════════════════════════════════════════════════════

interface MethodInfo {
  name: string;
  accessedFields: Set<string>;
}

interface ClassInfo {
  name: string;
  instanceFields: Set<string>;
  methods: MethodInfo[];
}

function calculateLcom(classInfo: ClassInfo): number {
  const methods = classInfo.methods;
  if (methods.length < 2) return 0;

  let m = 0;
  let q = 0;
  for (let i = 0; i < methods.length; i++) {
    for (let j = i + 1; j < methods.length; j++) {
      const shared = new Set(
        [...methods[i].accessedFields].filter((f) => methods[j].accessedFields.has(f)),
      );
      if (shared.size > 0) {
        q++;
      } else {
        m++;
      }
    }
  }
  return Math.max(0, m - q);
}

interface ModuleInfo {
  name: string;
  efferent: Set<string>;
  afferent: Set<string>;
}

function calculateInstability(module: ModuleInfo): number {
  const ca = module.afferent.size;
  const ce = module.efferent.size;
  const total = ca + ce;
  if (total === 0) return 0;
  return ce / total;
}

function demonstrateMetrics(): void {
  const lowCohesionClass: ClassInfo = {
    name: "LowCohesionOrderProcessor",
    instanceFields: new Set(["orders", "smtpHost", "smtpPort", "senderEmail"]),
    methods: [
      { name: "addOrder", accessedFields: new Set(["orders"]) },
      { name: "calculateTotal", accessedFields: new Set(["orders"]) },
      {
        name: "sendConfirmationEmail",
        accessedFields: new Set(["smtpHost", "smtpPort", "senderEmail"]),
      },
      { name: "generateInvoiceNumber", accessedFields: new Set(["orders"]) },
      { name: "exportOrdersToCsv", accessedFields: new Set(["orders"]) },
    ],
  };

  const lowCohesionClassAlt: ClassInfo = {
    name: "GodObjectUserService",
    instanceFields: new Set(["users", "dbConnection", "smtpHost", "smtpPort", "logger"]),
    methods: [
      { name: "createUser", accessedFields: new Set(["users", "dbConnection"]) },
      { name: "deleteUser", accessedFields: new Set(["users", "dbConnection"]) },
      { name: "sendWelcomeEmail", accessedFields: new Set(["smtpHost", "smtpPort"]) },
      { name: "sendPasswordReset", accessedFields: new Set(["smtpHost", "smtpPort"]) },
      { name: "logAction", accessedFields: new Set(["logger"]) },
    ],
  };

  const highCohesionClass: ClassInfo = {
    name: "PricingService",
    instanceFields: new Set(),
    methods: [{ name: "calculateTotal", accessedFields: new Set() }],
  };

  console.log("=== LCOM Metric ===");
  for (const cls of [lowCohesionClass, lowCohesionClassAlt, highCohesionClass]) {
    const lcom = calculateLcom(cls);
    console.log(`  ${cls.name}: LCOM = ${lcom}`);
    if (lcom > 0) {
      console.log(`    → Low cohesion. Consider splitting into ${lcom + 1} focused classes.`);
    } else {
      console.log("    → High cohesion. All methods share purpose.");
    }
  }

  console.log();
  console.log("=== Instability Metric ===");

  const utilsModule: ModuleInfo = {
    name: "utils",
    efferent: new Set(["os", "json", "datetime", "logging", "re", "collections"]),
    afferent: new Set(["orderService", "emailService", "reportService", "authService"]),
  };

  const authModule: ModuleInfo = {
    name: "authInterface",
    efferent: new Set(),
    afferent: new Set(["userService", "apiGateway", "adminPanel", "oauthHandler"]),
  };

  for (const mod of [utilsModule, authModule]) {
    const instability = calculateInstability(mod);
    console.log(`  ${mod.name}:`);
    console.log(`    Afferent (Ca) = ${mod.afferent.size}`);
    console.log(`    Efferent (Ce) = ${mod.efferent.size}`);
    console.log(`    Instability   = ${instability.toFixed(2)}`);
    if (instability < 0.3) {
      console.log("    → Stable. Many depend on this. Changes must be careful.");
    } else if (instability > 0.7) {
      console.log("    → Unstable. Easy to change. Few depend on this.");
    } else {
      console.log("    → Balanced. Moderate incoming/outgoing dependencies.");
    }
  }
}

// ═══════════════════════════════════════════════════════════
// SECTION 5: END-TO-END DEMO
// ═══════════════════════════════════════════════════════════

function main(): void {
  console.log("=".repeat(70));
  console.log("LESSON 16.02: Naming, Cohesion, Coupling — TypeScript Demo");
  console.log("=".repeat(70));

  // --- Naming demo ---
  console.log("\n--- Naming: Before → After ---");
  const before = namingBefore();
  const after = namingAfter();
  console.log(`  Before: ${JSON.stringify(before)}`);
  console.log(`  After:  ${JSON.stringify(after)}`);
  console.log(`  Before keys: unsearchable (${Object.keys(before).join(", ")})`);
  console.log(`  After keys:  intent-revealing (${Object.keys(after).join(", ")})`);

  // --- Cohesion demo ---
  console.log("\n--- Cohesion: Low → High ---");
  const lowCohesion = new LowCohesionOrderProcessor();
  lowCohesion.addOrder("Alice", "widget", 2);
  lowCohesion.addOrder("Bob", "gadget", 1);
  console.log(`  Low cohesion total: $${lowCohesion.calculateTotal().toFixed(2)}`);
  console.log(`  Invoice #: ${lowCohesion.generateInvoiceNumber()}`);
  console.log(`  CSV:\n${lowCohesion.exportOrdersToCsv()}`);

  const repo = new OrderRepository();
  repo.add(new Order("Alice", "widget", 2));
  repo.add(new Order("Bob", "gadget", 1));
  const pricing = new PricingService();
  const invoice = new InvoiceService(repo);
  const csvExport = new CsvExportService();
  console.log(`\n  High cohesion total: $${pricing.calculateTotal(repo.listAll()).toFixed(2)}`);
  console.log(`  Invoice #: ${invoice.generateInvoiceNumber()}`);
  console.log(`  CSV:\n${csvExport.exportOrders(repo.listAll())}`);

  // --- Coupling demo ---
  console.log("\n--- Coupling: Tight → Loose ---");
  const tight = new TightlyCoupledOrderService();
  const tightResult = tight.processOrder("Carol", "widget", 3);
  console.log(`  Tight coupling result: ${JSON.stringify(tightResult)}`);
  console.log("  ⚠ Global config mutated: discount set to 0!");

  const looseRepo = new OrderRepository();
  const pricingSvc = new PricingService();
  const discount = new PercentageDiscount(5);
  const notif = new EmailService("smtp.example.com", 587, "noreply@example.com");
  const loose = new LooselyCoupledOrderService(looseRepo, pricingSvc, discount, notif);
  const looseResult = loose.processOrder("Carol", "widget", 3);
  console.log(`\n  Loose coupling result: ${JSON.stringify(looseResult)}`);
  console.log("  ✅ No global state. Dependencies injected via constructor.");

  const flatDiscount = new FlatDiscount(2.0);
  const looseFlat = new LooselyCoupledOrderService(
    new OrderRepository(),
    new PricingService(),
    flatDiscount,
    notif,
  );
  const flatResult = looseFlat.processOrder("Dave", "gadget", 1);
  console.log(`  Flat discount result: ${JSON.stringify(flatResult)}`);
  console.log("  ✅ Swapped discount strategy without changing OrderService.");

  // --- Metrics demo ---
  console.log();
  demonstrateMetrics();

  console.log("\n" + "=".repeat(70));
  console.log("Key takeaway: Name with intent, refactor toward cohesion,");
  console.log("              decouple through injection and abstraction.");
  console.log("=".repeat(70));
}

main();