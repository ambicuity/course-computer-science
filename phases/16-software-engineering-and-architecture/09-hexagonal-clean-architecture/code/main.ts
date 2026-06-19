// Hexagonal / Clean Architecture
// Phase 16 — Software Engineering & Architecture
//
// Demonstrates hexagonal architecture with a user registration feature:
// domain core (pure business logic), ports (interfaces), adapters (implementations),
// use case (orchestration), and testability by swapping adapters.

// ============================================================================
// DOMAIN CORE — Pure business logic, zero external dependencies
// ============================================================================

class Email {
  readonly value: string;
  private constructor(value: string) {
    this.value = value;
  }

  static create(raw: string): Email {
    if (!raw || typeof raw !== "string") {
      throw new DomainError("Email cannot be empty");
    }
    const trimmed = raw.trim().toLowerCase();
    if (!trimmed.includes("@") || !trimmed.includes(".")) {
      throw new DomainError(`Invalid email format: "${raw}"`);
    }
    return new Email(trimmed);
  }

  equals(other: Email): boolean {
    return this.value === other.value;
  }
}

class Password {
  readonly value: string;
  private constructor(value: string) {
    this.value = value;
  }

  static create(raw: string): Password {
    if (!raw || raw.length < 8) {
      throw new DomainError("Password must be at least 8 characters");
    }
    return new Password(raw);
  }
}

class User {
  readonly id: string;
  readonly email: Email;
  readonly hashedPassword: string;
  readonly registeredAt: Date;

  constructor(id: string, email: Email, hashedPassword: string, registeredAt?: Date) {
    this.id = id;
    this.email = email;
    this.hashedPassword = hashedPassword;
    this.registeredAt = registeredAt ?? new Date();
  }

  changeEmail(newEmail: Email): User {
    return new User(this.id, newEmail, this.hashedPassword, this.registeredAt);
  }
}

class DomainError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "DomainError";
  }
}

type Result<T> =
  | { ok: true; value: T }
  | { ok: false; error: string };

function ok<T>(value: T): Result<T> {
  return { ok: true, value };
}

function err<T>(error: string): Result<T> {
  return { ok: false, error };
}

// ============================================================================
// PORTS — Interfaces defined by the core, implemented by the outside
// ============================================================================

// --- Driving port (primary) — what the application CAN do ---

interface RegisterUserUseCase {
  execute(email: string, password: string): Promise<Result<User>>;
}

// --- Driven ports (secondary) — what the application NEEDS ---

interface UserRepository {
  findByEmail(email: Email): Promise<User | null>;
  save(user: User): Promise<void>;
}

interface NotificationService {
  sendWelcomeEmail(user: User): Promise<void>;
}

interface PasswordHasher {
  hash(plain: string): Promise<string>;
}

// ============================================================================
// USE CASE — Orchestrates domain logic using driven ports
// ============================================================================

class RegisterUserInteractor implements RegisterUserUseCase {
  constructor(
    private userRepo: UserRepository,
    private notifier: NotificationService,
    private hasher: PasswordHasher,
  ) {}

  async execute(email: string, password: string): Promise<Result<User>> {
    let emailVo: Email;
    let passwordVo: Password;

    try {
      emailVo = Email.create(email);
      passwordVo = Password.create(password);
    } catch (e) {
      return err((e as DomainError).message);
    }

    const existing = await this.userRepo.findByEmail(emailVo);
    if (existing) {
      return err("Email already registered");
    }

    const hashedPassword = await this.hasher.hash(passwordVo.value);
    const user = new User(this.generateId(), emailVo, hashedPassword);

    await this.userRepo.save(user);
    await this.notifier.sendWelcomeEmail(user);

    return ok(user);
  }

  private generateId(): string {
    return `usr_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
  }
}

// ============================================================================
// ADAPTERS — Implementations of driven ports (infrastructure layer)
// ============================================================================

// --- In-memory adapters (for testing and development) ---

class InMemoryUserRepository implements UserRepository {
  private usersByEmail = new Map<string, User>();
  private usersById = new Map<string, User>();

  async findByEmail(email: Email): Promise<User | null> {
    return this.usersByEmail.get(email.value) ?? null;
  }

  async save(user: User): Promise<void> {
    this.usersByEmail.set(user.email.value, user);
    this.usersById.set(user.id, user);
  }

  count(): number {
    return this.usersByEmail.size;
  }

  reset(): void {
    this.usersByEmail.clear();
    this.usersById.clear();
  }
}

class ConsoleNotifier implements NotificationService {
  readonly sent: Array<{ to: string; subject: string }> = [];

  async sendWelcomeEmail(user: User): Promise<void> {
    const notification = {
      to: user.email.value,
      subject: "Welcome!",
    };
    this.sent.push(notification);
    console.log(`[NOTIFIER] Welcome email sent to: ${user.email.value}`);
  }
}

class SimplePasswordHasher implements PasswordHasher {
  async hash(plain: string): Promise<string> {
    let hash = 0;
    for (let i = 0; i < plain.length; i++) {
      hash = ((hash << 5) - hash + plain.charCodeAt(i)) | 0;
    }
    return `hash_${Math.abs(hash).toString(16)}`;
  }
}

// --- Simulated production adapters ---

class PostgresUserRepository implements UserRepository {
  private simulatedData = new Map<string, User>();

  async findByEmail(email: Email): Promise<User | null> {
    console.log(`[POSTGRES] SELECT * FROM users WHERE email = '${email.value}'`);
    return this.simulatedData.get(email.value) ?? null;
  }

  async save(user: User): Promise<void> {
    console.log(`[POSTGRES] INSERT INTO users (id, email) VALUES ('${user.id}', '${user.email.value}')`);
    this.simulatedData.set(user.email.value, user);
  }
}

class SendGridNotifier implements NotificationService {
  readonly apiKey: string;

  constructor(apiKey: string) {
    this.apiKey = apiKey;
  }

  async sendWelcomeEmail(user: User): Promise<void> {
    console.log(`[SENDGRID] POST https://api.sendgrid.com/v3/mail/send`);
    console.log(`[SENDGRID]   Authorization: Bearer ${this.apiKey.slice(0, 8)}...`);
    console.log(`[SENDGRID]   To: ${user.email.value} | Template: welcome_v2`);
  }
}

class BcryptPasswordHasher implements PasswordHasher {
  async hash(plain: string): Promise<string> {
    console.log(`[BCRYPT] Hashing password (${plain.length} chars) -> $2b$10$...`);
    return `$2b$10$simulated_bcrypt_hash_${plain.length}`;
  }
}

// ============================================================================
// DRIVING ADAPTER — REST controller (calls the use case through the driving port)
// ============================================================================

class RegisterUserController {
  constructor(private useCase: RegisterUserUseCase) {}

  async handle(request: { body: { email?: string; password?: string } }): Promise<{
    statusCode: number;
    body: Record<string, unknown>;
  }> {
    const { email, password } = request.body;

    if (!email || !password) {
      return {
        statusCode: 400,
        body: { error: "email and password are required" },
      };
    }

    const result = await this.useCase.execute(email, password);

    if (!result.ok) {
      return {
        statusCode: 409,
        body: { error: result.error },
      };
    }

    return {
      statusCode: 201,
      body: {
        id: result.value.id,
        email: result.value.email.value,
        registeredAt: result.value.registeredAt.toISOString(),
      },
    };
  }
}

// ============================================================================
// TESTS — Domain core tests require ZERO infrastructure
// ============================================================================

let testsPassed = 0;
let testsFailed = 0;

function assert(condition: boolean, message: string): void {
  if (!condition) {
    testsFailed++;
    console.error(`  FAIL: ${message}`);
    return;
  }
  testsPassed++;
}

async function runDomainTests(): Promise<void> {
  console.log("\n=== Domain Core Tests (no infrastructure, no mocks) ===\n");

  // Email value object
  const validEmail = Email.create("Alice@Example.COM");
  assert(validEmail.value === "alice@example.com", "Email normalizes to lowercase");

  let threw = false;
  try { Email.create(""); } catch { threw = true; }
  assert(threw, "Empty email throws DomainError");

  threw = false;
  try { Email.create("no-at-sign"); } catch { threw = true; }
  assert(threw, "Email without @ throws DomainError");

  // Password value object
  const validPassword = Password.create("securepass123");
  assert(validPassword.value === "securepass123", "Password stores raw value");

  threw = false;
  try { Password.create("short"); } catch { threw = true; }
  assert(threw, "Password under 8 chars throws DomainError");

  threw = false;
  try { Password.create(""); } catch { threw = true; }
  assert(threw, "Empty password throws DomainError");

  // User entity
  const email = Email.create("test@example.com");
  const user = new User("usr_1", email, "hashed_pwd");
  assert(user.email.value === "test@example.com", "User stores email");
  assert(user.id === "usr_1", "User stores id");

  const changedUser = user.changeEmail(Email.create("new@example.com"));
  assert(changedUser.email.value === "new@example.com", "changeEmail returns new user");
  assert(user.email.value === "test@example.com", "Original user is immutable");
}

async function runUseCaseTestsWithInMemoryAdapters(): Promise<void> {
  console.log("\n=== Use Case Tests (in-memory adapters — no database, no SMTP) ===\n");

  const repo = new InMemoryUserRepository();
  const notifier = new ConsoleNotifier();
  const hasher = new SimplePasswordHasher();
  const useCase = new RegisterUserInteractor(repo, notifier, hasher);

  // Happy path
  const result1 = await useCase.execute("alice@example.com", "securepass123");
  assert(result1.ok, "Registration succeeds for valid input");
  if (result1.ok) {
    assert(result1.value.email.value === "alice@example.com", "Registered email matches");
    assert(result1.value.hashedPassword.startsWith("hash_"), "Password is hashed");
    assert(repo.count() === 1, "Repo has 1 user after registration");
    assert(notifier.sent.length === 1, "Welcome email was sent");
    assert(notifier.sent[0].to === "alice@example.com", "Welcome email sent to correct address");
  }

  // Duplicate email
  const result2 = await useCase.execute("alice@example.com", "differentpass");
  assert(!result2.ok, "Duplicate email registration fails");
  if (!result2.ok) {
    assert(result2.error === "Email already registered", "Error message is descriptive");
  }
  assert(repo.count() === 1, "Repo still has 1 user after duplicate attempt");

  // Invalid email
  const result3 = await useCase.execute("not-an-email", "securepass123");
  assert(!result3.ok, "Invalid email fails");

  // Short password
  const result4 = await useCase.execute("bob@example.com", "short");
  assert(!result4.ok, "Short password fails");

  // Valid second user
  repo.reset();
  notifier.sent.length = 0;
  const result5 = await useCase.execute("bob@example.com", "bob_password_123");
  assert(result5.ok, "Second user registration succeeds");
  if (result5.ok) {
    assert(result5.value.email.value === "bob@example.com", "Second user email matches");
  }
}

async function runUseCaseTestsWithProductionAdapters(): Promise<void> {
  console.log("\n=== Use Case Tests (production adapters — simulated PostgreSQL/SendGrid) ===\n");

  const repo = new PostgresUserRepository();
  const notifier = new SendGridNotifier("sg_live_api_key_xxxx_yyyy_zzzz");
  const hasher = new BcryptPasswordHasher();
  const useCase = new RegisterUserInteractor(repo, notifier, hasher);

  const result = await useCase.execute("charlie@example.com", "charlie_pass_123");
  assert(result.ok, "Registration succeeds with production adapters");
  if (result.ok) {
    assert(result.value.hashedPassword.startsWith("$2b$10$"), "Bcrypt hash format");
  }

  // Demonstrate: same use case, different adapters, zero code changes
  console.log("\n  Note: Same RegisterUserInteractor, zero code changes, different adapters.");
}

async function runControllerTests(): Promise<void> {
  console.log("\n=== Driving Adapter Tests (REST controller) ===\n");

  const repo = new InMemoryUserRepository();
  const notifier = new ConsoleNotifier();
  const hasher = new SimplePasswordHasher();
  const useCase = new RegisterUserInteractor(repo, notifier, hasher);
  const controller = new RegisterUserController(useCase);

  // Missing fields
  const res1 = await controller.handle({ body: {} });
  assert(res1.statusCode === 400, "Missing fields returns 400");

  const res2 = await controller.handle({ body: { email: "test@example.com" } });
  assert(res2.statusCode === 400, "Missing password returns 400");

  // Successful registration
  const res3 = await controller.handle({ body: { email: "dave@example.com", password: "dave_password" } });
  assert(res3.statusCode === 201, "Valid registration returns 201");

  // Duplicate registration
  const res4 = await controller.handle({ body: { email: "dave@example.com", password: "another_pass" } });
  assert(res4.statusCode === 409, "Duplicate email returns 409");
}

// ============================================================================
// MAIN — Wire adapters and demonstrate the full flow
// ============================================================================

async function main(): Promise<void> {
  console.log("╔══════════════════════════════════════════════════════════╗");
  console.log("║   Hexagonal Architecture — User Registration Demo     ║");
  console.log("╚══════════════════════════════════════════════════════════╝");

  await runDomainTests();
  await runUseCaseTestsWithInMemoryAdapters();
  await runUseCaseTestsWithProductionAdapters();
  await runControllerTests();

  console.log("\n═══════════════════════════════════════════════════════════");
  console.log(`  Results: ${testsPassed} passed, ${testsFailed} failed`);
  console.log("═══════════════════════════════════════════════════════════\n");

  // Demonstrate adapter swapping — the entire point of hexagonal architecture
  console.log("=== Adapter Swapping Demo ===\n");
  console.log("In-Memory adapters (for tests):");
  const testRepo = new InMemoryUserRepository();
  const testNotifier = new ConsoleNotifier();
  const testHasher = new SimplePasswordHasher();
  const testUseCase = new RegisterUserInteractor(testRepo, testNotifier, testHasher);
  const testResult = await testUseCase.execute("test@memory.io", "testpassword123");
  console.log(`  Result: ${testResult.ok ? "OK" : "FAIL"} — stored in Map, notified via console\n`);

  console.log("Production adapters (for deployment):");
  const prodRepo = new PostgresUserRepository();
  const prodNotifier = new SendGridNotifier("sg_prod_key_abc123");
  const prodHasher = new BcryptPasswordHasher();
  const prodUseCase = new RegisterUserInteractor(prodRepo, prodNotifier, prodHasher);
  const prodResult = await prodUseCase.execute("prod@company.io", "prodpassword123");
  console.log(`  Result: ${prodResult.ok ? "OK" : "FAIL"} — stored in PostgreSQL, notified via SendGrid\n`);

  console.log(">>> Same RegisterUserInteractor. Zero code changes. Different adapters.\n");

  if (testsFailed > 0) {
    process.exit(1);
  }
}

main().catch((e) => {
  console.error("Fatal error:", e);
  process.exit(1);
});