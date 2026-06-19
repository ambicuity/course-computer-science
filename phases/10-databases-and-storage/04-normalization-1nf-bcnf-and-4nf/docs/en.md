# Normalization 1NF → BCNF (and 4NF)

> A relation is in good shape when every non-trivial FD is a key constraint. Everything else is redundancy waiting to corrupt your data.

**Type:** Build, Learn
**Languages:** Python, SQL
**Prerequisites:** Phase 10 Lessons 01–03 (relational model, relational algebra, SQL)
**Time:** ~75 minutes

## Learning Objectives

- Identify update, insert, and delete anomalies in a denormalized schema
- Compute attribute closure and minimal cover for a set of functional dependencies
- Decompose a relation schema into BCNF (lossless) and synthesize 3NF (dependency-preserving)
- Distinguish 1NF, 2NF, 3NF, BCNF, 4NF, and know when each applies
- Translate between a logical ER-style schema and its normalized relational equivalent
- Build a normalization tool in Python that automates closure, cover, and decomposition

## The Problem

You design a database for a school. One table seems convenient:

```
Enrollments(StudentID, StudentName, Major, AdvisorID, AdvisorName,
            CourseCode, CourseTitle, InstructorID, InstructorName, Grade)
```

A week later you notice three kinds of corruption:

1. **Update anomaly:** Professor "Kim" changes her name to "Kim-Chen". You must update *every* enrollment row where she's the instructor. Miss one, and the database has two names for the same person.
2. **Insert anomaly:** You hire a new advisor who hasn't advised anyone yet. You cannot add her to the database because there's no enrollment row for her — and StudentID is part of the primary key.
3. **Delete anomaly:** You delete the last student enrolled in course "CS101". That also deletes the course title, instructor name, and everything else about CS101 — even though the course still exists.

These are not bugs in your application code. They are bugs in your **schema**. The table is too big — it packs unrelated facts into one relation, so updating one fact forces you to touch many rows.

Normalization is the process of decomposing a relation into smaller, well-structured relations where each fact lives in exactly one place. Each normal form is a rule that eliminates a specific kind of redundancy.

## The Concept

### Functional Dependencies

A **functional dependency** (FD) X→Y means: if two tuples agree on all attributes in X, they must also agree on all attributes in Y. X **determines** Y.

Example: `StudentID → StudentName` means every student has exactly one name. `StudentID, CourseCode → Grade` means a student gets one grade per course.

FDs are the atomic unit of schema constraint. Everything in normalization flows from them.

### Armstrong's Axioms

Three rules let you derive all FDs implied by a given set:

| Axiom | Rule | Meaning |
|-------|------|---------|
| Reflexivity | If Y ⊆ X, then X → Y | Trivial dependencies always hold |
| Augmentation | If X → Y, then XZ → YZ | Adding the same attribute to both sides preserves the FD |
| Transitivity | If X → Y and Y → Z, then X → Z | Determination chains |

These are **sound** (everything derived is true) and **complete** (every true FD can be derived). Three derived rules are also useful:

- **Union:** X → Y and X → Z implies X → YZ
- **Decomposition:** X → YZ implies X → Y and X → Z
- **Pseudotransitivity:** X → Y and WY → Z implies WX → Z

### Attribute Closure

The **closure** of a set of attributes X under FDs F, written X⁺, is the set of all attributes that X determines. Algorithm:

```
closure = X
while closure changed:
    for each FD Y→Z in F:
        if Y ⊆ closure:
            closure = closure ∪ Z
```

X is a **superkey** if X⁺ contains all attributes of the relation. A **candidate key** is a minimal superkey (no proper subset is also a superkey).

### Normal Forms

```
1NF: all columns atomic (no repeating groups, no multi-valued attributes)
 │
2NF: 1NF + no partial dependencies on any candidate key
 │
3NF: 2NF + no transitive dependencies on any candidate key
 │
BCNF: 3NF + every determinant is a candidate key
 │
4NF: BCNF + no multi-valued dependencies beyond candidate keys
 │
5NF: 4NF + every join dependency implied by candidate keys
 │
DKNF: every constraint is a domain constraint or a key constraint
```

Each normal form eliminates a specific redundancy. In practice, most schemas stop at BCNF or 3NF.

#### 1NF — Atomic Columns

Every column holds a single, indivisible value. No lists, no JSON blobs (in the relational model), no repeating groups.

| StudentID | Courses         | Violation?        |
|-----------|-----------------|-------------------|
| 1         | CS101, MATH200  | Multi-valued      |
| 2         | CS101           | (repeating group) |

1NF version:

| StudentID | CourseCode |
|-----------|------------|
| 1         | CS101      |
| 1         | MATH200    |
| 2         | CS101      |

#### 2NF — No Partial Dependencies

A **partial dependency** is an FD where a proper subset of a candidate key determines a non-key attribute.

Given R(A, B, C, D) with CK = {A, B} and FD A → C: C depends on only part of the key. Decompose:

- R1(A, C)
- R2(A, B, D)

#### 3NF — No Transitive Dependencies

A **transitive dependency** is an FD where a non-key attribute determines another non-key attribute.

Given R(A, B, C) with CK = {A} and FD B → C: C transitively depends on A through B. Decompose:

- R1(B, C)
- R2(A, B)

A relation is in 3NF if for every non-trivial FD X → Y, either X is a superkey or Y's attributes are all **prime** (members of some candidate key).

#### BCNF — Every Determinant Is a Key

BCNF tightens 3NF: for every non-trivial FD X → Y, X **must** be a superkey. The "or Y is prime" loophole is closed.

The classic 3NF-but-not-BCNF case: R(A, B, C) with FDs {AB → C, C → B}. Candidate keys are {A, B} and {A, C}. The FD C → B violates BCNF (C is not a superkey) but satisfies 3NF (B is prime).

#### 4NF — No Multi-Valued Dependencies

A **multi-valued dependency** (MVD) X →→ Y means: given a value for X, the set of Y values is independent of all other attributes. 4NF requires that for every non-trivial MVD, X is a superkey.

Example: A person can have multiple degrees and multiple phone numbers — these are independent facts that need separate tables.

#### 5NF / DKNF (briefly)

**5NF** handles **join dependencies**: a relation is in 5NF if every join dependency is implied by a candidate key. **DKNF** (Domain-Key Normal Form) is the theoretical ideal: every constraint is either a domain constraint or a key constraint. In practice, almost no real schema reaches 5NF.

### Lossless vs Lossy Decomposition

A decomposition of R into R₁, R₂, ..., Rₙ is **lossless** if for every valid instance, the natural join of the decomposed relations yields exactly the original relation — no spurious tuples.

**Lossless join test** (binary decomposition): decomposing R into R₁ and R₂ is lossless iff R₁ ∩ R₂ → R₁ or R₁ ∩ R₂ → R₂ (i.e., the common attributes form a superkey in at least one of the decomposed relations).

### Dependency Preservation

A decomposition **preserves dependencies** if every FD in the original set can be enforced locally on the decomposed relations without cross-referencing. BCNF decomposition is always lossless but may lose FDs. 3NF synthesis guarantees both losslessness and dependency preservation.

## Build It

We'll build a Python normalization tool that automates the key algorithms.

### Step 1: Parse FDs and Compute Closure

```python
from typing import Set, List, Tuple, Optional
from itertools import combinations

Attribute = str
FD = Tuple[frozenset, frozenset]

def parse_fds(s: str) -> List[FD]:
    result = []
    for part in s.replace(' ', '').split(','):
        part = part.strip()
        if not part:
            continue
        lhs_str, rhs_str = part.split('→')
        result.append((frozenset(lhs_str), frozenset(rhs_str)))
    return result

def compute_closure(attrs: Set[str], fds: List[FD]) -> Set[str]:
    closure = set(attrs)
    changed = True
    while changed:
        changed = False
        for lhs, rhs in fds:
            if lhs <= closure and not rhs <= closure:
                closure |= rhs
                changed = True
    return closure
```

### Step 2: Compute Minimal Cover

A **minimal cover** is a set of FDs that is:
1. **Right-singleton:** every RHS is a single attribute
2. **Left-reduced:** no extraneous attributes on any LHS
3. **Non-redundant:** no FD can be derived from the others

```python
def decompose_rhs(fds: List[FD]) -> List[FD]:
    result = []
    for lhs, rhs in fds:
        for a in rhs:
            result.append((lhs, frozenset({a})))
    return result

def extraneous_left(fds: List[FD], idx: int, attr: str) -> bool:
    lhs, rhs = fds[idx]
    reduced_lhs = frozenset(a for a in lhs if a != attr)
    if reduced_lhs == lhs:
        return False
    key_in = compute_closure(set(reduced_lhs), fds)
    return rhs <= key_in

def eliminate_extraneous_left(fds: List[FD]) -> List[FD]:
    result = list(fds)
    for i in range(len(result)):
        lhs, rhs = result[i]
        for a in list(lhs):
            if extraneous_left(result, i, a):
                new_lhs = frozenset(x for x in lhs if x != a)
                result[i] = (new_lhs, rhs)
    return result

def redundant_fd(fds: List[FD], idx: int) -> bool:
    target_lhs, target_rhs = fds[idx]
    reduced = [fd for j, fd in enumerate(fds) if j != idx]
    closure = compute_closure(set(target_lhs), reduced)
    return target_rhs <= closure

def eliminate_redundant(fds: List[FD]) -> List[FD]:
    result = list(fds)
    i = 0
    while i < len(result):
        if redundant_fd(result, i):
            result.pop(i)
        else:
            i += 1
    return result

def minimal_cover(fds: List[FD]) -> List[FD]:
    fds = decompose_rhs(fds)
    fds = eliminate_extraneous_left(fds)
    fds = eliminate_redundant(fds)
    return fds
```

### Step 3: BCNF Decomposition

A relation is in BCNF if for every non-trivial FD X → Y, X is a superkey.

```python
def find_bcnf_violation(schema: Set[str], fds: List[FD]) -> Optional[FD]:
    for lhs, rhs in fds:
        if not (lhs <= schema and rhs <= schema):
            continue
        if rhs <= lhs:
            continue
        closure = compute_closure(set(lhs), fds) & schema
        if closure != schema:
            return (lhs, rhs)
    return None

def bcnf_decompose(schema: Set[str], fds: List[FD]) -> List[Set[str]]:
    result = [set(schema)]
    while True:
        changed = False
        new_result = []
        for rel in result:
            viol = find_bcnf_violation(rel, fds)
            if viol is not None:
                lhs, _ = viol
                closure = compute_closure(set(lhs), fds) & rel
                r1 = set(closure)
                r2 = (rel - r1) | set(lhs)
                if r1 and r2 and r1 != rel and r2 != rel:
                    new_result.append(r1)
                    new_result.append(r2)
                    changed = True
                else:
                    new_result.append(rel)
            else:
                new_result.append(rel)
        result = new_result
        if not changed:
            break
    return result
```

### Step 4: 3NF Synthesis

3NF synthesis groups FDs by LHS, then adds a key relation if needed.

```python
def candidate_keys(schema: Set[str], fds: List[FD]) -> List[Set[str]]:
    schema_set = set(schema)
    lhs_attrs: Set[str] = set()
    rhs_attrs: Set[str] = set()
    for lhs, rhs in fds:
        lhs_attrs |= set(lhs)
        rhs_attrs |= set(rhs)
    must_attrs = (lhs_attrs - rhs_attrs) | (schema_set - lhs_attrs - rhs_attrs)
    must_attrs &= schema_set
    maybe_attrs = (lhs_attrs & rhs_attrs) & schema_set
    if compute_closure(must_attrs, fds) == schema_set:
        return [must_attrs]
    keys: List[Set[str]] = []
    maybe_list = sorted(maybe_attrs)
    for r in range(len(maybe_list) + 1):
        for combo in combinations(maybe_list, r):
            test = must_attrs | set(combo)
            if any(k.issubset(test) for k in keys):
                continue
            if compute_closure(test, fds) == schema_set:
                keys.append(test)
        if keys:
            break
    return keys

def synth_3nf(schema: Set[str], fds: List[FD]) -> List[Set[str]]:
    g = minimal_cover(fds)
    groups: dict[frozenset, set] = {}
    for lhs, rhs in g:
        key = frozenset(lhs)
        if key not in groups:
            groups[key] = set(lhs)
        groups[key] |= set(rhs)
    result = list(groups.values())
    result = [r for r in result
              if not any(r != s and r <= s for s in result)]
    keys = candidate_keys(schema, fds)
    if not any(any(k <= r for k in keys) for r in result):
        result.append(keys[0])
    return result
```

### Step 5: Demo — School Database

```python
def show_section(title: str):
    print(f"\n{'='*60}")
    print(f"  {title}")
    print(f"{'='*60}")

def fmt_set(s: set) -> str:
    return ''.join(sorted(s))

def demo_school():
    # Attributes: S=tudentID, N=ame, M=ajor, A=dvisorID, D=visorName,
    #             C=ourseCode, T=itle, I=nstructorID, E=nstructorName, G=rade
    attr_names = {
        'S': 'StudentID', 'N': 'StudentName', 'M': 'Major',
        'A': 'AdvisorID', 'D': 'AdvisorName',
        'C': 'CourseCode', 'T': 'CourseTitle', 'I': 'InstructorID',
        'E': 'InstructorName', 'G': 'Grade'
    }
    schema = set('SNMADC TIEG'.replace(' ', ''))
    fd_str = "S→NMA, A→D, C→TI, I→E, SC→G"
    fds = parse_fds(fd_str)

    show_section("Original Schema & FDs")
    attrs_show = ', '.join(f'{k}={v}' for k, v in sorted(attr_names.items()))
    print(f"Attributes: {{{attrs_show}}}")
    print(f"Schema: {{{fmt_set(schema)}}}")
    print(f"FDs: {fd_str}")

    show_section("Attribute Closure Examples")
    for attrs in ['S', 'C', 'SC', 'A']:
        cl = compute_closure(set(attrs), fds)
        print(f"  {attrs}⁺ = {{{fmt_set(cl)}}}")

    show_section("Minimal Cover")
    mc = minimal_cover(fds)
    for lhs, rhs in mc:
        print(f"  {fmt_set(lhs)} → {fmt_set(rhs)}")

    show_section("Candidate Keys")
    cks = candidate_keys(schema, fds)
    for ck in cks:
        print(f"  {{{fmt_set(ck)}}}")

    show_section("BCNF Decomposition (lossless)")
    bcnf_rels = bcnf_decompose(schema, fds)
    for i, rel in enumerate(bcnf_rels, 1):
        names = ', '.join(attr_names.get(a, a) for a in sorted(rel))
        print(f"  R{i}: {{{fmt_set(rel)}}}  →  {names}")

    show_section("3NF Synthesis (dependency-preserving)")
    tnf_rels = synth_3nf(schema, fds)
    for i, rel in enumerate(tnf_rels, 1):
        names = ', '.join(attr_names.get(a, a) for a in sorted(rel))
        print(f"  R{i}: {{{fmt_set(rel)}}}  →  {names}")
```

### Step 6: 3NF-but-not-BCNF Example

```python
def demo_3nf_not_bcnf():
    show_section("3NF-but-not-BCNF Example")
    schema = set('ABC')
    fd_str = "AB→C, C→B"
    fds = parse_fds(fd_str)
    print(f"Schema: {{{fmt_set(schema)}}}, FDs: {fd_str}")
    cks = candidate_keys(schema, fds)
    print("Candidate keys:")
    for ck in cks:
        print(f"  {{{fmt_set(ck)}}}")
    bcnf_rels = bcnf_decompose(schema, fds)
    print("BCNF decomposition:")
    for i, rel in enumerate(bcnf_rels, 1):
        print(f"  R{i}: {{{fmt_set(rel)}}}")
    print("  (loses FD AB→C — can't enforce across decomposed tables)")
    tnf_rels = synth_3nf(schema, fds)
    print("3NF synthesis:")
    for i, rel in enumerate(tnf_rels, 1):
        print(f"  R{i}: {{{fmt_set(rel)}}}")
    print("  (preserves all FDs)")
```

## Use It

Real database design tools and ORMs use these algorithms internally:

- **PostgreSQL's `CHECK` and `FOREIGN KEY`** constraints encode FDs directly. A UNIQUE index declares a superkey. A NOT NULL + UNIQUE combo declares a candidate key.
- **ER-to-relational mapping** (used by every ORM: Django ORM, SQLAlchemy, Prisma) is essentially a manual normalization step: N:M relationships become junction tables (removing MVDs), and 1:N relationships decompose transitive dependencies.
- **Schema linters** like `squawk`, `pg_qualstats`, and `postgres-checkup` flag violations of normal form heuristics (e.g., repeating column names suggesting a missing junction table).

Our Python tool automates the theory but doesn't handle *semantic* decisions: only a human knows that "AdvisorID → AdvisorName" is a real dependency worth extracting. The tool computes the mechanical decomposition; the designer chooses which FDs to assert.

## Read the Source

- **PostgreSQL source:** `src/backend/commands/tablecmds.c` — the `ALTER TABLE` implementation that enforces constraints encoding FDs
- **SQLite source:** `src/analyze.c` — uses statistical analysis to detect implicit FDs in data (for query optimization)
- **CMU's Bustub:** `src/include/catalog/` — the schema catalog, including constraint enforcement

## Ship It

The reusable artifact is the normalization tool built in `code/main.py`. Save a copy to `outputs/normalization_tool.py` — you can use it in the capstone to verify your KV store's SQL frontend produces normalized schemas.

## Exercises

1. **Easy:** Given R(A, B, C, D) with FDs {A → B, B → C, C → D}, is R in 3NF? BCNF? Decompose to BCNF.

2. **Medium:** R(A, B, C, D, E) with FDs {AB → C, C → D, D → B, D → E}. Find all candidate keys. Decompose to BCNF. Does the decomposition preserve all FDs?

3. **Medium:** Given the 3NF-but-not-BCNF example R(A, B, C) with FDs {AB → C, C → B}, explain why a DBMS might choose 3NF over BCNF for this schema.

4. **Hard:** Extend the Python tool to handle MVDs — compute whether a relation is in 4NF and decompose if not.

5. **Hard:** Write a function that takes a table definition (as a CREATE TABLE statement) and recommends a normalized schema by inferring FDs from constraints (PRIMARY KEY, UNIQUE, FOREIGN KEY).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Functional dependency | A → B means A determines B | If two rows have the same A values, they must have the same B value — a logical constraint on the schema |
| 3NF | "The key, the whole key, and nothing but the key" | Every non-prime attribute depends on every candidate key fully and directly — but FDs with prime RHS are allowed |
| BCNF | Strict 3NF | Every non-trivial FD's LHS must be a superkey — no exceptions |
| Lossless decomposition | Can reconstruct original data via JOIN | The common attributes of the decomposed relations form a superkey in at least one of them |
| Dependency preservation | Can enforce all FDs locally | Every FD can be checked on a single decomposed relation without cross-table constraints |
| Minimal cover | Smallest equivalent FD set | Right-singleton, left-reduced, non-redundant — the canonical form for synthesis |
| Candidate key | Minimal superkey | A set of attributes that uniquely identifies a row, where no proper subset does |
| Anomaly | Weird database behavior | Update: same fact stored many places. Insert: can't add a fact without another. Delete: removing one fact removes another |

## Further Reading

- *Database System Concepts* (Silberschatz, Korth, Sudarshan) — Chapter 7: normalization, the canonical treatment with textbook examples.
- *Foundations of Databases* (Abiteboul, Hull, Vianu) — Chapters 8–11: the formal theory of FDs, MVDs, and join dependencies.
- [FD closure and BCNF visualizer](https://www.dbis.informatik.uni-rostock.de/dbis_tools/) — Interactive tool for experimenting with decomposition.
- [Normalization in PostgreSQL](https://www.postgresql.org/docs/current/ddl-constraints.html) — How constraints map to FDs.
- CMU 15-445: [Normalization lecture](https://www.youtube.com/watch?v=5Ww2Y1GqTBY) — Andy Pavlo's clear example-driven walkthrough.
- *An Introduction to Database Systems* (C. J. Date) — Detailed exploration of normalization theory up to 6NF.
