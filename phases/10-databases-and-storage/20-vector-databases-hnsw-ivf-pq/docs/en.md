# Vector Databases — HNSW, IVF, PQ

> When exact search costs `O(N·D)`, you learn to approximate.

**Type:** Build
**Languages:** Python, Rust
**Prerequisites:** Phase 10 Lessons 01–07 (indexing, B-trees), basic probability & clustering
**Time:** ~75 minutes

## Learning Objectives

- Explain why exact k-NN does not scale to millions of high-dimensional vectors
- Implement IVF with k-means clustering and tunable nprobe recall
- Implement PQ vector compression and asymmetric distance computation
- Build HNSW from scratch in Rust and measure recall against brute force
- Compare the tradeoffs: HNSW (high recall, moderate memory) vs IVF+PQ (low memory, large-scale)

## The Problem

You have 10 million 768-dimensional embeddings from a sentence transformer. A user types a query, you embed it, and you need the 10 most similar vectors in under 100 ms.

Exact k-NN would compute 10 million L2 distances — each requiring 768 multiplications and additions — then sort. On modern hardware that is roughly 30–50 seconds per query. No user will wait that long.

Databases solve this with indexes. B-trees and hash maps work for exact key lookups, but they cannot answer "find me the closest vectors" because **vectors have no natural total order**. You need a different kind of index — one built for approximate nearest neighbor (ANN) search. This lesson builds three of them from scratch.

## The Concept

### Vector Embeddings

Any data — text, image, audio — can be converted to a fixed-dimension float vector by a neural network. The key property: vectors that are close in Euclidean (L2) or cosine distance represent semantically similar data.

```
"cat" → [0.23, -0.45, 0.12, ..., 0.89]  (768-dim)
"dog" → [0.25, -0.42, 0.15, ..., 0.85]  (close to "cat")
"car" → [0.91, 0.33, -0.72, ..., -0.11]  (far from both)
```

### ANN vs Exact k-NN

Exact k-NN: `O(N·D)` per query — compute all distances, take k smallest.

ANN: preprocess data so queries touch only a fraction of vectors. Trade a controlled amount of recall for orders-of-magnitude speedup.

### Three Approaches

| Technique | Core idea | What you trade |
|-----------|-----------|----------------|
| IVF | Partition vectors into clusters; search only the nearest clusters | Missed recall if query falls near cluster boundary |
| PQ | Compress vectors into small codes; compute approximate distances from codebooks | Distance estimates are inexact |
| HNSW | Build a multi-layer proximity graph; traverse greedily from top | Memory (stores full vectors + adjacency lists) |

IVF, PQ, and IVF+PQ let you navigate the accuracy–speed–memory triangle. HNSW sits at the high-recall end.

## Build It

### Step 1: IVF — Inverted File Index (Python)

IVF is like a library: you organise books (vectors) onto shelves (centroids). When a query arrives, you walk to the nearest shelves and check only those books.

**Training:**
1. Run k-means to partition `N` vectors into `nlist` clusters.
2. Build `nlist` inverted lists — each list holds the IDs of vectors assigned to that centroid.

**Search (`nprobe`):**
1. Find the `nprobe` closest centroids to the query.
2. Scan all vectors in those `nprobe` inverted lists.
3. Return the top-k by true L2 distance.

The key parameter: `nprobe` — number of clusters to probe. More probes = higher recall, slower search.

```python
import numpy as np
from sklearn.cluster import KMeans

class IVF:
    def __init__(self, nlist: int = 100):
        self.nlist = nlist
        self.centroids = None
        self.inverted_lists = None
        self.vectors = None

    def train(self, vectors: np.ndarray):
        self.vectors = vectors
        kmeans = KMeans(n_clusters=self.nlist, random_state=42, n_init="auto")
        kmeans.fit(vectors)
        self.centroids = kmeans.cluster_centers_
        labels = kmeans.labels_
        self.inverted_lists = [[] for _ in range(self.nlist)]
        for i, label in enumerate(labels):
            self.inverted_lists[label].append(i)

    def search(self, query: np.ndarray, k: int, nprobe: int = 10):
        dists = np.linalg.norm(self.centroids - query, axis=1)
        nearest = np.argsort(dists)[:nprobe]
        candidates = {}
        for cid in nearest:
            for vid in self.inverted_lists[cid]:
                candidates[vid] = np.linalg.norm(self.vectors[vid] - query)
        sorted_candidates = sorted(candidates.items(), key=lambda x: x[1])
        return sorted_candidates[:k]
```

**Why this works:** k-means learns a Voronoi partition of the vector space. Queries near a centroid's region are likely to have their nearest neighbors in the same or adjacent cells. Probing `nprobe` cells covers the boundary region.

### Step 2: PQ — Product Quantization (Python)

PQ shrinks each vector to a handful of bytes. Instead of storing 128 floats (512 bytes), store 8 bytes of quantized codes.

**Training:**
1. Split each D-dim vector into M subvectors (each `D/M` dims).
2. For each of the M subvector spaces, run k-means with `Ks = 256` centroids (8-bit codebook).
3. `M × Ks` codebooks stored.

**Encoding:**
- For each subvector, replace it with the nearest centroid's index (0–255).
- A 128-dim vector with M=8 compresses to 8 bytes.

**Search — Asymmetric Distance Computation (ADC):**
- Query stays uncompressed (no information loss).
- For query subvector `q_m`, precompute distance to all `Ks` codewords in codebook `m`.
- Look up the distance for each encoded subvector index — one table lookup per subvector.
- Sum across M subvectors.

```python
class PQ:
    def __init__(self, M: int = 8, Ks: int = 256):
        self.M = M
        self.Ks = Ks
        self.codebooks = None  # M × Ks × (D/M)
        self.codes = None      # N × M (uint8)

    def train(self, vectors: np.ndarray):
        N, D = vectors.shape
        assert D % self.M == 0
        sub_dim = D // self.M
        self.codebooks = []
        code_list = []
        for m in range(self.M):
            sub = vectors[:, m * sub_dim : (m + 1) * sub_dim]
            kmeans = KMeans(n_clusters=self.Ks, random_state=42, n_init="auto")
            kmeans.fit(sub)
            self.codebooks.append(kmeans.cluster_centers_)
            code_list.append(kmeans.predict(sub))
        self.codes = np.column_stack(code_list).astype(np.uint8)

    def encode(self, vectors: np.ndarray) -> np.ndarray:
        N, D = vectors.shape
        sub_dim = D // self.M
        codes = np.zeros((N, self.M), dtype=np.uint8)
        for m in range(self.M):
            sub = vectors[:, m * sub_dim : (m + 1) * sub_dim]
            dists = np.linalg.norm(
                sub[:, np.newaxis, :] - self.codebooks[m][np.newaxis, :, :], axis=2
            )
            codes[:, m] = np.argmin(dists, axis=1)
        return codes

    def adc(self, query: np.ndarray, codes: np.ndarray) -> np.ndarray:
        D = query.shape[0]
        sub_dim = D // self.M
        dists = np.zeros(codes.shape[0])
        for m in range(self.M):
            q_sub = query[m * sub_dim : (m + 1) * sub_dim]
            lut = np.linalg.norm(self.codebooks[m] - q_sub, axis=1)
            dists += lut[codes[:, m]]
        return dists
```

**Compression ratio:** A 128-dim float32 vector = 512 bytes. With M=8, Ks=256: 8 bytes = **64× compression**. The distance computation becomes M table lookups + M×Ks precomputed distances per query.

### Step 3: IVF + PQ — Put Them Together

IVF narrows the search space; PQ cheapens each distance computation.

```python
class IVF_PQ:
    def __init__(self, nlist: int = 100, M: int = 8, Ks: int = 256):
        self.ivf = IVF(nlist=nlist)
        self.pq = PQ(M=M, Ks=Ks)
        self.codes = None

    def train(self, vectors: np.ndarray):
        self.ivf.train(vectors)
        self.pq.train(vectors)
        self.codes = self.pq.encode(vectors)

    def search(self, query: np.ndarray, k: int, nprobe: int = 10):
        dists = np.linalg.norm(self.ivf.centroids - query, axis=1)
        nearest = np.argsort(dists)[:nprobe]
        cand_ids = []
        for cid in nearest:
            cand_ids.extend(self.ivf.inverted_lists[cid])
        if not cand_ids:
            return []
        cand_codes = self.codes[cand_ids]
        pq_dists = self.pq.adc(query, cand_codes)
        idx = np.argsort(pq_dists)[:k]
        return [(cand_ids[i], pq_dists[i]) for i in idx]
```

**Pipeline:**
1. Query → find nearest `nprobe` centroids → get candidate vector IDs.
2. For each candidate, retrieve its PQ-compressed code.
3. Compute approximate distance via ADC.
4. Return top-k by approximate distance.

### Step 4: HNSW — Hierarchical Navigable Small World (Rust)

HNSW builds a multi-layer graph. The top layer is sparse (few nodes, long-range connections). Lower layers are denser. Search starts at the top and descends — like using a city map at different zoom levels.

**Key data structure:**

```
Layer 2:  [A]---[B]              (sparse, long jumps)
              |
Layer 1:  [A]---[B]---[C]        (medium density)
              |     |
Layer 0:  [A]---[B]---[C]---[D]  (all nodes, local connections)
```

**Insert algorithm:**
1. Generate a random level for the new node (exponential decay: `floor(-ln(unif(0,1)) * mL)` where `mL = 1/ln(M)`).
2. Starting from the entry point, greedily traverse each layer above the new node's level (ef=1) to find the best entry for the next layer down.
3. At the new node's level and below, search with `ef = efConstruction`, select `M` nearest neighbors, connect bidirectionally.
4. If a neighbor exceeds `M_max` connections, prune to the `M_max` closest.

**Search algorithm:**
1. Start at the entry point on the top layer. Greedily descend (ef=1) to layer 0.
2. At layer 0, search with dynamic candidate list of size `ef`. Maintain a min-heap of candidates (closest first) and a max-heap of results (farthest first for easy eviction).
3. While the closest candidate is closer than the farthest result (or result set < ef), expand: pop the closest candidate, visit its neighbors, add new candidates.
4. Return the top-k from the result set.

```rust
// See code/main.rs for the full implementation.

struct HNSW {
    vectors: Vec<Vec<f32>>,
    layers: Vec<Vec<Vec<usize>>>,  // layers[l][node_id] = neighbor ids
    entry_point: Option<usize>,
    max_layer: usize,
    m: usize,        // connections per node
    mmax: usize,     // max connections before pruning
    ef_con: usize,   // ef_construction
    ef: usize,       // search width
    ml: f32,         // 1 / ln(M)
}
```

**Level generation** uses exponential decay so most nodes live only at layer 0. With M=16 and mL≈0.36, roughly 94% of nodes are at layer 0, 5.7% at layer 1, 0.3% at layer 2 — a natural hierarchy.

**Key parameters:**
- `M` — out-degree per node. Higher M = better connectivity, more memory.
- `efConstruction` — search width during insert. Higher = better index quality, slower build.
- `ef` — search width during query. Higher = better recall, slower search.

## Use It

**FAISS** (Meta) — the production standard. `IndexIVFFlat` (IVF), `IndexIVFPQ` (IVF+PQ), `IndexHNSWFlat` (HNSW). FAISS adds SIMD-optimized distance kernels, multi-GPU support, and product-quantization refinements (OPQ for rotation-optimized quantization).

**pgvector** — PostgreSQL extension for vector search. Uses IVFFlat (with `lists` and `probes` parameters) and HNSW (with `m` and `ef_construction`). Supports both L2 and cosine distance.

Compare: your hand-built IVF handles 10K vectors; FAISS handles **billion-scale** indexes. The difference is SIMD, memory-mapped inverted lists, and multi-level quantization (SQ8, PQ, etc.). Your HNSW builds a single-threaded graph; production HNSW (in FAISS, hnswlib) uses lock-free concurrent inserts.

## Read the Source

- [FAISS `IndexIVF.cpp`](https://github.com/facebookresearch/faiss/blob/main/faiss/IndexIVF.cpp) — inverted file with multi-probe search
- [FAISS `ProductQuantizer.cpp`](https://github.com/facebookresearch/faiss/blob/main/faiss/impl/ProductQuantizer.cpp) — PQ training, encoding, and ADC with SIMD
- [pgvector `hnsw.c`](https://github.com/pgvector/pgvector/blob/main/src/hnsw.c) — HNSW in a Postgres extension (look at the insert and search loops)
- [hnswlib](https://github.com/nmslib/hnswlib) — Reference HNSW implementation in C++, the paper's author's code

## Ship It

This lesson's reusable artifacts are the Python (`code/main.py`) and Rust (`code/main.rs`) index implementations. Rerun the benchmarks as you vary parameters:

```bash
python3 code/main.py
cd code && cargo run --release
```

## Exercises

1. **Easy** — Run the Python benchmark with `nprobe=1, 5, 10, 50, 100` and plot recall vs. query time. At what nprobe does IVF match exact recall?
2. **Medium** — Add cosine distance to the HNSW implementation. Benchmark L2 vs. cosine on normalized vs. unnormalized vectors.
3. **Hard** — Implement multi-probe IVF: instead of probing the `nprobe` closest centroids, probe centroids whose Voronoi cells are adjacent to the query's closest centroid (fewer probes, same recall).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| nlist | "Number of clusters in IVF" | Centroids learned by k-means; each gets one inverted list |
| nprobe | "Clusters to search during query" | How many Voronoi cells you peek into — linear in search cost |
| M (PQ) | "Number of subvectors" | Split D-dim vector into M equal parts; each gets its own codebook |
| ADC | "Asymmetric distance computation" | Query uses full precision; stored vectors use quantized codes; no reconstruction |
| efConstruction | "Search width while building HNSW" | More candidates considered during insert = better graph quality |
| mL | "Level generation factor" | `1/ln(M)`; controls how quickly layers thin out |

## Further Reading

- [Efficient and robust approximate nearest neighbor search using Product Quantization (Jégou et al., 2011)](https://hal.inria.fr/inria-00514462v2/document) — The PQ paper
- [Efficient and robust approximate nearest neighbor search using Hierarchical Navigable Small World graphs (Malkov & Yashunin, 2018)](https://arxiv.org/abs/1603.09320) — The HNSW paper
- [FAISS: A Library for Efficient Similarity Search (Johnson, Douze, Jégou, 2019)](https://engineering.fb.com/2017/03/29/data-infrastructure/faiss-a-library-for-efficient-similarity-search/) — Engineering overview of billion-scale ANN
