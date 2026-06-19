"""
SQL — DDL, DML, Joins, Subqueries
Phase 10 — Databases & Storage Systems

A chainable SQL query builder that generates valid SQL strings.
Usage: python3 main.py
"""

from __future__ import annotations

import textwrap
from typing import Any


class Query:
    """Chainable SQL query builder.

    Usage:
        query = (Query()
                 .select("name", "age")
                 .from_("users")
                 .where("age > 21")
                 .order_by("name")
                 .limit(10))
        print(query.sql())
    """

    def __init__(self) -> None:
        self._select_clauses: list[str] = []
        self._from_clause: str | None = None
        self._joins: list[str] = []
        self._where_clauses: list[str] = []
        self._group_by_clauses: list[str] = []
        self._having_clauses: list[str] = []
        self._order_by_clauses: list[str] = []
        self._limit_count: int | None = None
        self._params: dict[str, Any] = {}
        self._is_subquery: bool = False
        self._alias: str | None = None

    def select(self, *columns: str) -> Query:
        self._select_clauses.extend(columns)
        return self

    def from_(self, table: str) -> Query:
        self._from_clause = table
        return self

    def join(self, table: str, condition: str, kind: str = "INNER") -> Query:
        kind = kind.upper().strip()
        if kind == "INNER":
            self._joins.append(f"INNER JOIN {table} ON {condition}")
        elif kind in ("LEFT", "RIGHT", "FULL"):
            self._joins.append(f"{kind} OUTER JOIN {table} ON {condition}")
        elif kind == "CROSS":
            self._joins.append(f"CROSS JOIN {table}")
        elif kind == "NATURAL":
            self._joins.append(f"NATURAL JOIN {table}")
        elif kind == "SELF":
            # self-join: use the same table with different alias
            self._joins.append(f"JOIN {table} ON {condition}")
        else:
            self._joins.append(f"{kind} JOIN {table} ON {condition}")
        return self

    def where(self, condition: str) -> Query:
        self._where_clauses.append(condition)
        return self

    def group_by(self, *columns: str) -> Query:
        self._group_by_clauses.extend(columns)
        return self

    def having(self, condition: str) -> Query:
        self._having_clauses.append(condition)
        return self

    def order_by(self, *columns: str) -> Query:
        self._order_by_clauses.extend(columns)
        return self

    def limit(self, count: int) -> Query:
        self._limit_count = count
        return self

    def params(self, **kwargs: Any) -> Query:
        self._params.update(kwargs)
        return self

    def subquery(self, alias: str) -> Query:
        """Mark this query as a subquery with a given alias."""
        self._is_subquery = True
        self._alias = alias
        return self

    def sql(self) -> str:
        parts: list[str] = []

        # SELECT
        if self._select_clauses:
            parts.append("SELECT " + ", ".join(self._select_clauses))
        else:
            parts.append("SELECT *")

        # FROM
        if self._from_clause:
            parts.append(f"FROM {self._from_clause}")

        # JOINs
        parts.extend(self._joins)

        # WHERE
        if self._where_clauses:
            parts.append("WHERE " + " AND ".join(self._where_clauses))

        # GROUP BY
        if self._group_by_clauses:
            parts.append("GROUP BY " + ", ".join(self._group_by_clauses))

        # HAVING
        if self._having_clauses:
            parts.append("HAVING " + " AND ".join(self._having_clauses))

        # ORDER BY
        if self._order_by_clauses:
            parts.append("ORDER BY " + ", ".join(self._order_by_clauses))

        # LIMIT
        if self._limit_count is not None:
            parts.append(f"LIMIT {self._limit_count}")

        clause = "\n".join(parts)

        if self._is_subquery and self._alias:
            return f"(\n{textwrap.indent(clause, '    ')}\n) {self._alias}"
        return clause

    def __str__(self) -> str:
        return self.sql()


# ── Demo / Test Script ──────────────────────────────────────────────


def demo_ddl() -> None:
    """Demonstrate DDL generation via builder patterns."""
    print("=" * 60)
    print("DDL — Schema Definition Patterns")
    print("=" * 60)

    create_table = """
CREATE TABLE users (
    id         SERIAL       PRIMARY KEY,
    email      VARCHAR(255) NOT NULL UNIQUE,
    username   VARCHAR(100) NOT NULL,
    age        INTEGER      CHECK (age >= 0),
    created_at TIMESTAMP    DEFAULT NOW()
);

CREATE TABLE orders (
    id         SERIAL        PRIMARY KEY,
    user_id    INTEGER       NOT NULL REFERENCES users(id),
    total      NUMERIC(10,2) CHECK (total > 0),
    status     TEXT          DEFAULT 'pending'
);

CREATE INDEX idx_users_email ON users (email);
CREATE INDEX idx_orders_user_id ON orders (user_id);
"""
    print(create_table.strip())


def demo_select() -> None:
    """Basic SELECT with filtering, grouping, ordering."""
    print("=" * 60)
    print("SELECT — Filtering, Grouping, Ordering")
    print("=" * 60)

    q = (Query()
         .select("u.username", "COUNT(o.id) AS order_count")
         .from_("users u")
         .join("orders o", "o.user_id = u.id")
         .where("u.age >= 21")
         .group_by("u.username")
         .having("COUNT(o.id) > 2")
         .order_by("order_count DESC")
         .limit(10))
    print(q)
    print()


def demo_join_types() -> None:
    """Demonstrate all JOIN types."""
    print("=" * 60)
    print("JOIN Types")
    print("=" * 60)

    scenarios = [
        ("INNER JOIN", Query().select("*").from_("users u")
         .join("orders o", "o.user_id = u.id", "INNER")),
        ("LEFT JOIN", Query().select("*").from_("users u")
         .join("orders o", "o.user_id = u.id", "LEFT")),
        ("RIGHT JOIN", Query().select("*").from_("orders o")
         .join("users u", "u.id = o.user_id", "RIGHT")),
        ("FULL OUTER JOIN", Query().select("*").from_("users u")
         .join("orders o", "o.user_id = u.id", "FULL")),
        ("CROSS JOIN", Query().select("*").from_("users")
         .join("orders", "1=1", "CROSS")),
        ("NATURAL JOIN", Query().select("*").from_("users")
         .join("orders", "", "NATURAL")),
        ("SELF JOIN", Query()
         .select("a.username AS user", "b.username AS referred_by")
         .from_("users a")
         .join("users b", "a.referred_by_id = b.id", "SELF")),
    ]

    for name, query in scenarios:
        print(f"-- {name}")
        print(query)
        print()


def demo_subqueries() -> None:
    """Scalar, table, correlated subqueries, and CTEs."""
    print("=" * 60)
    print("Subqueries & CTEs")
    print("=" * 60)

    # Scalar subquery
    print("-- Scalar subquery in SELECT")
    q = (Query()
         .select("username",
                 "(SELECT AVG(total) FROM orders WHERE user_id = users.id) AS avg_order")
         .from_("users"))
    print(q)
    print()

    # Table subquery with IN
    print("-- Table subquery with IN")
    q = (Query()
         .select("*")
         .from_("users")
         .where("id IN (SELECT user_id FROM orders WHERE status = 'paid')"))
    print(q)
    print()

    # Correlated subquery with EXISTS
    print("-- Correlated subquery with EXISTS")
    q = (Query()
         .select("*")
         .from_("users u")
         .where("EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id AND o.status = 'paid')"))
    print(q)
    print()

    # ANY / ALL
    print("-- ANY / ALL subqueries")
    q = (Query()
         .select("*")
         .from_("products")
         .where("price > ALL (SELECT price FROM products WHERE category = 'clearance')"))
    print(q)
    print()

    q = (Query()
         .select("*")
         .from_("users")
         .where("id = ANY (SELECT user_id FROM orders WHERE total > 100)"))
    print(q)
    print()

    # CTE with subquery builder
    print("-- CTE (WITH) — simulated by subquery composition")
    inner = (Query()
             .select("user_id", "COUNT(*) AS cnt")
             .from_("orders")
             .where("status = 'paid'")
             .group_by("user_id")
             .subquery("paid_orders"))
    outer = (Query()
             .select("u.username", "COALESCE(po.cnt, 0) AS paid_count")
             .from_("users u")
             .join(str(inner), "po.user_id = u.id", "LEFT")
             .order_by("paid_count DESC"))
    print(outer)
    print()


def demo_set_operations() -> None:
    """Set operations: UNION, INTERSECT, EXCEPT."""
    print("=" * 60)
    print("Set Operations")
    print("=" * 60)

    print("-- UNION (deduplicated)")
    print("SELECT city FROM customers")
    print("UNION")
    print("SELECT city FROM suppliers")
    print()

    print("-- UNION ALL (keeps duplicates)")
    print("SELECT city FROM customers")
    print("UNION ALL")
    print("SELECT city FROM suppliers")
    print()

    print("-- INTERSECT")
    print("SELECT user_id FROM orders WHERE status = 'paid'")
    print("INTERSECT")
    print("SELECT user_id FROM orders WHERE status = 'shipped'")
    print()

    print("-- EXCEPT")
    print("SELECT id FROM users")
    print("EXCEPT")
    print("SELECT user_id FROM orders")
    print()


def demo_window_functions() -> None:
    """Window functions: ROW_NUMBER, RANK, LAG/LEAD, aggregates."""
    print("=" * 60)
    print("Window Functions")
    print("=" * 60)

    q = (Query()
         .select(
             "username",
             "order_date",
             "total",
             "ROW_NUMBER() OVER (PARTITION BY username ORDER BY order_date DESC) AS rn",
             "RANK() OVER (ORDER BY total DESC) AS rank",
             "DENSE_RANK() OVER (ORDER BY total DESC) AS dense_rank",
             "LAG(total) OVER (PARTITION BY username ORDER BY order_date) AS prev_order",
             "LEAD(total) OVER (PARTITION BY username ORDER BY order_date) AS next_order",
             "SUM(total) OVER (PARTITION BY username) AS user_total",
         )
         .from_("user_orders"))
    print(q)
    print()


def demo_views() -> None:
    """Views and materialized views."""
    print("=" * 60)
    print("Views & Materialized Views")
    print("=" * 60)

    print("-- Standard view")
    print(textwrap.dedent("""\
    CREATE VIEW active_users AS
    SELECT id, username, email
    FROM users
    WHERE age >= 18 AND deleted_at IS NULL;
    """))
    print("-- Materialized view (PostgreSQL)")
    print(textwrap.dedent("""\
    CREATE MATERIALIZED VIEW monthly_sales AS
    SELECT DATE_TRUNC('month', created_at) AS month,
           SUM(total) AS revenue
    FROM orders
    GROUP BY month;

    REFRESH MATERIALIZED VIEW monthly_sales;
    """))


def demo_postgresql_dialect() -> None:
    """PostgreSQL-specific SQL features."""
    print("=" * 60)
    print("PostgreSQL Dialect Features")
    print("=" * 60)

    print("-- RETURNING clause")
    print("INSERT INTO users (email, username) VALUES ('a@b.com', 'a') RETURNING id;")
    print()

    print("-- ON CONFLICT (UPSERT)")
    print(textwrap.dedent("""\
    INSERT INTO users (email, username)
    VALUES ('a@b.com', 'a')
    ON CONFLICT (email) DO UPDATE SET username = EXCLUDED.username;
    """))
    print()

    print("-- DISTINCT ON")
    print("SELECT DISTINCT ON (category) id, name, price FROM products ORDER BY category, price;")
    print()

    print("-- ILIKE (case-insensitive)")
    print("SELECT * FROM users WHERE username ILIKE 'alice%';")
    print()

    print("-- TABLESAMPLE")
    print("SELECT * FROM users TABLESAMPLE SYSTEM (1);")
    print()


def main() -> None:
    demo_ddl()
    demo_select()
    demo_join_types()
    demo_subqueries()
    demo_set_operations()
    demo_window_functions()
    demo_views()
    demo_postgresql_dialect()


if __name__ == "__main__":
    main()
