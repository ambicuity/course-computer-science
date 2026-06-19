# Distributed File Systems — GFS, HDFS

> One master to rule metadata; three replicas to survive the rack fire.

**Type:** Learn
**Languages:** Python
**Prerequisites:** Phase 11 lessons 01–10 (especially Lesson 10 — Replication: Leader/Follower, Quorum)
**Time:** ~75 minutes

## Learning Objectives

- Explain why distributed file systems exist: single machines cannot store petabytes of data or serve the throughput required for large-scale analytics.
- Describe the GFS master-chunkserver architecture: single master (with shadow), 64 MB chunks, 3× replication, and how the master stays out of the data path.
- Walk through GFS mutations: primary replica determines serial order, pushes to secondaries; if primary fails, the master grants a new lease.
- Explain GFS's relaxed consistency model for record append (duplicates, gaps, idempotent applications).
- Describe HDFS architecture: NameNode (metadata, SPOF in original), DataNodes (block storage), 128 MB blocks, rack-aware replica placement.
- Trace the HDFS read path (client → NameNode for locations → nearest DataNode) and write pipeline (client → DataNode 1 → 2 → 3 → ACK).
- Compare GFS, HDFS, and modern object stores (GCS, S3, Azure Blob) on architecture, consistency, and use cases.
- Build a simplified GFS/HDFS in Python with chunk servers, a master/name node, replication, rack-aware placement, fault tolerance, and checkpointing.

## The Problem

You need to process 50 petabytes of web crawl data. A single machine has 20 TB of disk and 25 GB/s of sequential read bandwidth. You would need 2,500 machines just to hold the data — and that's before you account for the fact that disks fail, network links drop, and machines reboot without warning.

A local filesystem (ext4, XFS) manages blocks on a single disk. It assumes: one machine, one kernel, one failure domain. The moment your data spans thousands of machines, every one of those assumptions breaks. You need a **distributed file system** — a system that spreads a single logical file across many physical machines, replicates each piece for fault tolerance, and presents an interface that looks like a regular file to applications.

This lesson covers two foundational distributed file systems:

- **GFS** (Google File System, 2003) — the system that ran Google's web indexing pipeline and inspired an entire generation of distributed storage.
- **HDFS** (Hadoop Distributed File System, 2008) — the open-source clone of GFS that powered the Hadoop ecosystem and taught a generation of engineers that "NameNode restart" means 45 minutes of downtime.

Both systems share the same core insight: **separate metadata from data, keep metadata in memory on one master, and stream data directly between clients and storage nodes.**

## The Concept

### Why Not Just Use NFS or a SAN?

| Property | NFS/SAN | GFS/HDFS |
|----------|---------|----------|
| Scale | 1–10 machines | 1,000–10,000 machines |
| File size | KB–GB | GB–TB per file |
| Access pattern | Random read/write | Sequential read, append-only write |
| Fault model | Hardware RAID, dual controllers | Commodity hardware, expect failures |
| Throughput | ~GB/s | ~TB/s |
| Metadata | In kernel VFS | In user-space master memory |

NFS and SANs optimize for random read/write workloads on small numbers of reliable machines. GFS and HDFS optimize for **append-once, read-many** workloads (MapReduce, log processing, web indexing) on thousands of commodity machines that fail constantly.

### Google File System (GFS)

#### Architecture

```
                    ┌──────────────┐
                    │   GFS Master  │
                    │ (metadata in  │
                    │   RAM only)   │
                    └──────┬───────┘
                           │  metadata ops only
                    ┌──────┴──────┐
                    │             │
              ┌─────▼──┐   ┌─────▼──┐   ┌────────┐
              │Chunk Svr│   │Chunk Svr│   │Chunk Svr│
              │  (R1)   │   │  (R2)  │   │  (R3)  │
              └─────┬──┘   └────┬───┘   └───┬────┘
                    │            │            │
                    └────────────┴────────────┘
                         data path: client ↔ chunk servers
```

**Master** holds all metadata in memory:
- Namespace (directory tree, file names)
- File → chunk ID mapping
- Chunk → list of chunk servers holding that chunk
- Chunk lease state (which replica is the primary)

The master is **not in the data path**. Clients ask the master "where is chunk #7 of file /logs/crawl-2024?" and then read/write directly to the chunk server. This keeps the master lightweight — it can handle thousands of metadata operations per second without becoming a throughput bottleneck.

**Chunk servers** store 64 MB chunks on local disks. Each chunk has a globally unique ID (a 64-bit number composed of the master's timestamp and a sequence counter). Each chunk is replicated across 3 chunk servers by default.

**Shadow master** provides read-only access to metadata during master failover. It's not a hot standby — it reads the operation log and maintains a copy of the master's state. If the primary master fails, the shadow can serve reads while a new primary is elected.

#### Why 64 MB Chunks?

GFS uses 64 MB chunks — orders of magnitude larger than the 4 KB or 8 KB blocks in ext4 or XFS. Why?

1. **Master metadata fits in RAM.** A 50 PB cluster with 64 MB chunks has ~800K chunks. At ~64 bytes of metadata per chunk, that's ~50 MB — the master can hold this in memory and respond to lookups in microseconds. With 4 KB blocks, the same cluster would have ~12.5 billion blocks, requiring ~800 GB of metadata.
2. **Sequential read throughput.** A client reading a multi-GB file establishes a single TCP connection to a chunk server and streams the entire chunk. The connection setup cost (DNS lookup, TCP handshake, authentication) is amortized over 64 MB of data.
3. **Reduced master traffic.** A client reads an entire 64 MB chunk with one metadata lookup. With small blocks, the client would need one lookup per 4 KB — flooding the master.

The tradeoff: small files waste space (a 100-byte file still takes 64 MB on disk) and the master still has one metadata entry per chunk. GFS workloads are dominated by large files, so this is acceptable.

#### GFS Mutations: Writes and Record Append

A **mutation** is any operation that changes a chunk's data: write (overwrite at an offset) or record append (atomic append at the end).

**Write flow** (client overwrites a chunk at an offset):

1. Client asks master which chunk servers hold chunk C and which holds the lease (the primary).
2. Client pushes data to all replicas in a pipeline (or in parallel). Data flows through the chunk servers — the client doesn't buffer all replicas.
3. Client sends the write request to the primary.
4. Primary assigns a **serial order** (a sequence number) to the mutation.
5. Primary applies the mutation locally and forwards it to all secondaries in serial order.
6. Secondaries apply the mutation and ACK to the primary.
7. Primary ACKs to the client.

If any secondary fails, the primary reports a partial failure. The client can retry.

**Record append** (concurrent producers appending to the same file):

Record append is GFS's solution for multi-writer parallel append. It uses an **append-at-least-once** semantic:

1. Multiple clients append records to the same file simultaneously.
2. The primary picks an offset (the current end of the chunk, or the next chunk if this one is full).
3. If the append doesn't fit in the current chunk, the primary pads the remaining space and tells the secondaries to move to the next chunk.
4. The client retries the append on the new chunk.
5. **Result:** records may appear more than once (if a retry succeeds after the first attempt partially succeeded) and there may be padding gaps (empty regions between records).

**Applications must handle this:**
- MapReduce intermediate output uses record append, but MapReduce tasks are idempotent — they can re-process the same record without harm.
- Applications write checksums and record IDs. Readers skip padding and deduplicate records.

#### GFS Consistency Model

GFS provides **relaxed consistency**:

| Operation | Consistency Region | Defined? |
|-----------|-------------------|-----------|
| Successful write | The written region | Yes — consistent and defined |
| Concurrent write overlap | Overlapping region | No — may contain data from multiple writers |
| Successful record append | The record region | Defined, but may be duplicated |
| Failed mutation | The mutated region | No — may be inconsistent across replicas |

"Consistent" means all replicas have the same data. "Defined" means the data is what the mutation actually wrote (not garbage from a failed write overlapping with later writes). The key takeaway: **GFS trades strict consistency for performance.** Applications that use GFS are built to handle duplicates and gaps.

### Hadoop Distributed File System (HDFS)

HDFS is GFS's open-source descendant, designed specifically to run MapReduce workloads on commodity hardware.

#### Architecture

```
                    ┌──────────────┐
                    │   NameNode    │
                    │ (metadata in  │
                    │  RAM + disk)  │
                    └──────┬───────┘
                           │  block locations
              ┌────────────┼────────────┐
              │            │            │
        ┌─────▼──┐   ┌────▼───┐   ┌────▼───┐
        │DataNode │   │DataNode │   │DataNode│
        │  R1-1  │   │  R1-2  │   │ R1-3  │
        └────────┘   └────────┘   └────────┘
```

**NameNode** (GFS's "master"):
- Stores the **filesystem namespace** (directories, files, permissions) and the **block map** (file → list of block IDs, block ID → list of DataNodes holding that block).
- All metadata is in memory (fast lookups) and persisted to disk: **fsimage** (full snapshot) + **edits log** (incremental changes).
- The NameNode is a **single point of failure** in original HDFS. If it crashes, the entire cluster is inaccessible — DataNodes can't validate blocks, and clients can't find data.
- **Secondary NameNode** (a misnomer) periodically downloads fsimage and edits log, merges them (checkpointing), and uploads the merged fsimage back. It is **not** a hot standby. It's a checkpoint helper.

**DataNodes** (GFS's "chunkservers"):
- Store blocks (default 128 MB — HDFS switched from 64 MB to 128 MB for the same reasons GFS chose large chunks, and because modern drives are bigger).
- Send heartbeats and block reports to the NameNode.
- Handle read/write operations directly with clients.

#### HDFS Block Placement: Rack-Aware Replication

HDFS replicates each block 3× with rack awareness:

```
Replica 1: On the local rack (same rack as the client or NameNode's chosen node)
Replica 2: On a different rack (one cross-rack hop for fault tolerance)
Replica 3: On the same rack as replica 2, different node (avoid second cross-rack hop)
```

```
  Rack 1                    Rack 2
 ┌──────────┐            ┌──────────┐
 │ DN-1  ◄── replica 1  │ DN-3  ◄── replica 2
 │ DN-2     (local)      │ DN-4  ◄── replica 3
 └──────────┘            └──────────┘
                              │
                    Write pipeline: DN-1 → DN-3 → DN-4
```

Why this placement?

- **Fault tolerance:** One full rack can fail and you still have replicas on the other rack. Two racks = two failure domains.
- **Write bandwidth:** Only one cross-rack hop (DN-1 → DN-3). The second rack-internal replication (DN-3 → DN-4) stays within rack-level bandwidth.
- **Read bandwidth:** A client on Rack 2 can read from DN-3 or DN-4 (local rack). A client on Rack 1 reads from DN-1 (local rack). Most reads hit a local-rack replica.

#### HDFS Read Path

```
Client                NameNode              DataNodes
  │                      │                      │
  │── open("/data/crawl.log") ──│              │
  │                      │                      │
  │◄─ block locations ───│                      │
  │   (sorted by proximity)                     │
  │                      │                      │
  │── read block 0 ─────────────────────────────► DN-1
  │◄─ data ────────────────────────────────────── DN-1
  │                      │                      │
  │── read block 1 ─────────────────────────────► DN-3
  │◄─ data ────────────────────────────────────── DN-3
  │                      │                      │
```

1. Client calls `open()` on NameNode. NameNode returns the list of DataNodes holding each block, sorted by proximity (client-rack-local first).
2. Client reads data directly from the nearest DataNode. If that DataNode fails, the client tries the next one in the list.
3. The NameNode is **not in the data path** — same as GFS.

#### HDFS Write Path

```
Client                NameNode              DataNodes
  │                      │                      │
  │── create("/output/") │                      │
  │◄─ block targets ─────│  (DN-1, DN-3, DN-4)│
  │                      │                      │
  │── write block 0 ──────────────────────► DN-1
  │                      │                      │
  │         DN-1 ─────────────────────────► DN-3 ─────────► DN-4
  │                      │                      │
  │◄── ACK ◄────────────────────── DN-4 ──────│
  │                      │                      │
```

1. Client calls `create()` on NameNode. NameNode allocates a new block and chooses 3 DataNodes for replication (rack-aware placement).
2. Client writes data to the first DataNode in a **pipeline**: DN-1 receives the data, forwards to DN-3, which forwards to DN-4. Each DataNode writes to its local disk and acks upstream.
3. DN-4 acks DN-3, DN-3 acks DN-1, DN-1 acks the client.
4. If a DataNode in the pipeline fails, the pipeline is reconstructed without that node.

#### NameNode Checkpointing

The NameNode's persistent state is two things on disk:

- **fsimage:** A full snapshot of the namespace and block map. Read at startup.
- **edits log:** A sequential log of every metadata change since the last fsimage. Applied on top of the loaded fsimage during startup.

If the NameNode restarts, it:
1. Loads the last fsimage from disk.
2. Replays every operation in the edits log since that fsimage.
3. Starts serving requests.

For large clusters, the edits log can grow to millions of entries. **Checkpointing** by the Secondary NameNode periodically merges the edits log into a new fsimage, so restarts are fast.

### Comparison: GFS vs HDFS vs Modern Object Stores

| Property | GFS (2003) | HDFS (2008) | GCS / S3 / Azure Blob (2024) |
|----------|-----------|-------------|------------------------------|
| Master/NameNode | Single master + shadow | Single NameNode + Secondary (checkpoint) | Distributed metadata (e.g., S3 uses many metadata servers) |
| Block size | 64 MB | 128 MB (configurable) | Variable (S3: no fixed block size) |
| Replication | 3× configurable | 3× configurable | Erasure coding (e.g., 6+3 in GCS) |
| Consistency | Relaxed (duplicates, gaps on append) | Strong for create-then-read; eventual for intermediate writes | Strong (read-after-write consistent) |
| Append model | Record append (concurrent, at-least-once) | Append-only (create-then-seek) | Multipart upload (at-most-once) |
| Failure domain | Commodity servers | Commodity servers | Multi-AZ, multi-region |
| Metadata store | Single master RAM | Single NameNode RAM + disk | Distributed KV store (e.g., Spanner for GCS) |
| Workload | Sequential read, batch write | MapReduce (sequential scan) | Random read/write, analytics, ML training |

Modern object stores (S3, GCS, Azure Blob) have made GFS/HDFS-style architectures largely unnecessary for new applications:
- **Metadata is distributed** — no single-node bottleneck. S3 uses a fleet of metadata servers; GCS uses Spanner.
- **Erasure coding** (6 data + 3 parity shards) gives the durability of 3× replication with only 1.5× overhead.
- **Strong consistency** — read-after-write is guaranteed. No more handling duplicates and gaps.
- **No file size limit** — S3 objects can be 5 TB (via multipart upload).

But GFS and HDFS are still worth studying because the fundamental design trade-offs (metadata/data separation, large blocks, replication placement, pipeline writes) appear everywhere — in Kafka, in Cassandra, in Ceph, and in every system that stores more data than one machine can hold.

## Build It

We'll build a simplified GFS/HDFS in Python. The system includes:
- **ChunkServer**: stores chunks (byte arrays), supports read/write/append
- **MasterServer**: stores metadata (file → chunk IDs, chunk → chunkserver locations), handles create/read/lease
- **GFSClient**: high-level API (create_file, write, read, append)
- **Replication**: 3× replica placement, rack-aware
- **NameNode checkpointing** (simplified)
- **Fault tolerance**: kill a chunk server, reads still succeed from other replicas

### Step 1: ChunkServer — Storing Data Blocks

Each chunk server holds chunks identified by chunk IDs. It supports reading, writing, and appending to chunks.

```python
import hashlib

class ChunkServer:
    def __init__(self, server_id: str, rack_id: str):
        self.server_id = server_id
        self.rack_id = rack_id
        self.alive = True
        self.chunks: dict[str, bytearray] = {}
        self.chunk_versions: dict[str, int] = {}

    def write_chunk(self, chunk_id: str, data: bytes, offset: int = 0):
        if chunk_id not in self.chunks:
            self.chunks[chunk_id] = bytearray(0)
        chunk = self.chunks[chunk_id]
        needed = offset + len(data)
        if needed > len(chunk):
            chunk.extend(b'\x00' * (needed - len(chunk)))
        chunk[offset:offset + len(data)] = data
        self.chunk_versions[chunk_id] = self.chunk_versions.get(chunk_id, 0) + 1

    def read_chunk(self, chunk_id: str, offset: int = 0, length: int | None = None) -> bytes:
        if chunk_id not in self.chunks:
            raise KeyError(f"Chunk {chunk_id} not found on {self.server_id}")
        chunk = self.chunks[chunk_id]
        if length is None:
            return bytes(chunk[offset:])
        return bytes(chunk[offset:offset + length])

    def append_chunk(self, chunk_id: str, data: bytes) -> int:
        if chunk_id not in self.chunks:
            self.chunks[chunk_id] = bytearray(0)
        offset = len(self.chunks[chunk_id])
        self.chunks[chunk_id].extend(data)
        self.chunk_versions[chunk_id] = self.chunk_versions.get(chunk_id, 0) + 1
        return offset

    def delete_chunk(self, chunk_id: str):
        self.chunks.pop(chunk_id, None)
        self.chunk_versions.pop(chunk_id, None)

    def chunk_size(self, chunk_id: str) -> int:
        if chunk_id not in self.chunks:
            return 0
        return len(self.chunks[chunk_id])

    def checksum(self, chunk_id: str) -> str:
        if chunk_id not in self.chunks:
            return ""
        return hashlib.md5(self.chunks[chunk_id]).hexdigest()
```

### Step 2: MasterServer — Metadata and Coordination

The master stores the file namespace, chunk mapping, and chunk server locations. It grants leases to primary replicas and chooses rack-aware replica placement.

```python
CHUNK_SIZE = 64 * 1024 * 1024
REPLICATION_FACTOR = 3

class MasterServer:
    def __init__(self):
        self.files: dict[str, list[str]] = {}
        self.chunk_locations: dict[str, list[str]] = {}
        self.chunk_primary: dict[str, str] = {}
        self.chunk_servers: dict[str, ChunkServer] = {}
        self.chunk_counter = 0
        self.leases: dict[str, float] = {}

    def register_chunk_server(self, server: ChunkServer):
        self.chunk_servers[server.server_id] = server

    def create_file(self, filename: str) -> list[str]:
        self.files[filename] = []
        return []

    def allocate_chunk(self, filename: str, chunk_index: int) -> str:
        chunk_id = f"chunk_{self.chunk_counter}"
        self.chunk_counter += 1
        if filename not in self.files:
            self.files[filename] = []
        while len(self.files[filename]) <= chunk_index:
            self.files[filename].append("")
        self.files[filename][chunk_index] = chunk_id
        servers = list(self.chunk_servers.values())
        alive_servers = [s for s in servers if s.alive]
        if len(alive_servers) < REPLICATION_FACTOR:
            raise RuntimeError(f"Need {REPLICATION_FACTOR} alive servers, have {len(alive_servers)}")
        chosen = self._rack_aware_placement(alive_servers)
        self.chunk_locations[chunk_id] = [s.server_id for s in chosen]
        primary = chosen[0]
        self.chunk_primary[chunk_id] = primary.server_id
        for s in chosen:
            s.chunks[chunk_id] = bytearray(0)
            s.chunk_versions[chunk_id] = 0
        return chunk_id

    def _rack_aware_placement(self, servers: list[ChunkServer]) -> list[ChunkServer]:
        racks: dict[str, list[ChunkServer]] = {}
        for s in servers:
            racks.setdefault(s.rack_id, []).append(s)
        chosen = []
        first_rack = servers[0].rack_id
        first_rack_servers = [s for s in servers if s.rack_id == first_rack]
        chosen.append(first_rack_servers[0])
        other_racks = [rid for rid in racks if rid != first_rack]
        if other_racks:
            second_rack = other_racks[0]
            second_rack_servers = [s for s in servers if s.rack_id == second_rack]
            chosen.append(second_rack_servers[0])
            same_rack_second = [s for s in second_rack_servers if s.server_id != chosen[1].server_id]
            if same_rack_second:
                chosen.append(same_rack_second[0])
            else:
                remaining = [s for s in servers if s.server_id not in {c.server_id for c in chosen}]
                if remaining:
                    chosen.append(remaining[0])
        else:
            remaining = [s for s in servers if s.server_id not in {c.server_id for c in chosen}]
            for s in remaining[:2]:
                chosen.append(s)
        return chosen[:REPLICATION_FACTOR]

    def get_chunk_locations(self, filename: str, chunk_index: int) -> list[str]:
        if filename not in self.files:
            raise FileNotFoundError(f"File {filename} not found")
        chunks = self.files[filename]
        if chunk_index >= len(chunks):
            raise IndexError(f"Chunk index {chunk_index} out of range")
        chunk_id = chunks[chunk_index]
        return self.chunk_locations.get(chunk_id, [])

    def get_primary(self, chunk_id: str) -> str:
        return self.chunk_primary.get(chunk_id, "")

    def kill_server(self, server_id: str):
        if server_id in self.chunk_servers:
            self.chunk_servers[server_id].alive = False
            for chunk_id, locations in self.chunk_locations.items():
                if server_id in locations:
                    if self.chunk_primary.get(chunk_id) == server_id:
                        alive = [s for s in locations if s != server_id and self.chunk_servers[s].alive]
                        if alive:
                            self.chunk_primary[chunk_id] = alive[0]

    def get_file_chunks(self, filename: str) -> list[str]:
        if filename not in self.files:
            raise FileNotFoundError(f"File {filename} not found")
        return self.files[filename]
```

### Step 3: GFSClient — High-Level File Operations

The client handles file creation, writing, reading, and appending. For writes, it contacts the master for chunk locations, then pushes data to the primary which forwards to secondaries.

```python
class GFSClient:
    def __init__(self, master: MasterServer):
        self.master = master

    def create_file(self, filename: str):
        self.master.create_file(filename)

    def write(self, filename: str, data: bytes, offset: int = 0):
        chunk_index = offset // CHUNK_SIZE
        in_chunk_offset = offset % CHUNK_SIZE
        remaining = len(data)
        data_offset = 0
        while remaining > 0:
            chunk_id = self._ensure_chunk(filename, chunk_index)
            locations = self.master.get_chunk_locations(filename, chunk_index)
            primary_id = self.master.get_primary(chunk_id)
            space_in_chunk = CHUNK_SIZE - in_chunk_offset
            write_len = min(remaining, space_in_chunk)
            primary = self.master.chunk_servers[primary_id]
            primary.write_chunk(chunk_id, data[data_offset:data_offset + write_len], in_chunk_offset)
            for loc_id in locations:
                if loc_id != primary_id:
                    server = self.master.chunk_servers[loc_id]
                    if server.alive:
                        server.write_chunk(chunk_id, data[data_offset:data_offset + write_len], in_chunk_offset)
            remaining -= write_len
            data_offset += write_len
            chunk_index += 1
            in_chunk_offset = 0

    def read(self, filename: str, offset: int = 0, length: int | None = None) -> bytes:
        chunks = self.master.get_file_chunks(filename)
        if not chunks:
            return b""
        result = bytearray()
        start_chunk = offset // CHUNK_SIZE
        in_chunk_offset = offset % CHUNK_SIZE
        remaining = length if length is not None else float('inf')
        for ci in range(start_chunk, len(chunks)):
            if remaining <= 0:
                break
            chunk_id = chunks[ci]
            locations = self.master.chunk_locations.get(chunk_id, [])
            read_from = None
            for loc_id in locations:
                server = self.master.chunk_servers[loc_id]
                if server.alive:
                    read_from = server
                    break
            if read_from is None:
                raise RuntimeError(f"No alive replica for chunk {chunk_id}")
            read_len = min(int(remaining), CHUNK_SIZE - in_chunk_offset)
            data = read_from.read_chunk(chunk_id, in_chunk_offset, read_len)
            result.extend(data)
            remaining -= len(data)
            in_chunk_offset = 0
        return bytes(result)

    def append(self, filename: str, data: bytes) -> int:
        chunks = self.master.get_file_chunks(filename)
        if not chunks:
            self._ensure_chunk(filename, 0)
            chunks = self.master.get_file_chunks(filename)
        last_chunk_id = chunks[-1]
        last_server = None
        for loc_id in self.master.chunk_locations.get(last_chunk_id, []):
            server = self.master.chunk_servers[loc_id]
            if server.alive:
                last_server = server
                break
        if last_server is None:
            raise RuntimeError("No alive server for append")
        current_size = last_server.chunk_size(last_chunk_id)
        if current_size + len(data) <= CHUNK_SIZE:
            offset = self._append_to_chunk(last_chunk_id, data)
            for loc_id in self.master.chunk_locations.get(last_chunk_id, []):
                server = self.master.chunk_servers[loc_id]
                if server.alive and server.server_id != last_server.server_id:
                    server.write_chunk(last_chunk_id, data, offset)
            return (len(chunks) - 1) * CHUNK_SIZE + offset
        else:
            new_chunk_index = len(chunks)
            new_chunk_id = self._ensure_chunk(filename, new_chunk_index)
            self._append_to_chunk(new_chunk_id, data)
            for loc_id in self.master.chunk_locations.get(new_chunk_id, []):
                server = self.master.chunk_servers[loc_id]
                if server.alive:
                    server.write_chunk(new_chunk_id, data, 0)
            return new_chunk_index * CHUNK_SIZE

    def _ensure_chunk(self, filename: str, chunk_index: int) -> str:
        chunks = self.master.get_file_chunks(filename)
        while chunk_index >= len(chunks) or chunks[chunk_index] == "":
            self.master.allocate_chunk(filename, len(chunks) if chunk_index >= len(chunks) else chunk_index)
            chunks = self.master.get_file_chunks(filename)
        return chunks[chunk_index]

    def _append_to_chunk(self, chunk_id: str, data: bytes) -> int:
        primary_id = self.master.get_primary(chunk_id)
        primary = self.master.chunk_servers[primary_id]
        return primary.append_chunk(chunk_id, data)
```

### Step 4: NameNode Checkpoint (Simplified)

A simplified version of HDFS's checkpoint mechanism — periodically serialize the NameNode state to disk.

```python
import json
import os

class NameNodeCheckpoint:
    def __init__(self, checkpoint_dir: str = "/tmp/gfs_checkpoints"):
        self.checkpoint_dir = checkpoint_dir
        os.makedirs(checkpoint_dir, exist_ok=True)

    def save(self, master: MasterServer, version: int):
        state = {
            "version": version,
            "files": master.files,
            "chunk_locations": master.chunk_locations,
            "chunk_primary": master.chunk_primary,
            "chunk_counter": master.chunk_counter,
        }
        path = os.path.join(self.checkpoint_dir, f"fsimage_{version}.json")
        with open(path, "w") as f:
            json.dump(state, f, indent=2)
        edits_path = os.path.join(self.checkpoint_dir, f"edits_{version}.log")
        with open(edits_path, "w") as f:
            f.write(f"# Edit log base version {version}\n")
        return path

    def load(self, version: int) -> dict | None:
        path = os.path.join(self.checkpoint_dir, f"fsimage_{version}.json")
        if not os.path.exists(path):
            return None
        with open(path) as f:
            return json.load(f)

    def restore(self, version: int) -> MasterServer | None:
        state = self.load(version)
        if state is None:
            return None
        master = MasterServer()
        master.files = state["files"]
        master.chunk_locations = state["chunk_locations"]
        master.chunk_primary = state["chunk_primary"]
        master.chunk_counter = state["chunk_counter"]
        return master
```

### Step 5: Running the Full System

See `code/main.py` for the complete runnable system with demos showing:
- Creating a large file and observing chunk distribution across rack-aware servers
- Reading from nearest replica
- Fault tolerance: killing a chunk server and showing reads still succeed
- Checkpoint and restore

## Use It

**HDFS in production** (Apache Hadoop) adds what our simulation omits:

- **Persistent storage:** DataNodes store blocks as files on the local filesystem. Blocks have checksums (CRC32) verified on every read. Corrupted blocks are re-replicated from other replicas.
- **Heartbeats and block reports:** DataNodes send heartbeats to the NameNode every 3 seconds. If a DataNode stops heartbeating for 10 minutes, the NameNode declares it dead and re-replicates all its blocks.
- **Re-replication on failure:** When the NameNode detects a dead DataNode (or under-replicated blocks), it creates new replicas on other DataNodes to restore the replication factor.
- **Balancing:** An HDFS balancer moves blocks from overloaded DataNodes to underloaded ones, respecting rack topology.
- **HDFS Federation:** Modern HDFS supports multiple NameNodes, each managing a portion of the namespace (a "namespace volume"), to scale metadata beyond one machine's RAM.

**Google File System** evolved into **Colossus** (Google's internal successor, circa 2010), which replaced the single master with a distributed metadata layer, added encryption at rest, and supports erasure coding. Colossus is not open-source, but its design principles echo in GCS.

**Modern object stores** (Amazon S3, Google Cloud Storage, Azure Blob Storage) have effectively replaced HDFS for most new workloads:
- S3 uses a distributed metadata service (no single-node bottleneck) and erasure coding (11+9 configuration gives 99.999999999% durability with 1.8× overhead).
- GCS uses Spanner for metadata, providing strong consistency and global namespace.
- Azure Blob uses a partitioned metadata layer inspired by the original Azure Storage architecture paper (Calder et al., 2011).

The GFS/HDFS pattern — metadata server + data nodes + large blocks + rack-aware replication — is still the foundation, but modern systems distribute the metadata and use erasure coding instead of simple replication.

## Read the Source

- [Hadoop HDFS NameNode — FsNamesystem.java](https://github.com/apache/hadoop/blob/trunk/hadoop-hdfs-project/hadoop-hdfs/src/main/java/org/apache/hadoop/hdfs/server/namenode/FSDirectory.java) — the in-memory namespace tree and block map. Start with `getINode` to see how HDFS resolves paths.
- [Hadoop HDFS DataNode — BlockReceiver.java](https://github.com/apache/hadoop/blob/trunk/hadoop-hdfs-project/hadoop-hdfs/src/main/java/org/apache/hadoop/hdfs/server/datanode/BlockReceiver.java) — the write pipeline in action. DataNode receives a block, writes to local disk, and forwards to the next node in the pipeline.
- [GFS Paper — Sanjay Ghemawat, Howard Gobioff, Shun-Tak Leung (2003)](https://research.google/gfs-sosp2003.pdf) — the original paper. Still one of the best systems papers ever written.

## Ship It

The reusable artifact is in `outputs/`: a simplified GFS/HDFS implementation that you can import as a library:

```python
from main import ChunkServer, MasterServer, GFSClient, NameNodeCheckpoint
```

Reuse this in Phase 11 lessons on MapReduce (data locality) and the Raft capstone (replicated metadata).

## Exercises

1. **Easy** — Run the demo. Kill a chunk server and verify that reads still succeed from the remaining replicas. How does the system choose which replica to read from?
2. **Medium** — Implement HDFS-style write pipelining: instead of the client writing to all replicas independently, have the primary forward data to the second in a pipeline (DN-1 → DN-3 → DN-4). Measure the difference in total write operations.
3. **Hard** — Implement erasure coding as an alternative to 3× replication. Use Reed-Solomon (6 data + 3 parity shards) and show that the system can tolerate 3-node failures while using only 1.5× the storage of the original data. Compare read latency when all nodes are alive vs. when one or two parity shards are missing.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Chunk (GFS) / Block (HDFS) | "a piece of a file" | A 64 MB (GFS) or 128 MB (HDFS) region of a file, identified by a globally unique ID, replicated across multiple servers. Large enough to amortize metadata overhead and connection setup. |
| Master / NameNode | "the boss node" | A single server holding all filesystem metadata in RAM. Not in the data path for reads/writes — only serves metadata lookups. A scalability bottleneck and single point of failure. |
| Shadow master / Secondary NameNode | "the backup" | Not a hot standby. The shadow master serves read-only metadata during failover. The Secondary NameNode checkpoints the edits log — it cannot serve requests. |
| Record append | "concurrent write" | An atomic append operation in GFS that allows multiple producers to append to the same file. Provides at-least-once semantics: duplicates and padding gaps are possible. |
| Rack-aware placement | "spread replicas across racks" | Place replica 1 locally, replica 2 on a different rack, replica 3 on the same rack as replica 2. Two-rack fault tolerance with one cross-rack write hop. |
| Checkpointing | "saving state" | Merging the edits log into a new fsimage to limit NameNode restart time. The Secondary NameNode does this periodically. |
| Erasure coding | "better than replication" | An alternative to 3× replication that uses parity shards (e.g., 6 data + 3 parity = 1.5× overhead) to achieve the same fault tolerance with less storage. |

## Further Reading

- [The Google File System](https://research.google/gfs-sosp2003.pdf) (Ghemawat, Gobioff, Leung, 2003) — The original GFS paper. Essential reading for understanding the design trade-offs.
- [The Hadoop Distributed File System](https://www.aosabook.org/en/v1/hdfs.html) (Shvachko et al., 2010) — The architecture chapter from *Architecture of Open Source Applications*. Covers NameNode, DataNode, and the write pipeline in detail.
- [HDFS Architecture Guide](https://hadoop.apache.org/docs/stable/hadoop-project-dist/hadoop-hdfs/HdfsDesign.html) — The official Apache HDFS documentation.
- [Windows Azure Storage: A Highly Available Cloud Storage Service with Strong Consistency](https://www.semanticscholar.org/paper/Windows-Azure-Storage%3A-A-Highly-Available-Cloud-with-Calder-Flusy/paper) (Calder et al., 2011) — The Azure Blob Storage paper. Shows how modern object stores distribute metadata and provide strong consistency.
- [Erasure Coding in HDFS](https://hadoop.apache.org/docs/stable/hadoop-project-dist/hadoop-hdfs/HDFSErasureCoding.html) — How HDFS adds erasure coding as an alternative to 3× replication, reducing storage overhead from 3× to 1.5×.