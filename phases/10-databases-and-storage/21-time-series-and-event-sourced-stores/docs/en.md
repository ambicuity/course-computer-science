# Time-Series and Event-Sourced Stores

> Time-series stores optimize for append-heavy, time-ordered data; event-sourced stores turn every state change into an append-only record.

**Type:** Build
**Languages:** Python, SQL
**Prerequisites:** Phase 10 lessons 01–09 (storage engines, indexes), basic Python
**Time:** ~60 minutes

## Learning Objectives

- Design a time-series storage engine with time-windowed blocks and Gorilla compression
- Implement delta-of-delta + XOR compression for timestamps and float values
- Build downsampling/rollup pipelines for long-range queries
- Model an event-sourced system with append-only event logs and state rebuild
- Contrast event sourcing with CRUD and understand CQRS separation

## The Problem

A temperature sensor publishes 10,000 readings per second. Each reading is a `(timestamp, value)` pair, and the only query patterns are "what was the average temperature in the last hour?" and "show me daily highs for the past year." A standard relational database treats each insert as a random row write — every insert touches a B-Tree page, splits it, and writes a WAL entry, all for a tiny row that will never be updated. On the read side, scanning a billion-row table for a date-range query rebuilds the entire B-Tree index, wasting I/O on rows outside the window.

Now flip the scenario: you're building a financial ledger. Every account transaction must be immutable — you must never update or delete a row. Auditors need to know the account balance at *any* point in history, not just the current value. With a CRUD database, "balance" is a single row that gets updated. You've lost the audit trail. You've lost temporal queries. You've lost the ability to debug how a buggy transaction produced a bad state.

Time-series and event-sourced stores solve these two problems with the same core insight: **the append log is the fundamental data structure**, and you derive queryable views from it.

## The Concept

### Time-Series Data Model

```
metric_name + {tags} + (timestamp → value)
                      │
                      ▼
                ┌──────────┐
                │ ts=0-2h  │  ← newest block (accepts writes)
                ├──────────┤
                │ ts=2-4h  │  ← compacting → downsampled
                ├──────────┤
                │ ts=4-24h │  ← downsampled (5m resolution)
                ├──────────┤
                │ ts>24h   │  ← downsampled (1h resolution)
                └──────────┘
```

Every new data point has a timestamp **strictly greater than** (or equal to) the most recent point — writes are append-only and arrive in near-time order. Storage is split into **time windows** (e.g., 2-hour blocks). Within each block, data is stored columnarly: all timestamps in one array, all values in another. This makes range queries fast (scan one contiguous block) and compression effective (neighboring values are correlated).

### Inside a Time-Windowed Block

```
┌─────────────────────────────────────────┐
│ Block: 2024-01-15 00:00 – 02:00         │
├─────────────────────────────────────────┤
│ Header: metric, tag hash, start/end ts  │
├─────────────────────────────────────────┤
│ Timestamps (compressed, delta-of-delta) │
│ Value 0: 1705276800                      │
│ Value 1: 1705276801  (delta: +1)        │
│ Value 2: 1705276804  (delta: +3)        │
│   → delta of deltas: +2                 │
│ Value 3: 1705276805  (delta: +1)        │
│   → delta of deltas: -2                 │
├─────────────────────────────────────────┤
│ Values (XOR compressed)                 │
│ 27.5 → 0x403B800000000000              │
│ 27.6 → 0x403B99999999999A              │
│   → XOR: 0x00019999999999A             │
│ 28.1 → 0x403C19999999999A              │
│   → XOR with prev: 0x000180000000000   │
└─────────────────────────────────────────┘
```

**Delta-of-delta for timestamps** (Facebook Gorilla paper, SIGMOD 2015):
- Store the first timestamp as a 64-bit int.
- Store the first delta (ts₁ - ts₀) as a variable-length int.
- For subsequent points: store (delta_n - delta_{n-1}), which is often 0 or a small integer, requiring only a few bits.

**XOR for float values** (Gorilla):
- Store the first value as raw 64-bit IEEE 754.
- For subsequent values: XOR with the previous value. When consecutive readings are close, the XOR has long runs of leading+trailing zeros, which compress to just a few bits.

### Downsampling / Rollups

Raw 1-second readings for a year = ~31 million points. Queries over a year don't need second-level precision. Downsampling pre-aggregates data into coarser time buckets:

```
raw (1s) ──► 5m avg ──► 1h avg ──► 1d avg
           │         │          │
           │         │          └─ 366 values/year
           │         └──────────── 8760 values/year
           └────────────────────── 31M values/year
```

Each layer trades precision for storage. Retention policies delete raw data after N days and keep only downsampled data for longer windows.

### Event Sourcing

Instead of storing the *current state* of an entity, event sourcing stores every *state change* as an immutable event:

```
CRUD:    account_balance = $100  (mutated in place)
         UPDATE accounts SET balance = 150 WHERE id = 42
         (old balance = $100 is gone forever)

Event Sourcing:
         INSERT INTO events (aggregate_id, version, type, data)
         VALUES ('acct:42', 1, 'AccountOpened',  '{"owner": "Alice"}'),
                ('acct:42', 2, 'Deposited',      '{"amount": 100}'),
                ('acct:42', 3, 'Withdrew',       '{"amount": 50}'),
                ('acct:42', 4, 'Deposited',      '{"amount": 100}');
         → Current balance = fold(events, initial=0, fn=apply)
         → Balance at version 2 = $100 (just the first deposit)
```

**Current state = fold over the event stream.** To get the balance:
```
def apply(state, event):
    match event.type:
        case 'AccountOpened': return 0
        case 'Deposited':     return state + event.data['amount']
        case 'Withdrew':      return state - event.data['amount']

balance = functools.reduce(apply, events, 0)  # → $150
```

Benefits:
- **Audit trail**: every mutation is recorded with its full context.
- **Temporal queries**: what did the system look like at any past version?
- **Debugging**: replay events into a fixed handler to reproduce bugs.
- **CQRS**: the write model (event log) and read model (materialized view) are separate. You can rebuild the read model from scratch at any time.

### Event Store Examples

| Store | Approach |
|-------|----------|
| EventStoreDB | Purpose-built: events indexed by `{stream_id, version}`. Built-in projections. Atomically append to a stream. |
| Kafka | Log as event store: each partition is an ordered, immutable event sequence. High throughput. No built-in state rebuild. |
| PostgreSQL | Events in a table with `aggregate_id + version` unique constraint. Snapshots in another table. Materialized views for CQRS. |

## Build It

### Step 1: Time-Series Engine — Block and Compression

We build a single-file Python time-series engine at `code/main.py`. The engine stores metrics in time-windowed blocks, compresses timestamps and values using Gorilla-style encoding, and supports range queries with downsampling.

```python
import struct
import math
import random
import time
from typing import List, Tuple, Optional, Callable


def encode_delta_delta(timestamps: List[int]) -> bytes:
    if not timestamps:
        return b""
    buf = bytearray()
    buf += struct.pack("<Q", timestamps[0])
    if len(timestamps) == 1:
        return bytes(buf)
    prev_delta = timestamps[1] - timestamps[0]
    buf += struct.pack("<q", prev_delta)
    for i in range(2, len(timestamps)):
        delta = timestamps[i] - timestamps[i - 1]
        dod = delta - prev_delta
        u64 = dod ^ (1 << 63) if dod < 0 else dod
        while True:
            chunk = u64 & 0x7F
            u64 >>= 7
            if u64:
                chunk |= 0x80
            buf.append(chunk)
            if not u64:
                break
        prev_delta = delta
    return bytes(buf)


def decode_delta_delta(data: bytes) -> List[int]:
    if not data:
        return []
    pos = 0
    timestamps = []
    timestamps.append(struct.unpack_from("<Q", data, pos)[0])
    pos += 8
    if pos >= len(data):
        return timestamps
    prev_delta = struct.unpack_from("<q", data, pos)[0]
    pos += 8
    timestamps.append(timestamps[0] + prev_delta)
    while pos < len(data):
        u64 = 0
        shift = 0
        while True:
            byte = data[pos]
            pos += 1
            u64 |= (byte & 0x7F) << shift
            shift += 7
            if not (byte & 0x80):
                break
        if u64 & 1:
            dod = -((u64 >> 1) + 1)
        else:
            dod = u64 >> 1
        delta = prev_delta + dod
        timestamps.append(timestamps[-1] + delta)
        prev_delta = delta
    return timestamps


def encode_xor(values: List[float]) -> bytes:
    if not values:
        return b""
    buf = bytearray()
    prev_bits = struct.pack(">d", values[0])
    buf += prev_bits
    prev_val = struct.unpack(">Q", prev_bits)[0]
    for v in values[1:]:
        bits = struct.unpack(">Q", struct.pack(">d", v))[0]
        xor = prev_val ^ bits
        if xor == 0:
            buf.append(0)
        else:
            leading = leading_zeros(xor)
            trailing = trailing_zeros(xor)
            buf.append(1)
            buf.append(leading)
            meaningful = 64 - leading - trailing
            buf.append(meaningful)
            val = xor >> trailing
            for _ in range((meaningful + 7) // 8):
                buf.append(val & 0xFF)
                val >>= 8
        prev_val = bits
    return bytes(buf)


def decode_xor(data: bytes) -> List[float]:
    if not data:
        return []
    pos = 0
    values = []
    first_bits = struct.unpack(">Q", data[pos:pos + 8])
    pos += 8
    values.append(struct.unpack(">d", struct.pack(">Q", first_bits[0]))[0])
    prev_val = first_bits[0]
    while pos < len(data):
        control = data[pos]
        pos += 1
        if control == 0:
            values.append(values[-1])
        else:
            leading = data[pos]
            pos += 1
            meaningful = data[pos]
            pos += 1
            byte_count = (meaningful + 7) // 8
            xor = 0
            for i in range(byte_count - 1, -1, -1):
                xor = (xor << 8) | data[pos + i]
            xor <<= trailing_zeros_for_decode(xor, meaningful)
            prev_val ^= xor
            val_bytes = struct.pack(">Q", prev_val)
            values.append(struct.unpack(">d", val_bytes)[0])
    return values


def leading_zeros(x: int) -> int:
    return (64 - x.bit_length()) if x else 64


def trailing_zeros(x: int) -> int:
    return (x & -x).bit_length() - 1 if x else 64


def trailing_zeros_for_decode(x: int, meaningful: int) -> int:
    return 64 - meaningful - leading_zeros(x)
```

### Step 2: Block and TimeSeriesDB

```python
class Block:
    def __init__(self, metric: str, start_ts: int, end_ts: int):
        self.metric = metric
        self.start_ts = start_ts
        self.end_ts = end_ts
        self.timestamps: List[int] = []
        self.values: List[float] = []

    def insert(self, ts: int, value: float) -> None:
        self.timestamps.append(ts)
        self.values.append(value)

    def close(self) -> bytes:
        raw_ts = encode_delta_delta(self.timestamps)
        raw_vals = encode_xor(self.values)
        header = struct.pack("<QQQ", self.start_ts, self.end_ts, len(self.timestamps))
        return header + struct.pack("<I", len(raw_ts)) + raw_ts + raw_vals

    def raw_bytes(self) -> int:
        ts_bytes = len(encode_delta_delta(self.timestamps))
        val_bytes = len(encode_xor(self.values))
        return ts_bytes + val_bytes + 8 + 4


class DownsampledBlock:
    def __init__(self, metric: str, start_ts: int, end_ts: int, interval: int):
        self.metric = metric
        self.start_ts = start_ts
        self.end_ts = end_ts
        self.interval = interval
        self.buckets: List[Tuple[int, float, float, float, float, int]] = []
        # (bucket_start, min, max, sum, last, count)

    def insert(self, ts: int, value: float) -> None:
        bucket = (ts // self.interval) * self.interval
        for i, (bstart, bmin, bmax, bsum, blast, bcnt) in enumerate(self.buckets):
            if bstart == bucket:
                self.buckets[i] = (
                    bucket,
                    min(bmin, value),
                    max(bmax, value),
                    bsum + value,
                    value,
                    bcnt + 1,
                )
                return
        self.buckets.append((bucket, value, value, value, value, 1))

    def query(self) -> List[dict]:
        self.buckets.sort(key=lambda x: x[0])
        return [
            {
                "bucket": b[0],
                "min": b[1],
                "max": b[2],
                "avg": b[3] / b[5],
                "last": b[4],
                "count": b[5],
            }
            for b in self.buckets
        ]
```

### Step 3: Full TimeSeriesDB with Retention and Rollups

```python
class TimeSeriesDB:
    def __init__(self, block_secs: int = 7200):
        self.block_secs = block_secs
        self.blocks: List[Block] = []
        self.downsampled: List[DownsampledBlock] = []
        self.current: Block = self._new_block(0, 0)

    def _new_block(self, start: int, end: int) -> Block:
        return Block("default", start, end)

    def write(self, ts: int, value: float) -> None:
        window = (ts // self.block_secs) * self.block_secs
        if self.current.start_ts != window:
            self.blocks.append(self.current)
            self.current = self._new_block(window, window + self.block_secs)
        self.current.insert(ts, value)

    def query_range(
        self, tstart: int, tend: int
    ) -> List[Tuple[int, float]]:
        results: List[Tuple[int, float]] = []
        for block in self.blocks:
            if block.start_ts >= tend:
                break
            if block.end_ts <= tstart:
                continue
            for ts, val in zip(block.timestamps, block.values):
                if tstart <= ts < tend:
                    results.append((ts, val))
        for ts, val in zip(self.current.timestamps, self.current.values):
            if tstart <= ts < tend:
                results.append((ts, val))
        results.sort(key=lambda x: x[0])
        return results

    def query_downsampled(
        self, tstart: int, tend: int, interval: int
    ) -> List[dict]:
        ds = DownsampledBlock("default", tstart, tend, interval)
        raw = self.query_range(tstart, tend)
        for ts, val in raw:
            ds.insert(ts, val)
        return ds.query()

    def run_rollup(self, age_secs: int, interval: int) -> None:
        cutoff = int(time.time()) - age_secs
        remaining: List[Block] = []
        ds = DownsampledBlock("rollup", 0, cutoff, interval)
        for block in self.blocks:
            if block.end_ts < cutoff:
                for ts, val in zip(block.timestamps, block.values):
                    ds.insert(ts, val)
            else:
                remaining.append(block)
        self.blocks = remaining
        self.downsampled.append(ds)
```

### Step 4: Demo — Ingest 10K+ Points, Run Queries, Show Compression

```python
def main():
    db = TimeSeriesDB(block_secs=3600)
    base_ts = int(time.time()) - 10000
    raw_float_bytes = 0
    count = 0

    print("Ingesting 10,000 data points...")
    for i in range(10000):
        ts = base_ts + i
        val = 20.0 + 10.0 * math.sin(i / 100.0) + random.gauss(0, 1)
        db.write(ts, val)
        raw_float_bytes += 16
        count += 1

    block = db.current if db.current.timestamps else db.blocks[-1]
    compressed = len(encode_delta_delta(block.timestamps)) + len(
        encode_xor(block.values)
    )
    raw_ts = count * 8
    print(f"  Points: {count}")
    print(f"  Raw timestamp bytes: {raw_ts}")
    print(f"  Raw float bytes (ts+val): {raw_ts + raw_float_bytes}")
    print(f"  Compressed bytes (full block): {compressed}")
    print(f"  Compression ratio: {(raw_ts + raw_float_bytes) / max(compressed, 1):.1f}x")

    now = int(time.time())
    hour_ago = now - 3600
    results = db.query_range(base_ts, base_ts + 100)
    print(f"\nRange query (first 100 points): {len(results)} points returned")
    print(f"  First: ts={results[0][0]}, val={results[0][1]:.2f}")
    print(f"  Last:  ts={results[-1][0]}, val={results[-1][1]:.2f}")

    down = db.query_downsampled(base_ts, base_ts + 10000, 500)
    print(f"\nDownsampled (interval=500s): {len(down)} buckets")
    for b in down[:5]:
        print(f"  bucket={b['bucket']} avg={b['avg']:.2f} min={b['min']:.2f} max={b['max']:.2f}")

    db.run_rollup(5000, 300)
    print(f"\nAfter rollup: {len(db.blocks)} raw blocks, {len(db.downsampled)} downsampled blocks")

    mem = sum(b.raw_bytes() for b in db.blocks)
    print(f"  Raw block memory: {mem} bytes")
    ds_mem = sum(len(b.buckets) * 48 for b in db.downsampled)
    print(f"  Downsampled memory: {ds_mem} bytes")
```

### Step 5: SQL for Event Sourcing

In `code/main.sql`, we build an event-sourcing schema on PostgreSQL:

```sql
-- Schema for event-sourced aggregates
CREATE TABLE events (
    id          BIGSERIAL PRIMARY KEY,
    aggregate_id TEXT NOT NULL,
    version     INTEGER NOT NULL,
    event_type  TEXT NOT NULL,
    event_data  JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (aggregate_id, version)
);

CREATE INDEX idx_events_aggregate ON events (aggregate_id, version);

-- Append event with optimistic locking
-- Before inserting, the application must SELECT MAX(version) FROM events
-- WHERE aggregate_id = ? to get the current version, then:
-- INSERT INTO events (aggregate_id, version, event_type, event_data)
-- VALUES ('acct:42', 4, 'Deposited', '{"amount": 100}')
-- The UNIQUE(aggregate_id, version) constraint rejects concurrent writers.

-- Read event stream for an aggregate
SELECT aggregate_id, version, event_type, event_data
FROM events
WHERE aggregate_id = 'acct:42'
ORDER BY version;

-- Rebuild state by folding over events
-- Application code (example in PL/pgSQL):
-- CREATE OR REPLACE FUNCTION rebuild_state(p_aggregate_id TEXT)
-- RETURNS JSONB AS $$
-- DECLARE
--     state JSONB := '{}';
--     rec RECORD;
-- BEGIN
--     FOR rec IN
--         SELECT event_type, event_data FROM events
--         WHERE aggregate_id = p_aggregate_id
--         ORDER BY version
--     LOOP
--         IF rec.event_type = 'AccountOpened' THEN
--             state := jsonb_set(state, '{balance}', '0');
--             state := jsonb_set(state, '{owner}', rec.event_data->'owner');
--         ELSIF rec.event_type = 'Deposited' THEN
--             state := jsonb_set(
--                 state,
--                 '{balance}',
--                 to_jsonb((state->>'balance')::numeric + (rec.event_data->>'amount')::numeric)
--             );
--         ELSIF rec.event_type = 'Withdrew' THEN
--             state := jsonb_set(
--                 state,
--                 '{balance}',
--                 to_jsonb((state->>'balance')::numeric - (rec.event_data->>'amount')::numeric)
--             );
--         END IF;
--     END LOOP;
--     RETURN state;
-- END;
-- $$ LANGUAGE plpgsql;

-- Snapshot table for faster rebuilds
CREATE TABLE snapshots (
    aggregate_id TEXT PRIMARY KEY,
    state       JSONB NOT NULL,
    version     INTEGER NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- On read, fetch snapshot then replay events from snapshot.version + 1:
-- SELECT state FROM snapshots WHERE aggregate_id = 'acct:42';  -- version=100
-- SELECT event_type, event_data FROM events
-- WHERE aggregate_id = 'acct:42' AND version > 100
-- ORDER BY version;

-- CQRS materialized view (read model built from event stream)
CREATE MATERIALIZED VIEW account_summary AS
SELECT
    e.aggregate_id,
    s.state->>'owner' AS owner,
    (s.state->>'balance')::numeric AS balance,
    COUNT(e.id) AS total_events,
    MAX(e.version) AS current_version,
    MIN(e.created_at) AS first_event,
    MAX(e.created_at) AS last_event
FROM events e
LEFT JOIN snapshots s ON s.aggregate_id = e.aggregate_id
GROUP BY e.aggregate_id, s.state;
```

## Use It

### InfluxDB TSM Engine

InfluxDB's TSM (Time-Structured Merge Tree) is a LSM-tree variant specialized for time-series:
- Data is organized by **measurement + tag set** (like our metric + labels).
- Within each shard (time window), data is stored in **TSM files**: sorted, compressed, read-only files.
- Compression: timestamp columns use delta-delta encoding; float columns use XOR (Gorilla). Integer columns use regular delta encoding. String columns use dictionary compression.
- **Downsampling**: continuous queries (`CREATE CONTINUOUS QUERY ...`) periodically aggregate older data into lower-resolution measurements.

What InfluxDB has that ours doesn't:
- **Shard groups**: automatic time-based sharding across data nodes in the cluster.
- **WAL**: writes go to a write-ahead log first for durability, then flush to TSM.
- **Index**: in-memory inverted index for tag-based filtering (find all series matching `host=server1`).
- **Hinted handoff**: when a shard is unavailable, writes are queued on the write node.

### EventStoreDB

EventStoreDB is a purpose-built event store where the only data structure is the event stream:
- Every stream is identified by a `stream_id`. Events are appended atomically.
- Streams have metadata (max count, max age, ACLs).
- **Projections**: built-in JavaScript or C# handlers that produce new streams (read models) from existing ones.
- **Persistent subscriptions**: multiplex events to multiple consumers with checkpoint tracking.

What EventStoreDB has that ours doesn't:
- **Expected version**: atomic conditional appends (`ExpectedVersion.Any`, `ExpectedVersion.StreamExists`, specific version).
- **Transient/volatile subscriptions**: streams push events to subscribers without polling.
- **Scavenging**: background compaction that removes deleted events after retention period.
- **Clustering**: Raft-based consensus for multi-node deployments.

## Read the Source

- **Facebook Gorilla paper**: Section 4.1 (timestamp compression) and 4.2 (value compression) at `https://www.vldb.org/pvldb/vol8/p1816-teller.pdf`
- **InfluxDB TSM engine**: `tsdb/engine/tsm1/` in the influxdb repository — `writer.go` for TSM file format, `reader.go` for block decompression.
- **EventStoreDB**: `src/EventStore.Core/` — `TransactionFileWriter.cs` for the append-only log, `ScavengeManager.cs` for compaction.
- **PostgreSQL event sourcing**: `https://eventstore.com/docs/` — the "Getting Started" guide shows the same pattern as our SQL but with PostgreSQL advisory locks for concurrency.

## Ship It

The reusable artifacts produced by this lesson:

- **`code/main.py`** — standalone time-series engine with Gorilla compression, range queries, and downsampling.
- **`code/main.sql`** — self-contained PostgreSQL event sourcing schema with optimistic locking, state rebuild, snapshots, and a CQRS materialized view.

Both can be extracted and reused in the Phase 10 capstone (MVCC KV store) where the time-series engine handles metric storage and the event sourcing pattern provides the audit trail for transaction history.

## Exercises

1. **Easy** — Add a `DELETE` to the event sourcing SQL: create an `AccountClosed` event type and update the `rebuild_state` function to exclude closed accounts from queries.
2. **Medium** — Extend the time-series engine with Prometheus-style labels: support multiple tag dimensions per metric and add an inverted index mapping `{tag_key: tag_value} → [metric_name]`.
3. **Hard** — Implement a multi-level downsampling pipeline that automatically rolls raw 1s data through 5m → 1h → 1d buckets with configurable retention TTL per level. The engine should expire raw data after 7 days, 5m data after 30 days, and keep 1h+ data forever.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Time-series DB | A database for time-stamped data | A storage engine optimized for append-heavy, time-ordered writes with range-scan-heavy reads, using time-windowed blocks and purpose-built compression. |
| Gorilla compression | Facebook's time-series compression | Delta-of-delta encoding for timestamps (12 bits/point typical) and XOR encoding for float values (1–4 bits/point typical when values change slowly). |
| Downsampling | Reducing data resolution | Pre-aggregating raw points into coarser time buckets (e.g., 1s→5m avg) to compact historical data and speed up long-range queries. |
| Rollup | An aggregated view of older data | The process (periodic or on-the-fly) of computing downsampled values from raw blocks and replacing the raw data with the aggregated form. |
| Retention policy | How long data is kept | Rules that define TTL per resolution tier: raw 1s → 7d, 5m avg → 30d, 1h avg → forever. Data outside the window is automatically deleted. |
| Event sourcing | Store events, not state | An architectural pattern where all changes to an application's state are stored as an immutable, append-only sequence of events. Current state is derived by replaying events. |
| CQRS | Command Query Responsibility Segregation | Separating the write model (commands that produce events) from the read model (queries against materialized views). Enables independent optimization of reads and writes. |
| Aggregate | A consistency boundary in DDD | A cluster of domain objects treated as a single unit for data changes. Events are stored per aggregate, and the aggregate guarantees transactional consistency within its boundary. |

## Further Reading

- [Gorilla: A Fast, Scalable, In-Memory Time Series Database (VLDB 2015)](https://www.vldb.org/pvldb/vol8/p1816-teller.pdf) — The paper that defined the delta-delta + XOR compression scheme. Read section 4 for the encoding details.
- [InfluxDB TSM Engine Documentation](https://docs.influxdata.com/influxdb/v1/concepts/storage_engine/) — Explains the LSM-inspired TSM file format, WAL, and compaction.
- [Prometheus TSDB Internals](https://ganeshvernekar.com/blog/prometheus-tsdb-the-idea-behind/) — Ganesh Vernekar's deep dive into the 2-hour block structure, memory mapping, and compaction.
- [EventStoreDB Documentation](https://developers.eventstore.com/) — Official docs covering event streams, projections, subscriptions, and clustering.
- [Microsoft's CQRS/ES Pattern Guide](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing) — Practical guidance on when CQRS+ES makes sense and common pitfalls.
- Event Sourcing Made Simple (Kleppmann, 2020) — Talks.podcast — Explains the relationship between event logs and database internals, tying event sourcing to LSM-Trees and change data capture.
