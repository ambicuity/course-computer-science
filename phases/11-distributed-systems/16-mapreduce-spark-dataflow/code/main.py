"""
MapReduce, Spark, Dataflow — Phase 11

A simplified MapReduce framework with mapper, combiner, shuffle, reducer,
and fault tolerance simulation.
"""

from __future__ import annotations
import hashlib
from collections import defaultdict
from typing import Callable, Any

KeyValue = tuple[Any, Any]
MapFunc = Callable[[str, str], list[KeyValue]]
ReduceFunc = Callable[[Any, list[Any]], Any]
CombineFunc = Callable[[Any, list[Any]], Any] | None


def partition_key(key: Any, num_reducers: int) -> int:
    h = hashlib.md5(str(key).encode()).hexdigest()
    return int(h, 16) % num_reducers


class Mapper:
    def __init__(self, map_func: MapFunc):
        self.map_func = map_func

    def run(self, chunk: list[str]) -> list[KeyValue]:
        results: list[KeyValue] = []
        for line in chunk:
            if not line.strip():
                continue
            results.extend(self.map_func("", line))
        return results


class Shuffle:
    def __init__(self, num_reducers: int):
        self.num_reducers = num_reducers

    def partition(self, pairs: list[KeyValue]) -> dict[int, list[KeyValue]]:
        buckets: dict[int, list[KeyValue]] = {i: [] for i in range(self.num_reducers)}
        for key, value in pairs:
            bucket = partition_key(key, self.num_reducers)
            buckets[bucket].append((key, value))
        return buckets

    def group(self, buckets: dict[int, list[KeyValue]]) -> dict[int, dict[Any, list[Any]]]:
        grouped: dict[int, dict[Any, list[Any]]] = {}
        for bucket_id, pairs in buckets.items():
            groups: dict[Any, list[Any]] = defaultdict(list)
            for key, value in pairs:
                groups[key].append(value)
            grouped[bucket_id] = dict(groups)
        return grouped


class Combiner:
    def __init__(self, combine_func: CombineFunc):
        self.combine_func = combine_func

    def run(self, pairs: list[KeyValue]) -> list[KeyValue]:
        if self.combine_func is None:
            return pairs
        grouped: dict[Any, list[Any]] = defaultdict(list)
        for key, value in pairs:
            grouped[key].append(value)
        results: list[KeyValue] = []
        for key, values in grouped.items():
            results.append((key, self.combine_func(key, values)))
        return results


class Reducer:
    def __init__(self, reduce_func: ReduceFunc):
        self.reduce_func = reduce_func

    def run(self, grouped: dict[Any, list[Any]]) -> list[KeyValue]:
        results: list[KeyValue] = []
        for key in sorted(grouped.keys()):
            values = grouped[key]
            results.append((key, self.reduce_func(key, values)))
        return results


class MapReduceJob:
    def __init__(
        self,
        map_func: MapFunc,
        reduce_func: ReduceFunc,
        num_mappers: int = 4,
        num_reducers: int = 2,
        combiner_func: CombineFunc = None,
    ):
        self.map_func = map_func
        self.reduce_func = reduce_func
        self.num_mappers = num_mappers
        self.num_reducers = num_reducers
        self.combiner_func = combiner_func

    def split_input(self, data: str) -> list[list[str]]:
        lines = data.split("\n")
        chunk_size = max(1, len(lines) // self.num_mappers)
        if len(lines) % self.num_mappers != 0:
            chunk_size += 1
        chunks: list[list[str]] = []
        for i in range(0, len(lines), chunk_size):
            chunks.append(lines[i : i + chunk_size])
        while len(chunks) < self.num_mappers:
            chunks.append([])
        return chunks

    def run(self, data: str) -> list[KeyValue]:
        mapper = Mapper(self.map_func)
        shuffle = Shuffle(self.num_reducers)
        combiner = Combiner(self.combiner_func)
        reducer = Reducer(self.reduce_func)

        chunks = self.split_input(data)

        all_intermediate: list[KeyValue] = []
        for chunk in chunks:
            pairs = mapper.run(chunk)
            if self.combiner_func:
                pairs = combiner.run(pairs)
                all_intermediate.extend(pairs)
            else:
                all_intermediate.extend(pairs)

        buckets = shuffle.partition(all_intermediate)
        grouped = shuffle.group(buckets)

        results: list[KeyValue] = []
        for bucket_id in range(self.num_reducers):
            if bucket_id in grouped:
                bucket_results = reducer.run(grouped[bucket_id])
                results.extend(bucket_results)

        results.sort(key=lambda kv: str(kv[0]))
        return results

    def run_with_failure(
        self, data: str, fail_mapper_index: int | None = None
    ) -> tuple[list[KeyValue], dict[str, int]]:
        """
        Run MapReduce with simulated mapper failure.
        If fail_mapper_index is set, that mapper is 'killed' and re-executed.
        Returns (results, stats) where stats includes shuffle traffic info.
        """
        mapper = Mapper(self.map_func)
        shuffle = Shuffle(self.num_reducers)
        combiner = Combiner(self.combiner_func)
        reducer = Reducer(self.reduce_func)

        chunks = self.split_input(data)
        stats: dict[str, int] = {
            "total_map_pairs": 0,
            "post_combiner_pairs": 0,
            "shuffle_pairs": 0,
        }

        all_intermediate: list[KeyValue] = []
        for i, chunk in enumerate(chunks):
            pairs = mapper.run(chunk)
            stats["total_map_pairs"] += len(pairs)

            if fail_mapper_index == i:
                pairs_after_failure = mapper.run(chunk)
                pairs = pairs_after_failure

            if self.combiner_func:
                pairs = combiner.run(pairs)
            stats["post_combiner_pairs"] += len(pairs) if self.combiner_func else 0
            stats["shuffle_pairs"] += len(pairs)
            all_intermediate.extend(pairs)

        buckets = shuffle.partition(all_intermediate)
        grouped = shuffle.group(buckets)

        results: list[KeyValue] = []
        for bucket_id in range(self.num_reducers):
            if bucket_id in grouped:
                bucket_results = reducer.run(grouped[bucket_id])
                results.extend(bucket_results)

        results.sort(key=lambda kv: str(kv[0]))
        return results, stats


# ── Word Count ────────────────────────────────────────────────────────

def word_count_map(_, line: str) -> list[KeyValue]:
    import re
    words = re.findall(r"[a-zA-Z]+", line.lower())
    return [(w, 1) for w in words]


def word_count_reduce(key: str, values: list[int]) -> int:
    return sum(values)


# ── Inverted Index ────────────────────────────────────────────────────

def inverted_index_map(_, line: str) -> list[KeyValue]:
    import re
    parts = line.strip().split("\t", 1)
    if len(parts) != 2:
        return []
    doc_id, text = parts
    words = re.findall(r"[a-zA-Z]+", text.lower())
    return [(w, doc_id) for w in words]


def inverted_index_reduce(key: str, values: list[str]) -> list[str]:
    return sorted(set(values))


# ── Demos ─────────────────────────────────────────────────────────────

def section(title: str) -> None:
    print(f"\n{'=' * 65}")
    print(f"  {title}")
    print(f"{'=' * 65}")


def demo_word_count() -> None:
    section("Word Count — Basic MapReduce")

    text = (
        "the quick brown fox jumps over the lazy dog\n"
        "the fox was quick and the dog was lazy\n"
        "a quick brown fox and a lazy dog"
    )

    job = MapReduceJob(word_count_map, word_count_reduce, num_mappers=2, num_reducers=2)
    results = job.run(text)

    print(f"Input:\n{text}\n")
    print("Word counts:")
    for word, count in results:
        print(f"  {word}: {count}")


def demo_word_count_with_combiner() -> None:
    section("Word Count — With Combiner (Reduced Shuffle Traffic)")

    text = (
        "the quick brown fox jumps over the lazy dog\n"
        "the fox was quick and the dog was lazy\n"
        "a quick brown fox and a lazy dog"
    )

    job_no_combiner = MapReduceJob(
        word_count_map, word_count_reduce, num_mappers=2, num_reducers=2
    )
    _, stats_no = job_no_combiner.run_with_failure(text)

    job_combiner = MapReduceJob(
        word_count_map,
        word_count_reduce,
        num_mappers=2,
        num_reducers=2,
        combiner_func=word_count_reduce,
    )
    _, stats_yes = job_combiner.run_with_failure(text)

    print(f"Without combiner: {stats_no['shuffle_pairs']} pairs shuffled")
    print(f"With combiner:    {stats_yes['shuffle_pairs']} pairs shuffled")
    reduction = stats_no["shuffle_pairs"] - stats_yes["shuffle_pairs"]
    print(f"Reduction:        {reduction} pairs saved ({100 * reduction // stats_no['shuffle_pairs']}%)")


def demo_fault_tolerance() -> None:
    section("Fault Tolerance — Mapper Re-execution")

    text = (
        "hello world\n"
        "hello mapreduce\n"
        "world of distributed systems"
    )

    job = MapReduceJob(word_count_map, word_count_reduce, num_mappers=2, num_reducers=2)

    results_normal, _ = job.run_with_failure(text)
    results_with_failure, _ = job.run_with_failure(text, fail_mapper_index=0)

    print("Normal execution results:")
    for word, count in results_normal:
        print(f"  {word}: {count}")

    print("\nWith mapper-0 failure and re-execution:")
    for word, count in results_with_failure:
        print(f"  {word}: {count}")

    assert results_normal == results_with_failure, "Results differ after fault recovery!"
    print("\nResults match: deterministic mapper re-execution produces identical output.")


def demo_inverted_index() -> None:
    section("Inverted Index MapReduce")

    documents = (
        "doc1\tthe quick brown fox\n"
        "doc2\tthe lazy dog sleeps\n"
        "doc3\tthe quick fox and the dog"
    )

    job = MapReduceJob(
        inverted_index_map,
        inverted_index_reduce,
        num_mappers=2,
        num_reducers=2,
    )
    results = job.run(documents)

    print("Input documents:")
    for line in documents.strip().split("\n"):
        print(f"  {line}")

    print("\nInverted index:")
    for word, doc_ids in results:
        print(f"  {word}: {doc_ids}")


def demo_partitioning() -> None:
    section("Partitioning — Hash-Based Key Distribution")

    keys = ["apple", "banana", "cherry", "date", "elderberry", "fig", "grape"]
    num_reducers = 3

    print(f"Distributing {len(keys)} keys across {num_reducers} reducers:")
    buckets: dict[int, list[str]] = defaultdict(list)
    for key in keys:
        bucket = partition_key(key, num_reducers)
        buckets[bucket].append(key)

    for bucket_id in range(num_reducers):
        keys_in_bucket = buckets.get(bucket_id, [])
        print(f"  Reducer {bucket_id}: {keys_in_bucket}")


def demo_spark_lineage() -> None:
    section("Spark RDD Lineage — Lazy Transformations")

    lines = ["hello world", "hello spark", "world of data"]

    words = []
    for line in lines:
        words.extend(line.split())

    pairs = [(w, 1) for w in words]

    grouped: dict[str, list[int]] = defaultdict(list)
    for w, c in pairs:
        grouped[w].append(c)

    counts = {w: sum(cs) for w, cs in grouped.items()}

    print("RDD lineage (word count):")
    print("  1. textFile()         → load data")
    print("  2. flatMap(split)     → transformation (lazy)")
    print("  3. map((w,1))         → transformation (lazy)")
    print("  4. reduceByKey(+)     → transformation (lazy, wide dependency)")
    print("  5. collect()          → action (triggers execution)")
    print()
    print("Lineage graph preserved for fault recovery:")
    print("  textFile → flatMap → map → reduceByKey → collect")
    print()

    print("Word counts (computed via lineage):")
    for w in sorted(counts.keys()):
        print(f"  {w}: {counts[w]}")

    print()
    print("Comparison with MapReduce:")
    print("  MapReduce: Each step materializes to disk")
    print("  Spark:     Steps compose in memory, only collect() triggers execution")
    print("  Spark:     If a partition is lost, replay lineage from nearest checkpoint")


def demo_narrow_vs_wide() -> None:
    section("Narrow vs Wide Dependencies")

    print("Narrow dependency (1:1 — no shuffle):")
    print("  map(partition_0)      → partition_0'")
    print("  filter(partition_1)  → partition_1'")
    print("  flatMap(partition_2) → partition_2'")
    print("  → Can pipeline: map → filter → flatMap in ONE stage")
    print()

    print("Wide dependency (N:M — shuffle required):")
    print("  groupByKey:  ALL partitions → hash → new partitions")
    print("  join:        BOTH inputs → shuffle → new partitions")
    print("  sort:        ALL partitions → range-partition → sorted partitions")
    print("  → Must split into separate stages at each wide dependency")
    print()

    print("Stage boundaries in WordCount:")
    print("  Stage 1: textFile → flatMap → map       (all narrow)")
    print("  Stage 2: reduceByKey                    (wide — shuffle boundary)")
    print("  Stage 3: collect                         (action)")


def demo_dataflow_model() -> None:
    section("Dataflow/Beam Model — Windowing and Watermarks")

    print("Batch vs Streaming processing models:")
    print()
    print("  MapReduce:   Bounded data → read all → process → write all")
    print("  Spark:      Bounded data + iterative: keep RDDs in memory")
    print("  Dataflow:   Unbounded data + windowing + triggers + watermarks")
    print()

    events = [
        ("user1", "click",  0),
        ("user2", "click",  2),
        ("user1", "click",  4),
        ("user3", "click",  6),
        ("user2", "click",  8),
        ("user1", "click",  9),
        ("user4", "click",  11),
        ("user1", "click",  14),
    ]

    print("Events (user, action, event_time):")
    for user, action, t in events:
        print(f"  t={t:2d}: {user} {action}")

    print()
    window_size = 5
    print(f"Fixed windows (size={window_size}s):")
    windows: dict[tuple[int, int], list[tuple[str, str, int]]] = defaultdict(list)
    for user, action, t in events:
        w_start = (t // window_size) * window_size
        w_end = w_start + window_size
        windows[(w_start, w_end)].append((user, action, t))

    for (w_start, w_end), ws_events in sorted(windows.items()):
        print(f"  [{w_start}, {w_end}): {len(ws_events)} events")

    print()
    print("Watermark at t=7: all events ≤ t=7 are considered arrived")
    print("  Late event at t=4 arriving after watermark: handled by allowed lateness")
    print()
    print("Trigger types:")
    print("  - Event time trigger: fire when watermark passes window end")
    print("  - Processing time trigger: fire after N seconds of processing time")
    print("  - Count trigger: fire after N elements")
    print("  - Composite: early firings + final firing at watermark + late firings")


def demo_comparison_table() -> None:
    section("System Comparison")

    print(f"{'Property':<25} {'MapReduce':<20} {'Spark':<20} {'Dataflow/Beam':<20}")
    print("-" * 85)
    rows = [
        ("Model", "Batch only", "Batch + iterative", "Batch + streaming"),
        ("Intermediate data", "Disk", "Memory (lineage)", "Memory (windowed)"),
        ("Iterative workloads", "Poor (I/O/iter)", "Excellent (cache)", "Good (state)"),
        ("Fault tolerance", "Re-execute tasks", "Lineage recompute", "Checkpoint+replay"),
        ("Latency", "Minutes", "Seconds", "ms to seconds"),
        ("Streaming", "No", "Micro-batches", "True streaming"),
        ("Programming model", "Map + Reduce", "Transform + Action", "ParDo+GBK+Window"),
    ]
    for prop, mr, spark, df in rows:
        print(f"{prop:<25} {mr:<20} {spark:<20} {df:<20}")


def main() -> None:
    demo_word_count()
    demo_word_count_with_combiner()
    demo_fault_tolerance()
    demo_inverted_index()
    demo_partitioning()
    demo_spark_lineage()
    demo_narrow_vs_wide()
    demo_dataflow_model()
    demo_comparison_table()


if __name__ == "__main__":
    main()