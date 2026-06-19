#!/usr/bin/env python3
"""
Phase 10, Lesson 01 — What a Database Actually Is

This program demonstrates two foundational database concepts:
  1. An append-only log key-value store (Bitcask model)
  2. A minimal SQL query planner

Run:  python3 main.py
"""

import os
import re
import struct
import shlex


# =============================================================================
# Part 1: Append-Only Log Key-Value Store (Bitcask model)
# =============================================================================

class BitcaskKV:
    """A persistent key-value store using an append-only log with an in-memory hash index.

    On PUT:  append (key, value) to a sequential log file; update the hash index.
    On GET:  look up the key in the hash index, seek to the offset, read the value.
    On DELETE: append a tombstone record; remove the key from the index.
    On startup: replay the log to rebuild the index (crash recovery).
    """

    def __init__(self, path):
        self.path = path
        self.index = {}
        self.fd = open(path, "ab+")
        self._rebuild_index()

    def _rebuild_index(self):
        self.index.clear()
        self.fd.seek(0)
        while True:
            offset = self.fd.tell()
            header = self.fd.read(9)
            if len(header) < 9:
                break
            klen, vlen = struct.unpack(">II", header[:8])
            tombstone = header[8]
            key = self.fd.read(klen)
            if len(key) < klen:
                break
            if tombstone:
                self.index.pop(key, None)
            else:
                val = self.fd.read(vlen)
                if len(val) < vlen:
                    break
                self.index[key] = (offset, vlen, val)

    def put(self, key, value):
        if isinstance(key, str):
            key = key.encode()
        if isinstance(value, str):
            value = value.encode()
        self.fd.seek(0, os.SEEK_END)
        offset = self.fd.tell()
        klen = len(key)
        vlen = len(value)
        self.fd.write(struct.pack(">II", klen, vlen))
        self.fd.write(b"\x00")
        self.fd.write(key)
        self.fd.write(value)
        self.fd.flush()
        os.fsync(self.fd.fileno())
        self.index[key] = (offset, vlen, value)

    def get(self, key):
        if isinstance(key, str):
            key = key.encode()
        entry = self.index.get(key)
        if entry is None:
            return None
        return entry[2]

    def scan(self):
        results = []
        self.fd.seek(0)
        while True:
            header = self.fd.read(9)
            if len(header) < 9:
                break
            klen, vlen = struct.unpack(">II", header[:8])
            tombstone = header[8]
            key = self.fd.read(klen)
            if len(key) < klen:
                break
            if tombstone:
                self.fd.seek(vlen, os.SEEK_CUR)
            else:
                val = self.fd.read(vlen)
                if len(val) < vlen:
                    break
                results.append((key, val))
        return results

    def delete(self, key):
        if isinstance(key, str):
            key = key.encode()
        self.fd.seek(0, os.SEEK_END)
        klen = len(key)
        self.fd.write(struct.pack(">II", klen, 0))
        self.fd.write(b"\x01")
        self.fd.write(key)
        self.fd.flush()
        os.fsync(self.fd.fileno())
        self.index.pop(key, None)

    def close(self):
        self.fd.close()


def demo_bitcask():
    print("=" * 60)
    print("Part 1: Bitcask-style Append-Only KV Store")
    print("=" * 60)

    log_path = "/tmp/demo_bitcask.log"
    if os.path.exists(log_path):
        os.remove(log_path)

    db = BitcaskKV(log_path)
    db.put("username", "alice")
    db.put("score", "9000")
    db.put("level", "42")

    print(f"\n  get('username') = {db.get('username')}")
    print(f"  get('score')    = {db.get('score')}")
    print(f"  get('level')    = {db.get('level')}")
    print(f"  get('missing')  = {db.get('missing')}")

    db.delete("level")
    print(f"\n  After delete: get('level') = {db.get('level')}")

    db.put("score", "9500")
    print(f"  After update:  get('score') = {db.get('score')}")

    all_kv = db.scan()
    print(f"\n  Full scan ({len(all_kv)} live entries):")
    for k, v in all_kv:
        print(f"    {k.decode()} = {v.decode()}")
    db.close()

    print(f"\n  --- Reopening and rebuilding index from log ---")
    db2 = BitcaskKV(log_path)
    print(f"  get('username') = {db2.get('username')}")
    print(f"  get('score')    = {db2.get('score')}")
    print(f"  get('level')    = {db2.get('level')}")
    db2.close()
    os.remove(log_path)
    print()


# =============================================================================
# Part 2: Minimal SQL Query Planner
# =============================================================================

class PlanNode:
    """A node in a query plan tree."""

    def __init__(self, op, children=None, **params):
        self.op = op
        self.children = children or []
        self.params = params

    def __repr__(self, indent=0):
        pad = "  " * indent
        parts = [f"{pad}{self.op}"]
        for k, v in self.params.items():
            parts.append(f" {k}={v}")
        parts.append("\n")
        result = "".join(parts)
        for c in self.children:
            result += c.__repr__(indent + 1)
        return result


def parse_create(tokens):
    tokens.pop(0)
    name = tokens.pop(0)
    cols_str = " ".join(tokens).strip("()")
    columns = []
    for col in cols_str.split(","):
        col = col.strip()
        if not col:
            continue
        parts = col.split()
        col_name = parts[0]
        col_type = parts[1] if len(parts) > 1 else "TEXT"
        columns.append((col_name, col_type))
    return PlanNode("CreateTable", table=name, columns=columns)


def parse_select(tokens):
    idx = 0
    if tokens[idx].upper() == "SELECT":
        idx += 1
    cols = []
    while idx < len(tokens) and tokens[idx].upper() != "FROM":
        cols.append(tokens[idx])
        idx += 1
    columns = [c.strip(",") for c in cols if c != ","]
    table = None
    if idx < len(tokens) and tokens[idx].upper() == "FROM":
        idx += 1
        table = tokens[idx]
        idx += 1
    where_clause = None
    if idx < len(tokens) and tokens[idx].upper() == "WHERE":
        idx += 1
        where_clause = " ".join(tokens[idx:])
    node = PlanNode("SeqScan", table=table, columns=columns)
    if where_clause:
        node = PlanNode("Filter", children=[node], condition=where_clause)
    return node


def parse_insert(tokens):
    tokens.pop(0)
    table = tokens.pop(0)
    text = " ".join(tokens)
    m = re.search(r"\((.*?)\)\s*VALUES\s*\((.*?)\)", text, re.IGNORECASE)
    if m:
        columns = [c.strip() for c in m.group(1).split(",")]
        values = [v.strip() for v in m.group(2).split(",")]
        return PlanNode("Insert", table=table, columns=columns, values=values)
    m = re.search(r"VALUES\s*\((.*?)\)", text, re.IGNORECASE)
    if m:
        return PlanNode("Insert", table=table, columns=[], values=m.group(1).split(","))
    return PlanNode("Insert", table=table)


def parse_delete(tokens):
    if tokens and tokens[0].upper() == "FROM":
        tokens.pop(0)
    table = tokens.pop(0) if tokens else None
    where_clause = None
    if tokens and tokens[0].upper() == "WHERE":
        tokens.pop(0)
        where_clause = " ".join(tokens)
    node = PlanNode("SeqScan", table=table, columns=["*"])
    if where_clause:
        node = PlanNode("Filter", children=[node], condition=where_clause)
    return PlanNode("Delete", children=[node], table=table)


def parse_sql(sql):
    sql = sql.strip().rstrip(";")
    tokens = shlex.split(sql)
    if not tokens:
        return None
    keyword = tokens[0].upper()
    remaining = tokens[1:]
    if keyword == "CREATE":
        return parse_create(remaining)
    elif keyword == "SELECT":
        return parse_select(remaining)
    elif keyword == "INSERT":
        return parse_insert(remaining)
    elif keyword == "DELETE":
        return parse_delete(remaining)
    else:
        return PlanNode("Unknown", text=sql)


def demo_planner():
    print("=" * 60)
    print("Part 2: Minimal SQL Query Planner")
    print("=" * 60)

    queries = [
        "CREATE TABLE users (id INT, name TEXT, age INT);",
        "SELECT name, age FROM users WHERE age > 30;",
        "SELECT * FROM users;",
        "INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30);",
        "INSERT INTO logs VALUES ('event', 'started');",
        "DELETE FROM users WHERE id = 1;",
    ]
    for q in queries:
        print(f"\n  SQL: {q}")
        plan = parse_sql(q)
        for line in repr(plan).split("\n"):
            if line.strip():
                print(f"  {line}")
    print()


# =============================================================================
# Main
# =============================================================================

if __name__ == "__main__":
    demo_bitcask()
    demo_planner()
