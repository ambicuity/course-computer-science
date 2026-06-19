# The Relational Model & Relational Algebra

> Relations are sets of tuples. SQL is bags. Know the difference, or your queries will lie to you.

**Type:** Build, Learn
**Languages:** Python, SQL
**Prerequisites:** Phase 10 Lesson 01
**Time:** ~75 minutes

## Learning Objectives

- Distinguish the relational model (sets of tuples) from SQL's bag semantics
- Write and evaluate expressions using σ, π, ⋈, ×, ∪, −, ∩, ρ, δ, γ, τ
- Translate SQL `SELECT ... FROM ... WHERE` into a relational algebra tree
- Build a working relational algebra interpreter in Python
- Map Codd's rules onto the features of a real DBMS

## The Problem

You write `SELECT DISTINCT name FROM users WHERE age > 21` and get the right answer. Then someone asks: *why does that query need DISTINCT? Shouldn't a table be a set?*

It's because SQL tables are **multisets (bags)** by default — they allow duplicate rows. The relational model, on which SQL is supposedly based, treats relations as **sets of tuples** — no duplicates possible. That gap between theory and practice causes confusion at every level: why `SELECT * FROM t` can return the same row twice, why `COUNT(*)` behaves differently from `COUNT(DISTINCT *)`, why a `UNION` without `ALL` is slower than you expect.

Worse: when you try to understand how a query planner works — what happens between typing `SELECT` and getting results — the planner's internal representation is almost always a **relational algebra tree**. Every operator in that tree (scan, filter, join, aggregate) corresponds directly to a symbol (σ, π, ⋈, γ). If you cannot read those trees, you cannot optimize queries, you cannot debug slow queries, and you certainly cannot write a query planner.

## The Concept

### Relations ≠ Tables

A **relation** is a set of tuples with a fixed **schema** (ordered attribute names + types). A **tuple** is an ordered list of values, one per attribute. Because it's a *set*, no two tuples are identical — duplicates are impossible by definition.

SQL **tables** are *bags* (multisets) by default. Duplicates are allowed unless you say `DISTINCT`. Every SQL operation is slightly wrong if you think in sets:

| SQL construct | Semantics | Relational algebra equivalent |
|---|---|---|
| `SELECT ... FROM ...` | bag projection | π (but bags, so no dedup) |
| `SELECT DISTINCT ...` | set projection | π |
| `UNION` | set union | ∪ |
| `UNION ALL` | bag union | ∪ (bag version) |
| `FROM a, b` | cross product | × |
| `INNER JOIN ... ON ...` | theta-join | ⋈₀ |

### The Operators

All operators satisfy the **closure property**: they take one or two relations as input and produce a relation as output. This is what lets you compose them into trees.

| Symbol | Name | Signature | Meaning |
|---|---|---|---|
| σₚ(R) | Select | R → R | Keep tuples satisfying predicate p |
| π_{A1,...,An}(R) | Project | R → R | Keep only attributes A1..An (with dedup) |
| R ⋈ₚ S | Theta-join | R × S → R | Cross product + select on p |
| R ⋈ S | Natural join | R × S → R | Equi-join on all common attributes |
| R × S | Cross product | R × S → R | Every tuple of R paired with every tuple of S |
| R ∪ S | Union | R × R → R | Tuples in R or S (set union) |
| R − S | Difference | R × R → R | Tuples in R but not S |
| R ∩ S | Intersection | R × R → R | Tuples in both R and S |
| ρ_{B←A}(R) | Rename | R → R | Rename attribute A to B |
| δ(R) | Duplicate elimination | R → R | Remove duplicate tuples |
| γ_{agg, G}(R) | Aggregation | R → R | Group by G, compute aggs |
| τ_{A}(R) | Sort | R → R | Order by attribute A |

### Codd's 12 Rules

E. F. Codd defined 12 rules (really 13 — rule 0 is "everything else depends on this") that a DBMS must satisfy to be considered *relational*:

0. **Foundation:** A relational DBMS must manage data entirely through its relational capabilities.
1. **Information:** All data is represented as values in table cells.
2. **Guaranteed access:** Every value is addressable by table name, primary key, and column name.
3. **Nulls:** NULLs must be treated systematically (they are *not* values — they represent missing information).
4. **Active catalog:** The database's metadata is stored as relations (queryable via the same model).
5. **Comprehensive sub-language:** At least one language supports data definition, manipulation, integrity, and transactions (SQL qualifies).
6. **View updates:** All views that are theoretically updatable should be updatable by the system.
7. **Set-level operations:** INSERT, UPDATE, DELETE operate on sets of rows, not one at a time.
8. **Physical data independence:** Application programs are unaffected by changes to storage or access methods.
9. **Logical data independence:** Application programs are unaffected by changes to the logical structure (as long as the information is preserved).
10. **Integrity independence:** Integrity constraints are stored in the catalog, not in the application code.
11. **Distribution independence:** The data model works identically whether data is centralized or distributed.
12. **Nonsubversion:** Low-level access (e.g., via a record-at-a-time API) cannot bypass integrity rules.

No real DBMS satisfies all 12 perfectly — but every one claims to be relational, and these rules define the yardstick.

### Query Trees

A SQL query like:

```sql
SELECT DISTINCT u.name
FROM users u JOIN orders o ON u.id = o.user_id
WHERE o.amount > 100
```

Is a relational algebra tree:

```
δ                        -- duplicate elimination
│
π_{name}                 -- project only name
│
⋈_{u.id = o.user_id}    -- join users and orders
      │
σ_{o.amount > 100}       -- filter orders
      │
   ×                     -- cross product (implicit in JOIN)
  / \
users orders
```

The query planner rearranges this tree (e.g., pushing σ below ⋈) without changing the result — because algebra is algebra.

## Build It

### Step 1: Relation class

A relation is a schema plus a set of tuples. We'll store tuples as `dict`s so we access them by name.

```python
from __future__ import annotations
import re
from typing import Callable

class Relation:
    def __init__(self, name: str, schema: list[str], tuples: list[dict]):
        self.name = name
        self.schema = list(schema)  # ordered attribute names
        # Enforce set semantics — no duplicates
        seen = set()
        deduped = []
        for t in tuples:
            key = tuple(t[a] for a in self.schema)
            if key not in seen:
                seen.add(key)
                deduped.append(t)
        self.tuples = deduped

    def __repr__(self) -> str:
        return f"Relation({self.name}, {self.schema}, {len(self.tuples)} tuples)"
```

### Step 2: Relational algebra operators

```python
def select(rel: Relation, pred: Callable[[dict], bool]) -> Relation:
    return Relation(
        f"σ({rel.name})", rel.schema,
        [t for t in rel.tuples if pred(t)]
    )

def project(rel: Relation, attrs: list[str]) -> Relation:
    return Relation(
        f"π_{attrs}({rel.name})", attrs,
        [{a: t[a] for a in attrs} for t in rel.tuples]
    )

def cross(rel1: Relation, rel2: Relation) -> Relation:
    schema = rel1.schema + rel2.schema
    tuples = []
    for t1 in rel1.tuples:
        for t2 in rel2.tuples:
            tuples.append({**t1, **t2})
    return Relation(
        f"({rel1.name} × {rel2.name})", schema, tuples
    )

def theta_join(rel1: Relation, rel2: Relation,
               pred: Callable[[dict], bool]) -> Relation:
    return select(cross(rel1, rel2), pred)

def natural_join(rel1: Relation, rel2: Relation) -> Relation:
    common = [a for a in rel1.schema if a in rel2.schema]
    if not common:
        return cross(rel1, rel2)
    def pred(t: dict) -> bool:
        return all(t[a] == t[f"{a}_right"] for a in common)
    left_only = [a for a in rel1.schema if a not in common]
    right_only = [a for a in rel2.schema if a not in common]
    # Disambiguate: natural join keeps one copy of common attrs
    cross_rel = cross(rel1, rel2)
    joined = select(cross_rel, pred)
    new_schema = rel1.schema + [f"{a}_right" for a in right_only]
    new_tuples = []
    seen = set()
    for t in joined.tuples:
        row = {a: t[a] for a in rel1.schema}
        for a in right_only:
            row[a] = t[f"{a}_right"]
        key = tuple(row[a] for a in rel1.schema + right_only)
        if key not in seen:
            seen.add(key)
            new_tuples.append(row)
    return Relation(
        f"({rel1.name} ⋈ {rel2.name})",
        rel1.schema + right_only,
        new_tuples
    )

def union_(rel1: Relation, rel2: Relation) -> Relation:
    assert rel1.schema == rel2.schema, "Schema mismatch"
    return Relation(
        f"({rel1.name} ∪ {rel2.name})", rel1.schema,
        rel1.tuples + rel2.tuples
    )

def difference(rel1: Relation, rel2: Relation) -> Relation:
    assert rel1.schema == rel2.schema, "Schema mismatch"
    keys2 = set(tuple(t[a] for a in rel1.schema) for t in rel2.tuples)
    return Relation(
        f"({rel1.name} − {rel2.name})", rel1.schema,
        [t for t in rel1.tuples
         if tuple(t[a] for a in rel1.schema) not in keys2]
    )

def rename(rel: Relation, old: str, new: str) -> Relation:
    new_schema = [new if a == old else a for a in rel.schema]
    new_tuples = [{new if k == old else k: v for k, v in t.items()}
                  for t in rel.tuples]
    return Relation(
        f"ρ_{new←old}({rel.name})", new_schema, new_tuples
    )

def eliminate_duplicates(rel: Relation) -> Relation:
    # Already deduped by Relation constructor, but return explicitly
    return Relation(rel.name, rel.schema, rel.tuples)

def aggregate(rel: Relation, group_by: list[str],
              aggs: dict[str, Callable[[list], any]]) -> Relation:
    groups: dict[tuple, list[dict]] = {}
    for t in rel.tuples:
        key = tuple(t[a] for a in group_by)
        groups.setdefault(key, []).append(t)
    schema = group_by + list(aggs.keys())
    tuples = []
    for key, grp in groups.items():
        row = dict(zip(group_by, key))
        for agg_name, func in aggs.items():
            col = grp[0].get(agg_name)
            vals = [t.get(agg_name) for t in grp if t.get(agg_name) is not None]
            row[agg_name] = func(vals) if vals else 0
        tuples.append(row)
    return Relation(f"γ({rel.name})", schema, tuples)

def sort_(rel: Relation, by: list[str]) -> Relation:
    def sort_key(t: dict) -> tuple:
        return tuple(t[a] for a in by)
    sorted_tuples = sorted(rel.tuples, key=sort_key)
    result = Relation(rel.name, rel.schema, [])
    result.tuples = sorted_tuples
    return result
```

### Step 3: RA expression parser

We'll implement a tiny parser that handles expressions like:

`π_name(σ_age>21(Person))`

Grammar:

```
expr    := σ_cond(expr) | π_attrs(expr) | relation_name
cond    := attr op lit  (where op is >, <, =, >=, <=, !=)
attrs   := attr,attrs | attr
```

```python
def parse_and_evaluate(expr: str, relations: dict[str, Relation]) -> Relation:
    expr = expr.strip()
    # Match σ: σ_{cond}(inner)
    m = re.match(r'σ_(.+?)\((.+)\)', expr)
    if m:
        cond_str = m.group(1)
        inner = parse_and_evaluate(m.group(2), relations)
        # Parse condition: attr op value
        m2 = re.match(r'(\w+)\s*(>|<|>=|<=|=|!=)\s*(.+)', cond_str)
        if not m2:
            raise ValueError(f"Cannot parse condition: {cond_str}")
        attr, op, val = m2.group(1), m2.group(2), m2.group(3).strip()
        # Try numeric, else string
        try:
            val = int(val)
        except ValueError:
            val = val.strip("'\"")
        def pred(t: dict) -> bool:
            v = t.get(attr)
            if op == '>': return v > val
            if op == '<': return v < val
            if op == '>=': return v >= val
            if op == '<=': return v <= val
            if op == '=': return v == val
            if op == '!=': return v != val
            return False
        return select(inner, pred)
    # Match π: π_{attrs}(inner)
    m = re.match(r'π_(.+?)\((.+)\)', expr)
    if m:
        attrs = [a.strip() for a in m.group(1).split(',')]
        inner = parse_and_evaluate(m.group(2), relations)
        return project(inner, attrs)
    # Match ⋈ (theta): R⋈_cond(S) or just R⋈S for natural
    m = re.match(r'(.+?)⋈(.+)', expr)
    if m:
        left = parse_and_evaluate(m.group(1), relations)
        right = parse_and_evaluate(m.group(2), relations)
        return natural_join(left, right)
    # Match ∪, −
    m = re.match(r'(.+?)∪(.+)', expr)
    if m:
        left = parse_and_evaluate(m.group(1), relations)
        right = parse_and_evaluate(m.group(2), relations)
        return union_(left, right)
    m = re.match(r'(.+?)−(.+)', expr)
    if m:
        left = parse_and_evaluate(m.group(1), relations)
        right = parse_and_evaluate(m.group(2), relations)
        return difference(left, right)
    # Relation name
    if expr in relations:
        return relations[expr]
    raise ValueError(f"Cannot parse: {expr}")
```

### Step 4: Demo

```python
def demo():
    users = Relation("Users", ["id", "name", "age"], [
        {"id": 1, "name": "Alice", "age": 30},
        {"id": 2, "name": "Bob", "age": 20},
        {"id": 3, "name": "Charlie", "age": 25},
    ])
    orders = Relation("Orders", ["id", "user_id", "amount", "product"], [
        {"id": 1, "user_id": 1, "amount": 150, "product": "Laptop"},
        {"id": 2, "user_id": 2, "amount": 50, "product": "Mouse"},
        {"id": 3, "user_id": 1, "amount": 75, "product": "Keyboard"},
        {"id": 4, "user_id": 3, "amount": 200, "product": "Monitor"},
    ])
    rels = {"Users": users, "Orders": orders}

    # Query: names of users who placed orders > 100
    # π_name(σ_amount>100(Users ⋈ Orders))
    r = parse_and_evaluate("π_name(σ_amount>100(Users⋈Orders))", rels)
    print("Users with orders > 100:", r.tuples)

    # Query: ids of users older than 21
    r = parse_and_evaluate("π_id(σ_age>21(Users))", rels)
    print("Users older than 21:", r.tuples)

    # Union: users ∪ empty — identity
    empty = Relation("Empty", ["id", "name", "age"], [])
    r = parse_and_evaluate("Users∪Empty", {"Users": users, "Empty": empty})
    print("Users ∪ Empty:", len(r.tuples), "tuples")

    # Difference
    adults = select(users, lambda t: t["age"] >= 21)
    young = select(users, lambda t: t["age"] < 25)
    r = difference(adults, young)
    print("Adults − (age<25):", r.tuples)

    # Rename
    r = rename(users, "name", "full_name")
    print("Renamed schema:", r.schema)
```

## Use It

PostgreSQL's query planner represents every query as a tree of `PlanNode` structs. You can see them with `EXPLAIN`:

```sql
EXPLAIN (FORMAT JSON) SELECT DISTINCT u.name
FROM users u JOIN orders o ON u.id = o.user_id
WHERE o.amount > 100;
```

The output contains a tree with node types like `"Hash Join"`, `"Seq Scan"`, `"Filter"`, `"HashAggregate"` — each directly maps to an RA operator:

| EXPLAIN node | RA operator |
|---|---|
| Seq Scan | Relation scan |
| Filter | σ |
| Hash / Merge Join | ⋈ |
| HashAggregate / GroupAggregate | γ |
| Unique / HashAggregate + DISTINCT | δ |
| Sort | τ |

PostgreSQL also has an explicit `UNION ALL` plan node (bag union) vs `UNIQUE` after a `UNION` (set union), mirroring the same theory-practice gap.

Our toy interpreter does none of PostgreSQL's optimizations (join ordering, index scans, parallel execution, materialization) — but the core algebraic structure is identical. Every query planner is just an algebra optimizer.

## Read the Source

- **PostgreSQL source:** `src/backend/optimizer/plan/` — the planner converts a parsed SQL query (`Query` node) into a `PlannerInfo` and eventually a `Plan` tree. The `make_rel_from_joinlist` function in `or.c` is the direct implementation of RA join reordering.
- **SQLite source:** `sqlite3.c`, the `sqlite3WhereBegin()` function — SQLite's bytecode generator compiles a similar tree into virtual machine instructions.
- **CMU's Bustub:** `src/planner/` — Bustub is an educational DBMS that explicitly exposes RA expression classes (`LogicalAggregate`, `LogicalFilter`, `LogicalJoin`).

## Ship It

The reusable artifact is the RA interpreter built in `code/main.py`. Save a copy to `outputs/ra_interpreter.py` — you will need it when you build the query planner for the MVCC KV store capstone.

## Exercises

1. **Easy:** Translate this SQL query into relational algebra (both symbolic and tree form):
   ```sql
   SELECT name FROM employees WHERE salary > 50000 AND dept_id = 3;
   ```

2. **Easy:** Write an RA expression equivalent to `SELECT DISTINCT product FROM orders WHERE amount BETWEEN 50 AND 200`.

3. **Medium:** Given `Student(id, name)` and `Enrollment(student_id, course_id)`, express "names of students enrolled in course 42" in RA. Then translate it back to SQL. Show both.

4. **Medium:** Using the interpreter, express `π_name(σ_salary>60000(ρ_sal←salary(Employees)))`. Explain why the rename is needed if the schema uses "salary" but the scan predicate references "sal".

5. **Hard:** Implement division (÷) in the interpreter: R ÷ S finds all tuples in R that match *every* tuple in S (over common attributes). This is the RA operation underlying SQL's `NOT EXISTS (... AND NOT EXISTS (...))`. Show it works with a "students who take all courses" query.

6. **Hard:** Add a `δ` (distinct) node to the parser, then compare `∪` vs `∪ ALL` behavior for a bag-relation variant. Measure the overhead of deduplication in the interpreter.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Relation | A table | A **set** of tuples with a fixed schema — no duplicates allowed |
| Bag | A table with duplicates allowed | A multiset — SQL tables are bags by default; `DISTINCT` converts to set semantics |
| Select (σ) | `WHERE` clause | Filers tuples by a predicate; NOT the SQL `SELECT` keyword (which is projection) |
| Project (π) | `SELECT` columns | Keeps specified attributes; eliminates duplicate rows (sets) unless you're in bag mode |
| Join (⋈) | `JOIN ... ON` | Cross product + selection (theta-join) or equi-join on matching attribute names (natural join) |
| Closure | RA operators compose | Every operator takes relation(s) and returns a relation — you can nest them arbitrarily |
| Codd's Rules | 12 rules for relational databases | More like 13 (0–12); no real DBMS satisfies all perfectly, but they define the target |
| Query tree | Internal plan representation | An operator tree (σ, π, ⋈, etc.) that the planner optimizes before execution |

## Further Reading

- *Database System Concepts* (Silberschatz, Korth, Sudarshan) — Chapters 2 and 6: relational model and RA, the canonical treatment.
- *Foundations of Databases* (Abiteboul, Hull, Vianu) — The formal theory, including why bag semantics breaks algebraic equivalences.
- [Codd's 12 Rules (original 1985 paper)](https://www.relationaldbdesign.com/relational-database-analysis/module2/codd-twelve-rules.php) — Read what Codd actually wrote, not the sanitized summaries.
- [PostgreSQL EXPLAIN documentation](https://www.postgresql.org/docs/current/using-explain.html) — Learn to read the planner's output as an RA tree.
- [CMU 15-445: Relational Algebra lecture](https://www.youtube.com/watch?v=uik8HsQhjSs) — Andy Pavlo's clear, example-driven walkthrough.
- *SQL and Relational Theory* (C. J. Date) — How to think in sets when SQL forces you into bags.
