"""
NoSQL — KV, Document, Wide-Column, Graph
Phase 10 — Databases & Storage Systems

Usage: python3 main.py
"""

import json
import os
from collections import deque


# ============================================================================
# KVStore — in-memory dict with WAL-based disk persistence
# ============================================================================

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

    def compact(self):
        with open(self._wal, "w") as f:
            for k, v in self._data.items():
                f.write(json.dumps(("PUT", k, v)) + "\n")

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


# ============================================================================
# DocumentStore — JSON document storage with collection support
# ============================================================================

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


# ============================================================================
# GraphStore — adjacency list with nodes, edges, and BFS traversal
# ============================================================================

class GraphStore:
    def __init__(self):
        self._nodes = {}
        self._adj = {}

    def add_node(self, node_id, properties=None):
        self._nodes[node_id] = properties or {}

    def add_edge(self, from_id, to_id, label="", properties=None):
        self._adj.setdefault(from_id, []).append(
            (to_id, label, properties or {})
        )

    def get_node(self, node_id):
        return self._nodes.get(node_id)

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


# ============================================================================
# Demo — model an e-commerce schema in all three engines
# ============================================================================

def demo():
    print("=" * 60)
    print("KV STORE — e-commerce as opaque blobs")
    print("=" * 60)
    kv = KVStore("wal_demo.log")
    kv.put("user:1", json.dumps({"name": "Alice", "email": "alice@x.com"}))
    kv.put("order:1", json.dumps({"user_id": 1, "item": "laptop", "total": 1200}))
    kv.put("order:2", json.dumps({"user_id": 1, "item": "mouse", "total": 25}))

    user1 = json.loads(kv.get("user:1"))
    print(f"  User: {user1['name']}, email: {user1['email']}")
    print(f"  All keys: {list(kv.scan().keys())}")
    kv.delete("order:2")
    print(f"  After deleting order:2: {list(kv.scan().keys())}")
    kv.compact()
    print()

    print("=" * 60)
    print("DOCUMENT STORE — e-commerce with predicate filters")
    print("=" * 60)
    ds = DocumentStore()
    ds.insert("users", {"name": "Bob", "email": "bob@x.com", "since": 2024})
    ds.insert("users", {"name": "Carol", "email": "carol@x.com", "since": 2025})
    ds.insert("orders", {"user_id": 0, "item": "monitor", "total": 400})
    ds.insert("orders", {"user_id": 0, "item": "keyboard", "total": 80})
    ds.insert("orders", {"user_id": 1, "item": "ssd", "total": 150})

    print(f"  All users: {ds.find('users')}")
    print(f"  Bob: {ds.find('users', {'name': 'Bob'})}")
    bob_orders = ds.find("orders", {"user_id": 0})
    print(f"  Bob's orders: {bob_orders}")
    expensive = ds.find("orders", {"user_id": 0, "item": "monitor"})
    print(f"  Filtered (user 0 + item monitor): {expensive}")

    ds.update("users", {"name": "Bob"}, {"since": 2023})
    print(f"  After update: {ds.find('users', {'name': 'Bob'})}")

    ds.delete("orders", {"item": "keyboard"})
    print(f"  Orders after deleting keyboard: {ds.find('orders')}")
    print()

    print("=" * 60)
    print("GRAPH STORE — social product recommendations via BFS")
    print("=" * 60)
    gs = GraphStore()
    for uid in range(6):
        gs.add_node(uid, {"name": f"User{uid}"})

    gs.add_edge(0, 1, "friend")
    gs.add_edge(0, 2, "friend")
    gs.add_edge(1, 3, "friend")
    gs.add_edge(2, 3, "friend")
    gs.add_edge(3, 4, "friend")
    gs.add_edge(4, 5, "purchased", {"item": "laptop"})
    gs.add_edge(3, 5, "friend")

    path_04 = gs.shortest_path(0, 4)
    print(f"  Shortest path 0 -> 4: {path_04}")

    path_05 = gs.shortest_path(0, 5)
    print(f"  Shortest path 0 -> 5 (who bought laptop): {path_05}")

    print(f"  Friends of 0: {[n for n, *_ in gs.neighbors(0)]}")
    print(f"  Friends of 3: {[n for n, *_ in gs.neighbors(3)]}")
    node4 = gs.get_node(4)
    print(f"  Node 4: {node4}")

    os.remove("wal_demo.log")


def main():
    demo()


if __name__ == "__main__":
    main()
