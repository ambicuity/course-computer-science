# Isolation Level Scheduler

A configurable transaction scheduler that enforces any SQL isolation level — from
Read Uncommitted through Serializable — and demonstrates which anomalies occur
at each level.

## Files

- `isolation_scheduler.py` — standalone `Scheduler` class with:
  - Versioned key-value database (`Database`)
  - Per-transaction snapshots (frozen at `BEGIN` for RR/Snapshot/Serializable)
  - Statement-level reads for Read Committed
  - Serializable Snapshot Isolation (SSI) via conflict graph cycle detection
- SQL demo scripts in `code/main.sql` for PostgreSQL

## Usage

```python
from isolation_scheduler import Scheduler, IsolationLevel

sched = Scheduler(IsolationLevel.SERIALIZABLE)
sched.begin_transaction(1)
sched.begin_transaction(2)
sched.read(1, "X")
sched.read(2, "X")
sched.write(1, "X", 10)
sched.write(2, "X", 20)
sched.commit(1)   # succeeds
sched.commit(2)   # aborted — SSI detects conflict
```

## Where This Reappears

- **Phase 14 (Graphics & Visualization)**: concurrent rendering pipelines need
  consistent snapshots of scene state.
- **Phase 15 (Distributed Systems)**: distributed transactions and isolation
  levels in Spanner, CockroachDB, and FoundationDB.
- **Phase 17 (Testing & Verification)**: model-checking transaction schedules
  against isolation guarantees.
