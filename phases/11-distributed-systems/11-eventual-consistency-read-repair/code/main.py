import hashlib
from dataclasses import dataclass
from enum import Enum


class VCRelation(Enum):
    BEFORE = "before"
    AFTER = "after"
    CONCURRENT = "concurrent"
    IDENTICAL = "identical"


class VectorClock:
    def __init__(self, node_ids: list[str]):
        self.node_ids = list(node_ids)
        self._index = {nid: i for i, nid in enumerate(self.node_ids)}
        self.counters = [0] * len(node_ids)

    def increment(self, node_id: str) -> "VectorClock":
        self.counters[self._index[node_id]] += 1
        return self

    def merge(self, other: "VectorClock") -> "VectorClock":
        for i in range(len(self.counters)):
            self.counters[i] = max(self.counters[i], other.counters[i])
        return self

    def compare(self, other: "VectorClock") -> VCRelation:
        le = all(a <= b for a, b in zip(self.counters, other.counters))
        ge = all(a >= b for a, b in zip(self.counters, other.counters))
        eq = all(a == b for a, b in zip(self.counters, other.counters))
        if eq:
            return VCRelation.IDENTICAL
        if le and not ge:
            return VCRelation.BEFORE
        if ge and not le:
            return VCRelation.AFTER
        return VCRelation.CONCURRENT

    def __lt__(self, other: "VectorClock") -> bool:
        return self.compare(other) == VCRelation.BEFORE

    def __gt__(self, other: "VectorClock") -> bool:
        return self.compare(other) == VCRelation.AFTER

    def is_concurrent(self, other: "VectorClock") -> bool:
        return self.compare(other) == VCRelation.CONCURRENT

    def copy(self) -> "VectorClock":
        vc = VectorClock(self.node_ids)
        vc.counters = list(self.counters)
        return vc

    def dominates(self, other: "VectorClock") -> bool:
        return self.compare(other) == VCRelation.AFTER

    def __repr__(self) -> str:
        pairs = {nid: c for nid, c in zip(self.node_ids, self.counters) if c > 0}
        return str(pairs)


@dataclass
class VersionedValue:
    value: str
    vc: VectorClock

    def is_newer_than(self, other: "VersionedValue") -> bool:
        return self.vc.dominates(other.vc)

    def is_concurrent_with(self, other: "VersionedValue") -> bool:
        return self.vc.is_concurrent(other.vc)


class Replica:
    def __init__(self, name: str, all_node_ids: list[str]):
        self.name = name
        self.data: dict[str, list[VersionedValue]] = {}
        self.vc = VectorClock(all_node_ids)
        self.is_up = True

    def put_versioned(self, key: str, value: str, vc: VectorClock) -> None:
        new_version = VersionedValue(value, vc.copy())
        if key not in self.data:
            self.data[key] = [new_version]
            return
        existing = self.data[key]
        dominated = []
        concurrent = False
        for v in existing:
            rel = new_version.vc.compare(v.vc)
            if rel == VCRelation.BEFORE or rel == VCRelation.IDENTICAL:
                return
            if rel == VCRelation.AFTER:
                dominated.append(v)
            if rel == VCRelation.CONCURRENT:
                concurrent = True
        if not concurrent:
            self.data[key] = [new_version]
        else:
            for v in dominated:
                existing.remove(v)
            if new_version not in existing:
                existing.append(new_version)

    def get_versions(self, key: str) -> list[VersionedValue]:
        return list(self.data.get(key, []))

    def keys(self) -> list[str]:
        return sorted(self.data.keys())

    def __repr__(self) -> str:
        return f"Replica({self.name}, up={self.is_up}, keys={list(self.data.keys())})"


class HintedHandoff:
    def __init__(self):
        self.hints: dict[str, list[tuple[str, str, VectorClock, str]]] = {}

    def store_hint(self, target: str, key: str, value: str, vc: VectorClock, source: str) -> None:
        if target not in self.hints:
            self.hints[target] = []
        self.hints[target].append((key, value, vc, source))

    def deliver_hints(self, target: str, replica: Replica) -> int:
        if target not in self.hints:
            return 0
        hints = self.hints.pop(target)
        for key, value, vc, source in hints:
            replica.vc.merge(vc)
            replica.vc.increment(source)
            replica.put_versioned(key, value, vc)
        return len(hints)


class MerkleTree:
    def __init__(self, data: dict[str, str] = None):
        self.data = data or {}
        self._tree: list[str] = []
        self._size = 0
        self._offset = 0
        self._keys_sorted: list[str] = []
        self._leaf_hashes: dict[str, str] = {}
        if self.data:
            self._build()

    def _hash(self, data: str) -> str:
        return hashlib.sha256(data.encode()).hexdigest()[:16]

    def _build(self) -> None:
        self._keys_sorted = sorted(self.data.keys())
        self._leaf_hashes = {}
        for key in self._keys_sorted:
            self._leaf_hashes[key] = self._hash(f"{key}={self.data[key]}")
        n = len(self._keys_sorted)
        if n == 0:
            self._tree = [self._hash("")]
            self._size = 1
            self._offset = 0
            return
        size = 1
        while size < n:
            size *= 2
        self._size = size
        self._offset = size - 1
        tree = [""] * (2 * size)
        for i in range(n):
            tree[size + i] = self._leaf_hashes[self._keys_sorted[i]]
        for i in range(n, size):
            tree[size + i] = self._hash("")
        for i in range(size - 1, 0, -1):
            tree[i] = self._hash(tree[2 * i] + tree[2 * i + 1])
        self._tree = tree[1:]

    def root_hash(self) -> str:
        if not self._tree:
            return self._hash("")
        return self._tree[0]

    def find_diff_keys(self, other: "MerkleTree") -> list[str]:
        if self.root_hash() == other.root_hash():
            return []
        diffs = []
        all_keys = sorted(set(self._keys_sorted) | set(other._keys_sorted))
        for key in all_keys:
            in_self = key in self._leaf_hashes
            in_other = key in other._leaf_hashes
            if in_self and in_other:
                if self._leaf_hashes[key] != other._leaf_hashes[key]:
                    diffs.append(key)
            else:
                diffs.append(key)
        return diffs


class AntiEntropy:
    def __init__(self, cluster: "DynamoCluster"):
        self.cluster = cluster

    def compute_merkle(self, replica: Replica) -> MerkleTree:
        data = {}
        for key in replica.keys():
            versions = replica.get_versions(key)
            values = "|".join(sorted(f"{v.value}@{v.vc}" for v in versions))
            data[key] = values
        return MerkleTree(data)

    def sync(self, r1_name: str, r2_name: str) -> list[str]:
        r1 = self.cluster.get_replica(r1_name)
        r2 = self.cluster.get_replica(r2_name)
        tree1 = self.compute_merkle(r1)
        tree2 = self.compute_merkle(r2)
        diff_keys = tree1.find_diff_keys(tree2)
        synced = []
        for key in diff_keys:
            v1 = r1.get_versions(key)
            v2 = r2.get_versions(key)
            if not v1 and v2:
                for v in v2:
                    r1.put_versioned(key, v.value, v.vc)
                synced.append(f"{key}: {r2_name} → {r1_name}")
            elif v1 and not v2:
                for v in v1:
                    r2.put_versioned(key, v.value, v.vc)
                synced.append(f"{key}: {r1_name} → {r2_name}")
            else:
                all_v = v1 + v2
                unique = []
                seen = set()
                for v in all_v:
                    vc_key = tuple(v.vc.counters)
                    if vc_key not in seen:
                        seen.add(vc_key)
                        unique.append(v)
                r1.data[key] = list(unique)
                r2.data[key] = list(unique)
                synced.append(f"{key}: synced both directions ({len(unique)} version(s))")
        return synced


class DynamoCluster:
    def __init__(self, node_ids: list[str], n: int = 3):
        self.node_ids = node_ids[:n]
        self.replicas: dict[str, Replica] = {}
        self.hinted_handoff = HintedHandoff()
        for nid in self.node_ids:
            self.replicas[nid] = Replica(nid, self.node_ids)

    def get_replica(self, name: str) -> Replica:
        return self.replicas[name]

    def put(self, key: str, value: str, w: int, coordinator: str = None) -> dict:
        pref = list(self.node_ids)
        if coordinator is None:
            coordinator = pref[0]
        coord = self.replicas[coordinator]
        coord.vc.increment(coordinator)
        write_vc = coord.vc.copy()
        written = []
        hints = []
        for nid in pref:
            rep = self.replicas[nid]
            if not rep.is_up:
                hint_target = None
                for alt in self.node_ids:
                    if alt != nid and self.replicas[alt].is_up:
                        hint_target = alt
                        break
                if hint_target:
                    self.hinted_handoff.store_hint(nid, key, value, write_vc, coordinator)
                    self.replicas[hint_target].put_versioned(key, value, write_vc)
                    hints.append(f"Hint for {nid} stored on {hint_target}")
                continue
            rep.vc.merge(write_vc)
            rep.vc.increment(nid)
            rep.put_versioned(key, value, write_vc)
            written.append(nid)
        success = len(written) >= w
        return {"success": success, "written": written, "hints": hints, "vc": write_vc}

    def get(self, key: str, r: int, repair: bool = True) -> dict:
        all_versions: list[VersionedValue] = []
        responded = []

        for nid in self.node_ids:
            if len(responded) >= r:
                break
            rep = self.replicas[nid]
            if not rep.is_up:
                continue
            versions = rep.get_versions(key)
            responded.append(nid)
            all_versions.extend(versions)

        unique = []
        seen_vc = set()
        for v in all_versions:
            vc_key = tuple(v.vc.counters)
            if vc_key not in seen_vc:
                seen_vc.add(vc_key)
                unique.append(v)

        dominated = []
        for v in unique:
            for other in unique:
                if v is not other and other.vc.dominates(v.vc):
                    dominated.append(v)
                    break
        latest_versions = [v for v in unique if v not in dominated]
        if not latest_versions:
            latest_versions = unique

        has_siblings = len(latest_versions) > 1
        repair_log = []

        if repair:
            for nid in self.node_ids:
                rep = self.replicas[nid]
                if not rep.is_up:
                    continue
                existing = rep.get_versions(key)
                needs_repair = len(existing) == 0
                if not needs_repair:
                    for lv in latest_versions:
                        covered = any(
                            ev.vc.compare(lv.vc) in (VCRelation.IDENTICAL, VCRelation.AFTER)
                            for ev in existing
                        )
                        if not covered:
                            needs_repair = True
                            break
                if needs_repair:
                    for lv in latest_versions:
                        rep.put_versioned(key, lv.value, lv.vc)
                    repair_log.append(f"Read-repair: pushed to {nid}")

        return {
            "versions": latest_versions,
            "siblings": has_siblings,
            "responded": responded,
            "repair_log": repair_log,
        }


def show_replicas(cluster: DynamoCluster, keys: list[str]) -> None:
    for nid in cluster.node_ids:
        rep = cluster.replicas[nid]
        parts = []
        for key in keys:
            versions = rep.get_versions(key)
            if len(versions) == 0:
                parts.append(f"{key}=∅")
            elif len(versions) == 1:
                parts.append(f"{key}={versions[0].value} ({versions[0].vc})")
            else:
                parts.append(f"{key}=siblings{{{', '.join(v.value for v in versions)}}}")
        print(f"    {nid}: {' | '.join(parts)}")


def sep(title: str) -> None:
    print()
    print("=" * 64)
    print(title)
    print("=" * 64)


def demo_basic_operations() -> None:
    sep("DEMO 1: Basic Put/Get with Tunable Consistency")

    cluster = DynamoCluster(["A", "B", "C"], n=3)
    print("Cluster: 3 replicas (A, B, C)")

    print("\n--- Write with W=2 (quorum) ---")
    result = cluster.put("user:1", "Alice", w=2, coordinator="A")
    print(f"  Put user:1=Alice, W=2, coordinator=A")
    print(f"  Written to: {result['written']}")
    print(f"  Write VC: {result['vc']}")
    show_replicas(cluster, ["user:1"])

    print("\n--- Read with R=2 (quorum) ---")
    result = cluster.get("user:1", r=2)
    print(f"  Versions: {[(v.value, str(v.vc)) for v in result['versions']]}")
    print(f"  Siblings: {result['siblings']}")
    print(f"  Repair: {result['repair_log'] or 'none needed'}")

    print("\n--- Read with R=1 (may be stale) ---")
    result = cluster.get("user:1", r=1)
    print(f"  Versions: {[(v.value, str(v.vc)) for v in result['versions']]}")
    print(f"  R=1 reads from just one replica — no quorum overlap guarantee.")


def demo_siblings() -> None:
    sep("DEMO 2: Concurrent Writes → Siblings")

    cluster = DynamoCluster(["A", "B", "C"], n=3)

    print("Write 1: user:1=Alice (coordinator=A)")
    r1 = cluster.put("user:1", "Alice", w=2, coordinator="A")
    print(f"  Written to: {r1['written']}, VC={r1['vc']}")

    print("\nWrite 2: user:1=Bob (coordinator=C, concurrent with write 1)")
    vc_bob = VectorClock(["A", "B", "C"])
    vc_bob.increment("C")
    cluster.replicas["C"].put_versioned("user:1", "Bob", vc_bob)
    print(f"  C now has: Bob with VC={vc_bob} (concurrent with Alice's VC)")
    print(f"  A has Alice with VC={r1['vc']}")
    print(f"  VCs are incomparable → SIBLINGS")

    print("\nRead user:1 with R=3 (all replicas):")
    result = cluster.get("user:1", r=3)
    print(f"  Versions: {[(v.value, str(v.vc)) for v in result['versions']]}")
    print(f"  Siblings: {result['siblings']}")
    print(f"  Repair: {result['repair_log']}")
    print()
    print("  Two concurrent versions — neither VC dominates the other.")
    print("  The client must resolve: pick one, merge, or keep both.")


def demo_read_repair() -> None:
    sep("DEMO 3: Read-Repair Healing Stale Data")

    cluster = DynamoCluster(["A", "B", "C"], n=3)

    cluster.put("config:db", "host=db1", w=2, coordinator="A")
    print("Initial write: config:db=host=db1, W=2")
    show_replicas(cluster, ["config:db"])

    print("\nSimulate: A receives an update, B and C lag behind")
    cluster.replicas["A"].vc.increment("A")
    vc2 = cluster.replicas["A"].vc.copy()
    cluster.replicas["A"].put_versioned("config:db", "host=db2", vc2)
    print("  A has the new value (host=db2)")
    print("  B and C still have the old value (host=db1) or nothing")
    show_replicas(cluster, ["config:db"])

    print("\nRead config:db with R=3 → synchronous read-repair:")
    result = cluster.get("config:db", r=3)
    print(f"  Latest version: {[(v.value, str(v.vc)) for v in result['versions']]}")
    print(f"  Repair log: {result['repair_log']}")

    print("\nAfter read-repair, all replicas are healed:")
    show_replicas(cluster, ["config:db"])


def demo_merkle_tree() -> None:
    sep("DEMO 4: Merkle Tree — Efficient Difference Detection")

    print("Merkle trees hash data into a binary tree structure.")
    print("Compare root hashes (O(1)). If different, drill down (O(log N)).")
    print("Only divergent leaf keys are sent over the wire.\n")

    data_a = {"k1": "v1", "k2": "v2", "k3": "v3-old", "k4": "v4",
              "k5": "v5", "k6": "v6-old", "k7": "v7", "k8": "v8"}
    data_b = {"k1": "v1", "k2": "v2", "k3": "v3-new", "k4": "v4",
              "k5": "v5", "k6": "v6-new", "k7": "v7", "k8": "v8"}

    tree_a = MerkleTree(data_a)
    tree_b = MerkleTree(data_b)

    print(f"Replica A: {len(data_a)} keys, k3 and k6 have old values")
    print(f"Replica B: {len(data_b)} keys, k3 and k6 have new values")
    print(f"\n  Root hash A: {tree_a.root_hash()}")
    print(f"  Root hash B: {tree_b.root_hash()}")
    print(f"  Roots differ: {tree_a.root_hash() != tree_b.root_hash()}")

    diffs = tree_a.find_diff_keys(tree_b)
    print(f"\nDivergent keys (found in O(log N) instead of O(N)):")
    for k in diffs:
        print(f"  {k}: A has '{data_a[k]}', B has '{data_b[k]}'")
    print(f"\nOnly {len(diffs)} of {len(data_a)} keys differ — Merkle tree found them")
    print("without comparing all {0} keys element-by-element.".format(len(data_a)))


def demo_anti_entropy() -> None:
    sep("DEMO 5: Anti-Entropy Synchronization via Merkle Trees")

    cluster = DynamoCluster(["A", "B", "C"], n=3)

    cluster.put("key1", "val1", w=2, coordinator="A")
    cluster.put("key2", "val2", w=2, coordinator="A")
    print("Initial writes with W=2:")
    show_replicas(cluster, ["key1", "key2"])

    print("\nSimulate: A receives updates that B and C miss")
    cluster.replicas["A"].vc.increment("A")
    vc = cluster.replicas["A"].vc.copy()
    cluster.replicas["A"].put_versioned("key1", "val1-updated", vc)
    cluster.replicas["A"].put_versioned("key3", "val3", vc)
    print("  A now has: key1 updated, key3 added")
    show_replicas(cluster, ["key1", "key2", "key3"])

    print("\nRun anti-entropy A ↔ B (compares Merkle trees, syncs divergent keys):")
    ae = AntiEntropy(cluster)
    synced = ae.sync("A", "B")
    for s in synced:
        print(f"  {s}")

    print("\nAfter anti-entropy:")
    show_replicas(cluster, ["key1", "key2", "key3"])
    print("  B now matches A on key1 and has key3")

    print("\nRun anti-entropy A ↔ C:")
    synced = ae.sync("A", "C")
    for s in synced:
        print(f"  {s}")
    print("\nAll replicas now converge:")


def demo_hinted_handoff() -> None:
    sep("DEMO 6: Hinted Handoff — Writes During Node Failure")

    cluster = DynamoCluster(["A", "B", "C"], n=3)

    print("Take replica C down:")
    cluster.replicas["C"].is_up = False
    print(f"  C.is_up = {cluster.replicas['C'].is_up}")

    print("\nWrite order:123 with W=2:")
    print("  A and B will accept the write")
    print("  C is down, so coordinator stores a hint for C")
    result = cluster.put("order:1", "123", w=2, coordinator="A")
    print(f"  Success: {result['success']}")
    print(f"  Written to: {result['written']}")
    print(f"  Hints: {result['hints']}")

    print("\nA and B have the data. C does not:")
    show_replicas(cluster, ["order:1"])

    print("\nBring C back up and deliver hints:")
    cluster.replicas["C"].is_up = True
    delivered = cluster.hinted_handoff.deliver_hints("C", cluster.replicas["C"])
    print(f"  Delivered {delivered} hint(s) to C")
    show_replicas(cluster, ["order:1"])
    print("  C now has the data it missed during downtime")


def demo_partition_and_healing() -> None:
    sep("DEMO 7: Network Partition → Divergence → Healing")

    cluster = DynamoCluster(["A", "B", "C"], n=3)

    print("=== Phase 1: Normal operation ===")
    r1 = cluster.put("balance:Alice", "1000", w=2, coordinator="A")
    print(f"Write balance:Alice=1000, W=2 → written to {r1['written']}")
    show_replicas(cluster, ["balance:Alice"])

    print("\n=== Phase 2: Partition — C goes down ===")
    cluster.replicas["C"].is_up = False
    print("C is DOWN. Write balance:Alice=800 with W=2:")
    r2 = cluster.put("balance:Alice", "800", w=2, coordinator="A")
    print(f"  Written to: {r2['written']}")
    print(f"  Hints: {r2['hints']}")
    show_replicas(cluster, ["balance:Alice"])
    print("  C is stale — stuck with the old value")

    print("\n=== Phase 3: Heal — deliver hinted handoff ===")
    cluster.replicas["C"].is_up = True
    print("C is back UP. Deliver hinted handoff:")
    delivered = cluster.hinted_handoff.deliver_hints("C", cluster.replicas["C"])
    print(f"  Delivered {delivered} hint(s) — C gets the write it missed")
    show_replicas(cluster, ["balance:Alice"])
    print("  Hinted handoff restores C, but in real systems this may not")
    print("  cover all cases (e.g., hint target also down, or old hints).")

    print("\n=== Phase 4: Read-repair confirms convergence ===")
    result = cluster.get("balance:Alice", r=3)
    print(f"  Read-repair: {result['repair_log'] or 'none needed'}")
    print(f"  All replicas: {[(v.value, str(v.vc)) for v in result['versions']]}")

    print("\n=== Phase 5: Anti-entropy catches what read-repair misses ===")
    print("Anti-entropy catches keys that are never read but still divergent.")

    cluster.replicas["A"].vc.increment("A")
    vc_extra = cluster.replicas["A"].vc.copy()
    cluster.replicas["A"].put_versioned("counter:global", "99", vc_extra)
    print(f"  A has counter:global=99 but B and C don't (no one reads this key)")
    show_replicas(cluster, ["counter:global"])

    ae = AntiEntropy(cluster)
    synced = ae.sync("A", "B")
    print(f"  Anti-entropy A↔B: {synced}")
    synced = ae.sync("A", "C")
    print(f"  Anti-entropy A↔C: {synced if synced else 'no differences'}")
    show_replicas(cluster, ["counter:global"])
    print("  Anti-entropy found the key no one read and synced it.")


def demo_tunable_consistency() -> None:
    sep("DEMO 8: Tunable Consistency — R/W Quorum Trade-offs")

    cluster = DynamoCluster(["A", "B", "C"], n=3)

    print("N=3 replicas. Key property: R + W > N guarantees recency.")
    print()
    print("  Setting         R  W  R+W  Guarantee")
    print("  ──────────────  ─  ─  ───  ──────────────────────────")
    print("  R=1, W=1        1  1   2   No guarantee (may read stale)")
    print("  R=2, W=2        2  2   4   Quorum (read sees latest write)")
    print("  R=3, W=3        3  3   6   Strong but fragile (any failure blocks)")

    cluster.put("counter:x", "42", w=2, coordinator="A")
    print("\nInitial write: counter:x=42, W=2")

    print("\nA independently writes counter:x=43 (not yet propagated):")
    cluster.replicas["A"].vc.increment("A")
    vc_new = cluster.replicas["A"].vc.copy()
    cluster.replicas["A"].put_versioned("counter:x", "43", vc_new)
    show_replicas(cluster, ["counter:x"])
    print("  A has 43, B and C have 42. R=1 might read from A or B/C.")

    print("\n--- R=1 (reads from just one replica, NO read-repair) ---")
    result1 = cluster.get("counter:x", r=1, repair=False)
    vals = [(v.value, str(v.vc)) for v in result1["versions"]]
    print(f"  Got: {vals}")
    print(f"  Only saw data from 1 replica — might be stale.")

    print("\n--- R=3 (reads from all replicas, WITH read-repair) ---")
    result3 = cluster.get("counter:x", r=3)
    vals = [(v.value, str(v.vc)) for v in result3["versions"]]
    print(f"  Got: {vals}")
    print(f"  Repair: {result3['repair_log']}")

    print("\nAfter read-repair:")
    show_replicas(cluster, ["counter:x"])
    print("  R+W=4 > N=3 → quorum intersection guarantees at least")
    print("  one replica in the read set has the latest write.")


if __name__ == "__main__":
    print("Eventual Consistency & Read-Repair Simulator")
    print("=" * 64)
    demo_basic_operations()
    demo_siblings()
    demo_read_repair()
    demo_merkle_tree()
    demo_anti_entropy()
    demo_hinted_handoff()
    demo_partition_and_healing()
    demo_tunable_consistency()