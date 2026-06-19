/**
 * SOLID Principles — Demystified
 * Phase 16 — Software Engineering & Architecture
 *
 * Runnable demos for all five SOLID principles in TypeScript.
 * Run: npx ts-node main.ts   (or tsc && node main.js)
 */

// ═══════════════════════════════════════════════════════════════════════════
// S — Single Responsibility Principle
// ═══════════════════════════════════════════════════════════════════════════

console.log("=".repeat(60));
console.log("PRINCIPLE 1: Single Responsibility (SRP)");
console.log("=".repeat(60));

console.log("\n--- VIOLATION: God class does everything ---\n");

class EmployeeViolation {
  constructor(public name: string, public salary: number) {}

  calculatePay(): number {
    return this.salary * 1.1;
  }

  saveToDatabase(dbConn: { execute: (sql: string) => void }): void {
    dbConn.execute(`INSERT INTO employees VALUES ('${this.name}', ${this.salary})`);
  }

  generateReport(): string {
    return `Employee: ${this.name}, Salary: ${this.salary}`;
  }

  sendPayslipEmail(smtp: { send: (to: string, body: string) => void }): void {
    smtp.send(`${this.name}@co.com`, this.generateReport());
  }
}

console.log("EmployeeViolation has 4 reasons to change:");
console.log("  - Payroll policy  -> calculatePay");
console.log("  - DBA schema      -> saveToDatabase");
console.log("  - Design team     -> generateReport");
console.log("  - IT / email team -> sendPayslipEmail");

console.log("\n--- FIX: One responsibility per class ---\n");

class Employee {
  constructor(public name: string, public salary: number) {}
}

class PayCalculator {
  calculate(employee: Employee): number {
    return employee.salary * 1.1;
  }
}

class EmployeeRepository {
  save(dbConn: { execute: (sql: string, params: unknown[]) => void }, employee: Employee): void {
    dbConn.execute("INSERT INTO employees VALUES (?, ?)", [employee.name, employee.salary]);
  }
}

class ReportFormatter {
  format(employee: Employee): string {
    return `Employee: ${employee.name}, Salary: ${employee.salary}`;
  }
}

const emp = new Employee("Ada", 100_000);
const calc = new PayCalculator();
const fmt = new ReportFormatter();

console.log(`  Pay:        ${calc.calculate(emp)}`);
console.log(`  Report:     ${fmt.format(emp)}`);
console.log(`  Repository: saves (would call dbConn.execute)`);
console.log("  Each class has ONE reason to change.");

// ═══════════════════════════════════════════════════════════════════════════
// O — Open/Closed Principle
// ═══════════════════════════════════════════════════════════════════════════

console.log("\n" + "=".repeat(60));
console.log("PRINCIPLE 2: Open/Closed (OCP)");
console.log("=".repeat(60));

console.log("\n--- VIOLATION: Growing switch/case ---\n");

function calculateDiscountViolation(customerType: string, total: number): number {
  if (customerType === "regular") return total * 0.95;
  if (customerType === "premium") return total * 0.9;
  if (customerType === "vip") return total * 0.8;
  return total;
}

console.log(`  calculateDiscountViolation('regular', 100) = ${calculateDiscountViolation("regular", 100)}`);
console.log("  Adding 'student' requires editing this function — OCP violation!");

console.log("\n--- FIX: Strategy pattern — extend by adding classes ---\n");

interface DiscountStrategy {
  apply(total: number): number;
}

class RegularDiscount implements DiscountStrategy {
  apply(total: number): number { return total * 0.95; }
}

class PremiumDiscount implements DiscountStrategy {
  apply(total: number): number { return total * 0.90; }
}

class VIPDiscount implements DiscountStrategy {
  apply(total: number): number { return total * 0.80; }
}

class StudentDiscount implements DiscountStrategy {
  apply(total: number): number { return total * 0.85; }
}

function calculateDiscount(strategy: DiscountStrategy, total: number): number {
  return strategy.apply(total);
}

const strategies: Record<string, DiscountStrategy> = {
  regular: new RegularDiscount(),
  premium: new PremiumDiscount(),
  vip: new VIPDiscount(),
  student: new StudentDiscount(),
};

for (const [name, strategy] of Object.entries(strategies)) {
  const result = calculateDiscount(strategy, 100);
  console.log(`  ${name.padEnd(10)} -> ${result.toFixed(2)}`);
}
console.log("  Adding a new discount = adding a new class. No edits to existing code.");

// ═══════════════════════════════════════════════════════════════════════════
// L — Liskov Substitution Principle
// ═══════════════════════════════════════════════════════════════════════════

console.log("\n" + "=".repeat(60));
console.log("PRINCIPLE 3: Liskov Substitution (LSP)");
console.log("=".repeat(60));

console.log("\n--- VIOLATION: Square extends Rectangle ---\n");

class RectangleViolation {
  constructor(protected _width: number, protected _height: number) {}

  setWidth(w: number): void { this._width = w; }
  setHeight(h: number): void { this._height = h; }
  area(): number { return this._width * this._height; }
}

class SquareViolation extends RectangleViolation {
  constructor(side: number) { super(side, side); }

  setWidth(w: number): void { this._width = w; this._height = w; }
  setHeight(h: number): void { this._width = h; this._height = h; }
}

function printAreaViolation(shape: RectangleViolation): void {
  shape.setWidth(5);
  shape.setHeight(3);
  console.log(`  Expected area: 15, Got: ${shape.area()}`);
}

console.log("  Using Rectangle:");
printAreaViolation(new RectangleViolation(2, 3));
console.log("  Using Square (LSP violation!):");
printAreaViolation(new SquareViolation(2));
console.log("  Square breaks Rectangle's contract — width/height are not independent.");

console.log("\n--- FIX: Shared base without false promises ---\n");

interface Shape {
  area(): number;
}

class Rectangle2 implements Shape {
  constructor(public width: number, public height: number) {}
  area(): number { return this.width * this.height; }
}

class Square2 implements Shape {
  constructor(public side: number) {}
  area(): number { return this.side * this.side; }
}

function printArea(shape: Shape): void {
  console.log(`  ${shape.constructor.name} area = ${shape.area()}`);
}

printArea(new Rectangle2(5, 3));
printArea(new Square2(4));
console.log("  Both satisfy Shape.area(). No false promises about shared setters.");

// ═══════════════════════════════════════════════════════════════════════════
// I — Interface Segregation Principle
// ═══════════════════════════════════════════════════════════════════════════

console.log("\n" + "=".repeat(60));
console.log("PRINCIPLE 4: Interface Segregation (ISP)");
console.log("=".repeat(60));

console.log("\n--- VIOLATION: Fat interface forces stubs ---\n");

interface MachineViolation {
  printDoc(document: string): void;
  scanDoc(document: string): string;
  faxDoc(document: string): void;
}

class OldPrinterViolation implements MachineViolation {
  printDoc(document: string): void { console.log(`  Printing: ${document}`); }
  scanDoc(_document: string): string { throw new Error("Cannot scan"); }
  faxDoc(_document: string): void { throw new Error("Cannot fax"); }
}

const oldPrinter = new OldPrinterViolation();
oldPrinter.printDoc("report.pdf");
console.log("  OldPrinterViolation.scanDoc() throws Error — ISP violation!");

console.log("\n--- FIX: Role-based interfaces ---\n");

interface Printer {
  printDoc(document: string): void;
}

interface Scanner {
  scanDoc(document: string): string;
}

interface FaxMachine {
  faxDoc(document: string): void;
}

class SimplePrinter implements Printer {
  printDoc(document: string): void {
    console.log(`  Printing: ${document}`);
  }
}

class MultiFunctionDevice implements Printer, Scanner, FaxMachine {
  printDoc(document: string): void { console.log(`  Printing: ${document}`); }
  scanDoc(document: string): string { return `Scanned: ${document}`; }
  faxDoc(document: string): void { console.log(`  Faxing: ${document}`); }
}

const simpleP = new SimplePrinter();
const mfd = new MultiFunctionDevice();
simpleP.printDoc("letter.pdf");
mfd.printDoc("contract.pdf");
console.log(`  ${mfd.scanDoc("invoice.pdf")}`);
mfd.faxDoc("memo.pdf");
console.log("  SimplePrinter implements only Printer. No stubs, no throw new Errors.");

// ═══════════════════════════════════════════════════════════════════════════
// D — Dependency Inversion Principle
// ═══════════════════════════════════════════════════════════════════════════

console.log("\n" + "=".repeat(60));
console.log("PRINCIPLE 5: Dependency Inversion (DIP)");
console.log("=".repeat(60));

console.log("\n--- VIOLATION: UserService hard-wires MySQLDatabase ---\n");

class MySQLDatabaseViolation {
  query(sql: string): Record<string, unknown>[] {
    console.log(`  MySQL executing: ${sql}`);
    return [{ id: 1, name: "Ada" }];
  }
}

class UserServiceViolation {
  private db = new MySQLDatabaseViolation();

  getUser(userId: number): Record<string, unknown> {
    return this.db.query(`SELECT * FROM users WHERE id = ${userId}`)[0];
  }
}

const svcV = new UserServiceViolation();
const userV = svcV.getUser(1);
console.log(`  User: ${JSON.stringify(userV)}`);
console.log("  UserServiceViolation depends on MySQL — can't test without MySQL, can't swap DB.");

console.log("\n--- FIX: Depend on abstraction ---\n");

interface Database {
  query(sql: string): Record<string, unknown>[];
}

class MySQLDatabase implements Database {
  query(sql: string): Record<string, unknown>[] {
    console.log(`  MySQL executing: ${sql}`);
    return [{ id: 1, name: "Ada" }];
  }
}

class PostgresDatabase implements Database {
  query(sql: string): Record<string, unknown>[] {
    console.log(`  Postgres executing: ${sql}`);
    return [{ id: 1, name: "Ada" }];
  }
}

class FakeDatabase implements Database {
  query(_sql: string): Record<string, unknown>[] {
    return [{ id: 99, name: "Test User" }];
  }
}

class UserService {
  constructor(private db: Database) {}

  getUser(userId: number): Record<string, unknown> {
    const rows = this.db.query(`SELECT * FROM users WHERE id = ${userId}`);
    return rows[0] ?? {};
  }
}

console.log("  With MySQLDatabase:");
const svcMySQL = new UserService(new MySQLDatabase());
console.log(`    ${JSON.stringify(svcMySQL.getUser(1))}`);

console.log("  With PostgresDatabase:");
const svcPG = new UserService(new PostgresDatabase());
console.log(`    ${JSON.stringify(svcPG.getUser(1))}`);

console.log("  With FakeDatabase (for tests):");
const svcFake = new UserService(new FakeDatabase());
console.log(`    ${JSON.stringify(svcFake.getUser(1))}`);

console.log("\n  UserService depends on Database abstraction — swap freely.");

// ═══════════════════════════════════════════════════════════════════════════
// Summary
// ═══════════════════════════════════════════════════════════════════════════

console.log("\n" + "=".repeat(60));
console.log("SUMMARY");
console.log("=".repeat(60));
console.log(`
  S — Single Responsibility:  One reason to change per module.
  O — Open/Closed:            Extend by adding, not by editing.
  L — Liskov Substitution:    Subtypes keep the base type's promises.
  I — Interface Segregation:  Many small interfaces > one fat interface.
  D — Dependency Inversion:   Depend on abstractions, not concretions.

  All five are about managing dependencies — the root of sustainable code.
`);