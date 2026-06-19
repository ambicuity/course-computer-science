-- Transactions — ACID, Anomalies
-- SQL examples for PostgreSQL 16+
-- Run in two concurrent psql sessions (S1, S2) to observe anomalies.

-- ===================================================
-- 1. BASIC TRANSACTION SYNTAX
-- ===================================================

-- Atomic unit: BEGIN ... COMMIT / ROLLBACK
BEGIN;
UPDATE accounts SET balance = balance - 100 WHERE id = 1;
UPDATE accounts SET balance = balance + 100 WHERE id = 2;
COMMIT;

-- Rollback on error
BEGIN;
UPDATE accounts SET balance = balance - 100 WHERE id = 1;
-- Something goes wrong; undo everything
ROLLBACK;

-- ===================================================
-- 2. SET TRANSACTION ISOLATION LEVEL
-- ===================================================

BEGIN;
SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;
-- ... queries ...
COMMIT;

-- Shorthand for the default:
BEGIN ISOLATION LEVEL READ COMMITTED;

-- PostgreSQL's levels:
--   READ UNCOMMITTED  → behaves like READ COMMITTED (PG never allows dirty reads)
--   READ COMMITTED    → default; each statement sees latest committed data
--   REPEATABLE READ   → single snapshot at transaction start
--   SERIALIZABLE      → true serializability via SSI

-- ===================================================
-- 3. SETUP: WORK TABLE
-- ===================================================

DROP TABLE IF EXISTS accounts;
DROP TABLE IF EXISTS doctors;
DROP TABLE IF EXISTS inventory;

CREATE TABLE accounts (
    id    INT PRIMARY KEY,
    name  TEXT NOT NULL,
    balance INT NOT NULL CHECK (balance >= 0)
);

INSERT INTO accounts VALUES
    (1, 'Alice',   500),
    (2, 'Bob',     500);

CREATE TABLE doctors (
    id      INT PRIMARY KEY,
    name    TEXT NOT NULL,
    on_call BOOLEAN NOT NULL DEFAULT true
);

INSERT INTO doctors VALUES
    (1, 'Dr. Smith',  true),
    (2, 'Dr. Jones',  true);

CREATE TABLE inventory (
    item    TEXT PRIMARY KEY,
    qty     INT NOT NULL CHECK (qty >= 0)
);

INSERT INTO inventory VALUES
    ('widget', 10),
    ('gadget',  5);

-- ===================================================
-- 4. DIRTY READ PREVENTION
-- PostgreSQL NEVER allows dirty reads — even at
-- READ UNCOMMITTED, it behaves like READ COMMITTED.
-- ===================================================

-- S1:                      |  S2:
BEGIN;                       |
UPDATE accounts             |
  SET balance = 0           |
  WHERE id = 1;             |
                             |  BEGIN ISOLATION LEVEL READ UNCOMMITTED;
                             |  SELECT balance FROM accounts WHERE id = 1;
                             |  -- → sees 500 (not 0) — dirty read prevented!
ROLLBACK;                    |  COMMIT;

-- ===================================================
-- 5. NON-REPEATABLE READ AT READ COMMITTED
-- ===================================================

-- S1:                      |  S2:
BEGIN;                       |
SELECT balance              |
  FROM accounts             |
  WHERE id = 1;             |
  -- → sees 500             |
                             |  BEGIN;
                             |  UPDATE accounts
                             |    SET balance = 100
                             |    WHERE id = 1;
                             |  COMMIT;
SELECT balance              |
  FROM accounts             |
  WHERE id = 1;             |
  -- → sees 100 (different!) |
COMMIT;                      |

-- ===================================================
-- 6. NON-REPEATABLE READ PREVENTED AT REPEATABLE READ
-- ===================================================

-- S1:                      |  S2:
BEGIN ISOLATION LEVEL       |
  REPEATABLE READ;           |
SELECT balance              |
  FROM accounts             |
  WHERE id = 1;             |
  -- → sees 500             |
                             |  BEGIN;
                             |  UPDATE accounts
                             |    SET balance = 100
                             |    WHERE id = 1;
                             |  COMMIT;
SELECT balance              |
  FROM accounts             |
  WHERE id = 1;             |
  -- → sees 500 (same!)     |
COMMIT;                      |

-- ===================================================
-- 7. PHANTOM READ AT READ COMMITTED
-- ===================================================

-- S1:                      |  S2:
BEGIN;                       |
SELECT COUNT(*)             |
  FROM inventory;           |
  -- → 2                    |
                             |  BEGIN;
                             |  INSERT INTO inventory
                             |    VALUES ('doohickey', 3);
                             |  COMMIT;
SELECT COUNT(*)             |
  FROM inventory;           |
  -- → 3 (phantom!)         |
COMMIT;                      |

-- ===================================================
-- 8. PHANTOM AT REPEATABLE READ (PostgreSQL prevents it)
-- ===================================================

-- S1:                      |  S2:
BEGIN ISOLATION LEVEL       |
  REPEATABLE READ;           |
SELECT COUNT(*)             |
  FROM inventory;           |
  -- → 2                    |
                             |  BEGIN;
                             |  INSERT INTO inventory
                             |    VALUES ('thingie', 7);
                             |  COMMIT;
SELECT COUNT(*)             |
  FROM inventory;           |
  -- → 2 (still — phantom   |
  --     prevented by MVCC) |
COMMIT;                      |
-- After commit, S1 will   |
-- see thingie in a new txn.|

-- ===================================================
-- 9. LOST UPDATE — both read 500, both increment
-- ===================================================

-- Without explicit locking at READ COMMITTED:
-- S1:                      |  S2:
BEGIN;                       |
SELECT balance              |
  FROM accounts             |
  WHERE id = 1;             |
  -- → 500                  |
                             |  BEGIN;
                             |  SELECT balance
                             |    FROM accounts
                             |    WHERE id = 1;
                             |    -- → 500
UPDATE accounts             |
  SET balance = 550         |
  WHERE id = 1;             |
                             |  UPDATE accounts
                             |    SET balance = 550
                             |    WHERE id = 1;
                             |  -- (PostgreSQL row lock waits until S1 commits)
COMMIT;                      |
                             |  -- Now this UPDATE proceeds, but the
                             |  -- balance was already 550, so it writes
                             |  -- the SAME value — no loss here!
                             |  -- BUT if T2 set balance = 600, it would
                             |  -- overwrite T1's +50 with +100. Lost update!

-- To prevent lost update, use SELECT ... FOR UPDATE:
BEGIN;                       |
SELECT balance              |
  FROM accounts             |
  WHERE id = 1              |
  FOR UPDATE;               |
  -- → 500                  |
                             |  BEGIN;
                             |  SELECT balance
                             |    FROM accounts
                             |    WHERE id = 1
                             |    FOR UPDATE;
                             |    -- → WAITS (locked by S1)
UPDATE accounts             |
  SET balance = 550         |
  WHERE id = 1;             |
COMMIT;                      |
                             |  -- Now sees 550; adds increment to 550
                             |  UPDATE accounts
                             |    SET balance = 570
                             |    WHERE id = 1;
                             |  COMMIT;
                             |  -- Final: 570 — no lost update

-- ===================================================
-- 10. WRITE SKEW AT REPEATABLE READ
-- ===================================================

-- Doctors constraint: at least one must be on call.
-- Each transaction checks "is the other on call?"
-- and then goes off call. Both see the other on call.

-- S1:                      |  S2:
BEGIN ISOLATION LEVEL       |
  REPEATABLE READ;           |
SELECT on_call              |
  FROM doctors              |
  WHERE id = 2;             |
  -- → true                 |
                             |  BEGIN ISOLATION LEVEL
                             |    REPEATABLE READ;
                             |  SELECT on_call
                             |    FROM doctors
                             |    WHERE id = 1;
                             |    -- → true
UPDATE doctors              |
  SET on_call = false       |
  WHERE id = 1;             |
                             |  UPDATE doctors
                             |    SET on_call = false
                             |    WHERE id = 2;
COMMIT;                      |
                             |  COMMIT;
-- Both committed.           |
-- Nobody is on call!        |
-- (Write skew — REPEATABLE  |
--  READ does not prevent it)|

-- ===================================================
-- 11. WRITE SKEW PREVENTED AT SERIALIZABLE
-- ===================================================

-- Same scenario at SERIALIZABLE: one transaction aborts.
-- PostgreSQL detects the serialization conflict via SSI.

-- S1:                      |  S2:
BEGIN ISOLATION LEVEL       |
  SERIALIZABLE;              |
SELECT on_call              |
  FROM doctors              |
  WHERE id = 2;             |
  -- → true                 |
                             |  BEGIN ISOLATION LEVEL
                             |    SERIALIZABLE;
                             |  SELECT on_call
                             |    FROM doctors
                             |    WHERE id = 1;
                             |    -- → true
UPDATE doctors              |
  SET on_call = false       |
  WHERE id = 1;             |
                             |  UPDATE doctors
                             |    SET on_call = false
                             |    WHERE id = 2;
COMMIT;                      |
                             |  COMMIT;
                             |  -- → ERROR: could not serialize access
                             |  -- One of the two transactions aborts.
                             |  -- The invariant is preserved.

-- ===================================================
-- 12. SERIALIZABLE SUMMARY CHECK
-- ===================================================

-- Query to test the doctors invariant after write skew:
SELECT COUNT(*) AS off_call_count
FROM doctors
WHERE on_call = false;
-- If count = 2, the invariant is violated.
-- At SERIALIZABLE, this can never happen.
