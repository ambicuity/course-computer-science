# Columnar Storage & OLAP — Parquet, DuckDB

> OLTP asks "where's user 42's row?" — OLAP asks "what's the average revenue per region over 5 years?"

**Type:** Build + Learn
**Languages:** Python, SQL
**Prerequisites:** Phase 10 lessons 10 (vectorized execution), 05 (page storage), 07 (indexing)
**Time:** ~75 minutes

## Learning Objectives

- Differentiate OLTP (row-oriented, point queries, many small transactions) from OLAP (column-oriented, full-scan aggregation, large scans)
- Implement columnar compression techniques: RLE, dictionary encoding, delta encoding
- Explain the Parquet file format hierarchy: row group → column chunk → page
- Run OLAP queries with DuckDB and compare its vectorized execution against SQLite's row-at-a-time engine
- Build a minimal columnar store with scans, filters, and aggregations

## The Problem

You have 50 million sales records. You need: "total revenue by region for Q4." With a traditional row-oriented database (PostgreSQL, SQLite), the engine loads *every column* of *every row* just to touch `amount` and `region`. Most of that data is irrelevant — you're paying the full I/O and cache-miss cost for columns you'll never read.

Worse: row stores compress per-row — mixed types (int, text, date) in the same page mean poor compression ratios. A 1 TB fact table might compress to 100 GB in a columnar format but stay near 1 TB row-oriented.

Analytical workloads (aggregation, grouped statistics, window functions) are fundamentally different from transactional workloads (point lookups, small updates). Using the same storage engine for both means neither is optimal. Columnar storage exists to serve OLAP efficiently.

## The Concept

### Row-Oriented vs Column-Oriented

```
Row-oriented (OLTP — SQLite, PostgreSQL):
┌─────────┬──────────┬──────────┬──────────┐
│  id=1   │ name=Al  │ age=30   │ city=NYC │
├─────────┼──────────┼──────────┼──────────┤
│  id=2   │ name=Bo  │ age=25   │ city=LA  │
├─────────┼──────────┼──────────┼──────────┤
│  id=3   │ name=Ci  │ age=35   │ city=SF  │
└─────────┴──────────┴──────────┴──────────┘
Disk layout: [id1][name1][age1][city1][id2][name2][age2][city2]...

Column-oriented (OLAP — Parquet, DuckDB):
┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
│  id_col  │ │ name_col │ │ age_col  │ │ city_col │
│    1     │ │    Al    │ │   30     │ │   NYC    │
│    2     │ │    Bo    │ │   25     │ │   LA     │
│    3     │ │    Ci    │ │   35     │ │   SF     │
└──────────┘ └──────────┘ └──────────┘ └──────────┘
Disk layout: [id1][id2][id3]...[name1][name2][name3]...
```

When a query reads only `age` and `city`, the column store reads 2 contiguous byte ranges instead of skipping through every row. That means:
- **Less I/O**: only touched columns are loaded
- **Better cache utilization**: sequential scan of homogeneous data
- **SIMD-friendly**: same-type arrays vectorize naturally

### Columnar Compression

Same-type values per column unlock specialized compression:

| Technique | When it shines | Example |
|-----------|---------------|---------|
| **RLE** | Repeated values — low-cardinality enum | `region: [EU,EU,EU,NA,NA,APAC]` → `[(EU,3),(NA,2),(APAC,1)]` |
| **Bit-packing** | Small-range integers | `age: [24,25,24,26]` → 5 bits per value instead of 64 |
| **Dictionary** | Moderate-cardinality strings | `city: [NYC,LA,SF,NYC,LA]` → dict `{0:NYC,1:LA,2:SF}`, store `[0,1,2,0,1]` |
| **Delta** | Monotonic timestamps / ordered ints | `ts: [100, 105, 108, 120]` → base=100, deltas `[0,5,3,12]` |
| **Double-delta** | Second-order deltas (timestamps with stable interval) | Deltas of deltas → near-zero for periodic data |

A Parquet file **nests** these: a column chunk might be dictionary-encoded overall, with individual pages using RLE-bitpacking hybrids.

### Parquet File Format

```
Parquet File
├── Magic bytes ("PAR1")
├── Row Group 0
│   ├── Column Chunk: region
│   │   ├── Page 0 (DataPage): dictionary-encoded values
│   │   ├── Page 1 (DataPage): more values
│   │   └── Page 2 (DictionaryPage): the dictionary
│   │   └── Column metadata: min=APAC, max=NA, null_count=0
│   ├── Column Chunk: revenue
│   │   ├── Page 0 (DataPage): delta-encoded values
│   │   └── Column metadata: min=0.0, max=99999.99, null_count=12
│   └── Column Chunk: date
│       └── [...]
├── Row Group 1
│   └── [...]
├── Footer metadata (Thrift-encoded)
│   ├── Schema (flat or nested)
│   ├── Row group locations + sizes
│   ├── Column chunk statistics per chunk
│   └── Key-value application metadata
└── Footer length (4 bytes) + "PAR1"
```

Key concepts:
- **Row group**: a horizontal partition of rows (typically 128 MB–1 GB). All columns for that row range are co-located in the same row group for locality.
- **Column chunk**: all data for one column within a row group. Stored contiguously.
- **Page**: the unit of compression and encoding within a column chunk. Typical page size 4–8 KB uncompressed.
- **Page header**: metadata per page (encoding, compression, number of values).
- **Statistics per column chunk**: `min`, `max`, `null_count` enable **predicate pushdown** — if a query filters `region = 'APAC'` and the column chunk metadata says `min=EU, max=NA`, the entire chunk is skipped without reading.

### Parquet Nested Encoding (Dremel)

Parquet handles nested data (structs, lists, maps) via the **Dremel algorithm**: each value is annotated with a **definition level** (how many optional fields in the path are defined) and a **repetition level** (at what level a repeated field repeats). This flattens arbitrarily nested data into columns without shredding.

```
Address book: contacts[].phone[].number
Flat columns: contacts.phone.number
  def level: is contacts defined? is phone defined?
  rep level: which repeated field is repeating?
```

### DuckDB Architecture

DuckDB is an embedded OLAP database written in C++. It does not use a client-server model — it links directly into your process, like SQLite, but targets analytical workloads.

```
SQL Query
    │
    ▼
┌──────────┐   SQL parser → AST
│ Catalog  │
└────┬─────┘
     ▼
┌──────────┐   Binder: resolve names, types
│  Binder  │
└────┬─────┘
     ▼
┌──────────┐   Logical plan (relational algebra tree)
│ Planner  │
└────┬─────┘
     ▼
┌───────────┐   Rule-based + cost-based optimization
│Optimizer  │   (filter pushdown, join ordering, constant folding)
└────┬──────┘
     ▼
┌────────────────┐   Convert logical ops → physical operators
│Physical Planner│   (hash join vs. merge join, seq scan vs. index scan)
└────┬───────────┘
     ▼
┌──────────┐   Pull-based execution, but each operator
│ Executor │   processes **vectors** (1024–2048 tuples at a time)
└──────────┘   instead of one tuple at a time
```

**Vectorized execution**: operators pass batches of columnar data (vectors) instead of individual rows. A `SELECT sum(price)` scan reads 1024 prices at once into a flat array, SIMD-accumulates them, and passes the partial sum up. This avoids per-tuple function call overhead and maximizes CPU cache / SIMD utilization.

**Morsel-driven parallelism**: the execution engine splits the data into independent chunks ("morsels"), each processed by a thread. A task scheduler distributes morsels across threads with work-stealing for load balancing.

### DuckDB vs SQLite

| Property | SQLite | DuckDB |
|----------|--------|--------|
| Engine type | Row-oriented OLTP | Column-oriented OLAP |
| Execution | Per-tuple iterator (one row at a time) | Vectorized (1024+ rows at a time) |
| Storage | Pages with row data | Columnar chunks, optional Parquet |
| Concurrency | Single writer, MVCC | Multiple readers, single writer (MVCC) |
| Parallelism | Single-threaded | Morsel-driven multi-threaded |
| Best for | Point queries, inserts, small data | Aggregations, analytical joins, large data |
| File format | Custom .db file | Custom .db, also native Parquet I/O |
| Memory | Page cache, lazy loading | Vector buffers, out-of-core hash joins |

## Build It

We'll build a minimal columnar store in Python with compression, a Parquet-like binary format, and a query engine.

### Step 1: Column-wise Storage with Compression

```python
from __future__ import annotations
import struct, io, json, os, time, random, math
from typing import Any

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
            self.metadata[col_name] = {"compressed": True, "method": "rle", "count": len(runs), "original_len": len(col)}
        elif strategy == "dict" or (strategy == "auto" and col and isinstance(col[0], str)):
            codes, mapping = self.dict_encode(col)
            self.compressed[col_name] = {"type": "dict", "codes": codes, "dict": mapping}
            self.metadata[col_name] = {"compressed": True, "method": "dict", "dict_size": len(mapping), "original_len": len(col)}
        else:
            self.metadata[col_name] = {"compressed": False, "method": "raw", "count": len(col)}

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
```

### Step 2: Column Scan with Aggregations

```python
class ColumnScan:
    """Stateless aggregation engine over columnar data."""

    @staticmethod
    def count(col: list) -> int:
        return len(col)

    @staticmethod
    def sum_int(col: list[int]) -> int:
        total = 0
        for v in col:
            total += v
        return total

    @staticmethod
    def sum_float(col: list[float]) -> float:
        total = 0.0
        for v in col:
            total += v
        return total

    @staticmethod
    def avg(col: list[float]) -> float:
        if not col:
            return 0.0
        return ColumnScan.sum_float(col) / len(col)

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
```

### Step 3: Simple Query Engine

```python
class QueryEngine:
    """SELECT group_key, agg(col) FROM store WHERE filter_col > threshold GROUP BY group_key"""

    def __init__(self, store: ColumnStore):
        self.store = store

    def execute(self, *, agg_col: str, agg_func: str, group_col: str | None = None,
                filter_col: str | None = None, filter_op: str = "gt", filter_val: Any = None) -> dict:
        col_data = {name: self.store.scan_column(name) for name in self.store.columns}
        n = len(next(iter(col_data.values()))) if col_data else 0

        # Filter
        row_mask = [True] * n
        if filter_col and filter_val is not None:
            data = col_data[filter_col]
            if filter_op == "gt":
                row_mask = [d > filter_val for d in data]
            elif filter_op == "gte":
                row_mask = [d >= filter_val for d in data]

        # Apply mask across all columns
        filtered: dict[str, list] = {}
        for name, data in col_data.items():
            filtered[name] = [v for v, m in zip(data, row_mask) if m]

        # Group and aggregate
        if group_col:
            groups: dict[str | int | float, list] = {}
            for i, key in enumerate(filtered[group_col]):
                if key not in groups:
                    groups[key] = []
                groups[key].append(filtered[agg_col][i])
            result = {}
            for key, vals in groups.items():
                if agg_func == "sum":
                    result[key] = sum(vals) if isinstance(vals[0], (int, float)) else 0
                elif agg_func == "avg":
                    result[key] = sum(vals) / len(vals) if isinstance(vals[0], (int, float)) else 0.0
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
```

### Step 4: Simple Parquet-like Binary Format

```python
PARQUET_MAGIC = b"PAR1"

def write_parquet_like(store: ColumnStore, path: str) -> None:
    """Simple binary format: magic, row group per column, statistics, footer."""
    buf = io.BytesIO()
    buf.write(PARQUET_MAGIC)

    row_group_offset = buf.tell()
    row_group_meta = []

    for col_name in store.columns:
        col_data = store.scan_column(col_name)
        col_start = buf.tell()

        # Determine type code
        type_code = 0  # int
        if col_data and isinstance(col_data[0], float):
            type_code = 1
        elif col_data and isinstance(col_data[0], str):
            type_code = 2

        # Write column chunk header
        buf.write(struct.pack(">II", type_code, len(col_data)))

        # Write values
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

        # Compute statistics
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

    # Footer
    footer_offset = buf.tell()
    footer = json.dumps({"row_groups": [{"columns": row_group_meta}]}).encode("utf-8")
    buf.write(footer)
    buf.write(struct.pack(">I", len(footer)))
    buf.write(PARQUET_MAGIC)

    with open(path, "wb") as f:
        f.write(buf.getvalue())


def read_parquet_like(path: str, columns: list[str] | None = None) -> dict[str, list]:
    """Read our simple format, with min/max pruning if filter supplied."""
    with open(path, "rb") as f:
        data = f.read()

    buf = io.BytesIO(data)
    magic = buf.read(4)
    assert magic == PARQUET_MAGIC, f"Not a valid file: {magic}"

    # Read footer from end
    buf.seek(-8, io.SEEK_END)
    footer_len = struct.unpack(">I", buf.read(4))[0]
    end_magic = buf.read(4)
    assert end_magic == PARQUET_MAGIC

    # Read footer
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
            type_code = col_meta["type"]
            value_count = col_meta["value_count"]
            raw = col_meta.get("raw_values", None)

            vals = []
            if type_code == 0:  # int
                for _ in range(value_count):
                    vals.append(struct.unpack(">q", buf.read(8))[0])
            elif type_code == 1:  # float
                for _ in range(value_count):
                    vals.append(struct.unpack(">d", buf.read(8))[0])
            elif type_code == 2:  # string
                for _ in range(value_count):
                    slen = struct.unpack(">I", buf.read(4))[0]
                    vals.append(buf.read(slen).decode("utf-8"))
            result[cname] = vals
    return result
```

### Step 5: Demo — Row vs Columnar Performance

```python
def generate_data(n: int) -> ColumnStore:
    """Generate synthetic sales data."""
    regions = ["NA", "EU", "APAC", "LATAM", "MEA"]
    store = ColumnStore()
    store.add_column("region", [random.choice(regions) for _ in range(n)])
    store.add_column("product_id", [random.randint(1, 1000) for _ in range(n)])
    store.add_column("quantity", [random.randint(1, 50) for _ in range(n)])
    store.add_column("price", [round(random.uniform(5.0, 500.0), 2) for _ in range(n)])
    store.add_column("discount", [random.choice([0, 0, 0, 5, 10, 15, 20]) for _ in range(n)])
    store.add_column("revenue", [0.0] * n)

    # Compute revenue = quantity * price * (1 - discount/100)
    q = store.columns["quantity"]
    p = store.columns["price"]
    d = store.columns["discount"]
    store.columns["revenue"] = [q[i] * p[i] * (1 - d[i] / 100) for i in range(n)]

    return store


def demo(n_rows: int = 100_000) -> None:
    print(f"=== Columnar Storage Demo ({n_rows:,} rows) ===")
    store = generate_data(n_rows)

    # Row-oriented query: load all columns, iterate row by row, aggregate
    t0 = time.perf_counter()
    all_cols = {name: store.columns[name] for name in store.columns}
    n = len(next(iter(all_cols.values())))
    row_revenue: dict[str, float] = {}
    for i in range(n):
        r = all_cols["region"][i]
        rev = all_cols["revenue"][i]
        if rev > 100:  # filter
            row_revenue[r] = row_revenue.get(r, 0.0) + rev
    t_row = time.perf_counter() - t0

    # Columnar query: only touch region + revenue, compressed
    t0 = time.perf_counter()
    qe = QueryEngine(store)
    result_col = qe.execute(
        agg_col="revenue", agg_func="sum", group_col="region",
        filter_col="revenue", filter_op="gt", filter_val=100
    )
    t_col = time.perf_counter() - t0

    print(f"Row-oriented: {t_row:.4f}s")
    print(f"Columnar:     {t_col:.4f}s")
    speedup = t_row / t_col if t_col > 0 else float("inf")
    print(f"Speedup:      {speedup:.1f}x")
    print(f"Result: {result_col}")

    # Write Parquet-like
    pq_path = "/tmp/sales_demo.par1"
    write_parquet_like(store, pq_path)
    pq_size = os.path.getsize(pq_path)
    print(f"Written {pq_path} ({pq_size:,} bytes)")

    # Read back with column pruning
    t0 = time.perf_counter()
    pruned = read_parquet_like(pq_path, columns=["region", "revenue"])
    t_read = time.perf_counter() - t0
    print(f"Read back (2 columns): {len(pruned.get('region', []))} rows in {t_read:.4f}s")

    # Compression comparison
    store.compress("region", "rle")
    store.compress("product_id", "dict")
    store.compress("discount", "rle")

    for name in ["region", "product_id", "discount"]:
        meta = store.metadata.get(name, {})
        if meta.get("compressed"):
            orig = meta.get("original_len", 0)
            print(f"  {name}: compressed via {meta['method']} ({orig} values)")


if __name__ == "__main__":
    demo()
```

## Use It

### DuckDB in Practice

```bash
# Install DuckDB CLI
brew install duckdb  # macOS
# or download from duckdb.org

# Python bindings
pip install duckdb
```

```python
import duckdb

# Load Parquet directly
rel = duckdb.sql("SELECT * FROM read_parquet('sales.parquet')")
rel.show()

# Aggregation pushed down to Parquet
result = duckdb.sql("""
    SELECT region, SUM(revenue) as total_revenue
    FROM read_parquet('sales.parquet')
    WHERE revenue > 100
    GROUP BY region
    ORDER BY total_revenue DESC
""")
print(result)
```

DuckDB's Parquet reader uses the column chunk statistics for **predicate pushdown**: the `WHERE revenue > 100` filter checks each column chunk's `max` — if a chunk's max revenue is ≤ 100, the entire chunk is skipped without decompression.

### Our Mini Version vs Production Parquet

| Feature | Our version | Apache Parquet |
|---------|-------------|----------------|
| Magic bytes | `PAR1` | `PAR1` |
| Metadata | JSON in footer | Thrift-compressed schema + metadata |
| Encoding | Raw, RLE, dict | RLE, dictionary, delta, delta-length, delta-byte-array, hybrid |
| Nested data | Not supported | Dremel (def/rep levels) |
| Compression | None | Snappy, Zstd, Gzip, LZ4, Brotli |
| Statistics | min, max, null_count | min, max, null_count, distinct_count, bloom filter |
| Pages | Single page per chunk | Configurable page size, data + dictionary pages |
| Predicate pushdown | Min/max on read | Min/max + bloom filter per column chunk |

## Read the Source

- **DuckDB source**: [`src/execution/operator/scan/physical_column_data_scan.cpp`](https://github.com/duckdb/duckdb/blob/main/src/execution/operator/scan/physical_column_data_scan.cpp) — the physical scan operator that reads columnar data in vectors.
- **Parquet format spec**: [`parquet-format/src/main/thrift/parquet.thrift`](https://github.com/apache/parquet-format/blob/master/src/main/thrift/parquet.thrift) — the authoritative Thrift IDL defining the Parquet file structure.
- **Dremel paper reference**: The original Dremel paper (F. Melnik et al., VLDB 2010) describing the nested encoding used by Parquet.
- **DuckDB in-memory columnar storage**: [`src/storage/table/column_data.cpp`](https://github.com/duckdb/duckdb/blob/main/src/storage/table/column_data.cpp) — DuckDB's own columnar storage for persistent tables.

## Ship It

The reusable artifact is `outputs/columnar_demo.py` — a self-contained columnar store with compression, a simple binary format, and a query engine that demonstrates columnar advantages. Reuse it in Phase 10.22 (capstone) if you add an OLAP query path.

## Exercises

1. **Easy** — Add `COUNT(DISTINCT col)` aggregation to `ColumnScan` and verify with the demo data.
2. **Medium** — Implement delta encoding for timestamps (store base + deltas). Add a timestamp column to the demo and compress it.
3. **Hard** — Extend the Parquet-like writer to support multiple row groups. Write 50,000 rows per row group, then verify that column chunk min/max pruning skips row groups where `revenue` is entirely below threshold.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| OLTP vs OLAP | "OLTP is writes, OLAP is reads" | OLTP: many small random-access transactions, point queries. OLAP: large sequential scans, aggregation-heavy, bulk loads. |
| Columnar storage | "Store data in columns instead of rows" | Each column stored contiguously on disk → read only what you need, better compression (same type), SIMD-amenable. |
| Row group | "A chunk of rows in Parquet" | A horizontal partition — all columns for a range of rows, large enough (~128 MB+) for efficient sequential I/O. |
| Predicate pushdown | "Filter early" | Move WHERE clauses into the scan so column chunk metadata (min/max) skips irrelevant data before decompression. |
| Vectorized execution | "Process batches of rows" | Operators pass arrays of column values (vectors of 1024+ tuples) instead of one tuple at a time → amortize dispatch overhead, exploit SIMD. |
| Dremel encoding | "Magic for nested data" | Def/repetition levels flatten structs/lists into columns without shredding — every value carries its position in the nested tree. |

## Further Reading

- [DuckDB Documentation](https://duckdb.org/docs/) — official docs; start with "SQL Features" and "Extensions" (Parquet, HTTPFS, JSON).
- [Apache Parquet Documentation](https://parquet.apache.org/docs/) — format spec, encoding details, and ecosystem integrations.
- *"Dremel: Interactive Analysis of Web-Scale Datasets"* (Melnik et al., VLDB 2010) — the paper that defined the Dremel algorithm Parquet uses for nested data.
- *"Column-Stores vs. Row-Stores: How Different Are They Really?"* (Abadi et al., SIGMOD 2008) — the definitive academic comparison of the two storage paradigms.
