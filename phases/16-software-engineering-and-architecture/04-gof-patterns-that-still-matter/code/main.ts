/**
 * GoF Patterns That Still Matter — TypeScript Implementations
 * Phase 16 — Software Engineering & Architecture
 *
 * Seven patterns, each self-contained and runnable.
 * Run: npx tsx main.ts   (or: ts-node main.ts)
 */

// ── 1. Observer ──────────────────────────────────────────────────────────────

type Listener<T> = (data: T) => void;
type Unsubscribe = () => void;

class EventEmitter {
  private listeners = new Map<string, Set<Listener<unknown>>>();

  on<T>(event: string, fn: Listener<T>): Unsubscribe {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    const set = this.listeners.get(event)!;
    set.add(fn as Listener<unknown>);
    return () => {
      set.delete(fn as Listener<unknown>);
      if (set.size === 0) this.listeners.delete(event);
    };
  }

  once<T>(event: string, fn: Listener<T>): Unsubscribe {
    const wrapper: Listener<T> = (data) => {
      unsub();
      fn(data);
    };
    const unsub = this.on(event, wrapper);
    return unsub;
  }

  emit<T>(event: string, data: T): void {
    this.listeners.get(event)?.forEach((fn) => fn(data));
  }
}

function demoObserver(): void {
  console.log("=== Observer ===");
  const bus = new EventEmitter();
  const unsubA = bus.on<number>("price", (p) =>
    console.log(`  Dashboard A: ${p}`)
  );
  bus.on<number>("price", (p) =>
    console.log(`  Dashboard B: ${p}`)
  );
  console.log("  First emit:");
  bus.emit("price", 142.5);
  unsubA();
  console.log("  After unsubscribing A:");
  bus.emit("price", 143.0);
}

// ── 2. Strategy ──────────────────────────────────────────────────────────────

interface PaymentStrategy {
  pay(amount: number): string;
}

class CreditCardPayment implements PaymentStrategy {
  constructor(private cardNumber: string) {}
  pay(amount: number): string {
    return `  Charged $${amount.toFixed(2)} to card ending ${this.cardNumber.slice(-4)}`;
  }
}

class CryptoPayment implements PaymentStrategy {
  constructor(private wallet: string) {}
  pay(amount: number): string {
    return `  Transferred $${amount.toFixed(2)} from wallet ${this.wallet.slice(0, 8)}`;
  }
}

class PayPalPayment implements PaymentStrategy {
  constructor(private email: string) {}
  pay(amount: number): string {
    return `  Sent $${amount.toFixed(2)} via PayPal to ${this.email}`;
  }
}

class PaymentProcessor {
  private strategy: PaymentStrategy;

  constructor(strategy: PaymentStrategy) {
    this.strategy = strategy;
  }

  setStrategy(strategy: PaymentStrategy): void {
    this.strategy = strategy;
  }

  process(amount: number): string {
    return this.strategy.pay(amount);
  }
}

function demoStrategy(): void {
  console.log("\n=== Strategy ===");
  const processor = new PaymentProcessor(
    new CreditCardPayment("4242424242424242")
  );
  console.log(processor.process(99.99));
  processor.setStrategy(new CryptoPayment("0xABCDEF1234567890"));
  console.log(processor.process(49.99));
  processor.setStrategy(new PayPalPayment("user@example.com"));
  console.log(processor.process(24.99));
}

// ── 3. Command (with Undo) ──────────────────────────────────────────────────

interface Command {
  execute(): void;
  undo(): void;
}

class AddTextCommand implements Command {
  constructor(
    private editor: TextEditor,
    private text: string,
    private position: number
  ) {}

  execute(): void {
    const content = this.editor.content;
    this.editor.content =
      content.slice(0, this.position) + this.text + content.slice(this.position);
  }

  undo(): void {
    const content = this.editor.content;
    const end = this.position + this.text.length;
    this.editor.content = content.slice(0, this.position) + content.slice(end);
  }
}

class DeleteTextCommand implements Command {
  private deleted = "";

  constructor(
    private editor: TextEditor,
    private length: number,
    private position: number
  ) {}

  execute(): void {
    const content = this.editor.content;
    this.deleted = content.slice(this.position, this.position + this.length);
    this.editor.content =
      content.slice(0, this.position) +
      content.slice(this.position + this.length);
  }

  undo(): void {
    const content = this.editor.content;
    this.editor.content =
      content.slice(0, this.position) +
      this.deleted +
      content.slice(this.position);
  }
}

class TextEditor {
  content = "";
  private history: Command[] = [];
  private redoStack: Command[] = [];

  execute(command: Command): void {
    command.execute();
    this.history.push(command);
    this.redoStack = [];
  }

  undo(): boolean {
    const cmd = this.history.pop();
    if (!cmd) return false;
    cmd.undo();
    this.redoStack.push(cmd);
    return true;
  }

  redo(): boolean {
    const cmd = this.redoStack.pop();
    if (!cmd) return false;
    cmd.execute();
    this.history.push(cmd);
    return true;
  }
}

function demoCommand(): void {
  console.log("\n=== Command ===");
  const editor = new TextEditor();
  editor.execute(new AddTextCommand(editor, "Hello", 0));
  console.log(`  After 'Hello': '${editor.content}'`);
  editor.execute(new AddTextCommand(editor, " World", 5));
  console.log(`  After ' World': '${editor.content}'`);
  editor.undo();
  console.log(`  After undo: '${editor.content}'`);
  editor.redo();
  console.log(`  After redo: '${editor.content}'`);
  editor.execute(new DeleteTextCommand(editor, 5, 5));
  console.log(`  After delete: '${editor.content}'`);
  editor.undo();
  console.log(`  After undo delete: '${editor.content}'`);
}

// ── 4. Iterator ──────────────────────────────────────────────────────────────

class TreeNode<T> {
  constructor(
    public value: T,
    public left: TreeNode<T> | null = null,
    public right: TreeNode<T> | null = null
  ) {}
}

function* inOrder<T>(node: TreeNode<T> | null): Generator<T> {
  if (node === null) return;
  yield* inOrder(node.left);
  yield node.value;
  yield* inOrder(node.right);
}

function* preOrder<T>(node: TreeNode<T> | null): Generator<T> {
  if (node === null) return;
  yield node.value;
  yield* preOrder(node.left);
  yield* preOrder(node.right);
}

function demoIterator(): void {
  console.log("\n=== Iterator ===");
  const root = new TreeNode(
    4,
    new TreeNode(2, new TreeNode(1), new TreeNode(3)),
    new TreeNode(6, new TreeNode(5), new TreeNode(7))
  );
  console.log(`  In-order:  [${[...inOrder(root)].join(", ")}]`);
  console.log(`  Pre-order: [${[...preOrder(root)].join(", ")}]`);

  // Using the iterator manually
  const it = inOrder(root);
  let result = it.next();
  const firstThree: number[] = [];
  while (!result.done && firstThree.length < 3) {
    firstThree.push(result.value);
    result = it.next();
  }
  console.log(`  First 3 in-order: [${firstThree.join(", ")}]`);
}

// ── 5. Factory Method ────────────────────────────────────────────────────────

interface Notification {
  send(message: string): string;
}

interface NotificationFactory {
  create(): Notification;
}

class EmailNotification implements Notification {
  send(message: string): string {
    return `  Email sent: ${message}`;
  }
}

class SMSNotification implements Notification {
  send(message: string): string {
    return `  SMS sent: ${message}`;
  }
}

class PushNotification implements Notification {
  send(message: string): string {
    return `  Push notification: ${message}`;
  }
}

class EmailFactory implements NotificationFactory {
  create(): Notification {
    return new EmailNotification();
  }
}

class SMSFactory implements NotificationFactory {
  create(): Notification {
    return new SMSNotification();
  }
}

class PushFactory implements NotificationFactory {
  create(): Notification {
    return new PushNotification();
  }
}

class NotificationService {
  constructor(private factory: NotificationFactory) {}

  notify(message: string): string {
    const notification = this.factory.create();
    return notification.send(message);
  }
}

function demoFactory(): void {
  console.log("\n=== Factory Method ===");
  const factories: NotificationFactory[] = [
    new EmailFactory(),
    new SMSFactory(),
    new PushFactory(),
  ];
  for (const factory of factories) {
    const service = new NotificationService(factory);
    console.log(service.notify("Build succeeded!"));
  }
}

// ── 6. Decorator ─────────────────────────────────────────────────────────────

type AsyncFunction = (...args: unknown[]) => Promise<unknown>;

function logCalls<T extends AsyncFunction>(fn: T): T {
  const wrapper = async (...args: unknown[]) => {
    console.log(`  [LOG] Calling ${fn.name}`);
    const result = await fn(...args);
    console.log(`  [LOG] ${fn.name} returned: ${result}`);
    return result;
  };
  return wrapper as T;
}

function timeCalls<T extends AsyncFunction>(fn: T): T {
  const wrapper = async (...args: unknown[]) => {
    const start = performance.now();
    const result = await fn(...args);
    const elapsed = performance.now() - start;
    console.log(`  [TIME] ${fn.name} took ${elapsed.toFixed(2)}ms`);
    return result;
  };
  return wrapper as T;
}

function cacheCalls<T extends AsyncFunction>(fn: T): T {
  const cache = new Map<string, unknown>();
  const wrapper = async (...args: unknown[]) => {
    const key = JSON.stringify(args);
    if (cache.has(key)) {
      console.log(`  [CACHE] Hit for ${fn.name}(${args.join(", ")})`);
      return cache.get(key);
    }
    console.log(`  [CACHE] Miss for ${fn.name}(${args.join(", ")})`);
    const result = await fn(...args);
    cache.set(key, result);
    return result;
  };
  return wrapper as T;
}

// Build the decorator chain
const expensiveCompute = cacheCalls(
  timeCalls(
    logCalls(async function expensiveCompute(n: number): Promise<number> {
      await new Promise((r) => setTimeout(r, 10));
      return n * n;
    })
  )
);

async function demoDecorator(): Promise<void> {
  console.log("\n=== Decorator ===");
  console.log(`  Result: ${await expensiveCompute(5)}`);
  console.log(`  Result (cached): ${await expensiveCompute(5)}`);
  console.log(`  Result (new arg): ${await expensiveCompute(7)}`);
}

// ── 7. Adapter ───────────────────────────────────────────────────────────────

interface UserRepository {
  getUser(id: string): { id: string; name: string; email: string };
}

interface LegacyUser {
  user_id: string;
  first_name: string;
  last_name: string;
  email_address: string;
}

class LegacyAuthService {
  private db: Record<string, LegacyUser> = {
    "1": { user_id: "1", first_name: "Jane", last_name: "Doe", email_address: "jane@legacy.com" },
    "2": { user_id: "2", first_name: "John", last_name: "Smith", email_address: "john@legacy.com" },
  };

  fetchUser(uid: string): LegacyUser {
    return this.db[uid];
  }
}

class LegacyUserAdapter implements UserRepository {
  constructor(private legacyService: LegacyAuthService) {}

  getUser(id: string): { id: string; name: string; email: string } {
    const legacy = this.legacyService.fetchUser(id);
    return {
      id: legacy.user_id,
      name: `${legacy.first_name} ${legacy.last_name}`,
      email: legacy.email_address,
    };
  }
}

class ModernUserService {
  constructor(private repo: UserRepository) {}

  greet(userId: string): string {
    const user = this.repo.getUser(userId);
    return `  Hello, ${user.name} (${user.email})`;
  }
}

function demoAdapter(): void {
  console.log("\n=== Adapter ===");
  const legacy = new LegacyAuthService();
  const adapter = new LegacyUserAdapter(legacy);
  const service = new ModernUserService(adapter);
  console.log(service.greet("1"));
  console.log(service.greet("2"));
}

// ── Main ─────────────────────────────────────────────────────────────────────

async function main(): Promise<void> {
  demoObserver();
  demoStrategy();
  demoCommand();
  demoIterator();
  demoFactory();
  await demoDecorator();
  demoAdapter();
}

main();