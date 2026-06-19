-- Relational Model & Relational Algebra — SQL equivalents
--
-- Every SQL clause maps to a relational algebra operator:
--   SELECT list  →  π (project)
--   WHERE        →  σ (select)
--   JOIN         →  ⋈ (theta-join or natural join)
--   FROM a, b    →  × (cross product)
--   UNION        →  ∪ (set union, implies δ)
--   UNION ALL    →  ∪ (bag union, no dedup)
--   EXCEPT       →  − (difference)
--   INTERSECT    →  ∩ (intersection)
--   DISTINCT     →  δ (duplicate elimination)
--   GROUP BY     →  γ (aggregation)
--   ORDER BY     →  τ (sort)

CREATE TABLE IF NOT EXISTS users (
    user_id INTEGER PRIMARY KEY,
    name    TEXT NOT NULL,
    age     INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS orders (
    order_id INTEGER PRIMARY KEY,
    user_id  INTEGER NOT NULL REFERENCES users(user_id),
    amount   REAL NOT NULL,
    product  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS departments (
    dept_id   INTEGER PRIMARY KEY,
    dept_name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_dept (
    user_id INTEGER NOT NULL REFERENCES users(user_id),
    dept_id INTEGER NOT NULL REFERENCES departments(dept_id),
    PRIMARY KEY (user_id, dept_id)
);

INSERT INTO users VALUES
    (1, 'Alice',   30),
    (2, 'Bob',     20),
    (3, 'Charlie', 25),
    (4, 'Diana',   22);

INSERT INTO orders VALUES
    (1, 1, 150,   'Laptop'),
    (2, 2, 50,    'Mouse'),
    (3, 1, 75,    'Keyboard'),
    (4, 3, 200,   'Monitor'),
    (5, 4, 80,    'USB Hub');

INSERT INTO departments VALUES
    (1, 'Engineering'),
    (2, 'Sales');

INSERT INTO user_dept VALUES
    (1, 1),
    (2, 2),
    (3, 1),
    (4, 2);


-- π_name(σ_age>21(Users))
--   σ filters rows by predicate; π keeps only named columns.
SELECT DISTINCT name
FROM users
WHERE age > 21;


-- π_name(σ_amount>100 AND users.user_id=orders.user_id(Users × Orders))
--   Cross product (×) followed by σ selection implements a theta-join.
SELECT DISTINCT u.name
FROM users u, orders o                      -- × (cross product)
WHERE u.user_id = o.user_id                  -- σ (join condition)
  AND o.amount > 100;                        -- σ (filter)

-- Equivalent θ-join expressed with explicit JOIN:
SELECT DISTINCT u.name
FROM users u
JOIN orders o ON u.user_id = o.user_id    -- ⋈ (theta-join)
WHERE o.amount > 100;                       -- σ


-- π_dept_name(UserDept ⋈ Departments)
--   Natural join on common attribute dept_id.
SELECT DISTINCT d.dept_name
FROM user_dept ud
NATURAL JOIN departments d;                 -- ⋈ (natural join)


-- π_name(σ_dept_name=Engineering(Users ⋈ UserDept ⋈ Departments))
--   Multi-step: theta-join users→user_dept, then natural join→departments.
SELECT DISTINCT u.name
FROM users u
JOIN user_dept ud ON u.user_id = ud.user_id   -- ⋈ (theta-join)
NATURAL JOIN departments d                    -- ⋈ (natural join on dept_id)
WHERE d.dept_name = 'Engineering';            -- σ


-- γ_{user_id, SUM(amount), COUNT(*)}(Orders)
--   Group by user_id, aggregate sum and count.
SELECT user_id,
       SUM(amount) AS total_spent,
       COUNT(*)    AS order_count
FROM orders
GROUP BY user_id;


-- ρ (rename): SELECT user_id AS uid FROM users
SELECT user_id AS uid, name, age FROM users;


-- τ_{amount}(Orders) — sort
SELECT * FROM orders ORDER BY amount;


-- ∪ (set union): names of users older than 21 UNION younger than 23
--   UNION removes duplicates (set semantics = δ(π ∪ π)).
SELECT name FROM users WHERE age > 21
UNION
SELECT name FROM users WHERE age < 23;


-- ∪ ALL (bag union): same but allows duplicates
SELECT name FROM users WHERE age > 21
UNION ALL
SELECT name FROM users WHERE age < 23;


-- − (difference): users older than 21 EXCEPT users named Alice
SELECT name FROM users WHERE age > 21
EXCEPT
SELECT name FROM users WHERE name = 'Alice';


-- ∩ (intersection): users older than 21 INTERSECT younger than 30
SELECT name FROM users WHERE age > 21
INTERSECT
SELECT name FROM users WHERE age < 30;


-- The bag/set gap:
--   π_age(Users) as a SQL SELECT (bag — retains duplicates)
SELECT age FROM users;
--   π_age(Users) as a set (DISTINCT eliminates duplicates)
SELECT DISTINCT age FROM users;

-- In the relational model, π always eliminates duplicates.
-- In SQL, SELECT without DISTINCT does NOT — this is the fundamental
-- inconsistency between the relational model and SQL.
