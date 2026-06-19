"""Query Execution — Iterator vs Vectorized: Volcano-style iterator model executor."""

from typing import Any, Callable
import heapq


class Row(dict):
    pass


class Operator:
    def open(self):
        pass

    def next(self) -> Row | None:
        raise RuntimeError("abstract method")

    def close(self):
        pass


class SeqScan(Operator):
    def __init__(self, table: list[Row]):
        self.table = table
        self.idx = 0

    def open(self):
        self.idx = 0

    def next(self) -> Row | None:
        if self.idx >= len(self.table):
            return None
        row = self.table[self.idx]
        self.idx += 1
        return row

    def close(self):
        self.idx = 0


class Filter(Operator):
    def __init__(self, child: Operator, predicate: Callable[[Row], bool]):
        self.child = child
        self.predicate = predicate

    def open(self):
        self.child.open()

    def next(self) -> Row | None:
        while True:
            row = self.child.next()
            if row is None:
                return None
            if self.predicate(row):
                return row

    def close(self):
        self.child.close()


class Projection(Operator):
    def __init__(self, child: Operator, columns: list[str]):
        self.child = child
        self.columns = columns

    def open(self):
        self.child.open()

    def next(self) -> Row | None:
        row = self.child.next()
        if row is None:
            return None
        return Row({col: row[col] for col in self.columns})

    def close(self):
        self.child.close()


class NestedLoopJoin(Operator):
    def __init__(self, left: Operator, right: Operator,
                 condition: Callable[[Row, Row], bool]):
        self.left = left
        self.right = right
        self.condition = condition
        self.outer_row: Row | None = None
        self.right_rows: list[Row] = []

    def open(self):
        self.left.open()
        self.right.open()
        self.right_rows = []
        while True:
            row = self.right.next()
            if row is None:
                break
            self.right_rows.append(row)
        self.right.close()
        self.outer_row = None

    def next(self) -> Row | None:
        while True:
            if self.outer_row is None:
                self.outer_row = self.left.next()
                if self.outer_row is None:
                    return None
                self.right_idx = 0
            while self.right_idx < len(self.right_rows):
                rrow = self.right_rows[self.right_idx]
                self.right_idx += 1
                if self.condition(self.outer_row, rrow):
                    merged = Row(**self.outer_row, **rrow)
                    return merged
            self.outer_row = None

    def close(self):
        self.left.close()


class HashJoin(Operator):
    def __init__(self, left: Operator, right: Operator,
                 left_key: str, right_key: str):
        self.left = left
        self.right = right
        self.left_key = left_key
        self.right_key = right_key
        self.hash_table: dict[Any, list[Row]] = {}
        self.probe_rows: list[Row] = []
        self.probe_idx = 0

    def open(self):
        self.left.open()
        self.right.open()
        self.hash_table = {}
        while True:
            row = self.right.next()
            if row is None:
                break
            k = row[self.right_key]
            self.hash_table.setdefault(k, []).append(row)
        self.right.close()
        self.probe_rows = []
        self.probe_idx = 0

    def next(self) -> Row | None:
        while True:
            if not self.probe_rows:
                outer = self.left.next()
                if outer is None:
                    return None
                matched = self.hash_table.get(outer[self.left_key], [])
                self.probe_rows = [Row(**outer, **r) for r in matched]
                self.probe_idx = 0
            if self.probe_idx < len(self.probe_rows):
                row = self.probe_rows[self.probe_idx]
                self.probe_idx += 1
                return row
            self.probe_rows = []

    def close(self):
        self.left.close()


class Sort(Operator):
    def __init__(self, child: Operator, key: str, reverse: bool = False):
        self.child = child
        self.key = key
        self.reverse = reverse
        self.rows: list[Row] = []
        self.idx = 0

    def open(self):
        self.child.open()
        self.rows = []
        while True:
            row = self.child.next()
            if row is None:
                break
            self.rows.append(row)
        self.rows.sort(key=lambda r: r.get(self.key, ""), reverse=self.reverse)
        self.child.close()
        self.idx = 0

    def next(self) -> Row | None:
        if self.idx >= len(self.rows):
            return None
        row = self.rows[self.idx]
        self.idx += 1
        return row

    def close(self):
        self.rows = []
        self.idx = 0


class Aggregate(Operator):
    def __init__(self, child: Operator, group_by: list[str] | None,
                 agg_col: str, agg_func: str):
        self.child = child
        self.group_by = group_by
        self.agg_col = agg_col
        self.agg_func = agg_func
        self.results: list[Row] = []
        self.idx = 0

    def open(self):
        self.child.open()
        groups: dict[tuple, list[Row]] = {}
        while True:
            row = self.child.next()
            if row is None:
                break
            key = tuple(row.get(c) for c in self.group_by) if self.group_by else ()
            groups.setdefault(key, []).append(row)
        self.child.close()

        self.results = []
        for key, rows in groups.items():
            vals = [r.get(self.agg_col, 0) or 0 for r in rows]
            if self.agg_func == "count":
                result = len(vals)
            elif self.agg_func == "sum":
                result = sum(vals)
            elif self.agg_func == "avg":
                result = sum(vals) / len(vals) if vals else 0
            else:
                result = 0
            out = Row()
            if self.group_by:
                for i, c in enumerate(self.group_by):
                    out[c] = key[i]
            out[f"{self.agg_func}_{self.agg_col}"] = result
            self.results.append(out)
        self.idx = 0

    def next(self) -> Row | None:
        if self.idx >= len(self.results):
            return None
        row = self.results[self.idx]
        self.idx += 1
        return row

    def close(self):
        self.results = []
        self.idx = 0


class QueryBuilder:
    @staticmethod
    def build(plan: dict, tables: dict[str, list[Row]]) -> Operator:
        op = None
        for node_type, params in plan.items():
            if node_type == "SeqScan":
                op = SeqScan(tables[params["table"]])
            elif node_type == "Filter":
                op = Filter(op, params["predicate"])
            elif node_type == "Projection":
                op = Projection(op, params["columns"])
            elif node_type == "Sort":
                op = Sort(op, params["key"], params.get("reverse", False))
            elif node_type == "Aggregate":
                op = Aggregate(op, params.get("group_by"),
                               params["agg_col"], params["agg_func"])
            else:
                raise ValueError(f"Unknown operator: {node_type}")
        return op


def execute_and_print(op: Operator, label: str = ""):
    if label:
        print(label)
    op.open()
    count = 0
    while True:
        row = op.next()
        if row is None:
            break
        print(dict(row))
        count += 1
    op.close()
    print(f"({count} rows)\n")


def main():
    users = [
        Row(id=1, name="Alice", age=30, city="NYC"),
        Row(id=2, name="Bob", age=18, city="LA"),
        Row(id=3, name="Charlie", age=25, city="NYC"),
        Row(id=4, name="Diana", age=35, city="Chicago"),
        Row(id=5, name="Eve", age=22, city="LA"),
        Row(id=6, name="Frank", age=40, city="NYC"),
        Row(id=7, name="Grace", age=19, city="Chicago"),
        Row(id=8, name="Henry", age=28, city="LA"),
    ]

    orders = [
        Row(user_id=1, product="Laptop", amount=1200),
        Row(user_id=1, product="Mouse", amount=25),
        Row(user_id=3, product="Keyboard", amount=80),
        Row(user_id=4, product="Monitor", amount=350),
        Row(user_id=6, product="Desk", amount=450),
        Row(user_id=6, product="Chair", amount=600),
    ]

    tables = {"users": users, "orders": orders}

    execute_and_print(
        QueryBuilder.build({
            "SeqScan": {"table": "users"},
            "Filter": {"predicate": lambda r: r["age"] > 21},
            "Projection": {"columns": ["name", "age"]},
            "Sort": {"key": "name"},
        }, tables),
        "=== Query 1: SELECT name, age FROM users WHERE age > 21 ORDER BY name ===",
    )

    nl_join = NestedLoopJoin(
        SeqScan(users), SeqScan(orders),
        condition=lambda l, r: l["id"] == r["user_id"],
    )
    execute_and_print(
        nl_join,
        "=== Query 2: NestedLoopJoin: users ⋈ orders ===",
    )

    h_join = HashJoin(SeqScan(users), SeqScan(orders),
                      left_key="id", right_key="user_id")
    execute_and_print(
        h_join,
        "=== Query 3: HashJoin: users ⋈ orders ===",
    )

    execute_and_print(
        QueryBuilder.build({
            "SeqScan": {"table": "users"},
            "Aggregate": {"group_by": ["city"], "agg_col": "id", "agg_func": "count"},
        }, tables),
        "=== Query 4: SELECT city, count(*) FROM users GROUP BY city ===",
    )

    join_for_agg = HashJoin(SeqScan(users), SeqScan(orders),
                            left_key="id", right_key="user_id")
    agg = Aggregate(join_for_agg, group_by=["name"],
                    agg_col="amount", agg_func="avg")
    execute_and_print(
        agg,
        "=== Query 5: HashJoin + Aggregate: avg amount per user ===",
    )


if __name__ == "__main__":
    main()
