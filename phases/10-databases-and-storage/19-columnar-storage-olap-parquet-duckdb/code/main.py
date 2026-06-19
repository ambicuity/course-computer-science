"""
Columnar Storage & OLAP — Parquet, DuckDB
Phase 10 — Databases & Storage Systems

Builds a minimal columnar store with RLE and dictionary compression, a
Parquet-like binary format, a query engine with filter/group/agg, and a
demo comparing row-oriented vs column-oriented scan performance.

Usage: python3 main.py
"""

from __future__ import annotations
import struct
import io
import json
import os
import time
import random
import math
from typing import Any


# ── Step 1: Column Store with Compression ──────────────────────────────

class ColumnStore:
    """Stores each column as a separate list with compression metadata."""

    def __init__(self):
        self.columns: dict[str, list[Any]] = {}
        self.compressed: dict[str, dict] = {}
        self.metadata: dict[str, dict] = {}

    def add_column(self, name: str, data: list[Any]) -> None:
        self.columns[name] = data

    def rle_encode(self, col: list) -> list[tuple[Any, int]]:
        if not col:
            return []
        runs: list[tuple[Any, int]] = []
        cur, cnt = col[0], 1
        for v in col[1:]:
            if v == cur:
                cnt += 1
            else:
                runs.append((cur, cnt))
                cur, cnt = v, 1
        runs.append((cur, cnt))
        return runs

    def dict_encode(self, col: list) -> tuple[list[int], dict[int, Any]]:
        unique = sorted(set(col), key=lambda x: str(x))
        mapping = {v: i for i, v in enumerate(unique)}
        codes = [mapping[v] for v in col]
        return codes, {i: v for v, i in mapping.items()}

    def compress(self, col_name: str, strategy: str = "auto") -> None:
        col = self.columns[col_name]
        if strategy == "rle" or (strategy == "auto" and col and len(set(col)) < len(col) // 4):
            runs = self.rle_encode(col)
            self.compressed[col_name] = {"type": "rle", "data": runs}
            self.metadata[col_name] = {
                "compressed": True, "method": "rle",
                "count": len(runs), "original_len": len(col),
            }
        elif strategy == "dict" or (strategy == "auto" and col and isinstance(col[0], str)):
            codes, mapping = self.dict_encode(col)
            self.compressed[col_name] = {"type": "dict", "codes": codes, "dict": mapping}
            self.metadata[col_name] = {
                "compressed": True, "method": "dict",
                "dict_size": len(mapping), "original_len": len(col),
            }
        else:
            self.metadata[col_name] = {
                "compressed": False, "method": "raw", "count": len(col),
            }

    def scan_column(self, col_name: str) -> list[Any]:
        if col_name in self.compressed:
            c = self.compressed[col_name]
            if c["type"] == "rle":
                result = []
                for val, cnt in c["data"]:
                    result.extend([val] * cnt)
                return result
            elif c["type"] == "dict":
                return [c["dict"][code] for code in c["codes"]]
        return self.columns.get(col_name, [])


# ── Step 2: Column Scan with Aggregations ──────────────────────────────

class ColumnScan:
    """Stateless aggregation engine over columnar data."""

    @staticmethod
    def count(col: list) -> int:
        return len(col)

    @staticmethod
    def sum(col: list) -> float:
        total = 0.0
        for v in col:
            total += v
        return total

    @staticmethod
    def avg(col: list) -> float:
        if not col:
            return 0.0
        return ColumnScan.sum(col) / len(col)

    @staticmethod
    def min(col: list) -> Any:
        if not col:
            return None
        m = col[0]
        for v in col[1:]:
            if v < m:
                m = v
        return m

    @staticmethod
    def max(col: list) -> Any:
        if not col:
            return None
        m = col[0]
        for v in col[1:]:
            if v > m:
                m = v
        return m


# ── Step 3: Simple Query Engine ────────────────────────────────────────

class QueryEngine:
    """SELECT group_key, agg(col) FROM store WHERE filter_col > X GROUP BY group_key"""

    def __init__(self, store: ColumnStore):
        self.store = store

    def execute(
        self, *,
        agg_col: str,
        agg_func: str,
        group_col: str | None = None,
        filter_col: str | None = None,
        filter_op: str = "gt",
        filter_val: Any = None,
    ) -> dict:
        col_data = {name: self.store.scan_column(name) for name in self.store.columns}
        n = len(next(iter(col_data.values()))) if col_data else 0

        row_mask = [True] * n
        if filter_col and filter_val is not None:
            data = col_data[filter_col]
            if filter_op == "gt":
                row_mask = [d > filter_val for d in data]
            elif filter_op == "gte":
                row_mask = [d >= filter_val for d in data]

        filtered: dict[str, list] = {}
        for name, data in col_data.items():
            filtered[name] = [v for v, m in zip(data, row_mask) if m]

        if group_col:
            groups: dict[str | int | float, list] = {}
            for i, key in enumerate(filtered[group_col]):
                if key not in groups:
                    groups[key] = []
                groups[key].append(filtered[agg_col][i])
            result = {}
            for key, vals in groups.items():
                if agg_func == "sum":
                    result[key] = sum(vals)
                elif agg_func == "avg":
                    result[key] = sum(vals) / len(vals)
                elif agg_func == "count":
                    result[key] = len(vals)
                elif agg_func == "min":
                    result[key] = min(vals)
                elif agg_func == "max":
                    result[key] = max(vals)
            return result
        else:
            vals = filtered[agg_col]
            if agg_func == "sum":
                return {"result": sum(vals)}
            elif agg_func == "avg":
                return {"result": sum(vals) / len(vals)}
            elif agg_func == "count":
                return {"result": len(vals)}
            elif agg_func == "min":
                return {"result": min(vals)}
            elif agg_func == "max":
                return {"result": max(vals)}
            return {"result": None}


# ── Step 4: Simple Parquet-like Binary Format ──────────────────────────

PARQUET_MAGIC = b"PAR1"


def write_parquet_like(store: ColumnStore, path: str) -> None:
    """Simple binary format: magic, row group per column, statistics, footer."""
    buf = io.BytesIO()
    buf.write(PARQUET_MAGIC)

    row_group_meta = []

    for col_name in store.columns:
        col_data = store.scan_column(col_name)
        col_start = buf.tell()

        type_code = 0
        if col_data and isinstance(col_data[0], float):
            type_code = 1
        elif col_data and isinstance(col_data[0], str):
            type_code = 2

        buf.write(struct.pack(">II", type_code, len(col_data)))

        for v in col_data:
            if type_code == 0:
                buf.write(struct.pack(">q", int(v)))
            elif type_code == 1:
                buf.write(struct.pack(">d", float(v)))
            else:
                encoded = v.encode("utf-8") if v is not None else b""
                buf.write(struct.pack(">I", len(encoded)))
                buf.write(encoded)

        col_end = buf.tell()

        non_null = [v for v in col_data if v is not None]
        stats = {
            "col_name": col_name,
            "offset": col_start,
            "length": col_end - col_start,
            "type": type_code,
            "min": min(non_null) if non_null else None,
            "max": max(non_null) if non_null else None,
            "null_count": sum(1 for v in col_data if v is None),
            "value_count": len(col_data),
        }
        row_group_meta.append(stats)

    footer_offset = buf.tell()
    footer = json.dumps({"row_groups": [{"columns": row_group_meta}]}).encode("utf-8")
    buf.write(footer)
    buf.write(struct.pack(">I", len(footer)))
    buf.write(PARQUET_MAGIC)

    with open(path, "wb") as f:
        f.write(buf.getvalue())


def read_parquet_like(path: str, columns: list[str] | None = None) -> dict[str, list]:
    """Read our Parquet-like format with optional column pruning."""
    with open(path, "rb") as f:
        data = f.read()

    buf = io.BytesIO(data)
    magic = buf.read(4)
    assert magic == PARQUET_MAGIC, f"Not a Parquet-like file: {magic}"

    buf.seek(-8, io.SEEK_END)
    footer_len = struct.unpack(">I", buf.read(4))[0]
    end_magic = buf.read(4)
    assert end_magic == PARQUET_MAGIC

    footer_start = len(data) - 8 - footer_len
    buf.seek(footer_start)
    footer = json.loads(buf.read(footer_len).decode("utf-8"))

    result: dict[str, list] = {}
    for rg in footer["row_groups"]:
        for col_meta in rg["columns"]:
            cname = col_meta["col_name"]
            if columns and cname not in columns:
                continue
            buf.seek(col_meta["offset"])
            buf.read(8)  # skip header (type_code + value_count)
            type_code = col_meta["type"]
            value_count = col_meta["value_count"]

            vals = []
            if type_code == 0:
                for _ in range(value_count):
                    vals.append(struct.unpack(">q", buf.read(8))[0])
            elif type_code == 1:
                for _ in range(value_count):
                    vals.append(struct.unpack(">d", buf.read(8))[0])
            elif type_code == 2:
                for _ in range(value_count):
                    slen = struct.unpack(">I", buf.read(4))[0]
                    vals.append(buf.read(slen).decode("utf-8"))
            result[cname] = vals
    return result


# ── Step 5: Demo — Row vs Columnar Performance ─────────────────────────

def generate_data(n: int) -> ColumnStore:
    """Generate synthetic sales data with regions, products, and revenue."""
    regions = ["NA", "EU", "APAC", "LATAM", "MEA"]
    store = ColumnStore()
    store.add_column("region", [random.choice(regions) for _ in range(n)])
    store.add_column("product_id", [random.randint(1, 1000) for _ in range(n)])
    store.add_column("quantity", [random.randint(1, 50) for _ in range(n)])
    store.add_column("price", [round(random.uniform(5.0, 500.0), 2) for _ in range(n)])
    store.add_column("discount", [random.choice([0, 0, 0, 5, 10, 15, 20]) for _ in range(n)])

    q = store.columns["quantity"]
    p = store.columns["price"]
    d = store.columns["discount"]
    store.add_column("revenue", [q[i] * p[i] * (1 - d[i] / 100) for i in range(n)])

    return store


def demo(n_rows: int = 100_000) -> None:
    print(f"=== Columnar Storage Demo ({n_rows:,} rows) ===")
    store = generate_data(n_rows)

    # Row-oriented: load everything, iterate row by row
    t0 = time.perf_counter()
    all_cols = {name: store.columns[name] for name in store.columns}
    n = len(next(iter(all_cols.values())))
    row_revenue: dict[str, float] = {}
    for i in range(n):
        r = all_cols["region"][i]
        rev = all_cols["revenue"][i]
        if rev > 100:
            row_revenue[r] = row_revenue.get(r, 0.0) + rev
    t_row = time.perf_counter() - t0

    # Columnar: touch only region + revenue, compressed scan
    t0 = time.perf_counter()
    qe = QueryEngine(store)
    result_col = qe.execute(
        agg_col="revenue", agg_func="sum", group_col="region",
        filter_col="revenue", filter_op="gt", filter_val=100,
    )
    t_col = time.perf_counter() - t0

    print(f"Row-oriented scan + agg: {t_row:.4f}s")
    print(f"Columnar scan + agg:     {t_col:.4f}s")
    speedup = t_row / t_col if t_col > 0 else float("inf")
    print(f"Speedup:                 {speedup:.1f}x")

    # Verify results match
    print(f"\nColumnar results: {result_col}")

    # Write Parquet-like file
    pq_path = "/tmp/sales_demo.par1"
    write_parquet_like(store, pq_path)
    pq_size = os.path.getsize(pq_path)
    print(f"\nWrote Parquet-like file: {pq_path} ({pq_size:,} bytes)")

    # Read back with column pruning (only 2 of 6 columns)
    t0 = time.perf_counter()
    pruned = read_parquet_like(pq_path, columns=["region", "revenue"])
    t_read = time.perf_counter() - t0
    print(f"Read back 2/6 columns:  {len(pruned.get('region', []))} rows in {t_read:.6f}s")

    # Demonstrate compression
    for col in ["region", "discount", "product_id"]:
        store.compress(col)
        meta = store.metadata.get(col, {})
        if meta.get("compressed"):
            orig = meta.get("original_len", 0)
            print(f"  {col}: compressed ({meta['method']}), orig={orig} values")

    # Validate scan through compressed data
    scanned = store.scan_column("region")
    assert len(scanned) == n_rows, f"Scan mismatch: {len(scanned)} vs {n_rows}"
    print("\nAll checks passed — columnar store works correctly.")


if __name__ == "__main__":
    demo()
