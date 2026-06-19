-- Isolation Levels — Read Committed → Serializable
-- Phase 10 — Databases & Storage Systems
--
-- Run these queries in separate PostgreSQL sessions to observe isolation behavior.
-- Each section is self-contained. Comments show expected output.

-- ==========================================
-- Setup
-- ==========================================
CREATE TABLE IF NOT EXISTS bank_accounts (
    account_id INTEGER PRIMARY KEY,
    owner TEXT NOT NULL,
    balance NUMERIC(10, 2) NOT NULL
);

DELETE FROM bank_accounts;
INSERT INTO bank_accounts VALUES (1, 'Alice', 1000.00);
INSERT INTO bank_accounts VALUES (2, 'Bob', 500.00);
INSERT INTO bank_accounts VALUES (3, 'Charlie', 250.00);

-- ==========================================
-- Demo 1: Read Uncommitted
-- PostgreSQL treats READ UNCOMMITTED as READ COMMITTED
-- because its MVCC architecture cannot expose uncommitted writes.
-- The SQL standard allows dirty reads at RU, but PG never does them.
-- ==========================================
-- Session 1:
BEGIN ISOLATION LEVEL READ UNCOMMITTED;
    SELECT balance FROM bank_accounts WHERE account_id = 1;
    -- Returns: 1000.00 (committed data only)
    -- Even though PG says READ UNCOMMITTED, you never see dirty data.
COMMIT;

-- ==========================================
-- Demo 2: Read Committed (PostgreSQL default)
-- A non-repeatable read occurs: T1 reads the same row twice
-- and gets different values because T2 committed an update in between.
-- ==========================================
-- Session 1:
BEGIN ISOLATION LEVEL READ COMMITTED;
    SELECT balance FROM bank_accounts WHERE account_id = 1;
    -- Returns: 1000.00

-- Session 2 (run in a second connection):
BEGIN;
    UPDATE bank_accounts SET balance = balance - 100 WHERE account_id = 1;
    -- Don't commit yet.

-- Session 1 (again, while T2 is still active):
    SELECT balance FROM bank_accounts WHERE account_id = 1;
    -- Returns: 1000.00
    -- Read Committed prevents dirty reads: T1 does NOT see T2's uncommitted update.

-- Session 2:
COMMIT;
    -- T2 commits. T1's next read will see the new value.

-- Session 1 (third read):
    SELECT balance FROM bank_accounts WHERE account_id = 1;
    -- Returns: 900.00
    -- NON-REPEATABLE READ: T1 sees different values for the same row!
    -- First read: 1000.00, now: 900.00.
COMMIT;

-- ==========================================
-- Demo 3: Repeatable Read
-- T1 freezes a snapshot at the first query.
-- T2 can commit changes, but T1 always sees the snapshot.
-- ==========================================
-- First, reset the balance:
UPDATE bank_accounts SET balance = 1000.00 WHERE account_id = 1;

-- Session 1:
BEGIN ISOLATION LEVEL REPEATABLE READ;
    SELECT balance FROM bank_accounts WHERE account_id = 1;
    -- Returns: 1000.00

-- Session 2:
BEGIN;
    UPDATE bank_accounts SET balance = 0 WHERE account_id = 1;
    UPDATE bank_accounts SET balance = 2000 WHERE account_id = 2;
COMMIT;
    -- T2 commits: Alice has 0, Bob has 2000 in the committed state.

-- Session 1 (again):
    SELECT balance FROM bank_accounts WHERE account_id = 1;
    -- Returns: 1000.00 (still the snapshot value!)
    -- No non-repeatable read. T1 is isolated from T2's changes.
SELECT balance FROM bank_accounts WHERE account_id = 2;
    -- Returns: 500.00 (snapshot, not T2's 2000)
COMMIT;

-- Session 1 (after commit, in a new transaction):
SELECT balance FROM bank_accounts WHERE account_id = 1;
    -- Returns: 0.00 (now sees committed state)

-- ==========================================
-- Demo 4: Phantom Read
-- In PostgreSQL, REPEATABLE READ also prevents phantoms
-- (MVCC snapshot gives a consistent view of all rows).
-- The SQL standard says RR allows phantoms, but PG is stronger.
-- ==========================================
BEGIN ISOLATION LEVEL REPEATABLE READ;
    SELECT count(*) FROM bank_accounts WHERE balance > 300;
    -- Returns: 2 (Alice=1000, Bob=2000)

-- Session 2:
BEGIN;
    INSERT INTO bank_accounts VALUES (4, 'Diana', 400.00);
COMMIT;
    -- T2 inserts a new row matching the predicate.

-- Session 1 (again):
    SELECT count(*) FROM bank_accounts WHERE balance > 300;
    -- Returns: 2 (still the snapshot — no phantom row)
    -- PostgreSQL's MVCC prevents phantoms even at REPEATABLE READ.
COMMIT;

-- ==========================================
-- Demo 5: Serializable — Write Skew Prevention
-- Two doctors must have at least one on call at all times.
-- Under SERIALIZABLE, PostgreSQL detects the write skew
-- and aborts one transaction to preserve the invariant.
-- ==========================================
CREATE TABLE IF NOT EXISTS on_call (
    doctor_id INTEGER PRIMARY KEY,
    on_call BOOLEAN NOT NULL
);

DELETE FROM on_call;
INSERT INTO on_call VALUES (1, true);  -- Dr. Smith
INSERT INTO on_call VALUES (2, true);  -- Dr. Jones

-- Invariant: at least one doctor must be on call.

-- Session 1 (Dr. Smith wants to go off call):
BEGIN ISOLATION LEVEL SERIALIZABLE;
    SELECT count(*) FROM on_call WHERE on_call = true;
    -- Returns: 2 (both are on call)
    -- Smith checks: Jones is on call, so I can go off.

-- Session 2 (Dr. Jones wants to go off call simultaneously):
BEGIN ISOLATION LEVEL SERIALIZABLE;
    SELECT count(*) FROM on_call WHERE on_call = true;
    -- Also returns: 2 (both are on call)
    -- Jones checks: Smith is on call, so I can go off.

-- Session 2 (Jones proceeds):
    UPDATE on_call SET on_call = false WHERE doctor_id = 2;
COMMIT;
    -- This succeeds.

-- Session 1 (Smith proceeds):
    UPDATE on_call SET on_call = false WHERE doctor_id = 1;
COMMIT;
    -- Under SERIALIZABLE, PostgreSQL detects the read-write conflict:
    -- Both transactions read the same rows (both on call).
    -- Smith's COMMIT will fail with:
    -- ERROR:  could not serialize access due to read/write dependencies among transactions
    -- DETAIL:  Reason code: Canceled on identification as a pivot, during commit attempt.
    -- HINT:  The transaction might succeed if retried.
    --
    -- PostgreSQL's SSI implementation detected that allowing both commits
    -- would break serializability (the invariant "at least one on call"
    -- would be violated in any serial execution).
    --
    -- If Smith's COMMIT succeeds and Jones's fails, the invariant is preserved
    -- (at least Smith is still on call).
    -- If both retry, one will succeed and the other will fail again.

-- After the aborted transaction, verify the invariant:
SELECT count(*) FROM on_call WHERE on_call = true;
    -- Returns: 1 (invariant preserved — at least one doctor is on call)

-- ==========================================
-- Demo 6: Serializable — Preventing Lost Update
-- Two concurrent increment operations on the same counter.
-- Under READ COMMITTED, both read the same base value and one increment is lost.
-- Under SERIALIZABLE, one transaction is aborted.
-- ==========================================
CREATE TABLE IF NOT EXISTS counters (
    id INTEGER PRIMARY KEY,
    value INTEGER NOT NULL
);

DELETE FROM counters;
INSERT INTO counters VALUES (1, 100);

-- Session 1:
BEGIN ISOLATION LEVEL SERIALIZABLE;
    SELECT value FROM counters WHERE id = 1;
    -- Returns: 100

-- Session 2:
BEGIN ISOLATION LEVEL SERIALIZABLE;
    SELECT value FROM counters WHERE id = 1;
    -- Returns: 100 (same snapshot)

-- Session 1:
    UPDATE counters SET value = value + 10 WHERE id = 1;
COMMIT;
    -- Succeeds. Counter is now 110 in the committed state.

-- Session 2:
    UPDATE counters SET value = value + 20 WHERE id = 1;
    -- This UPDATE is based on the stale snapshot value (100).
    -- If SERIALIZABLE: COMMIT will fail with serialization error.
    -- If READ COMMITTED: COMMIT would succeed, overwriting 110 with 120 (lost update).
COMMIT;
    -- If SERIALIZABLE: ERROR — could not serialize access

-- ==========================================
-- Demo 7: Understanding MVCC Tuple Visibility
-- Each row version has xmin (creating transaction) and xmax (deleting/updating transaction).
-- These system columns show how MVCC enforces isolation.
-- ==========================================
-- Create a fresh table to see xmin/xmax behavior:
CREATE TABLE IF NOT EXISTS mvcc_demo (
    id INTEGER PRIMARY KEY,
    val INTEGER
);

DELETE FROM mvcc_demo;
INSERT INTO mvcc_demo VALUES (1, 100);

-- See the tuple's xmin (the transaction ID that created it):
SELECT xmin, xmax, id, val FROM mvcc_demo;
    -- xmin = the transaction ID of the INSERT.
    -- xmax = 0 (no one has updated/deleted this tuple).

-- In a REPEATABLE READ transaction, re-read the same tuple:
BEGIN ISOLATION LEVEL REPEATABLE READ;
    SELECT xmin, xmax, id, val FROM mvcc_demo;
    -- Same xmin/xmax: the transaction snapshot prevents seeing newer versions.
COMMIT;

-- ==========================================
-- Cleanup
-- ==========================================
DROP TABLE IF EXISTS bank_accounts CASCADE;
DROP TABLE IF EXISTS on_call CASCADE;
DROP TABLE IF EXISTS counters CASCADE;
DROP TABLE IF EXISTS mvcc_demo CASCADE;
