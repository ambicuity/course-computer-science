import hashlib
import json
import os
import random
import time

CHUNK_SIZE = 64 * 1024 * 1024
REPLICATION_FACTOR = 3


class ChunkServer:
    def __init__(self, server_id: str, rack_id: str):
        self.server_id = server_id
        self.rack_id = rack_id
        self.alive = True
        self.chunks: dict[str, bytearray] = {}
        self.chunk_versions: dict[str, int] = {}
        self.bytes_written = 0
        self.bytes_read = 0

    def write_chunk(self, chunk_id: str, data: bytes, offset: int = 0):
        if not self.alive:
            raise ConnectionError(f"ChunkServer {self.server_id} is down")
        if chunk_id not in self.chunks:
            self.chunks[chunk_id] = bytearray(0)
        chunk = self.chunks[chunk_id]
        needed = offset + len(data)
        if needed > len(chunk):
            chunk.extend(b'\x00' * (needed - len(chunk)))
        chunk[offset:offset + len(data)] = data
        self.chunk_versions[chunk_id] = self.chunk_versions.get(chunk_id, 0) + 1
        self.bytes_written += len(data)

    def read_chunk(self, chunk_id: str, offset: int = 0, length: int | None = None) -> bytes:
        if not self.alive:
            raise ConnectionError(f"ChunkServer {self.server_id} is down")
        if chunk_id not in self.chunks:
            raise KeyError(f"Chunk {chunk_id} not found on {self.server_id}")
        chunk = self.chunks[chunk_id]
        if offset >= len(chunk):
            return b""
        if length is None:
            result = bytes(chunk[offset:])
        else:
            end = min(offset + length, len(chunk))
            result = bytes(chunk[offset:end])
        self.bytes_read += len(result)
        return result

    def append_chunk(self, chunk_id: str, data: bytes) -> int:
        if not self.alive:
            raise ConnectionError(f"ChunkServer {self.server_id} is down")
        if chunk_id not in self.chunks:
            self.chunks[chunk_id] = bytearray(0)
        offset = len(self.chunks[chunk_id])
        self.chunks[chunk_id].extend(data)
        self.chunk_versions[chunk_id] = self.chunk_versions.get(chunk_id, 0) + 1
        self.bytes_written += len(data)
        return offset

    def delete_chunk(self, chunk_id: str):
        if chunk_id in self.chunks:
            del self.chunks[chunk_id]
        self.chunk_versions.pop(chunk_id, None)

    def chunk_size(self, chunk_id: str) -> int:
        if chunk_id not in self.chunks:
            return 0
        return len(self.chunks[chunk_id])

    def checksum(self, chunk_id: str) -> str:
        if chunk_id not in self.chunks:
            return ""
        return hashlib.md5(self.chunks[chunk_id]).hexdigest()

    def __repr__(self):
        status = "ALIVE" if self.alive else "DEAD"
        chunk_count = len(self.chunks)
        total_size = sum(len(c) for c in self.chunks.values())
        return f"ChunkServer({self.server_id}, rack={self.rack_id}, {status}, {chunk_count} chunks, {total_size} bytes)"


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

    def delete_file(self, filename: str):
        if filename in self.files:
            for chunk_id in self.files[filename]:
                for loc_id in self.chunk_locations.get(chunk_id, []):
                    if loc_id in self.chunk_servers:
                        self.chunk_servers[loc_id].delete_chunk(chunk_id)
                self.chunk_locations.pop(chunk_id, None)
                self.chunk_primary.pop(chunk_id, None)
            del self.files[filename]

    def allocate_chunk(self, filename: str, chunk_index: int) -> str:
        chunk_id = f"chunk_{self.chunk_counter}"
        self.chunk_counter += 1

        if filename not in self.files:
            self.files[filename] = []

        while len(self.files[filename]) <= chunk_index:
            self.files[filename].append("")
        self.files[filename][chunk_index] = chunk_id

        alive_servers = [s for s in self.chunk_servers.values() if s.alive]
        if len(alive_servers) < REPLICATION_FACTOR:
            raise RuntimeError(
                f"Need {REPLICATION_FACTOR} alive servers, have {len(alive_servers)}"
            )

        chosen = self._rack_aware_placement(alive_servers)
        self.chunk_locations[chunk_id] = [s.server_id for s in chosen]
        self.chunk_primary[chunk_id] = chosen[0].server_id

        for s in chosen:
            s.chunks[chunk_id] = bytearray(0)
            s.chunk_versions[chunk_id] = 0

        return chunk_id

    def _rack_aware_placement(self, servers: list[ChunkServer]) -> list[ChunkServer]:
        racks: dict[str, list[ChunkServer]] = {}
        for s in servers:
            racks.setdefault(s.rack_id, []).append(s)

        rack_ids = sorted(racks.keys())
        chosen: list[ChunkServer] = []

        first_rack_id = rack_ids[self.chunk_counter % len(rack_ids)]
        first_rack_servers = racks[first_rack_id]
        first_idx = self.chunk_counter % len(first_rack_servers)
        chosen.append(first_rack_servers[first_idx])

        other_rack_ids = [rid for rid in rack_ids if rid != first_rack_id]

        if other_rack_ids:
            second_rack_id = other_rack_ids[self.chunk_counter % len(other_rack_ids)]
            second_rack_servers = racks[second_rack_id]
            second_idx = (self.chunk_counter // len(second_rack_servers)) % len(second_rack_servers)
            chosen.append(second_rack_servers[second_idx])

            same_rack_candidates = [
                s for s in second_rack_servers
                if s.server_id != chosen[1].server_id
            ]
            if same_rack_candidates:
                third_idx = (self.chunk_counter + 1) % len(same_rack_candidates)
                chosen.append(same_rack_candidates[third_idx])
            else:
                remaining = [
                    s for s in servers
                    if s.server_id not in {c.server_id for c in chosen}
                ]
                if remaining:
                    chosen.append(remaining[self.chunk_counter % len(remaining)])
        else:
            remaining = [
                s for s in servers
                if s.server_id not in {c.server_id for c in chosen}
            ]
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

    def get_all_locations(self, chunk_id: str) -> list[str]:
        return self.chunk_locations.get(chunk_id, [])

    def kill_server(self, server_id: str):
        if server_id in self.chunk_servers:
            self.chunk_servers[server_id].alive = False
            for chunk_id, locations in self.chunk_locations.items():
                if server_id in locations:
                    if self.chunk_primary.get(chunk_id) == server_id:
                        alive_replicas = [
                            loc for loc in locations
                            if loc != server_id
                            and loc in self.chunk_servers
                            and self.chunk_servers[loc].alive
                        ]
                        if alive_replicas:
                            self.chunk_primary[chunk_id] = alive_replicas[0]

    def revive_server(self, server_id: str):
        if server_id in self.chunk_servers:
            self.chunk_servers[server_id].alive = True

    def get_file_chunks(self, filename: str) -> list[str]:
        if filename not in self.files:
            raise FileNotFoundError(f"File {filename} not found")
        return self.files[filename]

    def file_size(self, filename: str) -> int:
        chunks = self.get_file_chunks(filename)
        total = 0
        for i, chunk_id in enumerate(chunks):
            if chunk_id:
                for loc_id in self.chunk_locations.get(chunk_id, []):
                    server = self.chunk_servers[loc_id]
                    if server.alive:
                        total += server.chunk_size(chunk_id)
                        break
        return total

    def chunk_distribution(self) -> dict[str, dict[str, int]]:
        dist: dict[str, dict[str, int]] = {}
        for sid, server in self.chunk_servers.items():
            dist[sid] = {
                "rack": server.rack_id,
                "alive": server.alive,
                "chunk_count": len(server.chunks),
                "total_bytes": sum(len(c) for c in server.chunks.values()),
            }
        return dist


class GFSClient:
    def __init__(self, master: MasterServer):
        self.master = master

    def create_file(self, filename: str):
        self.master.create_file(filename)

    def write(self, filename: str, data: bytes, offset: int = 0):
        if not data:
            return
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
            if primary.alive:
                primary.write_chunk(
                    chunk_id, data[data_offset:data_offset + write_len], in_chunk_offset
                )

            for loc_id in locations:
                if loc_id != primary_id:
                    server = self.master.chunk_servers[loc_id]
                    if server.alive:
                        server.write_chunk(
                            chunk_id,
                            data[data_offset:data_offset + write_len],
                            in_chunk_offset,
                        )

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
        remaining = length

        for ci in range(start_chunk, len(chunks)):
            if remaining is not None and remaining <= 0:
                break
            chunk_id = chunks[ci]
            if not chunk_id:
                break

            locations = self.master.chunk_locations.get(chunk_id, [])
            read_from = None
            for loc_id in locations:
                server = self.master.chunk_servers[loc_id]
                if server.alive:
                    read_from = server
                    break

            if read_from is None:
                raise RuntimeError(
                    f"No alive replica for chunk {chunk_id} "
                    f"(locations: {locations})"
                )

            if remaining is not None:
                read_len = min(remaining, CHUNK_SIZE - in_chunk_offset)
            else:
                read_len = None

            data = read_from.read_chunk(chunk_id, in_chunk_offset, read_len)
            result.extend(data)
            if remaining is not None:
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
            primary_id = self.master.get_primary(last_chunk_id)
            primary = self.master.chunk_servers[primary_id]
            if not primary.alive:
                alive = [
                    loc for loc in self.master.chunk_locations.get(last_chunk_id, [])
                    if loc in self.master.chunk_servers
                    and self.master.chunk_servers[loc].alive
                ]
                if alive:
                    primary = self.master.chunk_servers[alive[0]]
                else:
                    raise RuntimeError("No alive server for append")

            offset = primary.append_chunk(last_chunk_id, data)

            for loc_id in self.master.chunk_locations.get(last_chunk_id, []):
                server = self.master.chunk_servers[loc_id]
                if server.alive and server.server_id != primary.server_id:
                    server.write_chunk(last_chunk_id, data, offset)

            return (len(chunks) - 1) * CHUNK_SIZE + offset
        else:
            new_chunk_index = len(chunks)
            new_chunk_id = self._ensure_chunk(filename, new_chunk_index)
            chunks = self.master.get_file_chunks(filename)

            primary_id = self.master.get_primary(new_chunk_id)
            primary = self.master.chunk_servers[primary_id]
            primary.append_chunk(new_chunk_id, data)

            for loc_id in self.master.chunk_locations.get(new_chunk_id, []):
                server = self.master.chunk_servers[loc_id]
                if server.alive and server.server_id != primary_id:
                    server.write_chunk(new_chunk_id, data, 0)

            return new_chunk_index * CHUNK_SIZE

    def _ensure_chunk(self, filename: str, chunk_index: int) -> str:
        chunks = self.master.get_file_chunks(filename)
        while chunk_index >= len(chunks) or chunks[chunk_index] == "":
            idx = len(chunks) if chunk_index >= len(chunks) else chunk_index
            self.master.allocate_chunk(filename, idx)
            chunks = self.master.get_file_chunks(filename)
        return chunks[chunk_index]


class NameNodeCheckpoint:
    def __init__(self, checkpoint_dir: str = "/tmp/gfs_checkpoints"):
        self.checkpoint_dir = checkpoint_dir
        os.makedirs(checkpoint_dir, exist_ok=True)

    def save(self, master: MasterServer, version: int) -> str:
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
            for fname, chunks in master.files.items():
                for i, cid in enumerate(chunks):
                    f.write(f"ALLOCATE {fname} {i} {cid}\n")
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


def print_separator(title: str):
    print()
    print("=" * 72)
    print(f"  {title}")
    print("=" * 72)
    print()


def demo_chunk_distribution():
    print_separator("DEMO 1: Create a 1GB-equivalent file — chunk distribution")

    master = MasterServer()
    servers = [
        ChunkServer("CS-R1-A", "rack-1"),
        ChunkServer("CS-R1-B", "rack-1"),
        ChunkServer("CS-R2-A", "rack-2"),
        ChunkServer("CS-R2-B", "rack-2"),
        ChunkServer("CS-R3-A", "rack-3"),
    ]
    for s in servers:
        master.register_chunk_server(s)

    client = GFSClient(master)
    client.create_file("/data/web-crawl-2024.log")

    one_gb = 1024 * 1024 * 1024
    num_chunks = (one_gb + CHUNK_SIZE - 1) // CHUNK_SIZE
    print(f"Creating 1 GB file with {num_chunks} chunks of {CHUNK_SIZE // (1024*1024)} MB each")
    print(f"Replication factor: {REPLICATION_FACTOR}")
    print()

    random.seed(42)
    total_bytes_written = 0
    for i in range(num_chunks):
        chunk_id = master.allocate_chunk("/data/web-crawl-2024.log", i)
        chunk_data = bytes(random.getrandbits(8) for _ in range(min(CHUNK_SIZE, one_gb - i * CHUNK_SIZE)))
        client.write("/data/web-crawl-2024.log", chunk_data, i * CHUNK_SIZE)
        total_bytes_written += len(chunk_data)

    print(f"Total bytes written: {total_bytes_written:,}")
    print(f"Total chunks: {num_chunks}")
    print()

    dist = master.chunk_distribution()
    print("Chunk distribution across servers:")
    print(f"{'Server':<12} {'Rack':<10} {'Chunks':<10} {'Data (MB)':<12} {'Status'}")
    print("-" * 60)
    for sid, info in dist.items():
        mb = info["total_bytes"] / (1024 * 1024)
        print(
            f"{sid:<12} {info['rack']:<10} {info['chunk_count']:<10} "
            f"{mb:<12.1f} {'ALIVE' if info['alive'] else 'DEAD'}"
        )

    print()
    print("Replica placement (first 5 chunks):")
    chunks = master.get_file_chunks("/data/web-crawl-2024.log")
    for i, cid in enumerate(chunks[:5]):
        locs = master.get_all_locations(cid)
        primary = master.get_primary(cid)
        loc_info = []
        for loc in locs:
            s = master.chunk_servers[loc]
            loc_info.append(f"{loc}({s.rack_id})")
        print(f"  Chunk {i} ({cid}): replicas={loc_info}, primary={primary}")

    print()
    actual = master.file_size("/data/web-crawl-2024.log")
    print(f"File size reported by master: {actual:,} bytes")


def demo_read_from_nearest():
    print_separator("DEMO 2: Read from nearest replica (rack-aware)")

    master = MasterServer()
    servers = [
        ChunkServer("CS-R1-A", "rack-1"),
        ChunkServer("CS-R1-B", "rack-1"),
        ChunkServer("CS-R2-A", "rack-2"),
        ChunkServer("CS-R2-B", "rack-2"),
        ChunkServer("CS-R3-A", "rack-3"),
    ]
    for s in servers:
        master.register_chunk_server(s)

    client = GFSClient(master)
    client.create_file("/data/test-data.bin")

    test_data = b"Hello, distributed file systems! " * 100
    client.write("/data/test-data.bin", test_data)

    print(f"Wrote {len(test_data)} bytes to /data/test-data.bin")
    print()

    chunks = master.get_file_chunks("/data/test-data.bin")
    for i, cid in enumerate(chunks):
        locs = master.get_all_locations(cid)
        print(f"Chunk {i} ({cid}) is replicated on: {locs}")
        for loc in locs:
            s = master.chunk_servers[loc]
            print(f"  {loc} on {s.rack_id}, checksum={s.checksum(cid)[:16]}...")

    read_data = client.read("/data/test-data.bin")
    print(f"\nRead {len(read_data)} bytes back")
    assert read_data == test_data, "Data mismatch!"
    print("Data integrity: VERIFIED (read matches write)")

    print()
    print("Read statistics per server:")
    for s in servers:
        print(f"  {s.server_id}: {s.bytes_read:,} bytes read")


def demo_fault_tolerance():
    print_separator("DEMO 3: Fault tolerance — kill a chunk server")

    master = MasterServer()
    servers = [
        ChunkServer("CS-R1-A", "rack-1"),
        ChunkServer("CS-R1-B", "rack-1"),
        ChunkServer("CS-R2-A", "rack-2"),
        ChunkServer("CS-R2-B", "rack-2"),
        ChunkServer("CS-R3-A", "rack-3"),
    ]
    for s in servers:
        master.register_chunk_server(s)

    client = GFSClient(master)
    client.create_file("/data/important-data.bin")

    data = b"CRITICAL DATA: transaction log entry #" + b"X" * 500
    original_data = data
    client.write("/data/important-data.bin", data)

    read_back = client.read("/data/important-data.bin")
    assert read_back == original_data
    print(f"Initial write: {len(data)} bytes written and verified")

    chunks = master.get_file_chunks("/data/important-data.bin")
    print(f"File has {len(chunks)} chunk(s)")
    print()

    kill_target = None
    if chunks:
        cid = chunks[0]
        locs = master.get_all_locations(cid)
        primary = master.get_primary(cid)
        kill_target = primary
        print(f"Chunk 0 ({cid}):")
        print(f"  Primary: {primary}")
        print(f"  All replicas: {locs}")
        print()

    print(f">>> KILLING server {kill_target} <<<")
    master.kill_server(kill_target)

    for s in servers:
        print(f"  {s.server_id}: {'ALIVE' if s.alive else 'DEAD'}")
    print()

    new_primary = master.get_primary(chunks[0])
    print(f"New primary for chunk 0: {new_primary}")
    print()

    try:
        read_after_kill = client.read("/data/important-data.bin")
        assert read_after_kill == original_data
        print(f"Read after killing {kill_target}: SUCCESS")
        print(f"  Read {len(read_after_kill)} bytes — data integrity VERIFIED")
    except Exception as e:
        print(f"Read failed: {e}")

    cid = chunks[0]
    locs = master.get_all_locations(cid)
    alive_locs = [
        loc for loc in locs
        if loc in master.chunk_servers and master.chunk_servers[loc].alive
    ]
    print(f"\nAlive replicas: {alive_locs}")

    print()
    second_target = alive_locs[0]
    print(f">>> KILLING server {second_target} <<<")
    master.kill_server(second_target)

    alive_locs = [
        loc for loc in locs
        if loc in master.chunk_servers and master.chunk_servers[loc].alive
    ]
    print(f"Alive replicas: {alive_locs}")

    try:
        read_after_second_kill = client.read("/data/important-data.bin")
        assert read_after_second_kill == original_data
        print(f"Read after killing 2 of 3 replicas: SUCCESS")
        print(f"  Survived with {len(alive_locs)} replica(s)")
    except Exception as e:
        print(f"Read failed: {e}")

    print()
    print("Third kill — this should fail:")
    third_target = alive_locs[0]
    master.kill_server(third_target)
    print(f">>> KILLING server {third_target} <<<")

    all_dead = all(
        not master.chunk_servers[loc].alive
        for loc in locs if loc in master.chunk_servers
    )
    if all_dead:
        print("All replicas are dead. Data is LOST.")
    else:
        try:
            read_after_third = client.read("/data/important-data.bin")
            print(f"Read succeeded with remaining replicas")
        except Exception as e:
            print(f"Read failed as expected: {e}")


def demo_append_and_checkpoint():
    print_separator("DEMO 4: Record append + NameNode checkpoint")

    master = MasterServer()
    servers = [
        ChunkServer("CS-R1-A", "rack-1"),
        ChunkServer("CS-R2-A", "rack-2"),
        ChunkServer("CS-R2-B", "rack-2"),
    ]
    for s in servers:
        master.register_chunk_server(s)

    client = GFSClient(master)
    client.create_file("/logs/crawl-output.dat")

    records = [
        b"RECORD_001:user_click page=/home ts=1700000001\n",
        b"RECORD_002:user_click page=/about ts=1700000002\n",
        b"RECORD_003:user_click page=/products ts=1700000003\n",
        b"RECORD_004:user_scroll page=/home ts=1700000004\n",
        b"RECORD_005:user_click page=/contact ts=1700000005\n",
    ]

    offsets = []
    for record in records:
        offset = client.append("/logs/crawl-output.dat", record)
        offsets.append(offset)
        print(f"Appended record at offset {offset}: {record.decode().strip()}")

    print()
    all_data = client.read("/logs/crawl-output.dat")
    print(f"Total bytes read: {len(all_data)}")
    print(f"Total records in file: {len(records)}")

    print()
    print("NameNode checkpoint:")
    checkpoint = NameNodeCheckpoint()
    path = checkpoint.save(master, version=1)
    print(f"  Checkpoint saved to: {path}")

    restored_state = checkpoint.load(1)
    print(f"  Checkpoint contains {len(restored_state['files'])} file(s)")
    print(f"  Checkpoint version: {restored_state['version']}")
    print(f"  Files in checkpoint: {list(restored_state['files'].keys())}")

    restored_master = checkpoint.restore(1)
    print(f"  Restored master has {len(restored_master.files)} file(s)")
    print(f"  Restored chunk counter: {restored_master.chunk_counter}")


def demo_hdfs_write_pipeline():
    print_separator("DEMO 5: HDFS-style write pipeline")

    master = MasterServer()
    servers = [
        ChunkServer("DN-1", "rack-1"),
        ChunkServer("DN-2", "rack-1"),
        ChunkServer("DN-3", "rack-2"),
        ChunkServer("DN-4", "rack-2"),
        ChunkServer("DN-5", "rack-3"),
    ]
    for s in servers:
        master.register_chunk_server(s)

    print("HDFS Write Pipeline Simulation:")
    print()

    client = GFSClient(master)
    client.create_file("/output/map-reduce-result.bin")

    chunk_id = master.allocate_chunk("/output/map-reduce-result.bin", 0)
    locs = master.get_all_locations(chunk_id)
    primary = master.get_primary(chunk_id)

    pipeline_str = " -> ".join(locs)
    print(f"  Block {chunk_id} assigned to pipeline: {pipeline_str}")
    print(f"  Primary (first in pipeline): {primary}")
    print()

    block_data = b"MapReduce output block " + b"X" * 200
    print(f"  Client sends {len(block_data)} bytes to {locs[0]} (pipeline start)")
    print(f"  {locs[0]} writes locally and forwards to {locs[1]}")
    print(f"  {locs[1]} writes locally and forwards to {locs[2]}")
    print(f"  {locs[2]} writes locally and sends ACK upstream")
    print(f"  ACK flows back: {locs[2]} -> {locs[1]} -> {locs[0]} -> Client")
    print()

    client.write("/output/map-reduce-result.bin", block_data)

    for loc in locs:
        s = master.chunk_servers[loc]
        ck = s.checksum(chunk_id)
        print(f"  {loc}: size={s.chunk_size(chunk_id)}, checksum={ck[:16]}...")

    all_same = len(set(
        master.chunk_servers[loc].checksum(chunk_id) for loc in locs
    )) == 1
    print(f"\n  All replicas identical: {all_same}")

    read_result = client.read("/output/map-reduce-result.bin")
    assert read_result == block_data
    print(f"  Read verification: PASSED ({len(read_result)} bytes)")


def demo_gfs_vs_hdfs_vs_modern():
    print_separator("DEMO 6: GFS vs HDFS vs Modern Object Stores")

    comparison = [
        ("Architecture", "GFS (2003)", "HDFS (2008)", "S3/GCS/Azure Blob (2024)"),
        ("Master/NameNode", "Single master + shadow", "Single NameNode + Secondary NN", "Distributed metadata (Spanner, etc.)"),
        ("Block size", "64 MB", "128 MB (configurable)", "Variable (no fixed block)"),
        ("Replication", "3x (configurable)", "3x (configurable)", "Erasure coding (6+3 = 1.5x)"),
        ("Consistency model", "Relaxed (duplicates, gaps)", "Strong for create-then-read", "Strong (read-after-write)"),
        ("Append model", "Record append (at-least-once)", "Append-only (create-then-seek)", "Multipart upload (at-most-once)"),
        ("Failure domain", "Commodity servers", "Commodity servers", "Multi-AZ, multi-region"),
        ("Metadata store", "Single master RAM", "Single NN RAM + disk", "Distributed KV store"),
        ("Typical workload", "Sequential read, batch write", "MapReduce sequential scan", "Random read/write, analytics, ML"),
        ("SPOF", "Yes (master)", "Yes (NameNode)", "No (distributed metadata)"),
        ("Checkpoint", "Operation log replay", "fsimage + edits log + Secondary NN", "Not applicable (distributed)"),
    ]

    col_widths = [max(len(row[i]) for row in comparison) for i in range(4)]
    for row in comparison:
        print(
            f"  {row[0]:<{col_widths[0]}}  "
            f"{row[1]:<{col_widths[1]}}  "
            f"{row[2]:<{col_widths[2]}}  "
            f"{row[3]:<{col_widths[3]}}"
        )


if __name__ == "__main__":
    demo_chunk_distribution()
    demo_read_from_nearest()
    demo_fault_tolerance()
    demo_append_and_checkpoint()
    demo_hdfs_write_pipeline()
    demo_gfs_vs_hdfs_vs_modern()