use std::collections::HashMap;

// ─── Domain Events ───────────────────────────────────────────────────────────
// Events are immutable facts. Once something has happened, it cannot be undone.
// The event store records these facts in order, and we rebuild state by replaying.

#[derive(Debug, Clone, PartialEq)]
enum AccountEvent {
    AccountOpened { account_id: String, initial_deposit: f64, owner: String },
    MoneyDeposited { account_id: String, amount: f64, description: String },
    MoneyWithdrawn { account_id: String, amount: f64, description: String },
    OverdraftLimitSet { account_id: String, limit: f64 },
    AccountClosed { account_id: String },
}

// ─── Commands ────────────────────────────────────────────────────────────────
// Commands express intent. They can be rejected if business rules are violated.
// The command handler validates against current state before emitting events.

#[derive(Debug, Clone)]
enum AccountCommand {
    OpenAccount { account_id: String, initial_deposit: f64, owner: String },
    Deposit { account_id: String, amount: f64, description: String },
    Withdraw { account_id: String, amount: f64, description: String },
    SetOverdraftLimit { account_id: String, limit: f64 },
    CloseAccount { account_id: String },
}

// ─── Domain Errors ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum DomainError {
    AccountAlreadyOpen(String),
    AccountNotFound(String),
    AccountClosed(String),
    InsufficientFunds { balance: f64, requested: f64, overdraft_limit: f64 },
    InvalidAmount(f64),
    CannotDepositToClosedAccount(String),
    CannotWithdrawFromClosedAccount(String),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::AccountAlreadyOpen(id) => write!(f, "Account {} is already open", id),
            DomainError::AccountNotFound(id) => write!(f, "Account {} not found", id),
            DomainError::AccountClosed(id) => write!(f, "Account {} is closed", id),
            DomainError::InsufficientFunds { balance, requested, overdraft_limit } => {
                write!(f, "Insufficient funds: balance={}, requested={}, overdraft_limit={}", balance, requested, overdraft_limit)
            }
            DomainError::InvalidAmount(amt) => write!(f, "Invalid amount: {}", amt),
            DomainError::CannotDepositToClosedAccount(id) => write!(f, "Cannot deposit to closed account {}", id),
            DomainError::CannotWithdrawFromClosedAccount(id) => write!(f, "Cannot withdraw from closed account {}", id),
        }
    }
}

impl std::error::Error for DomainError {}

// ─── Aggregate State ─────────────────────────────────────────────────────────
// The aggregate rebuilds its state by applying events in order.
// This is the core of event sourcing: state = fold(apply, events).

#[derive(Debug, Clone)]
struct AccountState {
    account_id: String,
    owner: String,
    balance: f64,
    overdraft_limit: f64,
    is_open: bool,
}

impl AccountState {
    fn default_for(account_id: &str) -> Self {
        AccountState {
            account_id: account_id.to_string(),
            owner: String::new(),
            balance: 0.0,
            overdraft_limit: 0.0,
            is_open: false,
        }
    }

    fn apply(&mut self, event: &AccountEvent) {
        match event {
            AccountEvent::AccountOpened { account_id, initial_deposit, owner } => {
                self.account_id = account_id.clone();
                self.owner = owner.clone();
                self.balance = *initial_deposit;
                self.is_open = true;
            }
            AccountEvent::MoneyDeposited { amount, .. } => {
                self.balance += amount;
            }
            AccountEvent::MoneyWithdrawn { amount, .. } => {
                self.balance -= amount;
            }
            AccountEvent::OverdraftLimitSet { limit, .. } => {
                self.overdraft_limit = *limit;
            }
            AccountEvent::AccountClosed { .. } => {
                self.is_open = false;
            }
        }
    }

    fn from_events(account_id: &str, events: &[AccountEvent]) -> Self {
        let mut state = Self::default_for(account_id);
        for event in events {
            state.apply(event);
        }
        state
    }
}

// ─── Event Store ─────────────────────────────────────────────────────────────
// The event store is append-only. Events are never modified or deleted.
// In production, this would be EventStoreDB, Kafka, or a database with an
// append-only table.

#[derive(Debug, Clone)]
struct EventStore {
    streams: HashMap<String, Vec<AccountEvent>>,
}

impl EventStore {
    fn new() -> Self {
        EventStore {
            streams: HashMap::new(),
        }
    }

    fn append(&mut self, account_id: &str, events: Vec<AccountEvent>) {
        let stream = self.streams.entry(account_id.to_string()).or_insert_with(Vec::new);
        for event in events {
            stream.push(event);
        }
    }

    fn load(&self, account_id: &str) -> Vec<AccountEvent> {
        self.streams
            .get(account_id)
            .cloned()
            .unwrap_or_default()
    }

    fn load_from_version(&self, account_id: &str, from_version: usize) -> Vec<AccountEvent> {
        self.streams
            .get(account_id)
            .map(|stream| stream[from_version..].to_vec())
            .unwrap_or_default()
    }

    fn version(&self, account_id: &str) -> usize {
        self.streams
            .get(account_id)
            .map(|stream| stream.len())
            .unwrap_or(0)
    }
}

// ─── Command Handler ─────────────────────────────────────────────────────────
// The command handler is the write side of CQRS. It:
// 1. Loads the aggregate by replaying events from the store
// 2. Validates the command against current state
// 3. Emits new events if the command is valid
// It never returns data — only events or errors.

fn handle_command(
    store: &mut EventStore,
    command: &AccountCommand,
) -> Result<Vec<AccountEvent>, DomainError> {
    match command {
        AccountCommand::OpenAccount { account_id, initial_deposit, owner } => {
            if *initial_deposit < 0.0 {
                return Err(DomainError::InvalidAmount(*initial_deposit));
            }
            let existing = store.load(account_id);
            if !existing.is_empty() {
                return Err(DomainError::AccountAlreadyOpen(account_id.clone()));
            }
            Ok(vec![AccountEvent::AccountOpened {
                account_id: account_id.clone(),
                initial_deposit: *initial_deposit,
                owner: owner.clone(),
            }])
        }

        AccountCommand::Deposit { account_id, amount, description } => {
            if *amount <= 0.0 {
                return Err(DomainError::InvalidAmount(*amount));
            }
            let events = store.load(account_id);
            if events.is_empty() {
                return Err(DomainError::AccountNotFound(account_id.clone()));
            }
            let state = AccountState::from_events(account_id, &events);
            if !state.is_open {
                return Err(DomainError::CannotDepositToClosedAccount(account_id.clone()));
            }
            Ok(vec![AccountEvent::MoneyDeposited {
                account_id: account_id.clone(),
                amount: *amount,
                description: description.clone(),
            }])
        }

        AccountCommand::Withdraw { account_id, amount, description } => {
            if *amount <= 0.0 {
                return Err(DomainError::InvalidAmount(*amount));
            }
            let events = store.load(account_id);
            if events.is_empty() {
                return Err(DomainError::AccountNotFound(account_id.clone()));
            }
            let state = AccountState::from_events(account_id, &events);
            if !state.is_open {
                return Err(DomainError::CannotWithdrawFromClosedAccount(account_id.clone()));
            }
            let available = state.balance + state.overdraft_limit;
            if available < *amount {
                return Err(DomainError::InsufficientFunds {
                    balance: state.balance,
                    requested: *amount,
                    overdraft_limit: state.overdraft_limit,
                });
            }
            Ok(vec![AccountEvent::MoneyWithdrawn {
                account_id: account_id.clone(),
                amount: *amount,
                description: description.clone(),
            }])
        }

        AccountCommand::SetOverdraftLimit { account_id, limit } => {
            if *limit < 0.0 {
                return Err(DomainError::InvalidAmount(*limit));
            }
            let events = store.load(account_id);
            if events.is_empty() {
                return Err(DomainError::AccountNotFound(account_id.clone()));
            }
            let state = AccountState::from_events(account_id, &events);
            if !state.is_open {
                return Err(DomainError::AccountClosed(account_id.clone()));
            }
            Ok(vec![AccountEvent::OverdraftLimitSet {
                account_id: account_id.clone(),
                limit: *limit,
            }])
        }

        AccountCommand::CloseAccount { account_id } => {
            let events = store.load(account_id);
            if events.is_empty() {
                return Err(DomainError::AccountNotFound(account_id.clone()));
            }
            let state = AccountState::from_events(account_id, &events);
            if !state.is_open {
                return Err(DomainError::AccountClosed(account_id.clone()));
            }
            Ok(vec![AccountEvent::AccountClosed {
                account_id: account_id.clone(),
            }])
        }
    }
}

// ─── Projections (Read Model) ────────────────────────────────────────────────
// Projections consume events and build read-optimized views. This is the
// query side of CQRS. Multiple projections can consume the same event stream.

#[derive(Debug, Clone)]
struct BalanceView {
    account_id: String,
    balance: f64,
    overdraft_limit: f64,
    available: f64,
    is_open: bool,
}

struct BalanceProjection;

impl BalanceProjection {
    fn project(account_id: &str, events: &[AccountEvent]) -> BalanceView {
        let state = AccountState::from_events(account_id, events);
        BalanceView {
            account_id: state.account_id.clone(),
            balance: state.balance,
            overdraft_limit: state.overdraft_limit,
            available: state.balance + state.overdraft_limit,
            is_open: state.is_open,
        }
    }
}

#[derive(Debug, Clone)]
struct TransactionEntry {
    event_type: String,
    amount: f64,
    description: String,
}

struct HistoryProjection;

impl HistoryProjection {
    fn project(events: &[AccountEvent]) -> Vec<TransactionEntry> {
        events.iter().map(|event| match event {
            AccountEvent::AccountOpened { initial_deposit, owner, .. } => TransactionEntry {
                event_type: "OPENED".to_string(),
                amount: *initial_deposit,
                description: format!("Account opened by {}", owner),
            },
            AccountEvent::MoneyDeposited { amount, description, .. } => TransactionEntry {
                event_type: "DEPOSIT".to_string(),
                amount: *amount,
                description: description.clone(),
            },
            AccountEvent::MoneyWithdrawn { amount, description, .. } => TransactionEntry {
                event_type: "WITHDRAWAL".to_string(),
                amount: -*amount,
                description: description.clone(),
            },
            AccountEvent::OverdraftLimitSet { limit, .. } => TransactionEntry {
                event_type: "OVERDRAFT_SET".to_string(),
                amount: *limit,
                description: format!("Overdraft limit set to {}", limit),
            },
            AccountEvent::AccountClosed { .. } => TransactionEntry {
                event_type: "CLOSED".to_string(),
                amount: 0.0,
                description: "Account closed".to_string(),
            },
        }).collect()
    }
}

// ─── Snapshots ───────────────────────────────────────────────────────────────
// Snapshots save the aggregate state at a point in time to avoid replaying
// all events on every command. Load the snapshot, then replay only events
// that came after.

const SNAPSHOT_INTERVAL: usize = 3;

#[derive(Debug, Clone)]
struct Snapshot {
    state: AccountState,
    version: usize,
}

struct SnapshotStore {
    snapshots: HashMap<String, Snapshot>,
}

impl SnapshotStore {
    fn new() -> Self {
        SnapshotStore { snapshots: HashMap::new() }
    }

    fn save(&mut self, account_id: &str, state: AccountState, version: usize) {
        self.snapshots.insert(account_id.to_string(), Snapshot { state, version });
    }

    fn load(&self, account_id: &str) -> Option<&Snapshot> {
        self.snapshots.get(account_id)
    }
}

fn load_with_snapshot(
    store: &EventStore,
    snapshot_store: &SnapshotStore,
    account_id: &str,
) -> AccountState {
    if let Some(snapshot) = snapshot_store.load(account_id) {
        let remaining = store.load_from_version(account_id, snapshot.version);
        let mut state = snapshot.state.clone();
        for event in &remaining {
            state.apply(event);
        }
        state
    } else {
        let events = store.load(account_id);
        AccountState::from_events(account_id, &events)
    }
}

fn maybe_save_snapshot(
    snapshot_store: &mut SnapshotStore,
    store: &EventStore,
    account_id: &str,
) {
    let version = store.version(account_id);
    if version > 0 && version % SNAPSHOT_INTERVAL == 0 {
        let events = store.load(account_id);
        let state = AccountState::from_events(account_id, &events);
        snapshot_store.save(account_id, state, version);
    }
}

// ─── Query Side ───────────────────────────────────────────────────────────────
// The query model reads from projections only. It never touches the event store
// directly for queries — that's the whole point of CQRS. Commands go through
// the command handler; queries go through projections.

struct QueryService<'a> {
    store: &'a EventStore,
}

impl<'a> QueryService<'a> {
    fn new(store: &'a EventStore) -> Self {
        QueryService { store }
    }

    fn get_balance(&self, account_id: &str) -> Option<BalanceView> {
        let events = self.store.load(account_id);
        if events.is_empty() {
            return None;
        }
        Some(BalanceProjection::project(account_id, &events))
    }

    fn get_history(&self, account_id: &str) -> Option<Vec<TransactionEntry>> {
        let events = self.store.load(account_id);
        if events.is_empty() {
            return None;
        }
        Some(HistoryProjection::project(&events))
    }
}

// ─── Demo ─────────────────────────────────────────────────────────────────────

fn main() {
    let mut store = EventStore::new();
    let mut snapshots = SnapshotStore::new();
    let account_id = "acc-001";

    println!("=== CQRS + Event Sourcing: Bank Account Demo ===\n");

    // Open account
    let events = handle_command(&mut store, &AccountCommand::OpenAccount {
        account_id: account_id.to_string(),
        initial_deposit: 100.0,
        owner: "Alice".to_string(),
    }).unwrap();
    store.append(account_id, events);
    maybe_save_snapshot(&mut snapshots, &store, account_id);

    // Deposit
    let events = handle_command(&mut store, &AccountCommand::Deposit {
        account_id: account_id.to_string(),
        amount: 50.0,
        description: "Salary".to_string(),
    }).unwrap();
    store.append(account_id, events);
    maybe_save_snapshot(&mut snapshots, &store, account_id);

    // Set overdraft limit
    let events = handle_command(&mut store, &AccountCommand::SetOverdraftLimit {
        account_id: account_id.to_string(),
        limit: 200.0,
    }).unwrap();
    store.append(account_id, events);
    maybe_save_snapshot(&mut snapshots, &store, account_id);

    // Withdraw within balance
    let events = handle_command(&mut store, &AccountCommand::Withdraw {
        account_id: account_id.to_string(),
        amount: 80.0,
        description: "Groceries".to_string(),
    }).unwrap();
    store.append(account_id, events);
    maybe_save_snapshot(&mut snapshots, &store, account_id);

    // Withdraw into overdraft
    let events = handle_command(&mut store, &AccountCommand::Withdraw {
        account_id: account_id.to_string(),
        amount: 200.0,
        description: "Emergency repair".to_string(),
    }).unwrap();
    store.append(account_id, events);
    maybe_save_snapshot(&mut snapshots, &store, account_id);

    // Try to exceed overdraft — should fail
    let result = handle_command(&mut store, &AccountCommand::Withdraw {
        account_id: account_id.to_string(),
        amount: 100.0,
        description: "Should fail".to_string(),
    });
    println!("Withdrawal exceeding overdraft: {:?}", result.unwrap_err());

    // Query: current balance via projection
    let query = QueryService::new(&store);
    let balance_view = query.get_balance(account_id).unwrap();
    println!("\n--- Balance View (Read Model) ---");
    println!("Account: {}", balance_view.account_id);
    println!("Balance: ${:.2}", balance_view.balance);
    println!("Overdraft Limit: ${:.2}", balance_view.overdraft_limit);
    println!("Available: ${:.2}", balance_view.available);
    println!("Is Open: {}", balance_view.is_open);

    // Query: transaction history via projection
    let history = query.get_history(account_id).unwrap();
    println!("\n--- Transaction History (Read Model) ---");
    for entry in &history {
        println!("{:<15} {:>8.2}  {}", entry.event_type, entry.amount, entry.description);
    }

    // Verify snapshot replay matches full replay
    let full_state = {
        let events = store.load(account_id);
        AccountState::from_events(account_id, &events)
    };
    let snapshot_state = load_with_snapshot(&store, &snapshots, account_id);
    println!("\n--- Snapshot Verification ---");
    println!("Full replay balance: ${:.2}", full_state.balance);
    println!("Snapshot replay balance: ${:.2}", snapshot_state.balance);
    assert!((full_state.balance - snapshot_state.balance).abs() < f64::EPSILON);
    println!("Snapshot replay matches full replay: OK");

    // Show the event store
    println!("\n--- Event Store (Immutable Log) ---");
    for (i, event) in store.load(account_id).iter().enumerate() {
        println!("v{}: {:?}", i + 1, event);
    }

    // Close account
    let events = handle_command(&mut store, &AccountCommand::CloseAccount {
        account_id: account_id.to_string(),
    }).unwrap();
    store.append(account_id, events);

    // Try deposit on closed account — should fail
    let result = handle_command(&mut store, &AccountCommand::Deposit {
        account_id: account_id.to_string(),
        amount: 10.0,
        description: "Should fail".to_string(),
    });
    println!("\nDeposit to closed account: {}", result.unwrap_err());

    println!("\n=== All business rules enforced. CQRS + ES demo complete. ===");
}