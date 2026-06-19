from __future__ import annotations
import re
from typing import Callable


class Relation:
    def __init__(self, name: str, schema: list[str], tuples: list[dict]):
        self.name = name
        self.schema = list(schema)
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


def select(rel: Relation, pred: Callable[[dict], bool]) -> Relation:
    return Relation(
        f"\u03c3({rel.name})", rel.schema,
        [t for t in rel.tuples if pred(t)]
    )


def project(rel: Relation, attrs: list[str]) -> Relation:
    return Relation(
        f"\u03c0_{attrs}({rel.name})", attrs,
        [{a: t[a] for a in attrs} for t in rel.tuples]
    )


def cross(rel1: Relation, rel2: Relation) -> Relation:
    right_schema = [
        a if a not in rel1.schema else f"{a}_right"
        for a in rel2.schema
    ]
    schema = rel1.schema + right_schema
    tuples = []
    for t1 in rel1.tuples:
        for t2 in rel2.tuples:
            row = dict(t1)
            for a, orig_a in zip(right_schema, rel2.schema):
                row[a] = t2[orig_a]
            tuples.append(row)
    # Store mapping so join ops can resolve _right names
    cross_rel = Relation(f"({rel1.name} \u00d7 {rel2.name})", schema, tuples)
    cross_rel._right_schema = right_schema
    cross_rel._orig_right_schema = rel2.schema
    return cross_rel


def theta_join(rel1: Relation, rel2: Relation,
               pred: Callable[[dict], bool]) -> Relation:
    return select(cross(rel1, rel2), pred)


def natural_join(rel1: Relation, rel2: Relation) -> Relation:
    common = [a for a in rel1.schema if a in rel2.schema]
    if not common:
        return cross(rel1, rel2)

    cross_rel = cross(rel1, rel2)
    # Build mapping: original right attr → name in cross product
    right_map = dict(zip(cross_rel._orig_right_schema, cross_rel._right_schema))
    def pred(t: dict) -> bool:
        return all(t[a] == t[right_map[a]] for a in common)

    right_only = [a for a in rel2.schema if a not in common]
    joined = select(cross_rel, pred)
    new_tuples = []
    seen = set()
    for t in joined.tuples:
        row = {a: t[a] for a in rel1.schema}
        for a in right_only:
            row[a] = t[right_map[a]]
        key = tuple(row[a] for a in rel1.schema + right_only)
        if key not in seen:
            seen.add(key)
            new_tuples.append(row)
    return Relation(
        f"({rel1.name} \u22c8 {rel2.name})",
        rel1.schema + right_only,
        new_tuples
    )


def union_(rel1: Relation, rel2: Relation) -> Relation:
    assert rel1.schema == rel2.schema, "Schema mismatch for union"
    return Relation(f"({rel1.name} \u222a {rel2.name})", rel1.schema,
                    rel1.tuples + rel2.tuples)


def intersection(rel1: Relation, rel2: Relation) -> Relation:
    assert rel1.schema == rel2.schema, "Schema mismatch for intersection"
    keys2 = set(tuple(t[a] for a in rel1.schema) for t in rel2.tuples)
    return Relation(
        f"({rel1.name} \u2229 {rel2.name})", rel1.schema,
        [t for t in rel1.tuples
         if tuple(t[a] for a in rel1.schema) in keys2]
    )


def difference(rel1: Relation, rel2: Relation) -> Relation:
    assert rel1.schema == rel2.schema, "Schema mismatch for difference"
    keys2 = set(tuple(t[a] for a in rel1.schema) for t in rel2.tuples)
    return Relation(
        f"({rel1.name} \u2212 {rel2.name})", rel1.schema,
        [t for t in rel1.tuples
         if tuple(t[a] for a in rel1.schema) not in keys2]
    )


def rename(rel: Relation, old: str, new: str) -> Relation:
    new_schema = [new if a == old else a for a in rel.schema]
    new_tuples = [{new if k == old else k: v for k, v in t.items()}
                  for t in rel.tuples]
    return Relation(f"\u03c1_{new}\u2190{old}({rel.name})",
                    new_schema, new_tuples)


def eliminate_duplicates(rel: Relation) -> Relation:
    return Relation(f"\u03b4({rel.name})", rel.schema, rel.tuples)


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
            vals = [t.get(agg_name) for t in grp if t.get(agg_name) is not None]
            row[agg_name] = func(vals) if vals else 0
        tuples.append(row)
    return Relation(f"\u03b3({rel.name})", schema, tuples)


def sort_(rel: Relation, by: list[str]) -> Relation:
    sorted_tuples = sorted(rel.tuples, key=lambda t: tuple(t[a] for a in by))
    result = Relation(f"\u03c4({rel.name})", rel.schema, [])
    result.tuples = sorted_tuples
    return result


def parse_and_evaluate(expr: str, relations: dict[str, Relation]) -> Relation:
    expr = expr.strip()
    m = re.match(r'\u03c3_(.+?)\((.+)\)', expr)
    if m:
        cond_str = m.group(1)
        inner = parse_and_evaluate(m.group(2), relations)
        m2 = re.match(r'(\w+)\s*(>|<|>=|<=|=|!=)\s*(.+)', cond_str)
        if not m2:
            raise ValueError(f"Cannot parse condition: {cond_str}")
        attr, op, val = m2.group(1), m2.group(2), m2.group(3).strip()
        try:
            val = int(val)
        except ValueError:
            val = val.strip("'\"")
        def pred(t: dict) -> bool:
            v = t.get(attr)
            if op == '>':
                return v > val
            if op == '<':
                return v < val
            if op == '>=':
                return v >= val
            if op == '<=':
                return v <= val
            if op == '=':
                return v == val
            if op == '!=':
                return v != val
            return False
        return select(inner, pred)
    m = re.match(r'\u03c0_(.+?)\((.+)\)', expr)
    if m:
        attrs = [a.strip() for a in m.group(1).split(',')]
        inner = parse_and_evaluate(m.group(2), relations)
        return project(inner, attrs)
    m = re.match(r'(.+?)\u22c8(.+)', expr)
    if m:
        left = parse_and_evaluate(m.group(1), relations)
        right = parse_and_evaluate(m.group(2), relations)
        return natural_join(left, right)
    m = re.match(r'(.+?)\u222a(.+)', expr)
    if m:
        left = parse_and_evaluate(m.group(1), relations)
        right = parse_and_evaluate(m.group(2), relations)
        return union_(left, right)
    m = re.match(r'(.+?)\u2212(.+)', expr)
    if m:
        left = parse_and_evaluate(m.group(1), relations)
        right = parse_and_evaluate(m.group(2), relations)
        return difference(left, right)
    if expr in relations:
        return relations[expr]
    raise ValueError(f"Cannot parse: {expr}")


def demo_queries():
    users = Relation("Users", ["user_id", "name", "age"], [
        {"user_id": 1, "name": "Alice", "age": 30},
        {"user_id": 2, "name": "Bob", "age": 20},
        {"user_id": 3, "name": "Charlie", "age": 25},
        {"user_id": 4, "name": "Diana", "age": 22},
    ])
    orders = Relation("Orders", ["order_id", "user_id", "amount", "product"], [
        {"order_id": 1, "user_id": 1, "amount": 150, "product": "Laptop"},
        {"order_id": 2, "user_id": 2, "amount": 50, "product": "Mouse"},
        {"order_id": 3, "user_id": 1, "amount": 75, "product": "Keyboard"},
        {"order_id": 4, "user_id": 3, "amount": 200, "product": "Monitor"},
        {"order_id": 5, "user_id": 4, "amount": 80, "product": "USB Hub"},
    ])
    departments = Relation("Departments", ["dept_id", "dept_name"], [
        {"dept_id": 1, "dept_name": "Engineering"},
        {"dept_id": 2, "dept_name": "Sales"},
    ])
    user_dept = Relation("UserDept", ["user_id", "dept_id"], [
        {"user_id": 1, "dept_id": 1},
        {"user_id": 2, "dept_id": 2},
        {"user_id": 3, "dept_id": 1},
        {"user_id": 4, "dept_id": 2},
    ])
    rels = {"Users": users, "Orders": orders,
            "Departments": departments, "UserDept": user_dept}

    print("=== RA Interpreter Demo ===\n")

    # π_name(σ_age>21(Users))
    r = parse_and_evaluate("\u03c0_name(\u03c3_age>21(Users))", rels)
    print("1. Users older than 21:", r.tuples)

    # Theta-join via cross × then select σ:
    #   π_name(σ_amount>100 ∧ Users.user_id=Orders.user_id(Users × Orders))
    cross_uo = cross(users, orders)
    joined = select(
        cross_uo,
        lambda t: t["user_id"] == t["user_id_right"] and t["amount"] > 100
    )
    r = project(joined, ["name"])
    print("2. Users with orders > $100 (\u03c3\u03c0 \u00d7):", r.tuples)

    # Natural join (common attrs = dept_id):
    #   π_name(UserDept ⋈ Departments)
    # UserDept has dept_id → matches Departments.dept_id naturally.
    r = project(natural_join(user_dept, departments), ["dept_name"])
    print("3. Dept names from natural join:", r.tuples)

    # Multi-step: rejoin result with Users
    #   π_name(σ_dept_name=Engineering(Users ⋈_theta UserDept ⋈_nat Departments))
    joined_ud = theta_join(
        users, user_dept,
        lambda t: t["user_id"] == t["user_id_right"]
    )
    joined_all = natural_join(joined_ud, departments)
    r = project(select(joined_all, lambda t: t["dept_name"] == "Engineering"),
                ["name"])
    print("4. Engineering users (theta + natural join):", r.tuples)

    # Union: π_name(Users) ∪ π_name(Extra)
    extra = Relation("Extra", ["name"], [
        {"name": "Alice"}, {"name": "Eve"}
    ])
    r = union_(
        project(users, ["name"]),
        extra
    )
    print("5. Users \u222a Extra (by name):", r.tuples)

    # Difference: σ_age≥21(Users) − σ_name_starts_with_D(Users)
    adult = select(users, lambda t: t["age"] >= 21)
    name_d = select(users, lambda t: t["name"].startswith("D"))
    r = difference(adult, name_d)
    print("6. Adults minus those named D:", r.tuples)

    # Intersection: π_name(Users) ∩ π_name(Extra)
    r = intersection(project(users, ["name"]), extra)
    print("7. Users \u2229 Extra (by name):", r.tuples)

    # Aggregation: γ_{user_id, SUM(amount), COUNT(order_id)} (Orders)
    grouped = aggregate(orders, ["user_id"],
                        {"amount": lambda vs: sum(vs),
                         "order_id": lambda vs: len(vs)})
    print("8. Order totals by user:", grouped.tuples)

    # Rename: ρ_{full_name←name}(Users)
    r = rename(users, "name", "full_name")
    print("9. Renamed schema:", r.schema)

    # Sort: τ_{age}(Users)
    r = sort_(users, ["age"])
    print("10. Sorted by age:", r.tuples)


if __name__ == "__main__":
    demo_queries()
