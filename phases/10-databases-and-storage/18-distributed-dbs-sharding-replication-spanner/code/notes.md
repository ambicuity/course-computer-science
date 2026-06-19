# Notes — Distributed DBs — Sharding, Replication, Spanner

Lesson is complete. See `docs/en.md` for the full lesson body and `code/main.go` for the implementation.

Key outputs:
- `ConsistentHashRing` with virtual nodes (add/remove/route)
- `ShardedKV` — ring + per-node KV stores with rebalancing demo
- Simplified `RaftNode` — leader election + log replication
