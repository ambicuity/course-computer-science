// CQRS and Event Sourcing — TypeScript Implementation
// Phase 16 — Software Engineering & Architecture

// ─── Domain Events ───────────────────────────────────────────────────────────
// Events are immutable facts representing things that have happened in the domain.
// They are the source of truth — state is always derived by replaying events.

type AccountEvent =
  | { type: "AccountOpened"; accountId: string; initialDeposit: number; owner: string }
  | { type: "MoneyDeposited"; accountId: string; amount: number; description: string }
  | { type: "MoneyWithdrawn"; accountId: string; amount: number; description: string }
  | { type: "OverdraftLimitSet"; accountId: string; limit: number }
  | { type: "AccountClosed"; accountId: string };

// ─── Commands ────────────────────────────────────────────────────────────────
// Commands express intent to change state. They can be rejected if business rules
// are violated. Commands are processed by the command handler (write side).

type AccountCommand =
  | { type: "OpenAccount"; accountId: string; initialDeposit: number; owner: string }
  | { type: "Deposit"; accountId: string; amount: number; description: string }
  | { type: "Withdraw"; accountId: string; amount: number; description: string }
  | { type: "SetOverdraftLimit"; accountId: string; limit: number }
  | { type: "CloseAccount"; accountId: string };

// ─── Domain Errors ───────────────────────────────────────────────────────────

class DomainError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "DomainError";
  }
}

class AccountAlreadyOpenError extends DomainError {
  constructor(id: string) { super(`Account ${id} is already open`); }
}

class AccountNotFoundError extends DomainError {
  constructor(id: string) { super(`Account ${id} not found`); }
}

class AccountClosedError extends DomainError {
  constructor(id: string) { super(`Account ${id} is closed`); }
}

class InsufficientFundsError extends DomainError {
  constructor(balance: number, requested: number, overdraftLimit: number) {
    super(`Insufficient funds: balance=${balance}, requested=${requested}, overdraft_limit=${overdraftLimit}`);
  }
}

class InvalidAmountError extends DomainError {
  constructor(amount: number) { super(`Invalid amount: ${amount}`); }
}

class CannotDepositToClosedError extends DomainError {
  constructor(id: string) { super(`Cannot deposit to closed account ${id}`); }
}

class CannotWithdrawFromClosedError extends DomainError {
  constructor(id: string) { super(`Cannot withdraw from closed account ${id}`); }
}

// ─── Aggregate State ─────────────────────────────────────────────────────────
// The aggregate rebuilds its state by applying events in order.
// This is the core of event sourcing: state = events.reduce(apply, initialState).

interface AccountState {
  accountId: string;
  owner: string;
  balance: number;
  overdraftLimit: number;
  isOpen: boolean;
}

const initialState = (accountId: string): AccountState => ({
  accountId,
  owner: "",
  balance: 0,
  overdraftLimit: 0,
  isOpen: false,
});

function applyEvent(state: AccountState, event: AccountEvent): AccountState {
  switch (event.type) {
    case "AccountOpened":
      return {
        ...state,
        accountId: event.accountId,
        owner: event.owner,
        balance: event.initialDeposit,
        isOpen: true,
      };
    case "MoneyDeposited":
      return { ...state, balance: state.balance + event.amount };
    case "MoneyWithdrawn":
      return { ...state, balance: state.balance - event.amount };
    case "OverdraftLimitSet":
      return { ...state, overdraftLimit: event.limit };
    case "AccountClosed":
      return { ...state, isOpen: false };
  }
}

function rebuildState(accountId: string, events: AccountEvent[]): AccountState {
  return events.reduce(applyEvent, initialState(accountId));
}

// ─── Event Store ─────────────────────────────────────────────────────────────
// The event store is append-only. Events are never modified or deleted.
// In production: EventStoreDB, Kafka topics, or an append-only database table.

class EventStore {
  private streams: Map<string, AccountEvent[]> = new Map();

  append(accountId: string, events: AccountEvent[]): void {
    const stream = this.streams.get(accountId) ?? [];
    this.streams.set(accountId, [...stream, ...events]);
  }

  load(accountId: string): AccountEvent[] {
    return this.streams.get(accountId) ?? [];
  }

  loadFromVersion(accountId: string, fromVersion: number): AccountEvent[] {
    const stream = this.streams.get(accountId) ?? [];
    return stream.slice(fromVersion);
  }

  version(accountId: string): number {
    return this.streams.get(accountId)?.length ?? 0;
  }
}

// ─── Command Handler (Write Side) ───────────────────────────────────────────
// The command handler validates business rules and emits events.
// It never returns data — only events or errors. This is CQRS's write side.

class CommandHandler {
  constructor(private store: EventStore) {}

  handle(command: AccountCommand): AccountEvent[] {
    switch (command.type) {
      case "OpenAccount":
        return this.openAccount(command);
      case "Deposit":
        return this.deposit(command);
      case "Withdraw":
        return this.withdraw(command);
      case "SetOverdraftLimit":
        return this.setOverdraftLimit(command);
      case "CloseAccount":
        return this.closeAccount(command);
    }
  }

  private openAccount(cmd: AccountCommand & { type: "OpenAccount" }): AccountEvent[] {
    if (cmd.initialDeposit < 0) {
      throw new InvalidAmountError(cmd.initialDeposit);
    }
    const existing = this.store.load(cmd.accountId);
    if (existing.length > 0) {
      throw new AccountAlreadyOpenError(cmd.accountId);
    }
    return [{
      type: "AccountOpened",
      accountId: cmd.accountId,
      initialDeposit: cmd.initialDeposit,
      owner: cmd.owner,
    }];
  }

  private deposit(cmd: AccountCommand & { type: "Deposit" }): AccountEvent[] {
    if (cmd.amount <= 0) {
      throw new InvalidAmountError(cmd.amount);
    }
    const events = this.store.load(cmd.accountId);
    if (events.length === 0) {
      throw new AccountNotFoundError(cmd.accountId);
    }
    const state = rebuildState(cmd.accountId, events);
    if (!state.isOpen) {
      throw new CannotDepositToClosedError(cmd.accountId);
    }
    return [{
      type: "MoneyDeposited",
      accountId: cmd.accountId,
      amount: cmd.amount,
      description: cmd.description,
    }];
  }

  private withdraw(cmd: AccountCommand & { type: "Withdraw" }): AccountEvent[] {
    if (cmd.amount <= 0) {
      throw new InvalidAmountError(cmd.amount);
    }
    const events = this.store.load(cmd.accountId);
    if (events.length === 0) {
      throw new AccountNotFoundError(cmd.accountId);
    }
    const state = rebuildState(cmd.accountId, events);
    if (!state.isOpen) {
      throw new CannotWithdrawFromClosedError(cmd.accountId);
    }
    const available = state.balance + state.overdraftLimit;
    if (available < cmd.amount) {
      throw new InsufficientFundsError(state.balance, cmd.amount, state.overdraftLimit);
    }
    return [{
      type: "MoneyWithdrawn",
      accountId: cmd.accountId,
      amount: cmd.amount,
      description: cmd.description,
    }];
  }

  private setOverdraftLimit(cmd: AccountCommand & { type: "SetOverdraftLimit" }): AccountEvent[] {
    if (cmd.limit < 0) {
      throw new InvalidAmountError(cmd.limit);
    }
    const events = this.store.load(cmd.accountId);
    if (events.length === 0) {
      throw new AccountNotFoundError(cmd.accountId);
    }
    const state = rebuildState(cmd.accountId, events);
    if (!state.isOpen) {
      throw new AccountClosedError(cmd.accountId);
    }
    return [{
      type: "OverdraftLimitSet",
      accountId: cmd.accountId,
      limit: cmd.limit,
    }];
  }

  private closeAccount(cmd: AccountCommand & { type: "CloseAccount" }): AccountEvent[] {
    const events = this.store.load(cmd.accountId);
    if (events.length === 0) {
      throw new AccountNotFoundError(cmd.accountId);
    }
    const state = rebuildState(cmd.accountId, events);
    if (!state.isOpen) {
      throw new AccountClosedError(cmd.accountId);
    }
    return [{
      type: "AccountClosed",
      accountId: cmd.accountId,
    }];
  }
}

// ─── Projections (Read Model) ─────────────────────────────────────────────────
// Projections consume events and build read-optimized views. This is the
// query side of CQRS. Multiple projections can consume the same event stream.

interface BalanceView {
  accountId: string;
  balance: number;
  overdraftLimit: number;
  available: number;
  isOpen: boolean;
}

class BalanceProjection {
  static project(accountId: string, events: AccountEvent[]): BalanceView {
    const state = rebuildState(accountId, events);
    return {
      accountId: state.accountId,
      balance: state.balance,
      overdraftLimit: state.overdraftLimit,
      available: state.balance + state.overdraftLimit,
      isOpen: state.isOpen,
    };
  }
}

interface TransactionEntry {
  eventType: string;
  amount: number;
  description: string;
}

class HistoryProjection {
  static project(events: AccountEvent[]): TransactionEntry[] {
    return events.map((event): TransactionEntry => {
      switch (event.type) {
        case "AccountOpened":
          return {
            eventType: "OPENED",
            amount: event.initialDeposit,
            description: `Account opened by ${event.owner}`,
          };
        case "MoneyDeposited":
          return {
            eventType: "DEPOSIT",
            amount: event.amount,
            description: event.description,
          };
        case "MoneyWithdrawn":
          return {
            eventType: "WITHDRAWAL",
            amount: -event.amount,
            description: event.description,
          };
        case "OverdraftLimitSet":
          return {
            eventType: "OVERDRAFT_SET",
            amount: event.limit,
            description: `Overdraft limit set to ${event.limit}`,
          };
        case "AccountClosed":
          return {
            eventType: "CLOSED",
            amount: 0,
            description: "Account closed",
          };
      }
    });
  }
}

// ─── Snapshots ───────────────────────────────────────────────────────────────
// Snapshots save aggregate state at a point in time to avoid replaying
// all events on every command. This is essential for aggregates with many events.

const SNAPSHOT_INTERVAL = 3;

interface Snapshot {
  state: AccountState;
  version: number;
}

class SnapshotStore {
  private snapshots: Map<string, Snapshot> = new Map();

  save(accountId: string, state: AccountState, version: number): void {
    this.snapshots.set(accountId, { state, version });
  }

  load(accountId: string): Snapshot | undefined {
    return this.snapshots.get(accountId);
  }
}

function loadWithSnapshot(
  store: EventStore,
  snapshotStore: SnapshotStore,
  accountId: string,
): AccountState {
  const snapshot = snapshotStore.load(accountId);
  if (snapshot) {
    const remaining = store.loadFromVersion(accountId, snapshot.version);
    return remaining.reduce(applyEvent, snapshot.state);
  }
  const events = store.load(accountId);
  return rebuildState(accountId, events);
}

function maybeSaveSnapshot(
  snapshotStore: SnapshotStore,
  store: EventStore,
  accountId: string,
): void {
  const version = store.version(accountId);
  if (version > 0 && version % SNAPSHOT_INTERVAL === 0) {
    const events = store.load(accountId);
    const state = rebuildState(accountId, events);
    snapshotStore.save(accountId, state, version);
  }
}

// ─── Query Service (Read Side) ────────────────────────────────────────────────
// The query service reads from projections only. It never touches the event
// store directly for queries — that's CQRS. Commands go through the command
// handler (write side); queries go through projections (read side).

class QueryService {
  constructor(private store: EventStore) {}

  getBalance(accountId: string): BalanceView | null {
    const events = this.store.load(accountId);
    if (events.length === 0) return null;
    return BalanceProjection.project(accountId, events);
  }

  getHistory(accountId: string): TransactionEntry[] | null {
    const events = this.store.load(accountId);
    if (events.length === 0) return null;
    return HistoryProjection.project(events);
  }
}

// ─── Demo ─────────────────────────────────────────────────────────────────────

function main(): void {
  const store = new EventStore();
  const snapshots = new SnapshotStore();
  const handler = new CommandHandler(store);
  const accountId = "acc-001";

  console.log("=== CQRS + Event Sourcing: Bank Account Demo ===\n");

  // Open account
  const openEvents = handler.handle({
    type: "OpenAccount", accountId, initialDeposit: 100, owner: "Alice",
  });
  store.append(accountId, openEvents);
  maybeSaveSnapshot(snapshots, store, accountId);

  // Deposit
  const depositEvents = handler.handle({
    type: "Deposit", accountId, amount: 50, description: "Salary",
  });
  store.append(accountId, depositEvents);
  maybeSaveSnapshot(snapshots, store, accountId);

  // Set overdraft limit
  const overdraftEvents = handler.handle({
    type: "SetOverdraftLimit", accountId, limit: 200,
  });
  store.append(accountId, overdraftEvents);
  maybeSaveSnapshot(snapshots, store, accountId);

  // Withdraw within balance
  const withdrawEvents = handler.handle({
    type: "Withdraw", accountId, amount: 80, description: "Groceries",
  });
  store.append(accountId, withdrawEvents);
  maybeSaveSnapshot(snapshots, store, accountId);

  // Withdraw into overdraft
  const overdraftWithdrawEvents = handler.handle({
    type: "Withdraw", accountId, amount: 200, description: "Emergency repair",
  });
  store.append(accountId, overdraftWithdrawEvents);
  maybeSaveSnapshot(snapshots, store, accountId);

  // Try to exceed overdraft — should fail
  try {
    handler.handle({
      type: "Withdraw", accountId, amount: 100, description: "Should fail",
    });
  } catch (err) {
    console.log(`Withdrawal exceeding overdraft: ${(err as DomainError).message}`);
  }

  // Query: current balance via projection (READ SIDE)
  const queryService = new QueryService(store);
  const balanceView = queryService.getBalance(accountId)!;
  console.log("\n--- Balance View (Read Model) ---");
  console.log(`Account: ${balanceView.accountId}`);
  console.log(`Balance: $${balanceView.balance.toFixed(2)}`);
  console.log(`Overdraft Limit: $${balanceView.overdraftLimit.toFixed(2)}`);
  console.log(`Available: $${balanceView.available.toFixed(2)}`);
  console.log(`Is Open: ${balanceView.isOpen}`);

  // Query: transaction history via projection (READ SIDE)
  const history = queryService.getHistory(accountId)!;
  console.log("\n--- Transaction History (Read Model) ---");
  for (const entry of history) {
    console.log(`${entry.eventType.padEnd(15)} ${entry.amount.toFixed(2).padStart(8)}  ${entry.description}`);
  }

  // Verify snapshot replay matches full replay
  const fullState = rebuildState(accountId, store.load(accountId));
  const snapshotState = loadWithSnapshot(store, snapshots, accountId);
  console.log("\n--- Snapshot Verification ---");
  console.log(`Full replay balance: $${fullState.balance.toFixed(2)}`);
  console.log(`Snapshot replay balance: $${snapshotState.balance.toFixed(2)}`);
  console.assert(Math.abs(fullState.balance - snapshotState.balance) < Number.EPSILON,
    "Snapshot replay must match full replay");
  console.log("Snapshot replay matches full replay: OK");

  // Show the event store
  console.log("\n--- Event Store (Immutable Log) ---");
  const allEvents = store.load(accountId);
  allEvents.forEach((event, i) => {
    console.log(`v${i + 1}: ${JSON.stringify(event)}`);
  });

  // Close account
  const closeEvents = handler.handle({ type: "CloseAccount", accountId });
  store.append(accountId, closeEvents);

  // Try deposit on closed account — should fail
  try {
    handler.handle({
      type: "Deposit", accountId, amount: 10, description: "Should fail",
    });
  } catch (err) {
    console.log(`\nDeposit to closed account: ${(err as DomainError).message}`);
  }

  console.log("\n=== All business rules enforced. CQRS + ES demo complete. ===");
}

main();