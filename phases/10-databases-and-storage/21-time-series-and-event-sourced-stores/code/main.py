"""
Time-Series and Event-Sourced Stores
Phase 10 — Databases & Storage Systems

A complete time-series engine with:
  - Delta-delta timestamp compression (Gorilla)
  - XOR float value compression (Gorilla)
  - Time-windowed block storage
  - Range queries (raw and downsampled)
  - Periodic rollup compaction
"""

import struct
import math
import random
import time
from typing import List, Tuple


# ── Gorilla compression ────────────────────────────────────────────────


def leading_zeros(x: int) -> int:
    return (64 - x.bit_length()) if x else 64


def trailing_zeros(x: int) -> int:
    return (x & -x).bit_length() - 1 if x else 64


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
        u64 = (abs(dod) << 1) | (1 if dod < 0 else 0)
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
    timestamps = [struct.unpack_from("<Q", data, pos)[0]]
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
        neg = u64 & 1
        dod_val = u64 >> 1
        dod = -dod_val if neg else dod_val
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
        xor_val = prev_val ^ bits
        if xor_val == 0:
            buf.append(0)
        else:
            lead = leading_zeros(xor_val)
            trail = trailing_zeros(xor_val)
            meaningful = 64 - lead - trail
            control_byte = 0b10_000000 | (lead >> 2)
            buf.append(control_byte)
            buf.append(lead & 0b11)
            buf.append(meaningful)
            shrunk = xor_val >> trail
            byte_count = (meaningful + 7) // 8
            for _ in range(byte_count):
                buf.append(shrunk & 0xFF)
                shrunk >>= 8
        prev_val = bits
    return bytes(buf)


def decode_xor(data: bytes) -> List[float]:
    if not data:
        return []
    pos = 0
    first_bits = struct.unpack(">Q", data[pos:pos + 8])[0]
    pos += 8
    values = [struct.unpack(">d", struct.pack(">Q", first_bits))[0]]
    prev_val = first_bits
    while pos < len(data):
        control = data[pos]
        pos += 1
        if control == 0:
            values.append(values[-1])
        elif control & 0b10000000:
            lead_hi = (control & 0b01111100) >> 2
            lead = (lead_hi << 2) | (data[pos] & 0b11)
            pos += 1
            meaningful = data[pos]
            pos += 1
            byte_count = (meaningful + 7) // 8
            xor_val = 0
            for _ in range(byte_count):
                xor_val |= data[pos] << (8 * _)
                pos += 1
            xor_val <<= 64 - meaningful - lead
            prev_val ^= xor_val
            val_bytes = struct.pack(">Q", prev_val)
            values.append(struct.unpack(">d", val_bytes)[0])
        else:
            values.append(values[-1])
    return values


# ── Block storage ──────────────────────────────────────────────────────


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

    def compressed_bytes(self) -> int:
        return (len(encode_delta_delta(self.timestamps))
                + len(encode_xor(self.values))
                + 8 + 4)


class DownsampledBlock:
    def __init__(self, metric: str, start_ts: int, end_ts: int, interval: int):
        self.metric = metric
        self.start_ts = start_ts
        self.end_ts = end_ts
        self.interval = interval
        self._buckets: dict = {}

    def insert(self, ts: int, value: float) -> None:
        bucket = (ts // self.interval) * self.interval
        if bucket not in self._buckets:
            self._buckets[bucket] = [value, value, value, value, 1]
        else:
            bmin, bmax, bsum, blast, bcnt = self._buckets[bucket]
            self._buckets[bucket] = [
                min(bmin, value),
                max(bmax, value),
                bsum + value,
                value,
                bcnt + 1,
            ]

    def query(self) -> List[dict]:
        return [
            {
                "bucket": k,
                "min": v[0],
                "max": v[1],
                "avg": v[2] / v[4],
                "last": v[3],
                "count": v[4],
            }
            for k, v in sorted(self._buckets.items())
        ]


# ── TimeSeriesDB ───────────────────────────────────────────────────────


class TimeSeriesDB:
    def __init__(self, block_secs: int = 7200):
        self.block_secs = block_secs
        self.blocks: List[Block] = []
        self.downsampled: List[DownsampledBlock] = []
        self.current: Block = self._make_block(0, 0)

    def _make_block(self, start: int, end: int) -> Block:
        return Block("default", start, end)

    def _flush_current(self) -> None:
        if self.current.timestamps:
            self.blocks.append(self.current)

    def write(self, ts: int, value: float) -> None:
        window = (ts // self.block_secs) * self.block_secs
        if self.current.start_ts != window:
            self._flush_current()
            self.current = self._make_block(window, window + self.block_secs)
        self.current.insert(ts, value)

    def query_range(self, tstart: int, tend: int) -> List[Tuple[int, float]]:
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

    def query_downsampled(self, tstart: int, tend: int, interval: int) -> List[dict]:
        ds = DownsampledBlock("default", tstart, tend, interval)
        for ts, val in self.query_range(tstart, tend):
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
        if ds._buckets:
            self.downsampled.append(ds)


# ── Demo ───────────────────────────────────────────────────────────────


def main() -> None:
    db = TimeSeriesDB(block_secs=3600)
    base_ts = int(time.time()) - 10000

    print("=== Time-Series Engine Demo ===\n")
    print("Ingesting 10,000 data points (sine wave + noise)...")
    for i in range(10000):
        ts = base_ts + i
        val = 20.0 + 10.0 * math.sin(i / 100.0) + random.gauss(0, 0.5)
        db.write(ts, val)

    raw_ts_bytes = 10000 * 8
    raw_val_bytes = 10000 * 8
    comp = db.current.compressed_bytes()
    print(f"  Points ingested: 10,000")
    print(f"  Raw bytes (ts+val): {raw_ts_bytes + raw_val_bytes}")
    print(f"  Current block compressed bytes (ts+val): {comp}")
    print(f"  Compression ratio: {(raw_ts_bytes + raw_val_bytes) / comp:.1f}x")

    results = db.query_range(base_ts, base_ts + 100)
    print(f"\nRange query [0..100): {len(results)} points")
    print(f"  First: ts={results[0][0]}, val={results[0][1]:.2f}")
    print(f"  Last:  ts={results[-1][0]}, val={results[-1][1]:.2f}")

    down = db.query_downsampled(base_ts, base_ts + 10000, 500)
    print(f"\nDownsampled query (interval=500s): {len(down)} buckets")
    for b in down[:5]:
        print(f"  [{b['bucket']}] avg={b['avg']:.2f} min={b['min']:.2f} "
              f"max={b['max']:.2f} count={b['count']}")

    db.run_rollup(5000, 300)
    print(f"\nAfter rollup (age > 5000s → 300s buckets):")
    print(f"  Raw blocks: {len(db.blocks)}")
    print(f"  Downsampled blocks: {len(db.downsampled)}")

    all_points = 0
    for b in db.blocks:
        all_points += len(b.timestamps)
    print(f"  Total raw points remaining: {all_points}")


if __name__ == "__main__":
    main()
