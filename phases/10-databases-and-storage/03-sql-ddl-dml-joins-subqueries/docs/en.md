# SQL — DDL, DML, Joins, Subqueries

> SQL — DDL, DML, Joins, Subqueries — the part of CS you can't skip.

**Type:** Learn
**Languages:** SQL, Python
**Prerequisites:** Phase 10 lessons 01–02
**Time:** ~75 minutes

## Learning Objectives

- Write DDL statements to define, alter, and drop database schema objects.
- Use DML (INSERT, UPDATE, DELETE, SELECT) with WHERE, GROUP BY, HAVING, ORDER BY, and LIMIT.
- Distinguish all JOIN types and choose the right one from a Venn-diagram mindset.
- Write scalar, row, table, and correlated subqueries plus CTEs (WITH).
- Apply set operations (UNION, INTERSECT, EXCEPT) and window functions (ROW_NUMBER, RANK, LAG/LEAD).
- Build a chainable SQL query builder in Python that generates valid SQL strings.
- Recognise PostgreSQL-specific SQL dialect features.

## The Problem

This lesson sits in **Phase 10 — Databases & Storage Systems**. Without SQL fluency you cannot query, define, or manipulate relational data. The capstone (an MVCC KV store with a SQL frontend) needs you to parse, generate, and understand SQL. Concretely, *not* knowing SQL means you cannot test your storage engine, write migrations, or debug query plans.

The next sections walk from DDL through window functions, then build a Python query-builder that glues it all together.

## The Concept

### DDL — Data Definition Language

DDL defines the schema — the shape of tables, constraints, and indexes.

**CREATE TABLE** with column types and constraints:

```sql
CREATE TABLE users (
    id          SERIAL       PRIMARY KEY,
    email       VARCHAR(255) NOT NULL UNIQUE,
    username    VARCHAR(100) NOT NULL,
    age         INTEGER      CHECK (age >= 0),
    created_at  TIMESTAMP    DEFAULT NOW()
);

CREATE TABLE orders (
    id         SERIAL     PRIMARY KEY,
    user_id    INTEGER    NOT NULL REFERENCES users(id),
    total      NUMERIC(10,2) CHECK (total > 0),
    status     TEXT       DEFAULT 'pending'
);
```

**Data types** vary by dialect. Common SQL-standard types:

| Category | Types |
|----------|-------|
| Numeric | INTEGER, SMALLINT, BIGINT, NUMERIC(p,s), REAL, DOUBLE |
| Character | CHAR(n), VARCHAR(n), TEXT |
| Temporal | DATE, TIME, TIMESTAMP, INTERVAL |
| Boolean | BOOLEAN |
| Binary | BYTEA, BLOB |
| JSON | JSON, JSONB (PostgreSQL) |
| Array | INTEGER[], TEXT[] (PostgreSQL) |

**Constraints:**

| Constraint | Effect |
|-----------|--------|
| NOT NULL | Column cannot store NULL |
| UNIQUE | Every value in column must differ |
| PRIMARY KEY | NOT NULL + UNIQUE (logical row identifier) |
| FOREIGN KEY | Values must exist in referenced column |
| CHECK | Row-level boolean predicate |
| DEFAULT | Fallback value when none supplied |
| EXCLUDE | (PostgreSQL) row-level exclusion constraint |

**ALTER TABLE** modifies existing schema:

```sql
ALTER TABLE users ADD COLUMN bio TEXT;
ALTER TABLE users ALTER COLUMN bio SET NOT NULL;
ALTER TABLE users DROP COLUMN bio;
ALTER TABLE users ADD CONSTRAINT uq_email UNIQUE (email);
ALTER TABLE orders ADD CONSTRAINT fk_user
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
```

**DROP TABLE** removes schema and data:

```sql
DROP TABLE IF EXISTS orders CASCADE;
```

**CREATE INDEX** speeds up queries:

```sql
CREATE INDEX idx_users_email ON users (email);
CREATE UNIQUE INDEX idx_users_username ON users (username);
CREATE INDEX idx_orders_user_status ON orders (user_id, status);
```

PostgreSQL offers B-tree (default), Hash, GiST, GIN, BRIN, and SP-GiST indexes.

### DML — Data Manipulation Language

**INSERT:**

```sql
INSERT INTO users (email, username, age)
VALUES ('alice@example.com', 'alice', 30);

INSERT INTO users (email, username, age)
VALUES ('bob@example.com', 'bob', 25),
       ('carol@example.com', 'carol', 28);

INSERT INTO users (email, username, age)
SELECT email, username, age FROM imported_users;
```

**UPDATE:**

```sql
UPDATE users SET age = 31 WHERE username = 'alice';
```

**DELETE:**

```sql
DELETE FROM orders WHERE status = 'cancelled';
```

**SELECT** — the workhorse:

```sql
SELECT u.username, COUNT(o.id) AS order_count
FROM users u
LEFT JOIN orders o ON o.user_id = u.id
WHERE u.age >= 21
GROUP BY u.id, u.username
HAVING COUNT(o.id) > 0
ORDER BY order_count DESC
LIMIT 10;
```

Clause evaluation order: FROM → JOIN → WHERE → GROUP BY → HAVING → SELECT → ORDER BY → LIMIT.

### Joins

Joins combine rows from two tables using a predicate. Venn diagrams below show which rows survive.

```
INNER JOIN           LEFT JOIN            RIGHT JOIN
┌─────┬─────┐      ┌─────┬─────┐        ┌─────┬─────┐
│  A  │  B  │      │  A  │  B  │        │  A  │  B  │
│ ████ │ ████ │      │ ████ │ ████ │        │ ████ │ ████ │
│ ████ │ ████ │      │ ████ │     │        │      │ ████ │
└─────┴─────┘      └─────┴─────┘        └─────┴─────┘
  only A∩B            all A              all B

FULL OUTER JOIN      CROSS JOIN
┌─────┬─────┐      ┌─────────────┐
│  A  │  B  │      │ A × B       │
│ ████ │ ████ │      │ every pair  │
│ ████ │ ████ │      │ of rows     │
└─────┴─────┘      └─────────────┘
  union of A,B        Cartesian product
```

```sql
-- INNER: only matching rows
SELECT * FROM users u
JOIN orders o ON o.user_id = u.id;

-- LEFT: all users, NULLs where no order
SELECT * FROM users u
LEFT JOIN orders o ON o.user_id = u.id;

-- RIGHT: all orders, NULLs where no user
SELECT * FROM users u
RIGHT JOIN orders o ON o.user_id = u.id;

-- FULL: all rows from both sides
SELECT * FROM users u
FULL OUTER JOIN orders o ON o.user_id = u.id;

-- CROSS: every user × every order (Cartesian)
SELECT * FROM users
CROSS JOIN orders;

-- SELF: table joined to itself
SELECT a.username AS user, b.username AS referred_by
FROM users a
JOIN users b ON a.referred_by_id = b.id;

-- NATURAL: join on all columns with same name (fragile — avoid in production)
SELECT * FROM users
NATURAL JOIN orders;
```

### Subqueries

A subquery is a SELECT nested inside another query.

**Scalar subquery** — returns one value:

```sql
SELECT username,
       (SELECT AVG(total) FROM orders WHERE user_id = users.id) AS avg_order
FROM users;
```

**Row subquery** — returns one row (used in comparisons):

```sql
SELECT * FROM orders
WHERE (user_id, total) = (SELECT id, 100 FROM users WHERE username = 'alice');
```

**Table subquery** — returns a set (used with IN, EXISTS, ANY, ALL):

```sql
-- IN
SELECT * FROM users
WHERE id IN (SELECT user_id FROM orders WHERE status = 'paid');

-- EXISTS (often faster than IN for large sets)
SELECT * FROM users u
WHERE EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id AND o.status = 'paid');

-- ANY
SELECT * FROM users
WHERE id = ANY (SELECT user_id FROM orders WHERE total > 100);

-- ALL
SELECT * FROM products
WHERE price > ALL (SELECT price FROM products WHERE category = 'clearance');
```

**Correlated subquery** — references outer query (re-executed per row):

```sql
SELECT u.username,
       (SELECT COUNT(*) FROM orders o WHERE o.user_id = u.id) AS order_count
FROM users u;
```

Correlated subqueries can be slow with large tables — often rewritten as JOINs or CTEs.

**CTE — Common Table Expression (WITH):**

```sql
WITH paid_orders AS (
    SELECT user_id, COUNT(*) AS cnt
    FROM orders
    WHERE status = 'paid'
    GROUP BY user_id
)
SELECT u.username, COALESCE(po.cnt, 0) AS paid_count
FROM users u
LEFT JOIN paid_orders po ON po.user_id = u.id
ORDER BY paid_count DESC;
```

CTEs make complex queries readable and support recursion:

```sql
WITH RECURSIVE org_tree AS (
    SELECT id, name, manager_id, 1 AS depth
    FROM employees WHERE manager_id IS NULL
    UNION ALL
    SELECT e.id, e.name, e.manager_id, t.depth + 1
    FROM employees e
    JOIN org_tree t ON e.manager_id = t.id
)
SELECT * FROM org_tree;
```

### Set Operations

Combine results from multiple SELECTs (same column count & compatible types):

```sql
-- UNION (deduplicated)
SELECT city FROM customers
UNION
SELECT city FROM suppliers;

-- UNION ALL (keeps duplicates, faster)
SELECT city FROM customers
UNION ALL
SELECT city FROM suppliers;

-- INTERSECT (rows in both)
SELECT user_id FROM orders WHERE status = 'paid'
INTERSECT
SELECT user_id FROM orders WHERE status = 'shipped';

-- EXCEPT (rows in first but not second)
SELECT id FROM users
EXCEPT
SELECT user_id FROM orders;
```

### Views

A view is a named query stored in the schema.

```sql
CREATE VIEW active_users AS
SELECT id, username, email FROM users WHERE age >= 18 AND deleted_at IS NULL;

-- Query the view like a table
SELECT * FROM active_users;

-- Drop
DROP VIEW IF EXISTS active_users;
```

**Materialized views** (PostgreSQL) store the result set physically:

```sql
CREATE MATERIALIZED VIEW monthly_sales AS
SELECT DATE_TRUNC('month', created_at) AS month,
       SUM(total) AS revenue
FROM orders
GROUP BY month;

-- Must be refreshed
REFRESH MATERIALIZED VIEW monthly_sales;
```

Views encapsulate complexity and enforce security (expose only certain columns).

### Window Functions

Window functions compute across a set of rows related to the current row, *without collapsing* them like GROUP BY.

```sql
SELECT username, order_date, total,
       ROW_NUMBER() OVER (PARTITION BY username ORDER BY order_date DESC) AS rn,
       RANK()       OVER (ORDER BY total DESC) AS rank,
       DENSE_RANK() OVER (ORDER BY total DESC) AS dense_rank,
       LAG(total)   OVER (PARTITION BY username ORDER BY order_date) AS prev_order,
       LEAD(total)  OVER (PARTITION BY username ORDER BY order_date) AS next_order,
       SUM(total)   OVER (PARTITION BY username) AS user_total,
       AVG(total)   OVER (ORDER BY order_date ROWS BETWEEN 6 PRECEDING AND CURRENT ROW) AS rolling_avg
FROM user_orders;
```

| Function | Purpose |
|----------|---------|
| ROW_NUMBER() | Unique incrementing integer per partition |
| RANK() | Same as row_number but ties share rank, next skips |
| DENSE_RANK() | Like RANK but no gaps |
| LAG(col, n) | Value from n rows before current |
| LEAD(col, n) | Value from n rows after current |
| SUM/AVG/COUNT OVER | Running / partitioned aggregate |
| FIRST_VALUE / LAST_VALUE | First/last in the window frame |
| NTILE(n) | Bucket rows into n groups |

Window frame syntax: `ROWS BETWEEN <start> AND <end>` with UNBOUNDED PRECEDING, n PRECEDING, CURRENT ROW, n FOLLOWING, UNBOUNDED FOLLOWING.

### NULL Behavior

NULL is not a value — it is the absence of a value. Key rules:

- `NULL = NULL` → not true (use `IS NULL`)
- `NULL IN (1, 2, 3)` → not false, it's NULL (unknown)
- `NULL + 5` → NULL
- `COUNT(*)` includes NULL rows; `COUNT(col)` excludes them
- Aggregate functions skip NULLs (`SUM`, `AVG`, etc.)

## Build It — SQL Query Builder in Python

Build a chainable SQL query builder that generates valid SQL strings.

### Step 1: Core Builder

Implement `select()`, `from_()`, `where()`, `group_by()`, `having()`, `order_by()`, `limit()` with method chaining.

```python
query = (Query()
         .select("u.username", "COUNT(o.id) AS order_count")
         .from_("users u")
         .join("orders o", "o.user_id = u.id", "LEFT")
         .where("u.age >= 21")
         .group_by("u.username")
         .having("COUNT(o.id) > 0")
         .order_by("order_count DESC")
         .limit(10))
print(query.sql())
# SELECT u.username, COUNT(o.id) AS order_count
# FROM users u
# LEFT JOIN orders o ON o.user_id = u.id
# WHERE u.age >= 21
# GROUP BY u.username
# HAVING COUNT(o.id) > 0
# ORDER BY order_count DESC
# LIMIT 10
```

### Step 2: Subquery Support

Add `.subquery()` to embed one query inside another.

```python
inner = (Query()
         .select("user_id", "COUNT(*) AS cnt")
         .from_("orders")
         .group_by("user_id"))
outer = (Query()
         .select("u.username", "s.cnt")
         .from_("users u")
         .join(inner.subquery("s"), "s.user_id = u.id"))
```

### Step 3: Parameterised Values

Add a params dict to separate SQL from data (preventing SQL injection):

```python
query = (Query()
         .select("*")
         .from_("users")
         .where("age > %(min_age)s")
         .params(min_age=21))
```

## Use It

PostgreSQL's SQL dialect adds features beyond the SQL standard:

| Feature | PostgreSQL | Standard SQL |
|---------|-----------|--------------|
| RETURNING | `INSERT ... RETURNING id` | Not in standard |
| ON CONFLICT | `INSERT ... ON CONFLICT DO UPDATE` | MERGE (different syntax) |
| DISTINCT ON | `SELECT DISTINCT ON (category) *` | Not available |
| ILIKE | Case-insensitive LIKE | Not available |
| SIMILAR TO | Regex-like patterns | Not available |
| GENERATED AS | Identity columns | Standard since SQL:2003 |
| EXCLUDE | Exclusion constraints | Not available |
| JSONB | Binary JSON with indexing | Not available |
| Array types | `INTEGER[]` column | Not available |
| BRIN indexes | Block-range indexes | Not available |
| TABLESAMPLE | Sampling rows | Standard but rarely implemented |

The SQL query builder in `code/main.py` generates standard SQL that works with PostgreSQL with minor dialect adjustments (e.g., using `ILIKE` in the PostgreSQL variant).

## Read the Source

- **PostgreSQL source**: `src/backend/parser/gram.y` — the 15,000-line yacc grammar defining all SQL syntax PostgreSQL supports.
- **SQLite source**: `src/main.c` — the tokeniser and virtual machine that compiles SQL to bytecode.
- **Python's `sqlparse` library**: `sqlparse/engine.py` — a production SQL formatter that parses and regenerates SQL.

## Ship It

The reusable artifact for this lesson lives in `outputs/`. It is:

- **A chainable SQL query builder** (`query_builder.py`) you can import into any Python project to generate SQL programmatically without string concatenation.
- **A SQL join + window function reference** (`sql_cheatsheet.md`) covering all join types, window functions, and PostgreSQL dialect notes.

## Exercises

1. **Easy** — Write a query that finds the top 5 users by total spend using a window function.
2. **Medium** — Extend the query builder to support `UNION` and `INTERSECT` as chainable methods.
3. **Hard** — Implement a recursive CTE query builder that builds an org chart, then add cycle detection.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| DDL | Schema definition | CREATE, ALTER, DROP statements that change table structures |
| DML | Data manipulation | INSERT, UPDATE, DELETE, SELECT — reading and writing rows |
| JOIN | Combining tables | Relational composition: INNER keeps matches, OUTER keeps one/both sides |
| Correlated subquery | Inner query depends on outer | Re-executed for every outer row — can be slow at scale |
| CTE | Named temporary result | WITH clause: like a view that lives for one query (and can be recursive) |
| Window function | Aggregate without GROUP BY | Computes over a frame of rows relative to each row |
| NULL | Unknown / missing | Three-valued logic: NULL = NULL is false, NULL IS NULL is true |
| Set operation | Row-wise union/intersect | UNION, INTERSECT, EXCEPT — stack results vertically |
| Materialized view | Cached query result | A view whose rows are stored on disk, refreshed manually |

## Further Reading

- **PostgreSQL Documentation**: [DDL](https://www.postgresql.org/docs/current/ddl.html), [Queries](https://www.postgresql.org/docs/current/queries.html), [Joins](https://www.postgresql.org/docs/current/queries-table-expressions.html), [Window Functions](https://www.postgresql.org/docs/current/functions-window.html)
- **Use the Index, Luke!** — A free book on SQL indexing: https://use-the-index-luke.com/
- **SQL Performance Explained** by Markus Winand
- **Joe Celko's SQL for Smarties** — Advanced SQL programming patterns
- **PGXN** (PostgreSQL Extension Network) — See real-world user-defined SQL functions
- **Modern SQL** — https://modern-sql.com/ (window functions, CTEs, and the SQL:2023 standard)
