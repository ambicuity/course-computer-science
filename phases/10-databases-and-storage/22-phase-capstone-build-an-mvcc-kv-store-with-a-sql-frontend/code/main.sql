-- Phase Capstone — MVCC KV Store with SQL Frontend
-- SQL demonstration of all features
-- Run with: cargo run

-- 1. Create tables
CREATE TABLE users (id INT, name TEXT, email TEXT);
CREATE TABLE orders (id INT, user_id INT, amount INT);

-- 2. Insert data
INSERT INTO users VALUES (1, 'Alice', 'alice@example.com');
INSERT INTO users VALUES (2, 'Bob', 'bob@example.com');
INSERT INTO users VALUES (3, 'Charlie', 'charlie@example.com');
INSERT INTO orders VALUES (1, 1, 100);
INSERT INTO orders VALUES (2, 1, 200);
INSERT INTO orders VALUES (3, 2, 150);

-- 3. Query data
SELECT * FROM users;
SELECT name, email FROM users;
SELECT * FROM users WHERE id = 1;
SELECT * FROM orders WHERE amount > 100;

-- 4. MVCC demonstration: snapshot isolation
-- Transaction 1: Alice checks her orders
BEGIN;
SELECT * FROM orders WHERE user_id = 1;
-- (Alice sees 2 orders: id=1 amount=100, id=2 amount=200)

-- Transaction 2 (concurrent, simulated by switching sessions): 
-- Bob also reads orders — sees the same snapshot as T1
-- (Bob would run: BEGIN; SELECT * FROM orders WHERE user_id = 1;)

-- Alice adds a new order
INSERT INTO orders VALUES (4, 1, 300);

-- Alice sees 3 orders now
SELECT * FROM orders WHERE user_id = 1;

-- Bob's concurrent transaction still sees 2 orders (snapshot isolation)
-- (Bob: SELECT * FROM orders WHERE user_id = 1; → 2 rows)

-- Alice commits
COMMIT;

-- After commit, new transactions see all 3 orders
SELECT * FROM orders WHERE user_id = 1;

-- 5. UPDATE
UPDATE users SET email = 'alice@newdomain.com' WHERE id = 1;
SELECT * FROM users WHERE id = 1;

-- 6. DELETE
DELETE FROM orders WHERE id = 3;
SELECT * FROM orders;

-- 7. Write-write conflict: first-committer-wins
-- T1: BEGIN; UPDATE users SET name = 'Alice Smith' WHERE id = 1;
-- T2: BEGIN; UPDATE users SET name = 'Alice Jones' WHERE id = 1;
-- T1 commits first → T2 must abort on commit (first-committer-wins)

-- 8. Rollback
BEGIN;
INSERT INTO users VALUES (4, 'Dave', 'dave@example.com');
SELECT * FROM users WHERE id = 4;
ROLLBACK;
-- Dave should no longer be visible
SELECT * FROM users WHERE id = 4;

-- 9. Final state
SELECT * FROM users;
SELECT * FROM orders;
