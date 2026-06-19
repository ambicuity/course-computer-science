# NoSQL — KV, Document, Wide-Column, Graph

> Four data models, zero JOINs — storing data at web scale.

**Type:** Build
**Languages:** Python
**Prerequisites:** Phase 10 lessons 01–16, basic SQL
**Time:** ~60 minutes

## Learning Objectives

- Explain why NoSQL emerged and when each family is the right choice.
- Implement a KV store, document store, and graph store from scratch in Python.
- Map a relational e-commerce schema onto all four NoSQL models and compare tradeoffs.
- Reason about CAP theorem implications for each category.

## The Problem

Your e-commerce startup takes off. Your PostgreSQL instance handles 1,000 writes/second fine. At 100,000 writes/second the master starts choking. You add replicas — but now you need to handle conflict resolution, schema changes every sprint, and product catalogs where every item has different attributes. Relational normalization hurts: every product page needs 14 JOINs across 8 tables.

Meanwhile, your friend at a social network needs to answer "what friends-of-friends-of-friends liked this post" in under 50ms — a query that would mean recursive CTEs and index-hopping through a JOIN explosion.

These are the problems NoSQL databases were built to solve. Each family throws away something SQL gives you (JOINs, schemas, transactions, or all three) in exchange for scale, flexibility, or a better fit for the data shape.

## The Concept

### The Four Families

```
                    NoSQL
                      │
      ┌───────────────┼───────────────┬───────────────┐
      │               │               │               │
   Key-Value      Document      Wide-Column       Graph
   (Redis,       (MongoDB,     (Cassandra,       (Neo4j,
    DynamoDB)     Couchbase)    Bigtable)          ArangoDB)
```

**Key-Value** — simplest model: a global hash map. Every lookup is O(1) by key. No query language, no secondary indexes, no relations. You get, put, delete. That's it.

**Document** — self-describing JSON/BSON documents with nested structure. Each document can have different fields. Secondary indexes on any field. Query with predicates, projections, aggregations. Like SQL tables where every row can have its own columns.

**Wide-Column** — sparse table with row key + column family + column qualifier + timestamp. Sort order is physical (data is stored sorted by row key). Column families group related columns. Sparse = missing columns cost nothing. Built for time-series, IoT, and any append-heavy workload.

**Graph** — nodes + edges + properties. Nodes are entities, edges are relationships. Traversal queries follow edges from a start node. Property graph model lets you attach key-value pairs to both nodes and edges.

### CAP Theorem per Category

| Family | Typical orientation | Why |
|--------|-------------------|-----|
| KV (Dynamo-style) | AP | Eventual consistency, hinted handoff, vector clocks |
| KV (etcd-style) | CP | Raft consensus, linearizable reads, single key space |
| Document (MongoDB) | CP (primary) / AP (secondary reads) | Primary writes, secondary reads may be stale |
| Document (Couchbase) | AP | Last-writer-wins, cross-datacenter replication |
| Wide-Column (Bigtable) | CP | Single tablet master, strong consistency |
| Wide-Column (Cassandra) | AP | Tunable consistency, gossip, hinted handoff |
| Graph (Neo4j) | CP | ACID transactions, single leader, causal clustering |

### Data Model Comparison: "Users with Orders"

| Relational | KV | Document | Wide-Column | Graph |
|-----------|-----|----------|-------------|-------|
| `users` table | `user:{id}` → `{name, email}` | `users` collection, one doc per user | `users` CF, row key = user_id, columns = name, email | Node label `User`, properties `{id, name, email}` |
| `orders` table | `order:{id}` → serialized order JSON | `orders` collection, embedded or referenced | `orders` CF, row key = order_id, columns = item, qty, total | Node label `Order`, edge `PLACED_BY` from User |
| JOIN users ↔ orders | Application does two gets | Embedded array in user doc, or reference | Denormalized into user row columns | Traverse `PLACED_BY` edge |
| "All orders for user X" | App gets user key, then order keys | `db.orders.find({user_id: X})` | Row-key scan on user+order composite key | `MATCH (u:User)-[:PLACED_BY]->(o:Order)` |

## Build It

We'll build three miniature engines: a KV store with write-ahead log persistence, a document store with collection-based filtering, and a graph store with BFS traversal. All three share the same directory in `code/main.py`.

### Step 1: KVStore — In-Memory Hash + WAL

The core is a Python dict. For persistence we write every mutation to an append-only write-ahead log before applying to memory. On startup, replay the log.

```python
import json, os, struct

class KVStore:
    def __init__(self, path="wal.log"):
        self._data = {}
        self._wal = path
        if os.path.exists(self._wal):
            self._replay()

    def put(self, key, value):
        self._data[key] = value
        self._append(("PUT", key, value))

    def get(self, key):
        return self._data.get(key)

    def delete(self, key):
        self._data.pop(key, None)
        self._append(("DEL", key, None))

    def scan(self):
        return dict(self._data)

    def _append(self, entry):
        with open(self._wal, "a") as f:
            f.write(json.dumps(entry) + "\n")

    def _replay(self):
        with open(self._wal) as f:
            for line in f:
                op, key, value = json.loads(line.strip())
                if op == "PUT":
                    self._data[key] = value
                elif op == "DEL":
                    self._data.pop(key, None)
```

### Step 2: DocumentStore — JSON Collection with Predicate Filter

Documents live in named collections. Each document gets an auto-incrementing `_id`. The `find` method accepts a simple equality predicate dict.

```python
class DocumentStore:
    def __init__(self):
        self._collections = {}
        self._ids = 0

    def insert(self, collection, document):
        doc = dict(document)
        doc["_id"] = self._ids
        self._ids += 1
        self._collections.setdefault(collection, []).append(doc)
        return doc["_id"]

    def find(self, collection, predicate=None):
        docs = self._collections.get(collection, [])
        if not predicate:
            return list(docs)
        return [d for d in docs
                if all(d.get(k) == v for k, v in predicate.items())]

    def update(self, collection, predicate, updates):
        for d in self.find(collection, predicate):
            d.update(updates)

    def delete(self, collection, predicate):
        self._collections[collection] = [
            d for d in self._collections.get(collection, [])
            if any(d.get(k) != v for k, v in predicate.items())
        ]
```

### Step 3: GraphStore — Adjacency List with BFS Traversal

Nodes store properties. Edges are directional pairs stored in an adjacency dict. We implement shortest-path BFS.

```python
from collections import deque

class GraphStore:
    def __init__(self):
        self._nodes = {}
        self._adj = {}

    def add_node(self, node_id, properties=None):
        self._nodes[node_id] = properties or {}

    def add_edge(self, from_id, to_id, label="", properties=None):
        self._adj.setdefault(from_id, []).append((to_id, label, properties or {}))

    def neighbors(self, node_id):
        return self._adj.get(node_id, [])

    def shortest_path(self, start, end):
        if start == end:
            return [start]
        q = deque([(start, [start])])
        visited = {start}
        while q:
            node, path = q.popleft()
            for neighbor, *_ in self._adj.get(node, []):
                if neighbor == end:
                    return path + [neighbor]
                if neighbor not in visited:
                    visited.add(neighbor)
                    q.append((neighbor, path + [neighbor]))
        return None

    def get_node(self, node_id):
        return self._nodes.get(node_id)
```

### Step 4: E-Commerce Demo

We model the same schema in all three engines and run queries.

```python
def demo():
    print("=" * 60)
    print("KV STORE — e-commerce as key blobs")
    print("=" * 60)
    kv = KVStore()
    kv.put("user:1", json.dumps({"name": "Alice", "email": "alice@x.com"}))
    kv.put("order:1", json.dumps({"user_id": 1, "item": "laptop", "total": 1200}))
    kv.put("order:2", json.dumps({"user_id": 1, "item": "mouse", "total": 25}))
    user1 = json.loads(kv.get("user:1"))
    print(f"  User: {user1['name']}, email: {user1['email']}")
    print(f"  All keys: {list(kv.scan().keys())}")
    print()

    print("=" * 60)
    print("DOCUMENT STORE — e-commerce with query filters")
    print("=" * 60)
    ds = DocumentStore()
    ds.insert("users", {"name": "Bob", "email": "bob@x.com"})
    ds.insert("orders", {"user_id": 0, "item": "monitor", "total": 400})
    ds.insert("orders", {"user_id": 0, "item": "keyboard", "total": 80})
    bob = ds.find("users", {"name": "Bob"})
    bob_orders = ds.find("orders", {"user_id": 0})
    print(f"  Users named Bob: {bob}")
    print(f"  Bob's orders: {bob_orders}")
    expensive = ds.find("orders", {"user_id": 0, "item": "monitor"})
    print(f"  Filtered (user 0, item monitor): {expensive}")
    print()

    print("=" * 60)
    print("GRAPH STORE — social product recommendations")
    print("=" * 60)
    gs = GraphStore()
    for uid in range(5):
        gs.add_node(uid, {"name": f"User{uid}"})
    gs.add_edge(0, 1, "friend")
    gs.add_edge(0, 2, "friend")
    gs.add_edge(1, 3, "friend")
    gs.add_edge(2, 3, "friend")
    gs.add_edge(3, 4, "friend")
    path = gs.shortest_path(0, 4)
    print(f"  Shortest path 0→4: {path}")
    print(f"  Friends of 0: {[n for n, *_ in gs.neighbors(0)]}")
```

## Use It

Compare your implementations against production systems:

- **Your KVStore vs Redis** — Redis uses an event loop with O(1) ops, has persistence (RDB snapshots, AOF log), and supports rich data structures (lists, sets, sorted sets). Your WAL is a simplified AOF.
- **Your DocumentStore vs MongoDB** — MongoDB uses BSON (binary JSON), has a query optimizer, secondary indexes (B-trees), replication via oplog, and sharding. Your predicate filter is a naive full collection scan — exactly how early MongoDB worked.
- **Your GraphStore vs Neo4j** — Neo4j stores nodes/relationships in fixed-size record files on disk with pointer-based traversal (index-free adjacency). Your BFS is the same algorithm but your adjacency list lives entirely in memory.

## Read the Source

- [Redis AOF persistence](https://github.com/redis/redis/blob/unstable/src/aof.c) — how Redis implements append-only file persistence, including rewrite to avoid unbounded log growth.
- [Cassandra's `StorageProxy`](https://github.com/apache/cassandra/blob/trunk/src/java/org/apache/cassandra/service/StorageProxy.java) — how a wide-column store routes reads/writes with tunable consistency.
- [Neo4j `GraphDatabaseService`](https://github.com/neo4j/neo4j/blob/5.0/public/community/neo4j/src/main/java/org/neo4j/server/http/neo4j/GraphDatabaseService.java) — index-free adjacency traversal.

## Ship It

`code/main.py` — a self-contained Python module with KV, document, and graph engines you can drop into any project that needs a lightweight embedded store.

## Exercises

1. **Easy** — Add a `compact()` method to KVStore that rewrites the WAL as a snapshot (only the latest value per key), then truncates the file.
2. **Medium** — Add secondary index support to DocumentStore: an `ensure_index(field)` that builds an internal `{field_value: [doc_ids]}` map and uses it in `find`.
3. **Hard** — Implement Dynamo-style hinted handoff in KVStore: when a "replica" is unreachable, queue the write locally and forward it when the replica comes back.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| NoSQL | SQL is bad | "Not only SQL" — a family of databases that trade relational features for scale, flexibility, or data-shape fit |
| Key-Value store | A hash map you can network | Exactly that — O(1) get/put/delete, no relations, no query language |
| Document store | Schemaless | Schema-flexible — each document self-describes its fields; no ALTER TABLE needed |
| Wide-column store | Columnar database | Sparse sorted map — row key → column family → column qualifier → value, stored sorted and sparsely |
| Graph database | Relationship database | Nodes + edges + properties, optimized for relationship traversal rather than JOIN-based relational algebra |
| CAP theorem | Pick 2 of 3 | Under network partition, you choose consistency or availability; the choice shapes the entire architecture |
| WAL | Write-ahead log | Append-only log of every mutation. On crash, replay to reconstruct state. |

## Further Reading

- [Designing Data-Intensive Applications (Kleppmann)](https://dataintensive.net) — Chapters 5–7 are the definitive treatment of replication, partitioning, and transactions across NoSQL systems.
- [DynamoDB Paper (Amazon, 2007)](https://www.allthingsdistributed.com/files/amazon-dynamo-sosp2007.pdf) — Original Dynamo design: consistent hashing, vector clocks, hinted handoff, gossip.
- [Bigtable Paper (Google, 2006)](https://research.google/pubs/pub27898/) — The wide-column model defined: tablets, SSTables, Compaction.
- [Neo4j Graph Data Science Manual](https://neo4j.com/docs/graph-data-science/current/) — Production traversal, pathfinding, and graph algorithms.
