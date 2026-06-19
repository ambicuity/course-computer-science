# MapReduce, Spark, Dataflow

> Process data that doesn't fit on one machine — by making parallelism a first-class primitive.

**Type:** Build
**Languages:** Python, Go
**Prerequisites:** Phase 11 lessons 01–15, especially lesson 15 (Distributed File Systems — GFS/HDFS)
**Time:** ~75 minutes

## Learning Objectives

- Explain the MapReduce programming model: map emits intermediate key-value pairs, reduce groups by key and aggregates.
- Describe the shuffle phase and how partitioning by key hash routes intermediate data to the correct reduce worker.
- Understand combiners as map-side local aggregation that reduces network traffic.
- Explain why MapReduce materializes intermediate results to disk and why this makes iterative algorithms (ML, graph) inefficient.
- Describe Spark's RDD abstraction: immutable, partitioned, lineage-based recovery, lazy transformations vs eager actions.
- Distinguish narrow dependencies (1:1, no shuffle) from wide dependencies (N:M, requires shuffle) and how they define stage boundaries.
- Explain the Dataflow/Beam model: unified batch + stream, windowing, watermarks, triggers, and allowed lateness.
- Compare MapReduce (batch, disk-based), Spark (batch + iterative, memory), and Dataflow/Beam (batch + streaming, windowed) trade-offs.
- Build a simplified MapReduce framework in Python and Go that supports map, shuffle, reduce, combiners, and fault tolerance.

## The Problem

You have 500 GB of web server logs spread across 100 machines. You need to count how many times each URL was accessed. You could write a script that reads each file, tallies counts, and merges the results — but that script must handle machine failures, network partitions, slow stragglers, and data skew (some URLs are orders of magnitude more popular than others).

Before MapReduce, every large-scale data processing job required custom distributed code: split the input, distribute work, handle failures, collect results. The code for failure handling dwarfed the actual computation. Google's insight in 2004 was to extract a recurring pattern — apply a function to each piece of data, then group and aggregate — and make the framework handle everything else.

This lesson builds that pattern from scratch: MapReduce for batch processing, Spark for iterative workloads, and the Dataflow/Beam model for unified batch and stream processing.

## The Concept

### MapReduce

The MapReduce model has two user-supplied functions:

```
map(key, value)      → list of (intermediate_key, intermediate_value)
reduce(key, list_of_values) → result_value
```

The framework handles everything else:

```
Input files
    │
    ▼
┌──────────┐    ┌──────────┐    ┌──────────┐
│ Mapper 0 │    │ Mapper 1 │    │ Mapper M │
└────┬─────┘    └────┬─────┘    └────┬─────┘
     │               │               │
     │  emit (k,v)   │  emit (k,v)   │  emit (k,v)
     │               │               │
     ▼               ▼               ▼
  ┌─────────── Partition by hash(key) % R ───────────┐
  │                                                     │
  │  ┌─────────┐  ┌─────────┐       ┌─────────┐       │
  │  │ Bucket 0│  │ Bucket 1│  ...  │ Bucket R│       │
  │  └────┬────┘  └────┬────┘       └────┬────┘       │
  │       │            │                  │             │
  └───────┼────────────┼──────────────────┼─────────────┘
          │            │                  │
          ▼            ▼                  ▼
     ┌─────────┐  ┌─────────┐       ┌─────────┐
     │Reducer 0│  │Reducer 1│  ...  │Reducer R│
     └────┬────┘  └────┬────┘       └────┬────┘
          │            │                  │
          ▼            ▼                  ▼
       Output 0     Output 1          Output R
```

**Map phase:** Input is split into M chunks. Each mapper reads its chunk, applies the user's `map` function, and emits `(key, value)` pairs. The mapper partitions output by `hash(key) % R` into R buckets (one per reducer), writing each bucket to local disk.

**Shuffle phase:** Each reduce worker fetches its assigned partition from every map worker. The framework sorts the fetched data by key and groups all values for the same key together.

**Reduce phase:** For each unique key, the reduce worker calls the user's `reduce(key, values)` function and writes the output.

### Combiner: Local Aggregation

A **combiner** runs on the map side before data is shuffled. It's a mini-reducer that merges intermediate values locally, cutting network traffic:

```
Without combiner:
  Mapper emits: ("the", 1), ("the", 1), ("the", 1), ("the", 1)
  → 4 pairs shipped over network to reducer

With combiner (sum):
  Combiner merges locally: ("the", 4)
  → 1 pair shipped over network to reducer
```

The combiner must be commutative and associative because the framework may apply it zero, one, or multiple times. Sum and max satisfy this; average does not.

### Fault Tolerance

If a map worker fails, the master re-executes its tasks on another worker. This works because map tasks are **deterministic** — given the same input, they produce the same output. If a reduce worker fails, its tasks are re-executed on another worker.

If a map worker produces output and then fails *after* some reducers have already fetched its output, the framework does not re-run those map tasks for the reducers that already have the data. Only reducers that hadn't yet fetched must re-fetch from the new worker.

### MapReduce Limitations

MapReduce materializes every intermediate result to disk. This creates an I/O bottleneck:

```
Iteration 1: Read → Map → Write to Disk → Shuffle → Read → Reduce → Write to Disk
Iteration 2: Read from Disk → Map → Write to Disk → Shuffle → Read → Reduce → Write to Disk
...
Iteration N: same pattern
```

For a machine learning algorithm that needs 100 iterations, that's 100 full read-shuffle-write cycles. Each cycle writes intermediate data to disk just to read it back in the next iteration. This is why MapReduce is poor for iterative algorithms — k-means, gradient descent, PageRank all need to revisit the same data many times.

MapReduce also doesn't support interactive queries. Each job is a batch process with significant startup latency.

### Spark: Resilient Distributed Datasets

Spark's core abstraction is the **RDD** (Resilient Distributed Dataset): an immutable, partitioned collection with lineage tracking. Instead of materializing every intermediate step to disk, Spark chains transformations in memory and only writes to disk when an action triggers execution.

```
RDD lineage graph:

textFile("logs/")
  .flatMap(line → words)         ← transformation (lazy)
  .map(word → (word, 1))         ← transformation (lazy)
  .reduceByKey((a, b) → a + b)   ← transformation (lazy, wide dependency)
  .collect()                      ← action (triggers execution)
```

**Transformations** (lazy): `map`, `filter`, `flatMap`, `join`, `groupByKey`, `reduceByKey`. They define a lineage graph but don't execute until an action is called.

**Actions** (eager): `collect`, `count`, `saveAsTextFile`, `reduce`. They trigger execution of the entire lineage.

**Lineage-based recovery:** When a partition is lost (node failure), Spark replays the transformations from the nearest checkpoint or the beginning of the lineage. No data replication needed — the recipe for reconstruction is the lineage itself.

### Narrow vs Wide Dependencies

```
Narrow dependency (1:1):
  map, filter, flatMap — each output partition depends on exactly one input partition

  Partition 0 ──→ Partition 0'
  Partition 1 ──→ Partition 1'
  Partition 2 ──→ Partition 2'

  → No shuffle needed. Pipelined within a stage.

Wide dependency (N:M):
  groupByKey, join, reduceByKey, sort — output partitions depend on multiple input partitions

  Partition 0 ──┐
  Partition 1 ──┼──→ Partition 0'  (all partitions contribute data)
  Partition 2 ──┘
  
  → Shuffle required. Forms a stage boundary.
```

A **stage** in Spark is a group of transformations that can be pipelined without a shuffle. The DAGScheduler identifies stage boundaries at wide dependencies and creates a DAG of stages.

### The Dataflow / Beam Model

Google's Dataflow (open-sourced as Apache Beam) unifies batch and stream processing under one model. The key insight: batch is just streaming with a single infinite window.

**Windowing** assigns data to finite chunks based on event time:

| Window Type | Description |
|-------------|-------------|
| Fixed | Constant-size, non-overlapping (e.g., every 5 minutes) |
| Sliding | Constant-size, overlapping (e.g., 5-minute windows sliding by 1 minute) |
| Session | Grouped by gap in activity (e.g., 30-minute inactivity timeout) |

**Watermarks** track progress of event time. A watermark at time T says "I believe all events with event time ≤ T have arrived." Late data arrives after the watermark passes — **allowed lateness** controls how long to wait before discarding.

**Triggers** determine when results are emitted for a window. You can trigger on:
- Event time watermark crossing window end
- Processing time delays
- Element counts
- Combinations (early firings + final firing at watermark + late firings)

```
Event time:  0    5    10   15   20   25   30
              ├────┼────┼────┼────┼────┼────┤
Window [0,10): ████████████░░░░░░░░░░░░░░░░░  data arrives late
              ↑ early trigger    ↑ watermark    ↑ late data
              (partial result)   (final result)  (allowed lateness)
```

### Comparison

| | MapReduce | Spark | Dataflow/Beam |
|---|---|---|---|
| Model | Batch only | Batch + iterative | Batch + streaming |
| Intermediate data | Disk (materialized) | Memory (lineage) | Memory (windowed) |
| Iterative workloads | Poor (full I/O per iteration) | Excellent (in-memory caching) | Good (state + timers) |
| Fault tolerance | Re-execute failed tasks | Lineage recompute | Checkpoint + replay |
| Latency | Minutes | Seconds | Milliseconds to seconds |
| Streaming | No | Micro-batches (Structured Streaming) | True streaming |
| Programming model | Map + Reduce | Transformations + Actions | ParDo + GroupByKey + Window |

## Build It

We'll build a simplified MapReduce framework in Python that supports mapper, combiner, shuffle, reducer, and fault tolerance. Then we'll build an equivalent in Go.

### Step 1: Core Types and Word Count Mapper

The simplest useful MapReduce job is word count. We define the type signatures and a mapper that tokenizes text and emits `(word, 1)` pairs.

### Step 2: Partitioning and Shuffle

The mapper emits intermediate pairs. We partition them by `hash(key) % R` to assign each key to a reduce bucket. The shuffle groups all `(key, [values])` together.

### Step 3: Reducer and Full Pipeline

The reducer applies a function to each key's values. We wire up the full pipeline: input → split → map → partition → combiner → shuffle → reduce → output.

### Step 4: Combiner — Local Aggregation

We add a combiner that runs on the map side. For word count, the combiner sums values for the same key before shuffling, reducing network traffic.

### Step 5: Fault Tolerance — Mapper Re-execution

We simulate a mapper failure and show that re-executing the same mapper on another worker produces the same result because map tasks are deterministic.

### Step 6: Inverted Index

A second MapReduce job that builds an inverted index: map emits `(word, document_id)`, reduce collects unique document IDs per word.

### Step 7: Go Implementation

A simplified Go MapReduce with master, worker, and word count.

See `code/main.py` for the complete Python implementation and `code/main.go` for the Go implementation.

## Use It

**Apache Hadoop** is the open-source implementation of Google's MapReduce. In Hadoop, you implement `Mapper` and `Reducer` Java interfaces (or use the Streaming API for other languages). The `Job` class configures M mappers, R reducers, input paths, combiner class, and output paths.

Key differences from our toy framework:
- Hadoop's shuffle uses a **sort-merge** approach: each map task writes sorted, partitioned output to local disk. Reduce tasks fetch and merge-sort their partitions. Our framework loads everything into memory.
- Hadoop has a sophisticated **speculative execution** mechanism: if a task is a straggler, the framework launches a duplicate on another node and takes the result of whichever finishes first.
- Hadoop's **InputFormat** and **OutputFormat** classes abstract over data sources (files, databases, S3). Our framework just splits text files by lines.

**Apache Spark** replaces Hadoop MapReduce for most iterative workloads. Compare our MapReduce to Spark's word count:

```python
# Our MapReduce framework
result = mr.run(word_count_map, word_count_reduce, input_data)

# Spark
sc.textFile("hdfs://logs/") \
  .flatMap(lambda line: line.split()) \
  .map(lambda word: (word, 1)) \
  .reduceByKey(lambda a, b: a + b) \
  .collect()
```

Spark's `reduceByKey` is both a transformation and a combiner — it applies the reduce function locally on the map side (combiner behavior) before shuffling. This is the default optimization our combiner provides manually.

**Apache Beam** (Google Dataflow's open-source SDK) unifies batch and streaming:

```python
# Beam word count (batch)
lines = p | "Read" >> ReadFromText("input.txt")
words = lines | "Split" >> FlatMap(lambda line: line.split())
counts = words | "Pair" >> Map(lambda w: (w, 1)) \
              | "Group" >> GroupByKey() \
              | "Sum" >> Map(lambda kv: (kv[0], sum(kv[1])))
```

For streaming, Beam adds windowing and triggers that our MapReduce framework doesn't address — MapReduce has no concept of event time or late data.

## Read the Source

- [Hadoop MapTaskImpl.java](https://github.com/apache/hadoop/blob/trunk/hadoop-mapreduce-project/hadoop-mapreduce-client/hadoop-mapreduce-client-app/src/main/java/org/apache/hadoop/mapreduce/v2/app/mapTaskImpl.java) — The map task implementation. Look at how it spills sorted output to disk and reports progress.
- [Spark RDD.scala](https://github.com/apache/spark/blob/master/core/src/main/scala/org/apache/spark/rdd/RDD.scala) — The core RDD trait. Look at `compute()` for how each partition's lineage is evaluated, and `getDependencies()` for narrow vs wide.
- [Apache Beam Model](https://beam.apache.org/documentation/programming-guide/) — The Beam programming guide, particularly the sections on windowing, triggers, and watermarks.

## Ship It

The reusable artifact for this lesson is in `outputs/`:

- **A self-contained MapReduce framework** in Python and Go that you can reuse to implement any MapReduce job (word count, inverted index, etc.) with combiner support and fault tolerance simulation.

## Exercises

1. **Easy** — Implement a MapReduce job that finds the maximum value per key (instead of summing). The mapper emits `(key, value)` and the reducer takes the max of all values for that key. Verify it works with our framework.
2. **Medium** — Implement a MapReduce job that computes average word length per starting letter (e.g., words starting with 'a' have average length 4.2). Explain why a simple combiner that averages won't work (averages aren't associative), and implement a correct combiner that emits `(letter, (total_length, count))`.
3. **Hard** — Extend the Python framework to support multi-step pipelines (the output of one MapReduce job is the input to the next). Use this to compute PageRank: one MapReduce step distributes rank, another aggregates incoming rank, and the pipeline iterates until convergence. Discuss how Spark's in-memory RDD caching eliminates the disk I/O between iterations.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| MapReduce | "Distributed computing" | A programming model where the user writes map and reduce functions; the framework handles input splitting, partitioning, shuffle, fault tolerance, and task scheduling. |
| Shuffle | "Data movement" | The phase where reduce workers fetch their partition of intermediate data from map workers. This is the dominant I/O cost in MapReduce. |
| Combiner | "Map-side reduce" | A local aggregation function run on the mapper before shuffle. Must be commutative and associative (e.g., sum, max, but not average). Reduces network traffic. |
| RDD | "A distributed array" | Resilient Distributed Dataset — Spark's core abstraction. Immutable, partitioned collection with lineage tracking. Transformations build the lineage graph; actions trigger execution. |
| Narrow dependency | "No shuffle" | Each output partition depends on at most one input partition. Map, filter, flatMap are narrow. They can be pipelined within a single stage. |
| Wide dependency | "Requires shuffle" | Output partitions depend on multiple input partitions. groupByKey, join, sort are wide. They form stage boundaries and require a shuffle. |
| Watermark | "How far we've processed" | A timestamp indicating that all events with event time ≤ the watermark are believed to have arrived. Late data arrives after the watermark passes. |
| Windowing | "Chunking by time" | Assigning infinite-stream data to finite groups based on event time. Fixed windows (every 5 min), sliding windows (5 min sliding by 1 min), session windows (gap-based). |

## Further Reading

- [MapReduce: Simplified Data Processing on Large Clusters](https://dl.acm.org/doi/10.1145/1327452.1327492) — Dean & Ghemawat, 2004. The original paper. Read for the programming model and fault tolerance design.
- [Resilient Distributed Datasets: A Fault-Tolerant Abstraction for In-Memory Cluster Computing](https://dl.acm.org/doi/10.1145/2228360.2228371) — Zaharia et al., 2012. The Spark paper. Read for the RDD abstraction, lineage-based recovery, and why in-memory matters for iterative algorithms.
- [The Dataflow Model: A Practical Approach to Balancing Correctness, Latency, and Cost in Massive-Scale, Unbounded, Out-of-Order Data Processing](https://dl.acm.org/doi/10.1145/2820743) — Akidau et al., 2015. The Dataflow/Beam paper. Read for windowing, watermarks, triggers, and the unified batch/streaming model.
- [Apache Beam Programming Guide](https://beam.apache.org/documentation/programming-guide/) — Practical guide to the Beam SDK with windowing, triggers, and stateful processing examples.