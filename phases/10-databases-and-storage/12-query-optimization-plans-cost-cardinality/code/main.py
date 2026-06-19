"""
Query Optimization — Plans, Cost, Cardinality
Phase 10 — Databases & Storage Systems

A from-scratch query optimizer demonstrating:
  - Table statistics (row count, page count, histograms, NDV, MCV)
  - Selectivity estimation for predicates using histograms
  - I/O + CPU cost model
  - System R dynamic programming for join ordering
  - TPC-H style 5-table demo
"""

from __future__ import annotations

import itertools
import math
import re
from dataclasses import dataclass, field
from typing import Callable, Optional


# ── Statistics & Histograms ──────────────────────────────────────────────────

@dataclass
class ColumnStats:
    """Per-column statistics used for cardinality estimation."""
    ndv: int                        # number of distinct values
    min_val: float
    max_val: float
    null_frac: float                # fraction of rows that are NULL
    most_common_vals: list[tuple]   # [(value, frequency), ...]
    histogram_bounds: list[float]   # equi-depth histogram boundaries


@dataclass
class TableStats:
    """Per-table statistics."""
    row_count: int
    page_count: int
    columns: dict[str, ColumnStats] = field(default_factory=dict)


# ── Selectivity Estimation ───────────────────────────────────────────────────

def estimate_selectivity_eq(col: ColumnStats, value) -> float:
    """Selectivity of `col = value` using MCV + uniform fallback."""
    for v, freq in col.most_common_vals:
        if v == value:
            return freq
    remaining_frac = 1.0 - sum(f for _, f in col.most_common_vals)
    remaining_ndv = col.ndv - len(col.most_common_vals)
    if remaining_ndv <= 0:
        return 1.0 / col.ndv if col.ndv > 0 else 0.0
    return remaining_frac / remaining_ndv


def estimate_selectivity_range(col: ColumnStats, op: str, value: float) -> float:
    """Selectivity of `col > value` or `col < value` using histogram."""
    bounds = col.histogram_bounds
    if len(bounds) < 2:
        return 0.5  # fallback

    if value <= bounds[0]:
        return 1.0 if op == ">" else 0.0
    if value >= bounds[-1]:
        return 0.0 if op == ">" else 1.0

    # Find the bucket that contains the value
    bucket_idx = 0
    for i in range(len(bounds) - 1):
        if bounds[i] <= value <= bounds[i + 1]:
            bucket_idx = i
            break
    else:
        bucket_idx = len(bounds) - 2

    b_low = bounds[bucket_idx]
    b_high = bounds[bucket_idx + 1]
    b_width = b_high - b_low
    bucket_frac = 1.0 / (len(bounds) - 1)

    if b_width == 0:
        pos_frac = 0.5
    else:
        pos_frac = (value - b_low) / b_width

    if op == "<":
        return (bucket_idx * bucket_frac) + (pos_frac * bucket_frac)
    else:  # ">"
        return ((len(bounds) - 2 - bucket_idx) * bucket_frac) + ((1.0 - pos_frac) * bucket_frac)


def estimate_selectivity_in(col: ColumnStats, values: list) -> float:
    """Selectivity of `col IN (vals)` — sum of individual equalities."""
    total = 0.0
    for v in values:
        total += estimate_selectivity_eq(col, v)
    return min(total, 1.0)


def estimate_selectivity_like(col: ColumnStats, pattern: str) -> float:
    """Selectivity of `col LIKE pattern` — heuristic based on prefix."""
    prefix = re.match(r"^([^%_]+)", pattern)
    if prefix:
        return 0.01  # selective prefix match
    return 0.05  # wildcard-heavy


def estimate_selectivity_between(col: ColumnStats, low: float, high: float) -> float:
    """Selectivity of `col BETWEEN low AND high`."""
    sel_high = estimate_selectivity_range(col, "<", high)
    sel_low = estimate_selectivity_range(col, "<", low)
    return sel_high - sel_low


# ── SelectivityEstimator ─────────────────────────────────────────────────────

class SelectivityEstimator:
    """Combines statistics and predicate analysis to estimate selectivity."""

    def __init__(self, stats: dict[str, TableStats]):
        self.stats = stats

    def selectivity(self, table: str, predicate: tuple) -> float:
        """Estimate selectivity for a predicate on a table.
        Predicate format: (column, op, value) where op is '=', '>', '<', 'IN', 'LIKE', 'BETWEEN'.
        For BETWEEN, value is (low, high).
        """
        col_stats = self.stats[table].columns.get(predicate[0])
        if col_stats is None:
            return 0.1  # default fallback

        op = predicate[1]
        value = predicate[2]

        if op == "=":
            return estimate_selectivity_eq(col_stats, value) * (1.0 - col_stats.null_frac)
        elif op == ">":
            return estimate_selectivity_range(col_stats, ">", value) * (1.0 - col_stats.null_frac)
        elif op == "<":
            return estimate_selectivity_range(col_stats, "<", value) * (1.0 - col_stats.null_frac)
        elif op == "IN":
            return estimate_selectivity_in(col_stats, value) * (1.0 - col_stats.null_frac)
        elif op == "LIKE":
            return estimate_selectivity_like(col_stats, value) * (1.0 - col_stats.null_frac)
        elif op == "BETWEEN":
            return estimate_selectivity_between(col_stats, value[0], value[1]) * (1.0 - col_stats.null_frac)
        return 0.1

    def cardinality(self, table: str, predicate: Optional[tuple] = None) -> int:
        """Estimated row count after applying predicate."""
        rows = self.stats[table].row_count
        if predicate:
            rows = max(1, int(rows * self.selectivity(table, predicate)))
        return rows


# ── Cost Model ───────────────────────────────────────────────────────────────

@dataclass
class Cost:
    total: float = 0.0

    def __add__(self, other: Cost) -> Cost:
        return Cost(self.total + other.total)

    def __radd__(self, other) -> Cost:
        if isinstance(other, Cost):
            return Cost(self.total + other.total)
        return self

    def __repr__(self) -> str:
        return f"{self.total:.1f}"


class CostModel:
    """Simple cost model: I/O + CPU.

    I/O cost: seq_page_cost * seq_page_reads + random_page_cost * random_page_reads.
    CPU cost: cpu_tuple_cost * tuples_processed + cpu_operator_cost * operator_invocations.
    """
    seq_page_cost: float = 1.0
    random_page_cost: float = 4.0
    cpu_tuple_cost: float = 0.01
    cpu_operator_cost: float = 0.0025

    def scan_cost(self, page_count: int) -> Cost:
        seq_reads = page_count
        cpu_tuples = page_count * 100  # assume ~100 tuples per page
        total = (self.seq_page_cost * seq_reads
                 + self.cpu_tuple_cost * cpu_tuples)
        return Cost(total)

    def filter_cost(self, input_card: int) -> Cost:
        total = self.cpu_operator_cost * input_card
        return Cost(total)

    def join_cost(self, outer_card: int, inner_card: int,
                  outer_pages: int, inner_pages: int,
                  join_type: str = "hash") -> Cost:
        if join_type == "nested_loop":
            random_reads = outer_card * inner_pages
            cpu_cost = outer_card * inner_card
            total = (self.random_page_cost * random_reads
                     + self.cpu_tuple_cost * cpu_cost)
        elif join_type == "hash":
            seq_reads = outer_pages + inner_pages
            cpu_cost = outer_card + inner_card
            total = (self.seq_page_cost * seq_reads
                     + self.cpu_tuple_cost * cpu_cost
                     + self.cpu_operator_cost * cpu_cost)
        else:  # merge join
            seq_reads = outer_pages + inner_pages
            cpu_cost = outer_card + inner_card
            total = (self.seq_page_cost * seq_reads
                     + self.cpu_tuple_cost * cpu_cost)
        return Cost(total)

    def sort_cost(self, input_card: int) -> Cost:
        comparisons = input_card * int(math.log2(max(input_card, 2)))
        total = self.cpu_operator_cost * comparisons
        return Cost(total)

    def projection_cost(self, input_card: int) -> Cost:
        total = self.cpu_operator_cost * input_card
        return Cost(total)


# ── Plan Nodes ───────────────────────────────────────────────────────────────

@dataclass
class PlanNode:
    cardinality: int
    cost: Cost

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}(card={self.cardinality}, cost={self.cost})"


@dataclass
class ScanNode(PlanNode):
    table: str = ""

    def __repr__(self) -> str:
        return f"Scan({self.table})"


@dataclass
class FilterNode(PlanNode):
    predicate: tuple = ()
    child: Optional[PlanNode] = None

    def __repr__(self) -> str:
        return f"Filter[{self.predicate}]({self.child})"


@dataclass
class JoinNode(PlanNode):
    join_type: str = "hash"
    left: Optional[PlanNode] = None
    right: Optional[PlanNode] = None

    def __repr__(self) -> str:
        return f"{self.join_type.upper()}Join({self.left}, {self.right})"


@dataclass
class ProjectionNode(PlanNode):
    columns: list[str] = field(default_factory=list)
    child: Optional[PlanNode] = None

    def __repr__(self) -> str:
        return f"Project({self.columns}, {self.child})"


@dataclass
class SortNode(PlanNode):
    key: str = ""
    child: Optional[PlanNode] = None

    def __repr__(self) -> str:
        return f"Sort({self.key}, {self.child})"


# ── Join Enumerator (System R DP) ────────────────────────────────────────────

@dataclass
class JoinTree:
    """A join tree (or single table) with its properties for DP."""
    tables: frozenset[str]
    cardinality: int
    cost: Cost
    plan: PlanNode
    # For bushy trees, the left/right partition
    left: Optional["JoinTree"] = None
    right: Optional["JoinTree"] = None


class JoinEnumerator:
    """System R-style dynamic programming join enumerator.

    Supports left-deep trees by default; also demonstrates bushy tree
    enumeration by considering all non-empty proper subsets.
    """

    def __init__(self, tables: list[str], join_predicates: list[tuple],
                 estimator: SelectivityEstimator, cost_model: CostModel,
                 base_cardinalities: dict[str, int], base_pages: dict[str, int],
                 bushy: bool = False):
        self.tables = tables
        self.join_predicates = join_predicates
        self.estimator = estimator
        self.cost_model = cost_model
        self.base_cardinalities = base_cardinalities
        self.base_pages = base_pages
        self.bushy = bushy

        # DP table: frozenset[str] -> best JoinTree
        self.best: dict[frozenset[str], JoinTree] = {}

    def _build_scan(self, table: str) -> JoinTree:
        pages = self.base_pages[table]
        card = self.base_cardinalities[table]
        cost = self.cost_model.scan_cost(pages)
        plan = ScanNode(cardinality=card, cost=cost, table=table)
        return JoinTree(
            tables=frozenset([table]),
            cardinality=card,
            cost=cost,
            plan=plan
        )

    def _get_join_sel(self, t1: str, t2: str) -> float:
        for pred in self.join_predicates:
            # pred format: (left_table, left_col, right_table, right_col)
            if {pred[0], pred[2]} == {t1, t2}:
                # join selectivity: assume foreign-key / primary-key style
                # cardinality of join ≈ card(t1) * card(t2) / max(ndv(left), ndv(right))
                col1 = self.estimator.stats[pred[0]].columns.get(pred[1])
                col2 = self.estimator.stats[pred[2]].columns.get(pred[3])
                ndv = max((col1.ndv if col1 else 1), (col2.ndv if col2 else 1))
                return 1.0 / ndv
        return 0.1  # Cartesian product fallback

    def enumerate(self) -> Optional[JoinTree]:
        # Step 1: seed with single-table accesses
        for t in self.tables:
            self.best[frozenset([t])] = self._build_scan(t)

        # Step 2: DP over increasing subset sizes
        for size in range(2, len(self.tables) + 1):
            for subset in itertools.combinations(self.tables, size):
                subset_set = frozenset(subset)
                best_tree = self._find_best_join(subset_set)
                if best_tree:
                    self.best[subset_set] = best_tree

        all_tables = frozenset(self.tables)
        return self.best.get(all_tables)

    def _find_best_join(self, subset: frozenset[str]) -> Optional[JoinTree]:
        best_tree: Optional[JoinTree] = None

        # Consider all ways to split subset into two non-empty parts
        for left_size in range(1, len(subset)):
            for left_subset in itertools.combinations(subset, left_size):
                left_set = frozenset(left_subset)
                right_set = subset - left_set

                if left_set not in self.best or right_set not in self.best:
                    continue

                # For left-deep trees: right side must be a single table
                if not self.bushy and len(right_set) > 1:
                    continue

                left_tree = self.best[left_set]
                right_tree = self.best[right_set]

                # Determine which tables are in the left/right of the join
                left_tables = list(left_set)
                right_tables = list(right_set)

                # Estimate join cardinality
                # For each cross-table pair, apply join selectivity
                card = left_tree.cardinality * right_tree.cardinality
                for lt in left_tables:
                    for rt in right_tables:
                        card *= self._get_join_sel(lt, rt)

                card = max(1, int(card))

                # Estimate join cost
                left_pages = sum(self.base_pages[t] for t in left_tables)
                right_pages = sum(self.base_pages[t] for t in right_tables)

                # Choose join type based on sizes
                if card < 10000:
                    join_type = "hash"
                elif left_tree.cardinality < right_tree.cardinality:
                    join_type = "nested_loop"
                else:
                    join_type = "hash"

                cost = (left_tree.cost
                        + right_tree.cost
                        + self.cost_model.join_cost(
                            left_tree.cardinality, right_tree.cardinality,
                            left_pages, right_pages, join_type))

                # Build plan tree
                plan = JoinNode(
                    cardinality=card, cost=cost,
                    join_type=join_type,
                    left=left_tree.plan, right=right_tree.plan
                )

                tree = JoinTree(
                    tables=subset,
                    cardinality=card,
                    cost=cost,
                    plan=plan,
                    left=left_tree,
                    right=right_tree
                )

                if best_tree is None or cost.total < best_tree.cost.total:
                    best_tree = tree

        return best_tree


# ── Helper to build predicate selectivity ────────────────────────────────────

def apply_predicates(table: str, predicates: list[tuple],
                     estimator: SelectivityEstimator,
                     base_card: int) -> tuple[int, Cost]:
    """Apply a chain of predicates (AND) and return (cardinality, filter_cost)."""
    card = float(base_card)
    total_cost = Cost(0)
    for pred in predicates:
        sel = estimator.selectivity(table, pred)
        card *= sel
        total_cost += CostModel().filter_cost(int(card))
    return max(1, int(card)), total_cost


# ── Plan Printer ─────────────────────────────────────────────────────────────

def print_plan(node: PlanNode, indent: str = "") -> None:
    """Pretty-print a plan tree."""
    if isinstance(node, ScanNode):
        print(f"{indent}Seq Scan on {node.table}  (cost={node.cost}, rows={node.cardinality})")
    elif isinstance(node, FilterNode):
        print(f"{indent}Filter: {node.predicate}  (cost={node.cost}, rows={node.cardinality})")
        if node.child:
            print_plan(node.child, indent + "  ")
    elif isinstance(node, JoinNode):
        print(f"{indent}{node.join_type.upper()} Join  (cost={node.cost}, rows={node.cardinality})")
        if node.left:
            print_plan(node.left, indent + "  ")
        if node.right:
            print_plan(node.right, indent + "  ")
    elif isinstance(node, ProjectionNode):
        cols = ", ".join(node.columns)
        print(f"{indent}Project: {cols}  (cost={node.cost}, rows={node.cardinality})")
        if node.child:
            print_plan(node.child, indent + "  ")
    elif isinstance(node, SortNode):
        print(f"{indent}Sort: {node.key}  (cost={node.cost}, rows={node.cardinality})")
        if node.child:
            print_plan(node.child, indent + "  ")


# ── Demo: TPC-H Style Query ──────────────────────────────────────────────────

def build_tpch_stats() -> dict[str, TableStats]:
    """Build statistics for a TPC-H style 5-table schema.

    Tables: customer, orders, lineitem, nation, supplier
    Query: list all customers in a given nation with orders > threshold.
    """
    return {
        "customer": TableStats(
            row_count=150000,
            page_count=1500,
            columns={
                "c_custkey": ColumnStats(
                    ndv=150000, min_val=1, max_val=150000,
                    null_frac=0.0,
                    most_common_vals=[],
                    histogram_bounds=list(range(0, 150001, 10000))),
                "c_nationkey": ColumnStats(
                    ndv=25, min_val=0, max_val=24,
                    null_frac=0.0,
                    most_common_vals=[(0, 0.04), (1, 0.04), (2, 0.04)],
                    histogram_bounds=list(range(0, 25, 2))),
                "c_name": ColumnStats(
                    ndv=150000, min_val=0, max_val=150000,
                    null_frac=0.0,
                    most_common_vals=[],
                    histogram_bounds=[0, 50000, 100000, 150000]),
            }
        ),
        "orders": TableStats(
            row_count=1500000,
            page_count=15000,
            columns={
                "o_orderkey": ColumnStats(
                    ndv=1500000, min_val=1, max_val=1500000,
                    null_frac=0.0,
                    most_common_vals=[],
                    histogram_bounds=list(range(0, 1500001, 100000))),
                "o_custkey": ColumnStats(
                    ndv=150000, min_val=1, max_val=150000,
                    null_frac=0.0,
                    most_common_vals=[],
                    histogram_bounds=list(range(0, 150001, 10000))),
                "o_totalprice": ColumnStats(
                    ndv=500000, min_val=1000, max_val=500000,
                    null_frac=0.0,
                    most_common_vals=[],
                    histogram_bounds=list(range(1000, 500001, 50000))),
                "o_orderstatus": ColumnStats(
                    ndv=3, min_val=0, max_val=2,
                    null_frac=0.0,
                    most_common_vals=[("F", 0.5), ("O", 0.3), ("P", 0.2)],
                    histogram_bounds=[]),
            }
        ),
        "lineitem": TableStats(
            row_count=6000000,
            page_count=60000,
            columns={
                "l_orderkey": ColumnStats(
                    ndv=1500000, min_val=1, max_val=1500000,
                    null_frac=0.0,
                    most_common_vals=[],
                    histogram_bounds=list(range(0, 1500001, 100000))),
                "l_quantity": ColumnStats(
                    ndv=50, min_val=1, max_val=50,
                    null_frac=0.0,
                    most_common_vals=[],
                    histogram_bounds=list(range(0, 51, 5))),
                "l_shipdate": ColumnStats(
                    ndv=2500, min_val=7305, max_val=10957,
                    null_frac=0.0,
                    most_common_vals=[],
                    histogram_bounds=list(range(7305, 10958, 300))),
            }
        ),
        "nation": TableStats(
            row_count=25,
            page_count=1,
            columns={
                "n_nationkey": ColumnStats(
                    ndv=25, min_val=0, max_val=24,
                    null_frac=0.0,
                    most_common_vals=[],
                    histogram_bounds=list(range(0, 25, 2))),
                "n_name": ColumnStats(
                    ndv=25, min_val=0, max_val=24,
                    null_frac=0.0,
                    most_common_vals=[("BRAZIL", 0.04), ("USA", 0.04), ("CANADA", 0.04)],
                    histogram_bounds=[]),
            }
        ),
        "supplier": TableStats(
            row_count=10000,
            page_count=100,
            columns={
                "s_suppkey": ColumnStats(
                    ndv=10000, min_val=1, max_val=10000,
                    null_frac=0.0,
                    most_common_vals=[],
                    histogram_bounds=list(range(0, 10001, 1000))),
                "s_nationkey": ColumnStats(
                    ndv=25, min_val=0, max_val=24,
                    null_frac=0.0,
                    most_common_vals=[(0, 0.04), (1, 0.04), (2, 0.04)],
                    histogram_bounds=list(range(0, 25, 2))),
            }
        ),
    }


def run_demo() -> None:
    """Run a TPC-H style query optimization demo.

    Query: list customer names and order totals for customers in 'BRAZIL'
           with orders over $100,000, shipped before 1995-01-01.
    """
    print("=" * 65)
    print("QUERY OPTIMIZATION DEMO — TPC-H Style (5 tables)")
    print("=" * 65)

    stats = build_tpch_stats()
    estimator = SelectivityEstimator(stats)
    cost_model = CostModel()

    # Tables involved
    tables = ["customer", "orders", "lineitem", "nation", "supplier"]

    # Base cardinalities and page counts
    base_cards = {t: stats[t].row_count for t in tables}
    base_pages = {t: stats[t].page_count for t in tables}

    # Join predicates (foreign-key relationships)
    join_preds = [
        ("customer", "c_custkey", "orders", "o_custkey"),
        ("orders", "o_orderkey", "lineitem", "l_orderkey"),
        ("customer", "c_nationkey", "nation", "n_nationkey"),
        ("supplier", "s_nationkey", "nation", "n_nationkey"),
    ]

    print("\n── Table Statistics ──")
    for t in tables:
        s = stats[t]
        print(f"  {t}: {s.row_count:>8} rows, {s.page_count:>6} pages")

    print(f"\n── Join Predicates ──")
    for lp, lc, rp, rc in join_preds:
        print(f"  {lp}.{lc} = {rp}.{rc}")

    print(f"\n── Predicate Selectivity Estimates ──")
    # Query predicates
    filters = [
        ("nation", ("n_name", "=", "BRAZIL")),
        ("orders", ("o_totalprice", ">", 100000)),
        ("lineitem", ("l_shipdate", "<", 9497)),  # 1995-01-01 as Julian-ish
    ]
    for table, pred in filters:
        sel = estimator.selectivity(table, pred)
        card = estimator.cardinality(table, pred)
        print(f"  {table}: {pred[0]} {pred[1]} {pred[2]}  → sel={sel:.4f}, card={card}")

    print("\n── Enumerating Join Orders (System R DP) ──")

    # Left-deep tree enumeration
    enumerator = JoinEnumerator(
        tables, join_preds, estimator, cost_model,
        base_cards, base_pages, bushy=False
    )
    result = enumerator.enumerate()

    print("\n  Optimal Left-Deep Plan:")
    if result:
        print_plan(result.plan)
        print(f"\n  Total cost: {result.cost}")
        print(f"  Output cardinality: {result.cardinality}")

    # Bushy tree enumeration
    enumerator_bushy = JoinEnumerator(
        tables, join_preds, estimator, cost_model,
        base_cards, base_pages, bushy=True
    )
    result_bushy = enumerator_bushy.enumerate()

    print("\n  Optimal Bushy Plan (if different):")
    if result_bushy:
        print_plan(result_bushy.plan)
        print(f"  Total cost: {result_bushy.cost}")
        print(f"  Output cardinality: {result_bushy.cardinality}")

    # Compare sub-optimal alternatives
    print("\n── Sub-optimal Plan Comparison ──")
    alt_plans = [
        ["customer", "orders", "lineitem", "nation", "supplier"],
        ["nation", "customer", "orders", "lineitem", "supplier"],
        ["supplier", "nation", "customer", "orders", "lineitem"],
    ]
    for i, plan_order in enumerate(alt_plans):
        enc = JoinEnumerator(
            plan_order, join_preds, estimator, cost_model,
            base_cards, base_pages, bushy=False
        )
        r = enc.enumerate()
        if r:
            print(f"  Order {i+1} {plan_order}: cost={r.cost}")

    print("\n── What If: Bad Cardinality Estimate ──")
    # Simulate what happens when NDV is grossly underestimated on a join column
    wrong_stats = build_tpch_stats()
    wrong_stats["customer"].columns["c_custkey"].ndv = 5    # true: 150000
    wrong_stats["orders"].columns["o_custkey"].ndv = 5      # true: 150000
    wrong_estimator = SelectivityEstimator(wrong_stats)
    wrong_enc = JoinEnumerator(
        tables, join_preds, wrong_estimator, cost_model,
        base_cards, base_pages, bushy=False
    )
    wrong_r = wrong_enc.enumerate()
    if wrong_r:
        est_card = wrong_r.cardinality
        true_card = result.cardinality if result else 0
        print(f"  NDV underestimation: c_custkey=5, o_custkey=5 (true NDV=150k)")
        print(f"  Estimated output cardinality : {est_card:>10,}")
        print(f"  True (correct stats) cardinality: {true_card:>10,}")
        print(f"  Ratio off by: {est_card / max(true_card, 1):,.0f}x")
        # Also show what plan the bad stats produce vs the good stats
        print(f"\n  Plan with WRONG stats (NDV=5):")
        print_plan(wrong_r.plan)
        print(f"  => Bad cardinality estimates lead to wrong join order selection!")

    print("\n" + "=" * 65)
    print("DEMO COMPLETE")
    print("=" * 65)


def main() -> None:
    run_demo()


if __name__ == "__main__":
    main()
